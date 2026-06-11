//! CVM — sampling-based distinct counting (Chakraborty, Vinodchandran & Meel, 2022).
//!
//! Every other cardinality estimator in this crate ([`HyperLogLog`](crate::cardinality::HyperLogLog),
//! [`KmvSketch`](crate::cardinality::KmvSketch), …) counts distinct elements by *hashing*. The CVM
//! algorithm — popularized by Donald Knuth's note "The CVM Algorithm for Estimating Distinct Elements
//! in Streams" — instead uses **random sampling** and no hash functions at all, yet gives an unbiased
//! estimate in `O((1/ε²)·log(...))` space.
//!
//! It keeps a buffer of at most `capacity` distinct elements and a retention probability `p` (starting
//! at 1). On each element it drops any existing copy, re-inserts it with probability `p`, and — when
//! the buffer fills — sub-samples the buffer by an independent fair coin per element and halves `p`.
//! Because every element currently in the buffer is there with probability exactly `p`, the count
//! `|buffer| / p` is an **unbiased** estimate of the number of distinct elements seen.
//!
//! The buffer size trades space for accuracy; the relative error is roughly `1/√capacity`. A
//! caller-seedable RNG makes runs reproducible.

use crate::common::{Result, SketchError};
use rand::Rng;
use std::collections::HashSet;
use std::hash::Hash;

/// A sampling-based distinct-count estimator over items of type `T`.
///
/// # Example
/// ```
/// use sketch_oxide::cardinality::CvmSketch;
///
/// let mut cvm = CvmSketch::with_seed(4096, 1).unwrap();
/// // 50_000 distinct items, each seen several times.
/// for i in 0..50_000u64 {
///     for _ in 0..3 { cvm.insert(i); }
/// }
/// let est = cvm.estimate_distinct();
/// assert!((est - 50_000.0).abs() < 0.15 * 50_000.0, "distinct estimate {est}");
/// ```
#[derive(Debug, Clone)]
pub struct CvmSketch<T: Hash + Eq + Clone> {
    capacity: usize,
    /// Retention probability `p` (the buffer holds each surviving element with this probability).
    p: f64,
    buffer: HashSet<T>,
    rng: rand::rngs::SmallRng,
}

impl<T: Hash + Eq + Clone> CvmSketch<T> {
    /// Creates an estimator with a buffer of `capacity` elements, seeded from the OS.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `capacity < 2`.
    pub fn new(capacity: usize) -> Result<Self> {
        use rand::SeedableRng;
        Self::from_rng(capacity, rand::rngs::SmallRng::from_os_rng())
    }

    /// Creates an estimator with a fixed RNG seed (reproducible).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `capacity < 2`.
    pub fn with_seed(capacity: usize, seed: u64) -> Result<Self> {
        use rand::SeedableRng;
        Self::from_rng(capacity, rand::rngs::SmallRng::seed_from_u64(seed))
    }

    fn from_rng(capacity: usize, rng: rand::rngs::SmallRng) -> Result<Self> {
        if capacity < 2 {
            return Err(SketchError::InvalidParameter {
                param: "capacity".to_string(),
                value: capacity.to_string(),
                constraint: "must be >= 2".to_string(),
            });
        }
        Ok(Self {
            capacity,
            p: 1.0,
            buffer: HashSet::new(),
            rng,
        })
    }

    /// Records one occurrence of `item`.
    pub fn insert(&mut self, item: T) {
        // Drop any existing copy, then re-admit with probability p.
        self.buffer.remove(&item);
        if self.rng.random::<f64>() < self.p {
            self.buffer.insert(item);
        }
        // When full, sub-sample by a fair coin per element and halve p.
        while self.buffer.len() >= self.capacity {
            let rng = &mut self.rng;
            self.buffer.retain(|_| rng.random::<bool>());
            self.p *= 0.5;
        }
    }

    /// Unbiased estimate of the number of distinct items seen: `|buffer| / p`.
    pub fn estimate_distinct(&self) -> f64 {
        self.buffer.len() as f64 / self.p
    }

    /// Number of elements currently buffered.
    #[inline]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Whether the buffer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Buffer capacity.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Current retention probability `p`.
    #[inline]
    pub fn retention_probability(&self) -> f64 {
        self.p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_small_capacity() {
        assert!(CvmSketch::<u64>::new(0).is_err());
        assert!(CvmSketch::<u64>::new(1).is_err());
        assert!(CvmSketch::<u64>::new(2).is_ok());
    }

    #[test]
    fn exact_when_under_capacity() {
        // Fewer distinct items than capacity ⇒ p stays 1 ⇒ exact distinct count.
        let mut cvm = CvmSketch::with_seed(10_000, 1).unwrap();
        for i in 0..2000u64 {
            cvm.insert(i);
        }
        assert_eq!(cvm.retention_probability(), 1.0);
        assert_eq!(cvm.estimate_distinct(), 2000.0);
    }

    #[test]
    fn duplicates_do_not_change_estimate() {
        let mut cvm = CvmSketch::with_seed(4096, 7).unwrap();
        for i in 0..30_000u64 {
            for _ in 0..5 {
                cvm.insert(i);
            }
        }
        let est = cvm.estimate_distinct();
        assert!((est - 30_000.0).abs() < 0.15 * 30_000.0, "estimate {est}");
        assert!(cvm.len() < 4096);
    }

    #[test]
    fn large_distinct_count_within_error() {
        let mut cvm = CvmSketch::with_seed(8192, 99).unwrap();
        for i in 0..200_000u64 {
            cvm.insert(i);
        }
        let est = cvm.estimate_distinct();
        assert!((est - 200_000.0).abs() < 0.10 * 200_000.0, "estimate {est}");
        assert!(cvm.retention_probability() < 1.0);
    }

    #[test]
    fn unbiased_over_many_seeds() {
        // Averaging the estimate over independent runs converges to the truth.
        let truth = 40_000.0;
        let runs = 60u64;
        let mut sum = 0.0;
        for seed in 0..runs {
            let mut cvm = CvmSketch::with_seed(1024, seed).unwrap();
            for i in 0..40_000u64 {
                cvm.insert(i);
            }
            sum += cvm.estimate_distinct();
        }
        let mean = sum / runs as f64;
        assert!((mean - truth).abs() < 0.03 * truth, "mean {mean}");
    }

    #[test]
    fn empty_is_zero() {
        let cvm = CvmSketch::<u64>::with_seed(128, 1).unwrap();
        assert!(cvm.is_empty());
        assert_eq!(cvm.estimate_distinct(), 0.0);
    }
}
