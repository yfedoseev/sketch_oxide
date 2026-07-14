//! Differentially private quantiles via the exponential mechanism.
//!
//! Estimating a quantile (median, p95, …) under ε-differential privacy cannot be done by adding
//! noise to the value — a quantile's sensitivity is unbounded. The standard solution (Smith, "Privacy-
//! preserving statistical estimation with optimal convergence rates", STOC 2011) is the **exponential
//! mechanism**: score each gap between consecutive sorted data points by how close its rank is to the
//! target rank, sample a gap with probability ∝ `exp(ε · score / 2)` weighted by the gap's width, and
//! return a uniform point inside it. Adding or removing one record shifts every gap's rank by at most
//! 1, so the score has sensitivity 1 and the mechanism is ε-DP.
//!
//! The data range `[lo, hi]` is public (a domain bound), as the exponential mechanism requires; values
//! are clamped into it. The caller supplies the RNG so production code can pass a CSPRNG (privacy
//! depends on unpredictable sampling) while tests seed it.

use crate::common::{Result, SketchError};
use rand::Rng;

/// An ε-DP quantile estimator over the public range `[lo, hi]`.
///
/// # Example
/// ```
/// use sketch_oxide::privacy::DpQuantile;
/// use rand::{rngs::StdRng, SeedableRng};
///
/// let mut rng = StdRng::seed_from_u64(1);
/// let dq = DpQuantile::new(2.0, 0.0, 10_000.0).unwrap();
/// let data: Vec<f64> = (0..10_000).map(|i| i as f64).collect();
///
/// // The private median is close to the true median (~5000) at this ε.
/// let m = dq.quantile(&data, 0.5, &mut rng);
/// assert!((m - 5000.0).abs() < 500.0, "median {m}");
/// ```
#[derive(Debug, Clone)]
pub struct DpQuantile {
    epsilon: f64,
    lo: f64,
    hi: f64,
}

impl DpQuantile {
    /// Creates an estimator with privacy budget `epsilon` over the public range `[lo, hi]`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `epsilon` is not positive/finite or `lo >= hi`.
    pub fn new(epsilon: f64, lo: f64, hi: f64) -> Result<Self> {
        if !epsilon.is_finite() || epsilon <= 0.0 {
            return Err(SketchError::InvalidParameter {
                param: "epsilon".to_string(),
                value: epsilon.to_string(),
                constraint: "must be a positive finite number".to_string(),
            });
        }
        if !lo.is_finite() || !hi.is_finite() || lo >= hi {
            return Err(SketchError::InvalidParameter {
                param: "lo/hi".to_string(),
                value: format!("{lo}/{hi}"),
                constraint: "must be finite with lo < hi".to_string(),
            });
        }
        Ok(Self { epsilon, lo, hi })
    }

    /// Returns an ε-DP estimate of the `phi`-quantile of `data` (`phi ∈ [0, 1]`).
    pub fn quantile<R: Rng>(&self, data: &[f64], phi: f64, rng: &mut R) -> f64 {
        if data.is_empty() {
            return self.lo + (self.hi - self.lo) * phi.clamp(0.0, 1.0);
        }
        // Sorted, clamped copy.
        let mut sorted: Vec<f64> = data.iter().map(|&x| x.clamp(self.lo, self.hi)).collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = sorted.len();
        let target = phi.clamp(0.0, 1.0) * n as f64;

        // Gap i lies between boundary i and i+1, with boundaries lo, x_1, …, x_n, hi. A value in gap
        // i has rank i. Score = −|i − target|; sensitivity 1 ⇒ weight ∝ exp(ε·score/2)·width.
        let n_gaps = n + 1;
        let mut log_w = vec![f64::NEG_INFINITY; n_gaps];
        for (i, lw) in log_w.iter_mut().enumerate() {
            let left = if i == 0 { self.lo } else { sorted[i - 1] };
            let right = if i == n { self.hi } else { sorted[i] };
            let width = right - left;
            if width <= 0.0 {
                continue; // zero-width gap contributes nothing
            }
            let score = -(i as f64 - target).abs();
            *lw = width.ln() + self.epsilon * score / 2.0;
        }

        // Sample a gap proportionally to its weight (log-sum-exp for stability).
        let max_lw = log_w.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let total: f64 = log_w.iter().map(|&l| (l - max_lw).exp()).sum();
        let mut u = rng.random::<f64>() * total;
        let mut chosen = 0;
        for (i, &l) in log_w.iter().enumerate() {
            let w = (l - max_lw).exp();
            if u < w {
                chosen = i;
                break;
            }
            u -= w;
            chosen = i;
        }

        // Uniform point inside the chosen gap.
        let left = if chosen == 0 {
            self.lo
        } else {
            sorted[chosen - 1]
        };
        let right = if chosen == n { self.hi } else { sorted[chosen] };
        left + rng.random::<f64>() * (right - left)
    }

