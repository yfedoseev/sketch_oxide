//! Recordinality — distinct counting by counting hash "records" (Helmi, Lumbroso, Martínez, Viola,
//! 2012).
//!
//! Recordinality views the stream as a random permutation and estimates the number of distinct
//! elements from how often a new *k-record* occurs. It keeps the `k` smallest distinct hash values
//! seen so far and a counter `R` of how many elements have ever been inserted into that set (`R = k`
//! after the first `k` distinct elements, plus one for every later element whose hash beats the
//! current `k`-th smallest). The estimator
//!
//! ```text
//! D̂ = k · (1 + 1/k)^(R − k + 1) − 1
//! ```
//!
//! is **unbiased** for the number of distinct elements. Because the retained `k` hashes are also a
//! uniform sample of the distinct elements, Recordinality doubles as a distinct-element sampler.
//!
//! Unlike [`KmvSketch`](crate::cardinality::KmvSketch) (which estimates from the *value* of the
//! `k`-th smallest hash), Recordinality estimates from the *number of updates* to the bottom-`k` set —
//! a genuinely different estimator from the same bottom-`k` state.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use std::collections::BTreeSet;

/// Default hashing seed.
const REC_SEED: u64 = 0x5EC0_4D17_A117_7E50;

/// A Recordinality distinct-count estimator keeping the `k` smallest hashes.
///
/// # Example
/// ```
/// use sketch_oxide::cardinality::Recordinality;
///
/// let mut rec = Recordinality::new(4096).unwrap();
/// for i in 0..100_000u64 {
///     for _ in 0..3 { rec.insert(&i.to_le_bytes()); } // each distinct value seen 3×
/// }
/// let est = rec.estimate();
/// assert!((est - 100_000.0).abs() < 0.25 * 100_000.0, "estimate {est}");
/// ```
#[derive(Debug, Clone)]
pub struct Recordinality {
    k: usize,
    seed: u64,
    /// The `k` smallest distinct hashes seen.
    smallest: BTreeSet<u64>,
    /// Number of insertions into `smallest` (the record count `R`).
    records: u64,
}

impl Recordinality {
    /// Creates an estimator retaining the `k` smallest hashes.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `k == 0`.
    pub fn new(k: usize) -> Result<Self> {
        Self::with_seed(k, REC_SEED)
    }

    /// Creates an estimator with a specific hashing seed (useful for ensembles/averaging).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `k == 0`.
    pub fn with_seed(k: usize, seed: u64) -> Result<Self> {
        if k == 0 {
            return Err(SketchError::InvalidParameter {
                param: "k".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            k,
            seed,
            smallest: BTreeSet::new(),
            records: 0,
        })
    }

    /// Records one occurrence of `item`.
    pub fn insert(&mut self, item: &[u8]) {
        let h = xxhash(item, self.seed);
        if self.smallest.contains(&h) {
            return; // duplicate of a retained element
        }
        if self.smallest.len() < self.k {
            self.smallest.insert(h);
            self.records += 1;
            return;
        }
        // Set is full: a new k-record only if h beats the current maximum of the bottom-k.
        let max = *self
            .smallest
            .iter()
            .next_back()
            .expect("non-empty when full");
        if h < max {
            self.smallest.remove(&max);
            self.smallest.insert(h);
            self.records += 1;
        }
    }

    /// Unbiased estimate of the number of distinct items: `k·(1 + 1/k)^(R − k + 1) − 1`, or the exact
    /// count while fewer than `k` distinct items have been seen.
    pub fn estimate(&self) -> f64 {
        if self.smallest.len() < self.k {
            return self.smallest.len() as f64; // exact: we have seen every distinct item
        }
        let kf = self.k as f64;
        let exponent = (self.records - self.k as u64 + 1) as f64;
        kf * (1.0 + 1.0 / kf).powf(exponent) - 1.0
    }

    /// The retained sample of distinct hashes (a uniform sample of the distinct elements).
    pub fn sample(&self) -> &BTreeSet<u64> {
        &self.smallest
    }

    /// Number of `k`-records observed (`R`).
    #[inline]
    pub fn records(&self) -> u64 {
        self.records
    }

    /// Sample size `k`.
    #[inline]
    pub fn k(&self) -> usize {
        self.k
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insert_distinct(rec: &mut Recordinality, n: u64) {
        for i in 0..n {
            rec.insert(&i.to_le_bytes());
        }
    }

    #[test]
    fn rejects_zero_k() {
        assert!(Recordinality::new(0).is_err());
        assert!(Recordinality::new(64).is_ok());
    }

    #[test]
    fn exact_below_k() {
        let mut rec = Recordinality::new(1000).unwrap();
        insert_distinct(&mut rec, 300);
        assert_eq!(rec.estimate(), 300.0);
    }

    #[test]
    fn duplicates_do_not_change_estimate() {
        // A stream with duplicates must yield the exact same state as the de-duplicated stream:
        // once a hash is evicted it can never re-enter (the bottom-k maximum only shrinks).
        let mut with_dups = Recordinality::new(512).unwrap();
        for i in 0..20_000u64 {
            for _ in 0..4 {
                with_dups.insert(&i.to_le_bytes());
            }
        }
        let mut no_dups = Recordinality::new(512).unwrap();
        insert_distinct(&mut no_dups, 20_000);
        assert_eq!(with_dups.records(), no_dups.records());
        assert_eq!(with_dups.estimate(), no_dups.estimate());
    }

    #[test]
    fn unbiased_over_many_seeds() {
        // Recordinality has higher variance than KMV, but averaging many seeds converges to truth.
        let truth = 50_000.0;
        let runs = 64u64;
        let mut sum = 0.0;
        for seed in 0..runs {
            let mut rec = Recordinality::with_seed(512, seed.wrapping_mul(0x9E37_79B9)).unwrap();
            insert_distinct(&mut rec, 50_000);
            sum += rec.estimate();
        }
        let mean = sum / runs as f64;
        assert!((mean - truth).abs() < 0.07 * truth, "mean {mean}");
    }

    #[test]
    fn empty_is_zero() {
        let rec = Recordinality::new(128).unwrap();
        assert_eq!(rec.estimate(), 0.0);
        assert_eq!(rec.records(), 0);
    }
}
