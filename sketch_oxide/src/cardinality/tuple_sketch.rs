//! Tuple Sketch — Theta sketch with a per-key summary.
//!
//! A Tuple Sketch (Apache DataSketches) is a Theta sketch where every retained key also
//! carries a [`Summary`] value. It is the workhorse of reach/frequency analysis and A/B
//! testing: count distinct users *and* aggregate a per-user quantity (impressions, spend,
//! conversions) over the same sampled population, with full set operations (union /
//! intersection / difference) that fold the summaries correctly.
//!
//! It wraps the generic [`ThetaCore<S>`](super::ThetaCore) engine; hashing lives here so a
//! `TupleSketch<NoSummary>` behaves exactly like [`ThetaSketch`](super::ThetaSketch).

use crate::cardinality::theta_core::{Summary, ThetaCore};
use crate::cardinality::SumDoubles;
use crate::error::{Result, SketchError};
use std::hash::{Hash, Hasher};

/// A Theta sketch carrying a per-key [`Summary`] of type `S`.
///
/// # Example — reach and frequency
/// ```
/// use sketch_oxide::cardinality::{TupleSketch, SumDoubles};
///
/// // Per user, accumulate [impressions, spend].
/// let mut s = TupleSketch::<SumDoubles>::new(12).unwrap();
/// s.update(&"user_a", SumDoubles(vec![3.0, 1.50]));
/// s.update(&"user_a", SumDoubles(vec![2.0, 0.50])); // same user folds in
/// s.update(&"user_b", SumDoubles(vec![1.0, 4.00]));
///
/// assert!((s.estimate() - 2.0).abs() < 0.001); // 2 distinct users
/// let totals = s.estimated_column_sums();        // ~[6 impressions, 6.00 spend]
/// assert!((totals[0] - 6.0).abs() < 0.1 && (totals[1] - 6.0).abs() < 0.1);
/// ```
#[derive(Debug, Clone)]
pub struct TupleSketch<S: Summary> {
    core: ThetaCore<S>,
    seed: u64,
}

impl<S: Summary> TupleSketch<S> {
    /// Default hash seed (matches `ThetaSketch` / Apache DataSketches).
    const DEFAULT_SEED: u64 = 9001;

    /// Creates a Tuple Sketch with `lg_k` log2-capacity (4..=26).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `lg_k` is out of range.
    pub fn new(lg_k: u8) -> Result<Self> {
        Ok(Self {
            core: ThetaCore::new(lg_k)?,
            seed: Self::DEFAULT_SEED,
        })
    }

    /// Creates a Tuple Sketch with a custom hash seed (must match for set operations).
    pub fn with_seed(lg_k: u8, seed: u64) -> Result<Self> {
        Ok(Self {
            core: ThetaCore::new(lg_k)?,
            seed,
        })
    }

    fn hash_item<T: Hash>(&self, item: &T) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        self.seed.hash(&mut hasher);
        item.hash(&mut hasher);
        hasher.finish()
    }

    /// Adds `item` with `summary`. If the key is already retained, the summaries are folded
    /// via [`Summary::combine`].
    pub fn update<T: Hash>(&mut self, item: &T, summary: S) {
        let hash = self.hash_item(item);
        self.core.update(hash, summary);
    }

    /// Estimated number of distinct keys.
    pub fn estimate(&self) -> f64 {
        self.core.estimate()
    }

    /// Whether no keys are retained.
    pub fn is_empty(&self) -> bool {
        self.core.is_empty()
    }

    /// Number of retained entries.
    pub fn num_retained(&self) -> usize {
        self.core.num_retained()
    }

    /// Iterates the retained `(summary)` values (the sampled population).
    pub fn summaries(&self) -> impl Iterator<Item = &S> {
        self.core.entries().values()
    }

    /// Theta scaling factor `u64::MAX / theta` — multiply a sum over the retained sample by
    /// this to estimate the same sum over the full distinct population.
    fn scale(&self) -> f64 {
        let theta = self.core.theta();
        if theta == u64::MAX {
            1.0
        } else {
            u64::MAX as f64 / theta as f64
        }
    }

    fn check_seed(&self, other: &Self) -> Result<()> {
        if self.seed != other.seed {
            return Err(SketchError::IncompatibleSketches {
                reason: format!("seed mismatch: {} vs {}", self.seed, other.seed),
            });
        }
        Ok(())
    }

    /// Union: keys from either sketch, summaries of shared keys folded.
    pub fn union(&self, other: &Self) -> Result<Self> {
        self.check_seed(other)?;
        Ok(Self {
            core: self.core.union(&other.core)?,
            seed: self.seed,
        })
    }

    /// Intersection: keys in both, summaries folded.
    pub fn intersect(&self, other: &Self) -> Result<Self> {
        self.check_seed(other)?;
        Ok(Self {
            core: self.core.intersect(&other.core)?,
            seed: self.seed,
        })
    }

    /// Difference `A − B`: keys in `self` not in `other`, summaries from `self`.
    pub fn difference(&self, other: &Self) -> Result<Self> {
        self.check_seed(other)?;
        Ok(Self {
            core: self.core.difference(&other.core)?,
            seed: self.seed,
        })
    }
}

