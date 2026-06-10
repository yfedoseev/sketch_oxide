//! Strata Estimator — estimates set-difference size before reconciliation.
//!
//! A fixed-rate [`Iblt`](super::Iblt) only decodes reliably when it is sized for the actual
//! number of differences. The Strata Estimator (Eppstein et al., "What's the Difference?
//! Efficient Set Reconciliation Without Prior Context", SIGCOMM 2011) supplies that number:
//! exchange two compact estimators and recover an estimate of `|A △ B|`, then size the IBLT
//! accordingly.
//!
//! # How it works
//!
//! Each key is assigned to a *stratum* by the number of trailing zeros of its hash, so it
//! lands in stratum `i` with probability `2^-(i+1)`. Each stratum is a small IBLT. To
//! estimate the difference, the two parties subtract their corresponding strata and decode
//! from the deepest stratum (fewest elements, easiest to decode) toward the shallowest. The
//! first stratum that fails to decode marks the resolution limit: the differences counted in
//! the deeper, successfully-decoded strata are scaled back up by the sampling probability to
//! estimate the total.

use crate::common::hash::xxhash;
use crate::common::{Reconcilable, Result, SketchError};
use crate::reconciliation::Iblt;

/// Hash seed for stratum assignment. Independent of the IBLT's internal hashing.
const STRATA_SEED: u64 = 0x5354_5241_5441_5f53; // "STRATA_S"

/// Number of cells in each stratum's IBLT. ~80 gives reliable decoding of the small number
/// of differences that reach a stratum (Eppstein et al.).
const STRATUM_CELLS: usize = 80;

/// Byte width of each IBLT cell.
const CELL_SIZE: usize = 16;

/// A Strata Estimator: a stack of small IBLTs, one per stratum, that together estimate the
/// symmetric-difference size between two key sets.
///
/// # Example
/// ```
/// use sketch_oxide::reconciliation::StrataEstimator;
///
/// let mut alice = StrataEstimator::new(32).unwrap();
/// let mut bob = StrataEstimator::new(32).unwrap();
/// for i in 0..10_000u32 {
///     alice.insert(&i.to_le_bytes());
/// }
/// for i in 1_000..11_000u32 {
///     bob.insert(&i.to_le_bytes());
/// }
/// // True symmetric difference is 2000; the estimate is in the right ballpark.
/// let est = alice.estimate_difference(&bob).unwrap();
/// assert!(est > 1000 && est < 4000, "estimate {est}");
/// ```
#[derive(Clone)]
pub struct StrataEstimator {
    strata: Vec<Iblt>,
}

impl StrataEstimator {
    /// Creates an estimator with `num_strata` levels (32 is a good default — it resolves
    /// differences up to billions). More strata cost more space but extend the range.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `num_strata` is 0 or greater than 64.
    pub fn new(num_strata: usize) -> Result<Self> {
        if num_strata == 0 || num_strata > 64 {
            return Err(SketchError::InvalidParameter {
                param: "num_strata".to_string(),
                value: num_strata.to_string(),
                constraint: "must be in [1, 64]".to_string(),
            });
        }
        // Each stratum is a small fixed-rate IBLT. expected_diff is chosen so num_cells lands
        // near STRATUM_CELLS (Iblt uses num_cells = 2 * expected_diff).
        let strata = (0..num_strata)
            .map(|_| Iblt::new(STRATUM_CELLS / 2, CELL_SIZE))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { strata })
    }

    /// Number of strata.
    #[inline]
    pub fn num_strata(&self) -> usize {
        self.strata.len()
    }

    /// The stratum a key belongs to: `min(trailing_zeros(hash), num_strata - 1)`.
    fn stratum_of(&self, key: &[u8]) -> usize {
        let h = xxhash(key, STRATA_SEED);
        (h.trailing_zeros() as usize).min(self.strata.len() - 1)
    }

    /// Inserts a key into its stratum.
    pub fn insert(&mut self, key: &[u8]) {
        let s = self.stratum_of(key);
        // The IBLT stores key/value pairs; the estimator only needs keys, so value == key.
        // insert only errors on internal misuse, which cannot happen here.
        let _ = self.strata[s].insert(key, key);
    }

    /// Estimates `|A △ B|` between this estimator's key set and `other`'s.
    ///
    /// Decodes strata from deepest to shallowest, summing recovered differences until a
    /// stratum fails to decode, then scales that running count up by the sampling
    /// probability of the failing level. If every stratum decodes, the sum is returned
    /// directly.
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the two estimators have different strata
    /// counts.
    pub fn estimate_difference(&self, other: &Self) -> Result<usize> {
        if self.strata.len() != other.strata.len() {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "strata count mismatch: {} vs {}",
                    self.strata.len(),
                    other.strata.len()
                ),
            });
        }

        let mut count = 0usize;
        for i in (0..self.strata.len()).rev() {
            let mut diff = self.strata[i].clone();
            diff.subtract(&other.strata[i])?;
            match diff.decode() {
                Ok(d) => count += d.total_changes(),
                Err(_) => {
                    // Levels > i sample each differing key with total probability 2^-(i+1);
                    // scale the count recovered so far back up to the full difference.
                    return Ok(count.saturating_mul(1usize << (i + 1)));
                }
            }
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn estimator_over(range: std::ops::Range<u32>, num_strata: usize) -> StrataEstimator {
        let mut e = StrataEstimator::new(num_strata).unwrap();
        for i in range {
            e.insert(&i.to_le_bytes());
        }
        e
    }

    #[test]
    fn rejects_bad_strata_count() {
        assert!(StrataEstimator::new(0).is_err());
        assert!(StrataEstimator::new(65).is_err());
        assert!(StrataEstimator::new(32).is_ok());
    }

    #[test]
    fn identical_sets_estimate_zero() {
        let a = estimator_over(0..5_000, 32);
        let b = estimator_over(0..5_000, 32);
        assert_eq!(a.estimate_difference(&b).unwrap(), 0);
    }

    #[test]
    fn estimates_moderate_difference() {
        // |A|=|B|=10000 sharing 9000 => symmetric difference 2000.
        let a = estimator_over(0..10_000, 32);
        let b = estimator_over(1_000..11_000, 32);
        let est = a.estimate_difference(&b).unwrap();
        assert!(est > 1000 && est < 4000, "estimate {est} for true 2000");
    }

    #[test]
    fn estimates_small_difference_well() {
        // Small differences fully decode in the shallow strata => near-exact.
        let a = estimator_over(0..1_000, 32);
        let b = estimator_over(0..1_050, 32);
        let est = a.estimate_difference(&b).unwrap();
        assert!((20..=100).contains(&est), "estimate {est} for true 50");
    }

    #[test]
    fn estimate_is_symmetric() {
        let a = estimator_over(0..8_000, 32);
        let b = estimator_over(2_000..10_000, 32);
        let ab = a.estimate_difference(&b).unwrap();
        let ba = b.estimate_difference(&a).unwrap();
        assert_eq!(ab, ba, "difference estimate must be symmetric");
    }

    #[test]
    fn mismatched_strata_errors() {
        let a = StrataEstimator::new(16).unwrap();
        let b = StrataEstimator::new(32).unwrap();
        assert!(a.estimate_difference(&b).is_err());
    }
}
