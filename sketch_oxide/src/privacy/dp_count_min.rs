//! Differentially private Count-Min sketch.
//!
//! A standard Count-Min sketch built without noise, plus a one-shot **release** that adds
//! calibrated discrete-Laplace noise to every counter to make the published sketch
//! `ε`-differentially private. Because each record touches exactly one cell per row, a
//! single record changes `depth` counters by 1 — an L1 sensitivity of `depth` — so each
//! counter receives discrete Laplace noise of scale `depth / ε`. Any query on the released
//! sketch (a point estimate is a `min` over rows) is then `ε`-DP by post-processing.
//!
//! Build with [`DpCountMin`], release with [`DpCountMin::privatize`] into a
//! [`PrivateCountMin`].

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use crate::privacy::mechanisms::discrete_laplace;
use rand::Rng;

/// A Count-Min sketch with exact integer counters, ready to be released under DP.
#[derive(Debug, Clone)]
pub struct DpCountMin {
    depth: usize,
    width: usize,
    counters: Vec<i64>,
}

impl DpCountMin {
    /// Creates a `depth × width` Count-Min sketch.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `depth` or `width` is 0.
    pub fn new(depth: usize, width: usize) -> Result<Self> {
        if depth == 0 || width == 0 {
            return Err(SketchError::InvalidParameter {
                param: if depth == 0 { "depth" } else { "width" }.to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            depth,
            width,
            counters: vec![0i64; depth * width],
        })
    }

    #[inline]
    fn column(&self, item: &[u8], r: usize) -> usize {
        (xxhash(item, r as u64) % self.width as u64) as usize
    }

    /// Adds `count` occurrences of `item`.
    pub fn update(&mut self, item: &[u8], count: i64) {
        for r in 0..self.depth {
            let c = self.column(item, r);
            self.counters[r * self.width + c] += count;
        }
    }

    /// Non-private point estimate (min over rows).
    pub fn estimate(&self, item: &[u8]) -> i64 {
        (0..self.depth)
            .map(|r| self.counters[r * self.width + self.column(item, r)])
            .min()
            .unwrap_or(0)
    }

    /// Number of hash rows.
    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Counters per row.
    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }

    /// Releases an `ε`-differentially private copy by adding discrete-Laplace noise of scale
    /// `depth / ε` to every counter. The RNG **must** be a CSPRNG for the guarantee to hold
    /// (see [`secure_rng`](crate::privacy::mechanisms::secure_rng)).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `epsilon <= 0`.
    pub fn privatize<R: Rng + ?Sized>(&self, epsilon: f64, rng: &mut R) -> Result<PrivateCountMin> {
        if !epsilon.is_finite() || epsilon <= 0.0 {
            return Err(SketchError::InvalidParameter {
                param: "epsilon".to_string(),
                value: epsilon.to_string(),
                constraint: "must be a finite value > 0".to_string(),
            });
        }
        // L1 sensitivity = depth (one record touches one cell per row).
        let scale = self.depth as f64 / epsilon;
        let mut noisy = Vec::with_capacity(self.counters.len());
        for &c in &self.counters {
            noisy.push(c + discrete_laplace(rng, scale)?);
        }
        Ok(PrivateCountMin {
            depth: self.depth,
            width: self.width,
            counters: noisy,
            epsilon,
        })
    }
}

/// A released, `ε`-differentially private Count-Min sketch. Queries are post-processing of
/// the noisy counters, so they preserve the `ε` guarantee.
#[derive(Debug, Clone)]
pub struct PrivateCountMin {
    depth: usize,
    width: usize,
    counters: Vec<i64>,
    epsilon: f64,
}

impl PrivateCountMin {
    #[inline]
    fn column(&self, item: &[u8], r: usize) -> usize {
        (xxhash(item, r as u64) % self.width as u64) as usize
    }

    /// Private point estimate (min over rows). May be slightly negative from noise; use
    /// [`estimate_clamped`](Self::estimate_clamped) for a non-negative count.
    pub fn estimate(&self, item: &[u8]) -> i64 {
        (0..self.depth)
            .map(|r| self.counters[r * self.width + self.column(item, r)])
            .min()
            .unwrap_or(0)
    }

    /// Private point estimate clamped to be non-negative.
    pub fn estimate_clamped(&self, item: &[u8]) -> i64 {
        self.estimate(item).max(0)
    }

    /// The `ε` this sketch was released with.
    #[inline]
    pub fn epsilon(&self) -> f64 {
        self.epsilon
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::StdRng, SeedableRng};

    fn rng() -> StdRng {
        StdRng::seed_from_u64(0xC0FFEE)
    }

    #[test]
    fn rejects_zero_dims() {
        assert!(DpCountMin::new(0, 10).is_err());
        assert!(DpCountMin::new(4, 0).is_err());
        assert!(DpCountMin::new(4, 10).is_ok());
    }

    #[test]
    fn non_private_estimate_is_accurate() {
        let mut cm = DpCountMin::new(5, 2048).unwrap();
        for _ in 0..500 {
            cm.update(b"hot", 1);
        }
        for i in 0..1000u64 {
            cm.update(&i.to_le_bytes(), 1);
        }
        let e = cm.estimate(b"hot");
        assert!((500..550).contains(&e), "estimate {e}");
    }

    #[test]
    fn privatize_validates_epsilon() {
        let cm = DpCountMin::new(4, 64).unwrap();
        let mut r = rng();
        assert!(cm.privatize(0.0, &mut r).is_err());
        assert!(cm.privatize(-1.0, &mut r).is_err());
        assert!(cm.privatize(1.0, &mut r).is_ok());
    }

    #[test]
    fn private_estimate_is_close_for_large_counts() {
        let mut cm = DpCountMin::new(5, 4096).unwrap();
        for _ in 0..10_000 {
            cm.update(b"hot", 1);
        }
        let mut r = rng();
        let private = cm.privatize(1.0, &mut r).unwrap();
        let e = private.estimate_clamped(b"hot");
        // Noise scale = depth/eps = 5; for a true count 10000 the relative error is tiny.
        assert!((e - 10_000).abs() < 200, "private estimate {e}");
    }

    #[test]
    fn noise_is_actually_added() {
        // Across many keys the private estimates should not all equal the exact ones.
        let mut cm = DpCountMin::new(5, 1024).unwrap();
        for i in 0..200u64 {
            cm.update(&i.to_le_bytes(), 10);
        }
        let mut r = rng();
        let private = cm.privatize(0.5, &mut r).unwrap();
        let differ = (0..200u64)
            .filter(|i| private.estimate(&i.to_le_bytes()) != cm.estimate(&i.to_le_bytes()))
            .count();
        assert!(
            differ > 100,
            "expected noise to perturb most estimates, got {differ}"
        );
    }
}
