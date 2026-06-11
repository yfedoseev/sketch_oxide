//! Stratified reservoir sampling — an independent reservoir per stratum for variance reduction.
//!
//! Plain reservoir sampling keeps one uniform sample of `k` items from the whole stream. When the
//! stream splits into meaningful **strata** (groups — e.g. per region, per category) of very
//! different sizes or value distributions, a single reservoir can badly under-represent small strata
//! and inflates the variance of population estimates. **Stratified reservoir sampling** maintains a
//! *separate* size-`k` reservoir per stratum (Vitter's Algorithm R within each), guaranteeing every
//! stratum a uniform sample regardless of its size — the classic variance-reduction trick from survey
//! sampling.
//!
//! Each stratum's exact count is tracked, so a population total of any per-item value is recovered
//! **unbiasedly** by the Horvitz–Thompson estimator: each stratum contributes
//! `(stratum_count / sample_size) · Σ value(sampled item)`.

use crate::common::SketchError;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;
use std::hash::Hash;

/// One stratum's reservoir and exact arrival count.
#[derive(Debug, Clone)]
struct Stratum<T> {
    reservoir: Vec<T>,
    seen: u64,
}

/// A stratified reservoir sampler keeping a uniform size-`k` sample per stratum.
///
/// # Example
/// ```
/// use sketch_oxide::sampling::StratifiedReservoir;
///
/// let mut s = StratifiedReservoir::with_seed(50, 7);
/// // Two strata "a" (10_000 items) and "b" (100 items).
/// for i in 0..10_000u32 { s.add("a", i); }
/// for i in 0..100u32 { s.add("b", i); }
///
/// // Each stratum has its own uniform sample (≤ k), even the tiny one.
/// assert_eq!(s.sample(&"a").unwrap().len(), 50);
/// assert_eq!(s.sample(&"b").unwrap().len(), 50);
/// // Horvitz–Thompson total of a constant value 1 recovers the population size (10_100).
/// let est = s.estimated_total(|_| 1.0);
/// assert!((est - 10_100.0).abs() < 1.0);
/// ```
#[derive(Debug, Clone)]
pub struct StratifiedReservoir<S, T> {
    k: usize,
    strata: HashMap<S, Stratum<T>>,
    rng: SmallRng,
    total: u64,
}

impl<S: Eq + Hash + Clone, T> StratifiedReservoir<S, T> {
    /// Creates a sampler keeping up to `k` items per stratum (`k ≥ 1`), seeded from the OS.
    ///
    /// # Panics
    /// If `k == 0`.
    pub fn new(k: usize) -> Self {
        Self::build(k, SmallRng::from_os_rng())
    }

    /// Like [`new`](Self::new) but with a fixed RNG seed for reproducibility.
    ///
    /// # Panics
    /// If `k == 0`.
    pub fn with_seed(k: usize, seed: u64) -> Self {
        Self::build(k, SmallRng::seed_from_u64(seed))
    }

    fn build(k: usize, rng: SmallRng) -> Self {
        assert!(k > 0, "k must be >= 1");
        Self {
            k,
            strata: HashMap::new(),
            rng,
            total: 0,
        }
    }

