//! Distinct Sampling — a bounded, uniform sample of the *distinct* items of a stream.
//!
//! Distinct Sampling (Gibbons, "Distinct Sampling for Highly-Accurate Answers to Distinct Values
//! Queries and Event Reports", VLDB 2001) keeps a capacity-bounded sample drawn uniformly from the
//! set of distinct items — regardless of how often each item repeats — so it answers distinct-count
//! and *subset* distinct-count queries (how many distinct items satisfy a predicate) in small space.
//!
//! Each item `x` is assigned a geometric *level* `ℓ(x)` = the number of trailing zeros of its hash
//! (so `Pr[ℓ(x) ≥ L] = 2^{−L}`). The sketch keeps a current level `L` and all distinct items seen
//! with `ℓ(x) ≥ L`. When the sample would exceed its capacity, `L` is raised and every now-too-shallow
//! item is evicted — exactly the coarsening that keeps the sample bounded while every surviving item
//! is retained independently with probability `2^{−L}`. The distinct count is therefore estimated by
//! `|sample| · 2^L`, and the distinct count of any predicate by `|{x ∈ sample : pred(x)}| · 2^L` —
//! both **unbiased**.
//!
//! Unlike [`ReservoirSampling`](crate::sampling::ReservoirSampling) (a sample of stream *positions*,
//! biased toward frequent items), this is a sample of distinct *values*; unlike a pure cardinality
//! sketch it retains the items themselves, enabling arbitrary subset queries after the fact.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use std::collections::HashSet;
use std::hash::Hash;

/// Hashing seed for level assignment.
const LEVEL_SEED: u64 = 0xD157_1C70_5A11_71E5;

/// A uniform sample of the distinct items of a stream, of at most `capacity` items.
///
/// # Example
/// ```
/// use sketch_oxide::sampling::DistinctSampling;
///
/// let mut ds = DistinctSampling::new(4096).unwrap();
/// // 50_000 distinct items, each inserted several times (duplicates don't change the answer).
/// for i in 0..50_000u64 {
///     for _ in 0..3 { ds.insert(i.to_le_bytes()); }
/// }
/// let est = ds.estimate_distinct();
/// assert!((est - 50_000.0).abs() < 0.15 * 50_000.0, "distinct estimate {est}");
/// ```
#[derive(Debug, Clone)]
pub struct DistinctSampling<T: Hash + Eq + Clone> {
    capacity: usize,
    /// Current sampling level `L`; items with `ℓ(x) ≥ L` are kept.
    level: u32,
    sample: HashSet<T>,
}

impl<T: Hash + Eq + Clone + AsRef<[u8]>> DistinctSampling<T> {
    /// Creates a sampler holding at most `capacity` distinct items.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `capacity` is 0.
    pub fn new(capacity: usize) -> Result<Self> {
        if capacity == 0 {
            return Err(SketchError::InvalidParameter {
                param: "capacity".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            capacity,
            level: 0,
            sample: HashSet::new(),
        })
    }

    /// Level `ℓ(x)` of an item: the number of trailing zeros of its hash.
    #[inline]
    fn level_of(item: &[u8]) -> u32 {
        let h = xxhash(item, LEVEL_SEED);
        // h == 0 (probability 2^-64) maps to the maximum level.
        if h == 0 {
            64
        } else {
            h.trailing_zeros()
        }
    }

    /// Records one occurrence of `item`.
    pub fn insert(&mut self, item: T) {
        if Self::level_of(item.as_ref()) < self.level {
            return; // too shallow for the current level
        }
        self.sample.insert(item);
        // Coarsen until back within capacity.
        while self.sample.len() > self.capacity {
            self.level += 1;
            let lvl = self.level;
            self.sample.retain(|x| Self::level_of(x.as_ref()) >= lvl);
        }
    }

    /// Unbiased estimate of the number of distinct items seen: `|sample| · 2^L`.
    pub fn estimate_distinct(&self) -> f64 {
        self.sample.len() as f64 * 2f64.powi(self.level as i32)
    }

    /// Unbiased estimate of the number of distinct items satisfying `pred`:
    /// `|{x ∈ sample : pred(x)}| · 2^L`.
    pub fn estimate_distinct_where<F: Fn(&T) -> bool>(&self, pred: F) -> f64 {
        let matching = self.sample.iter().filter(|x| pred(*x)).count();
        matching as f64 * 2f64.powi(self.level as i32)
    }

    /// The sampled distinct items (a uniform `2^{−L}` sub-sample of the true distinct set).
    pub fn sample(&self) -> &HashSet<T> {
        &self.sample
    }

    /// Number of items currently in the sample.
    #[inline]
    pub fn len(&self) -> usize {
        self.sample.len()
    }

    /// Whether the sample is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.sample.is_empty()
    }

    /// Current sampling level `L`.
    #[inline]
    pub fn level(&self) -> u32 {
        self.level
    }

    /// Maximum number of sampled items.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_capacity() {
        assert!(DistinctSampling::<Vec<u8>>::new(0).is_err());
        assert!(DistinctSampling::<Vec<u8>>::new(1024).is_ok());
    }

    #[test]
    fn exact_when_under_capacity() {
        // Fewer distinct items than capacity ⇒ level stays 0 ⇒ exact distinct count.
        let mut ds = DistinctSampling::new(10_000).unwrap();
        for i in 0..2000u64 {
            ds.insert(i.to_le_bytes().to_vec());
        }
        assert_eq!(ds.level(), 0);
        assert_eq!(ds.estimate_distinct(), 2000.0);
    }

    #[test]
    fn duplicates_do_not_change_estimate() {
        let mut ds = DistinctSampling::new(4096).unwrap();
        for i in 0..30_000u64 {
            for _ in 0..5 {
                ds.insert(i.to_le_bytes().to_vec());
            }
        }
        let est = ds.estimate_distinct();
        assert!((est - 30_000.0).abs() < 0.15 * 30_000.0, "estimate {est}");
        assert!(ds.len() <= 4096);
    }

    #[test]
    fn large_distinct_count_within_error() {
        let mut ds = DistinctSampling::new(8192).unwrap();
        for i in 0..200_000u64 {
            ds.insert(i.to_le_bytes().to_vec());
        }
        let est = ds.estimate_distinct();
        assert!((est - 200_000.0).abs() < 0.10 * 200_000.0, "estimate {est}");
        assert!(ds.level() > 0);
    }

    #[test]
    fn subset_query_is_unbiased() {
        // Half the distinct items are "even"; the subset distinct-count estimate should reflect that.
        let mut ds = DistinctSampling::new(8192).unwrap();
        for i in 0..100_000u64 {
            ds.insert(i.to_le_bytes().to_vec());
        }
        let evens = ds.estimate_distinct_where(|x| {
            let mut b = [0u8; 8];
            b.copy_from_slice(x);
            u64::from_le_bytes(b) % 2 == 0
        });
        // True even-distinct count is 50_000.
        assert!(
            (evens - 50_000.0).abs() < 0.12 * 100_000.0,
            "even estimate {evens}"
        );
    }

    #[test]
    fn empty_is_zero() {
        let ds = DistinctSampling::<Vec<u8>>::new(128).unwrap();
        assert!(ds.is_empty());
        assert_eq!(ds.estimate_distinct(), 0.0);
    }
}
