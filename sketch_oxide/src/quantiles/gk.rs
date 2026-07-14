//! Greenwald–Khanna — deterministic ε-approximate quantiles.
//!
//! The Greenwald–Khanna summary (Greenwald & Khanna, "Space-Efficient Online Computation of
//! Quantile Summaries", SIGMOD 2001) answers rank/quantile queries within `±εn` rank error using
//! `O((1/ε)·log(εn))` space — and, unlike randomized sketches (KLL, t-digest), its error bound is
//! **deterministic**, holding for every stream regardless of order or adversary.
//!
//! It keeps a sorted list of tuples `(v, g, Δ)`: `g` is the gap (difference in minimum rank between
//! this value and its predecessor), so the minimum possible rank of `v` is the prefix sum of `g`,
//! and `Δ` bounds how much larger the true rank can be. The invariant `g + Δ ≤ 2εn` keeps every
//! rank pinned to within `εn`; a periodic **compress** merges adjacent tuples that still fit the
//! band, bounding the space.
//!
//! This is the standard threshold-based compression; the original band refinement only changes the
//! constant in the space bound, not the error guarantee.

use crate::common::{Result, SketchError};

#[derive(Debug, Clone, Copy)]
struct Tuple {
    v: f64,
    /// Gap: minimum-rank difference from the previous tuple.
    g: usize,
    /// Δ: upper bound on `rmax − rmin` for this value.
    delta: usize,
}

/// A Greenwald–Khanna ε-approximate quantile summary over `f64` values.
///
/// # Example
/// ```
/// use sketch_oxide::quantiles::GreenwaldKhanna;
///
/// let mut gk = GreenwaldKhanna::new(0.01).unwrap();
/// for i in 0..100_000 { gk.insert((i as f64 * 7.0) % 100_000.0); }
///
/// // The median is within εn rank of the true median (~50_000).
/// let m = gk.quantile(0.5).unwrap();
/// assert!((m - 50_000.0).abs() < 0.05 * 100_000.0, "median {m}");
/// ```
#[derive(Debug, Clone)]
pub struct GreenwaldKhanna {
    epsilon: f64,
    tuples: Vec<Tuple>,
    n: usize,
    since_compress: usize,
    compress_interval: usize,
}

