//! Generic Theta-sketch core, parameterised over a per-key summary.
//!
//! [`ThetaCore<S>`] is the shared engine behind the plain [`ThetaSketch`](super::ThetaSketch)
//! (with the empty [`NoSummary`]) and, in a later wave, the Tuple Sketch (with a real
//! summary such as [`SumDoubles`]). It maintains the retained-hash set, the sampling
//! threshold `theta`, and the capacity logic; each summary rides along with its hash and is
//! folded via [`Summary::combine`] whenever a key is seen again or kept by a set operation.
//!
//! The core is **hash-agnostic**: callers hash items themselves and pass the resulting
//! `u64`. This keeps hashing policy (seed, algorithm) with the concrete sketch, so wrapping
//! an existing sketch over this core does not change its estimates.

use crate::error::{Result, SketchError};
use std::collections::HashMap;

/// A per-key summary attached to each retained hash in a Theta/Tuple sketch.
///
/// The summary for a key is folded with [`combine`](Summary::combine) when the same key is
/// seen again, and when a set operation (union/intersection) keeps a key that is present in
/// both inputs. Implementations should make `combine` commutative and associative so set
/// operations are order-independent.
pub trait Summary: Clone {
    /// Folds `other` into `self`.
    fn combine(&mut self, other: &Self);
}

/// The empty summary — turns a Tuple sketch back into a plain Theta set.
///
/// Zero-sized, so `ThetaCore<NoSummary>` costs essentially the same as a set of hashes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoSummary;

impl Summary for NoSummary {
    #[inline]
    fn combine(&mut self, _other: &Self) {}
}

/// An "array of doubles" summary: a fixed-width vector of `f64` combined element-wise by
/// sum. The Apache DataSketches workhorse for reach/frequency and A/B-test aggregation.
///
/// If two summaries differ in length, the shorter is treated as zero-padded.
#[derive(Clone, Debug, PartialEq)]
pub struct SumDoubles(pub Vec<f64>);

impl SumDoubles {
    /// A summary of `width` zeros.
    pub fn zeros(width: usize) -> Self {
        Self(vec![0.0; width])
    }

    /// The underlying values.
    #[inline]
    pub fn values(&self) -> &[f64] {
        &self.0
    }
}

impl Summary for SumDoubles {
    fn combine(&mut self, other: &Self) {
        let common = self.0.len().min(other.0.len());
        for i in 0..common {
            self.0[i] += other.0[i];
        }
        if other.0.len() > self.0.len() {
            self.0.extend_from_slice(&other.0[self.0.len()..]);
        }
    }
}

/// Generic Theta-sketch core parameterised over a per-key [`Summary`].
///
/// Holds retained `(hash, summary)` pairs below the sampling threshold `theta`, reducing
/// `theta` to enforce the capacity `k = 2^lg_k`. Supports the Theta set operations, with
/// summaries folded per the [`Summary`] contract.
#[derive(Clone, Debug)]
pub struct ThetaCore<S: Summary> {
    lg_k: u8,
    k: usize,
    entries: HashMap<u64, S>,
    theta: u64,
}

impl<S: Summary> ThetaCore<S> {
    /// Minimum / maximum `lg_k`, matching Apache DataSketches.
    pub const MIN_LG_K: u8 = 4;
    /// Maximum `lg_k`.
    pub const MAX_LG_K: u8 = 26;

    /// Creates an empty core with capacity `k = 2^lg_k`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `lg_k` is outside `[4, 26]`.
    pub fn new(lg_k: u8) -> Result<Self> {
        if !(Self::MIN_LG_K..=Self::MAX_LG_K).contains(&lg_k) {
            return Err(SketchError::InvalidParameter {
                param: "lg_k".to_string(),
                value: lg_k.to_string(),
                constraint: format!("must be in range [{}, {}]", Self::MIN_LG_K, Self::MAX_LG_K),
            });
        }
        let k = 1_usize << lg_k;
        Ok(Self {
            lg_k,
            k,
            entries: HashMap::with_capacity(k),
            theta: u64::MAX,
        })
    }

    /// Reconstructs a core directly from its parts (used by deserialization).
    ///
    /// Unlike replaying [`update`](Self::update), this sets `theta` to the exact
    /// stored value rather than re-deriving it from capacity — required so a
    /// sketch that was at capacity round-trips to the same estimate.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `lg_k` is out of range.
    pub(crate) fn from_raw_parts(lg_k: u8, theta: u64, entries: HashMap<u64, S>) -> Result<Self> {
        if !(Self::MIN_LG_K..=Self::MAX_LG_K).contains(&lg_k) {
            return Err(SketchError::InvalidParameter {
                param: "lg_k".to_string(),
                value: lg_k.to_string(),
                constraint: format!("must be in range [{}, {}]", Self::MIN_LG_K, Self::MAX_LG_K),
            });
        }
        Ok(Self {
            lg_k,
            k: 1_usize << lg_k,
            entries,
            theta,
        })
    }