impl TupleSketch<SumDoubles> {
    /// Estimates the per-column sum over the **full distinct population** from the retained
    /// sample (sum over retained summaries, scaled by `u64::MAX / theta`).
    pub fn estimated_column_sums(&self) -> Vec<f64> {
        let scale = self.scale();
        let mut sums: Vec<f64> = Vec::new();
        for summary in self.summaries() {
            for (i, &v) in summary.values().iter().enumerate() {
                if i >= sums.len() {
                    sums.resize(i + 1, 0.0);
                }
                sums[i] += v;
            }
        }
        for s in &mut sums {
            *s *= scale;
        }
        sums
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cardinality::NoSummary;

    #[test]
    fn counts_distinct_keys() {
        let mut s = TupleSketch::<NoSummary>::new(12).unwrap();
        for i in 0..1000u64 {
            s.update(&i, NoSummary);
        }
        assert!((s.estimate() - 1000.0).abs() < 30.0);
    }

    #[test]
    fn same_key_folds_summary() {
        let mut s = TupleSketch::<SumDoubles>::new(12).unwrap();
        s.update(&"u", SumDoubles(vec![1.0, 2.0]));
        s.update(&"u", SumDoubles(vec![3.0, 4.0]));
        assert!((s.estimate() - 1.0).abs() < 0.001);
        let sums = s.estimated_column_sums();
        assert!((sums[0] - 4.0).abs() < 0.001 && (sums[1] - 6.0).abs() < 0.001);
    }

    #[test]
    fn column_sums_estimate_population() {
        // 2000 distinct users, each with value 1 => population sum ~2000 even after sampling.
        let mut s = TupleSketch::<SumDoubles>::new(10).unwrap(); // small k forces sampling
        for i in 0..2000u64 {
            s.update(&i, SumDoubles(vec![1.0]));
        }
        assert!(s.num_retained() <= s.estimate() as usize + 1);
        let total = s.estimated_column_sums()[0];
        assert!(
            (total - 2000.0).abs() < 0.2 * 2000.0,
            "estimated sum {total}"
        );
    }

    #[test]
    fn union_folds_shared_keys() {
        let mut a = TupleSketch::<SumDoubles>::new(12).unwrap();
        let mut b = TupleSketch::<SumDoubles>::new(12).unwrap();
        a.update(&"shared", SumDoubles(vec![1.0]));
        a.update(&"a_only", SumDoubles(vec![1.0]));
        b.update(&"shared", SumDoubles(vec![10.0]));
        b.update(&"b_only", SumDoubles(vec![1.0]));

        let u = a.union(&b).unwrap();
        assert!((u.estimate() - 3.0).abs() < 0.01); // shared, a_only, b_only
                                                    // shared key's summary folded to 11.
        let total: f64 = u.estimated_column_sums()[0];
        assert!((total - 13.0).abs() < 0.01, "union column sum {total}");
    }

    #[test]
    fn intersect_and_difference() {
        let mut a = TupleSketch::<NoSummary>::new(12).unwrap();
        let mut b = TupleSketch::<NoSummary>::new(12).unwrap();
        for i in 0..100u64 {
            a.update(&i, NoSummary);
        }
        for i in 50..150u64 {
            b.update(&i, NoSummary);
        }
        assert!((a.intersect(&b).unwrap().estimate() - 50.0).abs() < 5.0);
        assert!((a.difference(&b).unwrap().estimate() - 50.0).abs() < 5.0);
    }

    #[test]
    fn seed_mismatch_errors() {
        let a = TupleSketch::<NoSummary>::with_seed(12, 1).unwrap();
        let b = TupleSketch::<NoSummary>::with_seed(12, 2).unwrap();
        assert!(a.union(&b).is_err());
    }
}