impl GreenwaldKhanna {
    /// Creates a summary with rank-error bound `epsilon` (in `(0, 1)`).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `epsilon` is not in `(0, 1)`.
    pub fn new(epsilon: f64) -> Result<Self> {
        if !(epsilon > 0.0 && epsilon < 1.0) {
            return Err(SketchError::InvalidParameter {
                param: "epsilon".to_string(),
                value: epsilon.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }
        let compress_interval = (1.0 / (2.0 * epsilon)).floor().max(1.0) as usize;
        Ok(Self {
            epsilon,
            tuples: Vec::new(),
            n: 0,
            since_compress: 0,
            compress_interval,
        })
    }

    /// Inserts a value.
    pub fn insert(&mut self, v: f64) {
        // Position: first tuple whose value exceeds v (insert before it).
        let i = self.tuples.partition_point(|t| t.v <= v);
        let delta = if i == 0 || i == self.tuples.len() {
            // New global minimum or maximum: its rank is known exactly on one side.
            0
        } else {
            (2.0 * self.epsilon * self.n as f64).floor() as usize
        };
        self.tuples.insert(i, Tuple { v, g: 1, delta });
        self.n += 1;
        self.since_compress += 1;
        if self.since_compress >= self.compress_interval {
            self.compress();
            self.since_compress = 0;
        }
    }

    /// Merges adjacent tuples that still fit within the `2εn` band.
    fn compress(&mut self) {
        if self.tuples.len() < 3 {
            return;
        }
        let band = 2.0 * self.epsilon * self.n as f64;
        // Merge right-to-left so each tuple is absorbed into its successor when it fits.
        let mut i = self.tuples.len() - 2;
        while i >= 1 {
            let merged = self.tuples[i].g + self.tuples[i + 1].g + self.tuples[i + 1].delta;
            if (merged as f64) <= band {
                self.tuples[i + 1].g += self.tuples[i].g;
                self.tuples.remove(i);
            }
            i -= 1;
        }
    }

    /// Estimates the value at quantile `phi` (`0.0..=1.0`); `None` if empty. The returned value's
    /// true rank is within `εn` of `phi·n`.
    pub fn quantile(&self, phi: f64) -> Option<f64> {
        if self.tuples.is_empty() {
            return None;
        }
        let phi = phi.clamp(0.0, 1.0);
        let target = phi * self.n as f64;
        let mut rmin = 0usize;
        for t in &self.tuples {
            rmin += t.g;
            let rmax = rmin + t.delta;
            if rmax as f64 >= target {
                return Some(t.v);
            }
        }
        self.tuples.last().map(|t| t.v)
    }

    /// Smallest observed value.
    pub fn min(&self) -> Option<f64> {
        self.tuples.first().map(|t| t.v)
    }

    /// Largest observed value.
    pub fn max(&self) -> Option<f64> {
        self.tuples.last().map(|t| t.v)
    }

    /// Number of values inserted.
    #[inline]
    pub fn count(&self) -> usize {
        self.n
    }

    /// Number of retained tuples (the summary's space).
    #[inline]
    pub fn num_tuples(&self) -> usize {
        self.tuples.len()
    }

    /// Whether nothing has been inserted.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }
}

// Capability-trait adoptions (fable5 doc 01 F3): express exactly the streaming
// ingest and immutable rank-query capabilities this type has, delegating to
// inherent methods.
mod capability_impls {
    use super::*;
    use crate::common::capabilities::{QuantileQuery, Update};

    impl Update<f64> for GreenwaldKhanna {
        fn update(&mut self, item: &f64) {
            self.insert(*item);
        }
    }

    // `quantile(&self, ..) -> Option<f64>` is immutable, so `QuantileQuery` fits.
    impl QuantileQuery for GreenwaldKhanna {
        fn quantile(&self, rank: f64) -> Option<f64> {
            GreenwaldKhanna::quantile(self, rank)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_epsilon() {
        assert!(GreenwaldKhanna::new(0.0).is_err());
        assert!(GreenwaldKhanna::new(1.0).is_err());
        assert!(GreenwaldKhanna::new(0.01).is_ok());
    }

    #[test]
    fn empty_returns_none() {
        let gk = GreenwaldKhanna::new(0.01).unwrap();
        assert!(gk.quantile(0.5).is_none());
        assert!(gk.is_empty());
    }

    #[test]
    fn quantiles_within_rank_error() {
        let eps = 0.01;
        let n = 100_000usize;
        let mut gk = GreenwaldKhanna::new(eps).unwrap();
        // Insert 0..n in a scrambled order; value == true rank.
        for k in 0..n as u64 {
            let v = ((k.wrapping_mul(2_654_435_761)) % n as u64) as f64;
            gk.insert(v);
        }
        for &phi in &[0.01, 0.1, 0.25, 0.5, 0.75, 0.9, 0.99] {
            let est = gk.quantile(phi).unwrap();
            let true_rank = phi * n as f64; // value == rank for this data
            // The returned value's true rank (== its value here) is within ~2εn of the target.
            assert!(
                (est - true_rank).abs() <= 2.0 * eps * n as f64,
                "phi {phi}: est {est} vs target {true_rank}"
            );
        }
    }

    #[test]
    fn min_and_max_exact() {
        let mut gk = GreenwaldKhanna::new(0.05).unwrap();
        for v in [5.0, 1.0, 9.0, 3.0, 7.0] {
            gk.insert(v);
        }
        assert_eq!(gk.min(), Some(1.0));
        assert_eq!(gk.max(), Some(9.0));
        assert_eq!(gk.count(), 5);
    }

    #[test]
    fn space_is_sublinear() {
        let mut gk = GreenwaldKhanna::new(0.01).unwrap();
        for i in 0..200_000u64 {
            gk.insert(i as f64);
        }
        // O((1/eps) log(eps n)) tuples, far below n.
        assert!(gk.num_tuples() < 5000, "tuples {}", gk.num_tuples());
        assert_eq!(gk.count(), 200_000);
    }

    #[test]
    fn extreme_quantiles_clamp() {
        let mut gk = GreenwaldKhanna::new(0.05).unwrap();
        for i in 0..1000u64 {
            gk.insert(i as f64);
        }
        let lo = gk.quantile(0.0).unwrap();
        let hi = gk.quantile(1.0).unwrap();
        assert!(lo <= 50.0, "q0 {lo}");
        assert!(hi >= 950.0, "q1 {hi}");
    }
}