    /// Privacy parameter `ε`.
    #[inline]
    pub fn epsilon(&self) -> f64 {
        self.epsilon
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn rejects_bad_params() {
        assert!(DpQuantile::new(0.0, 0.0, 1.0).is_err());
        assert!(DpQuantile::new(-1.0, 0.0, 1.0).is_err());
        assert!(DpQuantile::new(1.0, 5.0, 5.0).is_err());
        assert!(DpQuantile::new(1.0, 5.0, 1.0).is_err());
        assert!(DpQuantile::new(1.0, 0.0, 1.0).is_ok());
    }

    #[test]
    fn median_is_accurate_at_high_epsilon() {
        let mut rng = StdRng::seed_from_u64(7);
        let dq = DpQuantile::new(4.0, 0.0, 10_000.0).unwrap();
        let data: Vec<f64> = (0..10_000).map(|i| i as f64).collect();
        // Average over a few draws to reduce noise variance in the assertion.
        let mut sum = 0.0;
        for _ in 0..10 {
            sum += dq.quantile(&data, 0.5, &mut rng);
        }
        let est = sum / 10.0;
        assert!((est - 5000.0).abs() < 300.0, "median estimate {est}");
    }

    #[test]
    fn percentiles_are_ordered() {
        let mut rng = StdRng::seed_from_u64(3);
        let dq = DpQuantile::new(4.0, 0.0, 10_000.0).unwrap();
        let data: Vec<f64> = (0..10_000).map(|i| i as f64).collect();
        let mean = |phi: f64, rng: &mut StdRng| {
            let mut s = 0.0;
            for _ in 0..20 {
                s += dq.quantile(&data, phi, rng);
            }
            s / 20.0
        };
        let p25 = mean(0.25, &mut rng);
        let p50 = mean(0.5, &mut rng);
        let p75 = mean(0.75, &mut rng);
        assert!(p25 < p50 && p50 < p75, "p25 {p25} p50 {p50} p75 {p75}");
        assert!((p25 - 2500.0).abs() < 500.0);
        assert!((p75 - 7500.0).abs() < 500.0);
    }

    #[test]
    fn output_stays_in_range() {
        let mut rng = StdRng::seed_from_u64(99);
        let dq = DpQuantile::new(1.0, -50.0, 50.0).unwrap();
        let data: Vec<f64> = (0..1000).map(|i| (i % 100) as f64 - 50.0).collect();
        for _ in 0..1000 {
            let q = dq.quantile(&data, rng.random::<f64>(), &mut rng);
            assert!((-50.0..=50.0).contains(&q), "out of range: {q}");
        }
    }

    #[test]
    fn empty_data_returns_range_interpolation() {
        let mut rng = StdRng::seed_from_u64(1);
        let dq = DpQuantile::new(1.0, 0.0, 100.0).unwrap();
        assert_eq!(dq.quantile(&[], 0.5, &mut rng), 50.0);
        assert_eq!(dq.quantile(&[], 0.0, &mut rng), 0.0);
    }
}
