//! Density Sketch — streaming kernel density estimation over a reservoir sample.
//!
//! A Density Sketch (in the spirit of the Apache DataSketches KDE sketch) summarizes the *shape* of a
//! one-dimensional stream rather than a single statistic: it answers "how dense is the data around
//! value `x`?", supporting anomaly detection, distribution monitoring, and mode finding. It keeps a
//! uniform reservoir sample of the stream and evaluates a Gaussian kernel density estimate over that
//! sample:
//!
//! ```text
//! f̂(x) = 1 / (m · h · √(2π)) · Σ_i exp( −½ · ((x − x_i)/h)² )
//! ```
//!
//! where the `x_i` are the `m` reservoir points and `h` is the bandwidth. Because reservoir sampling
//! keeps a uniform sample of the whole stream, the estimate converges to the true density as the
//! sample grows. Silverman's rule-of-thumb bandwidth is available from the current sample via
//! [`DensitySketch::silverman_bandwidth`].
//!
//! Complements the moment/quantile summaries elsewhere in the crate: those give numbers, this gives a
//! callable density function.

use crate::common::{Result, SketchError};
use rand::Rng;
use std::f64::consts::PI;

/// A streaming Gaussian kernel density estimator backed by a reservoir of `capacity` points.
///
/// # Example
/// ```
/// use sketch_oxide::statistics::DensitySketch;
/// use rand::{rngs::StdRng, Rng, SeedableRng};
///
/// let mut rng = StdRng::seed_from_u64(1);
/// let mut ds = DensitySketch::with_seed(4096, 0.15, 7).unwrap();
/// // Feed standard-normal samples.
/// for _ in 0..50_000 {
///     let u1: f64 = rng.random::<f64>().max(1e-12);
///     let u2: f64 = rng.random();
///     let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
///     ds.update(z);
/// }
/// // N(0,1) density at 0 is 1/√(2π) ≈ 0.399.
/// assert!((ds.density(0.0) - 0.399).abs() < 0.05, "density {}", ds.density(0.0));
/// ```
#[derive(Debug, Clone)]
pub struct DensitySketch {
    capacity: usize,
    bandwidth: f64,
    sample: Vec<f64>,
    /// Total points seen (for reservoir sampling).
    seen: u64,
    rng: rand::rngs::SmallRng,
}

impl DensitySketch {
    /// Creates a sketch with a reservoir of `capacity` points and Gaussian-kernel `bandwidth`,
    /// seeded from the OS.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `capacity` is 0 or `bandwidth` is not positive and finite.
    pub fn new(capacity: usize, bandwidth: f64) -> Result<Self> {
        use rand::SeedableRng;
        Self::from_rng(capacity, bandwidth, rand::rngs::SmallRng::from_os_rng())
    }

    /// Creates a sketch with a fixed RNG seed (reproducible).
    ///
    /// # Errors
    /// As [`DensitySketch::new`].
    pub fn with_seed(capacity: usize, bandwidth: f64, seed: u64) -> Result<Self> {
        use rand::SeedableRng;
        Self::from_rng(
            capacity,
            bandwidth,
            rand::rngs::SmallRng::seed_from_u64(seed),
        )
    }