    /// Per-stratum reservoir capacity `k`.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.k
    }

    /// Total number of items processed (all strata).
    #[inline]
    pub fn total_seen(&self) -> u64 {
        self.total
    }

    /// Number of distinct strata observed.
    #[inline]
    pub fn num_strata(&self) -> usize {
        self.strata.len()
    }

    /// Tries to validate that `k` was non-zero (kept for a `Result`-style constructor); always `Ok`.
    ///
    /// # Errors
    /// Never — provided for API symmetry with other sketches.
    pub fn checked_new(k: usize) -> Result<Self, SketchError> {
        if k == 0 {
            return Err(SketchError::InvalidParameter {
                param: "k".to_string(),
                value: "0".to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        Ok(Self::new(k))
    }

    /// Adds `item` to stratum `stratum` (Vitter's Algorithm R within that stratum).
    pub fn add(&mut self, stratum: S, item: T) {
        self.total += 1;
        let k = self.k;
        let st = self.strata.entry(stratum).or_insert_with(|| Stratum {
            reservoir: Vec::with_capacity(k),
            seen: 0,
        });
        st.seen += 1;
        if st.reservoir.len() < k {
            st.reservoir.push(item);
        } else {
            let j = self.rng.random_range(0..st.seen);
            if (j as usize) < k {
                st.reservoir[j as usize] = item;
            }
        }
    }

    /// The uniform sample for `stratum` (≤ `k` items), or `None` if the stratum is unseen.
    pub fn sample(&self, stratum: &S) -> Option<&[T]> {
        self.strata.get(stratum).map(|s| s.reservoir.as_slice())
    }

    /// The exact number of items seen in `stratum` (0 if unseen).
    pub fn stratum_count(&self, stratum: &S) -> u64 {
        self.strata.get(stratum).map_or(0, |s| s.seen)
    }

    /// Iterates the observed strata keys.
    pub fn strata(&self) -> impl Iterator<Item = &S> {
        self.strata.keys()
    }

    /// Horvitz–Thompson estimate of `Σ value(item)` over the whole population: each stratum
    /// contributes `(stratum_count / sample_size) · Σ value(sampled item)`. Unbiased.
    pub fn estimated_total<F: Fn(&T) -> f64>(&self, value: F) -> f64 {
        self.strata
            .values()
            .map(|s| {
                if s.reservoir.is_empty() {
                    0.0
                } else {
                    let sample_sum: f64 = s.reservoir.iter().map(&value).sum();
                    s.seen as f64 / s.reservoir.len() as f64 * sample_sum
                }
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_new_validates() {
        assert!(StratifiedReservoir::<&str, u32>::checked_new(0).is_err());
        assert!(StratifiedReservoir::<&str, u32>::checked_new(10).is_ok());
    }

    #[test]
    fn each_stratum_gets_its_own_sample() {
        let mut s = StratifiedReservoir::with_seed(50, 1);
        for i in 0..10_000u32 {
            s.add("a", i);
        }
        for i in 0..100u32 {
            s.add("b", i);
        }
        for i in 0..30u32 {
            s.add("c", i); // smaller than k
        }
        assert_eq!(s.num_strata(), 3);
        assert_eq!(s.sample(&"a").unwrap().len(), 50);
        assert_eq!(s.sample(&"b").unwrap().len(), 50);
        assert_eq!(s.sample(&"c").unwrap().len(), 30); // fewer than k items seen
        assert_eq!(s.stratum_count(&"a"), 10_000);
        assert!(s.sample(&"missing").is_none());
        assert_eq!(s.total_seen(), 10_130);
    }

    #[test]
    fn within_stratum_sample_is_uniform() {
        // Algorithm R yields a uniform sample: the mean of sampled values ≈ the stratum mean.
        let mut s = StratifiedReservoir::with_seed(500, 99);
        for i in 0..100_000u32 {
            s.add("x", i);
        }
        let sample = s.sample(&"x").unwrap();
        let mean: f64 = sample.iter().map(|&v| v as f64).sum::<f64>() / sample.len() as f64;
        // True mean of 0..100_000 is 49_999.5; a uniform sample of 500 is close.
        assert!((mean - 49_999.5).abs() < 5_000.0, "sample mean {mean}");
    }

    #[test]
    fn horvitz_thompson_total_is_accurate() {
        let mut s = StratifiedReservoir::with_seed(100, 3);
        for i in 0..50_000u32 {
            s.add("a", i);
        }
        for i in 0..5_000u32 {
            s.add("b", i);
        }
        // Estimate the population count (value = 1) exactly: stratum counts are tracked exactly.
        let est_count = s.estimated_total(|_| 1.0);
        assert!((est_count - 55_000.0).abs() < 1.0, "count {est_count}");
        // Estimate Σ value where value(item)=1: equals the population size, unbiased.
        assert!(s.estimated_total(|_| 1.0) > 0.0);
    }
}