    /// Records a pre-hashed key with its summary.
    ///
    /// Hashes at or above `theta` are ignored (sampling). If the key is already retained,
    /// the summaries are folded; otherwise it is inserted, reducing `theta` if capacity is
    /// exceeded.
    pub fn update(&mut self, hash: u64, summary: S) {
        if hash >= self.theta {
            return;
        }
        match self.entries.get_mut(&hash) {
            Some(existing) => existing.combine(&summary),
            None => {
                self.entries.insert(hash, summary);
                if self.entries.len() > self.k {
                    self.rebuild_with_lower_theta();
                }
            }
        }
    }

    /// Estimated cardinality: retained count scaled by the sampling rate.
    pub fn estimate(&self) -> f64 {
        if self.entries.is_empty() {
            return 0.0;
        }
        let count = self.entries.len() as f64;
        if self.theta == u64::MAX {
            count
        } else {
            count * (u64::MAX as f64 / self.theta as f64)
        }
    }

    /// Whether no entries are retained.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Number of retained entries.
    #[inline]
    pub fn num_retained(&self) -> usize {
        self.entries.len()
    }

    /// Current sampling threshold (`u64::MAX` means exact mode).
    #[inline]
    pub fn theta(&self) -> u64 {
        self.theta
    }

    /// Nominal capacity `k`.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.k
    }

    /// `log2(k)`.
    #[inline]
    pub fn lg_k(&self) -> u8 {
        self.lg_k
    }

    /// Read-only view of the retained entries (for set ops in concrete sketches).
    #[inline]
    pub fn entries(&self) -> &HashMap<u64, S> {
        &self.entries
    }

    /// Errors if two cores have different capacities (`lg_k`).
    pub fn check_compatible(&self, other: &Self) -> Result<()> {
        if self.lg_k != other.lg_k {
            return Err(SketchError::IncompatibleSketches {
                reason: format!("lg_k mismatch: {} vs {}", self.lg_k, other.lg_k),
            });
        }
        Ok(())
    }

    /// Union `A ∪ B`: every key from either side below the combined `theta`; summaries of
    /// shared keys are folded.
    pub fn union(&self, other: &Self) -> Result<Self> {
        self.check_compatible(other)?;
        let new_theta = self.theta.min(other.theta);
        let mut entries: HashMap<u64, S> = HashMap::with_capacity(self.k);

        for (&hash, summary) in &self.entries {
            if hash < new_theta {
                entries.insert(hash, summary.clone());
            }
        }
        for (&hash, summary) in &other.entries {
            if hash < new_theta {
                entries
                    .entry(hash)
                    .and_modify(|existing| existing.combine(summary))
                    .or_insert_with(|| summary.clone());
            }
        }

        Ok(Self {
            lg_k: self.lg_k,
            k: self.k,
            entries,
            theta: new_theta,
        })
    }

    /// Intersection `A ∩ B`: keys present in both below the combined `theta`; summaries
    /// folded.
    pub fn intersect(&self, other: &Self) -> Result<Self> {
        self.check_compatible(other)?;
        let new_theta = self.theta.min(other.theta);
        let mut entries: HashMap<u64, S> = HashMap::new();

        for (&hash, summary) in &self.entries {
            if hash < new_theta {
                if let Some(other_summary) = other.entries.get(&hash) {
                    let mut merged = summary.clone();
                    merged.combine(other_summary);
                    entries.insert(hash, merged);
                }
            }
        }

        Ok(Self {
            lg_k: self.lg_k,
            k: self.k,
            entries,
            theta: new_theta,
        })
    }

    /// Difference `A − B`: keys in `self` not in `other`, below the combined `theta`;
    /// summaries are taken from `self`.
    pub fn difference(&self, other: &Self) -> Result<Self> {
        self.check_compatible(other)?;
        let new_theta = self.theta.min(other.theta);
        let mut entries: HashMap<u64, S> = HashMap::new();

        for (&hash, summary) in &self.entries {
            if hash < new_theta && !other.entries.contains_key(&hash) {
                entries.insert(hash, summary.clone());
            }
        }

        Ok(Self {
            lg_k: self.lg_k,
            k: self.k,
            entries,
            theta: new_theta,
        })
    }

    /// Reduces `theta` to the `k`-th smallest retained hash, dropping entries above it, to
    /// restore the capacity invariant after an insert.
    fn rebuild_with_lower_theta(&mut self) {
        let mut sorted: Vec<u64> = self.entries.keys().copied().collect();
        sorted.sort_unstable();
        if sorted.len() > self.k {
            let new_theta = sorted[self.k - 1];
            self.entries.retain(|&hash, _| hash < new_theta);
            self.theta = new_theta;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_lg_k_rejected() {
        assert!(ThetaCore::<NoSummary>::new(3).is_err());
        assert!(ThetaCore::<NoSummary>::new(27).is_err());
        assert!(ThetaCore::<NoSummary>::new(12).is_ok());
    }

    #[test]
    fn exact_mode_counts_distinct() {
        let mut core = ThetaCore::<NoSummary>::new(12).unwrap();
        for h in 0..100u64 {
            core.update(h, NoSummary);
        }
        // Distinct hashes, all below theta=MAX, exact mode.
        assert_eq!(core.estimate(), 100.0);
        assert_eq!(core.num_retained(), 100);
        // Re-inserting is idempotent.
        core.update(50, NoSummary);
        assert_eq!(core.num_retained(), 100);
    }

    #[test]
    fn capacity_triggers_sampling() {
        let mut core = ThetaCore::<NoSummary>::new(4).unwrap(); // k = 16
        for h in 0..1000u64 {
            core.update(h, NoSummary);
        }
        assert!(core.num_retained() <= 16, "got {}", core.num_retained());
        assert!(core.theta() < u64::MAX, "sampling should be active");
    }

    #[test]
    fn set_ops_on_nosummary() {
        let mut a = ThetaCore::<NoSummary>::new(12).unwrap();
        let mut b = ThetaCore::<NoSummary>::new(12).unwrap();
        for h in 0..100u64 {
            a.update(h, NoSummary);
        }
        for h in 50..150u64 {
            b.update(h, NoSummary);
        }
        assert_eq!(a.union(&b).unwrap().num_retained(), 150);
        assert_eq!(a.intersect(&b).unwrap().num_retained(), 50);
        assert_eq!(a.difference(&b).unwrap().num_retained(), 50);
    }

    #[test]
    fn incompatible_lg_k_errors() {
        let a = ThetaCore::<NoSummary>::new(12).unwrap();
        let b = ThetaCore::<NoSummary>::new(10).unwrap();
        assert!(a.union(&b).is_err());
    }

    #[test]
    fn sum_doubles_combine_folds_per_key() {
        let mut a = ThetaCore::<SumDoubles>::new(12).unwrap();
        let mut b = ThetaCore::<SumDoubles>::new(12).unwrap();
        // Key 1 seen in both; key 2 only in a; key 3 only in b.
        a.update(1, SumDoubles(vec![1.0, 2.0]));
        a.update(2, SumDoubles(vec![5.0, 5.0]));
        b.update(1, SumDoubles(vec![10.0, 20.0]));
        b.update(3, SumDoubles(vec![7.0, 7.0]));

        let u = a.union(&b).unwrap();
        assert_eq!(u.num_retained(), 3);
        assert_eq!(u.entries().get(&1).unwrap().values(), &[11.0, 22.0]);
        assert_eq!(u.entries().get(&2).unwrap().values(), &[5.0, 5.0]);

        let i = a.intersect(&b).unwrap();
        assert_eq!(i.num_retained(), 1);
        assert_eq!(i.entries().get(&1).unwrap().values(), &[11.0, 22.0]);
    }

    #[test]
    fn sum_doubles_same_key_accumulates() {
        let mut core = ThetaCore::<SumDoubles>::new(12).unwrap();
        core.update(42, SumDoubles(vec![1.0]));
        core.update(42, SumDoubles(vec![2.5]));
        assert_eq!(core.num_retained(), 1);
        assert_eq!(core.entries().get(&42).unwrap().values(), &[3.5]);
    }
}

/// Capability-trait adoptions (see `crate::common::capabilities`).
///
/// No `Update` impl: the inherent `update` takes a pre-computed `u64` hash plus
/// a per-item `summary`, so it does not match the single hashable-item signature
/// of `Update<T>`. Cardinality estimation is delegated below.
mod capability_impls {
    use super::*;
    use crate::common::capabilities::CardinalityEstimate;

    impl<S: Summary> CardinalityEstimate for ThetaCore<S> {
        fn estimate_cardinality(&self) -> f64 {
            self.estimate()
        }
    }
}