    fn from_rng(capacity: usize, bandwidth: f64, rng: rand::rngs::SmallRng) -> Result<Self> {
        if capacity == 0 {
            return Err(SketchError::InvalidParameter {
                param: "capacity".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        if !(bandwidth.is_finite() && bandwidth > 0.0) {
            return Err(SketchError::InvalidParameter {
                param: "bandwidth".to_string(),
                value: bandwidth.to_string(),
                constraint: "must be a positive finite number".to_string(),
            });
        }
        Ok(Self {
            capacity,
            bandwidth,
            sample: Vec::with_capacity(capacity),
            seen: 0,
            rng,
        })
    }

    /// Adds one value to the stream (uniform reservoir sampling).
    pub fn update(&mut self, x: f64) {
        if !x.is_finite() {
            return;
        }
        self.seen += 1;
        if self.sample.len() < self.capacity {
            self.sample.push(x);
        } else {
            let j = self.rng.random_range(0..self.seen);
            if (j as usize) < self.capacity {
                self.sample[j as usize] = x;
            }
        }
    }

    /// Estimated probability density at `x`.
    pub fn density(&self, x: f64) -> f64 {
        let m = self.sample.len();
        if m == 0 {
            return 0.0;
        }
        let h = self.bandwidth;
        let coef = 1.0 / (m as f64 * h * (2.0 * PI).sqrt());
        let s: f64 = self
            .sample
            .iter()
            .map(|&xi| (-0.5 * ((x - xi) / h).powi(2)).exp())
            .sum();
        coef * s
    }

    /// Silverman's rule-of-thumb bandwidth from the current sample: `1.06 · σ · n^(−1/5)`.
    /// Returns `None` if fewer than two points have been sampled.
    pub fn silverman_bandwidth(&self) -> Option<f64> {
        let n = self.sample.len();
        if n < 2 {
            return None;
        }
        let mean = self.sample.iter().sum::<f64>() / n as f64;
        let var = self.sample.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (n as f64 - 1.0);
        let sigma = var.sqrt();
        Some(1.06 * sigma * (n as f64).powf(-0.2))
    }

    /// Number of points in the reservoir.
    #[inline]
    pub fn len(&self) -> usize {
        self.sample.len()
    }

    /// Whether the reservoir is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.sample.is_empty()
    }

    /// Total number of values observed.
    #[inline]
    pub fn count(&self) -> u64 {
        self.seen
    }

    /// Current kernel bandwidth.
    #[inline]
    pub fn bandwidth(&self) -> f64 {
        self.bandwidth
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    /// A standard-normal variate via Box–Muller.
    fn normal(rng: &mut StdRng) -> f64 {
        let u1: f64 = rng.random::<f64>().max(1e-12);
        let u2: f64 = rng.random();
        (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos()
    }

    fn normal_pdf(x: f64) -> f64 {
        (1.0 / (2.0 * PI).sqrt()) * (-0.5 * x * x).exp()
    }

    #[test]
    fn rejects_bad_params() {
        assert!(DensitySketch::new(0, 0.1).is_err());
        assert!(DensitySketch::new(64, 0.0).is_err());
        assert!(DensitySketch::new(64, -1.0).is_err());
        assert!(DensitySketch::new(64, f64::NAN).is_err());
        assert!(DensitySketch::new(64, 0.1).is_ok());
    }

    #[test]
    fn approximates_normal_density() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut ds = DensitySketch::with_seed(8192, 0.15, 7).unwrap();
        for _ in 0..80_000 {
            ds.update(normal(&mut rng));
        }
        for &x in &[0.0, 0.5, 1.0, -1.0] {
            let est = ds.density(x);
            let truth = normal_pdf(x);
            assert!(
                (est - truth).abs() < 0.05,
                "x {x}: est {est}, truth {truth}"
            );
        }
    }

    #[test]
    fn density_low_in_tails() {
        let mut rng = StdRng::seed_from_u64(11);
        let mut ds = DensitySketch::with_seed(8192, 0.2, 3).unwrap();
        for _ in 0..80_000 {
            ds.update(normal(&mut rng));
        }
        // Far in the tail the density is near zero.
        assert!(ds.density(5.0) < 0.01, "tail density {}", ds.density(5.0));
        // The mode is denser than the tail.
        assert!(ds.density(0.0) > ds.density(3.0));
    }

    #[test]
    fn reservoir_is_bounded_and_silverman_reasonable() {
        let mut rng = StdRng::seed_from_u64(5);
        let mut ds = DensitySketch::with_seed(1000, 0.2, 9).unwrap();
        for _ in 0..100_000 {
            ds.update(normal(&mut rng));
        }
        assert_eq!(ds.len(), 1000);
        assert_eq!(ds.count(), 100_000);
        // σ ≈ 1, so Silverman's bandwidth is ≈ 1.06 · 1000^(−1/5) ≈ 0.27.
        let h = ds.silverman_bandwidth().unwrap();
        assert!(h > 0.1 && h < 0.5, "silverman {h}");
    }

    #[test]
    fn empty_density_is_zero() {
        let ds = DensitySketch::with_seed(64, 0.1, 1).unwrap();
        assert!(ds.is_empty());
        assert_eq!(ds.density(0.0), 0.0);
        assert!(ds.silverman_bandwidth().is_none());
    }
}
