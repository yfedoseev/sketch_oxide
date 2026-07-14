//! Local differential privacy — frequency estimation via the Count-Mean-Sketch (CMS).
//!
//! Single-value LDP oracles like [`OlhFrequencyOracle`](crate::privacy::OlhFrequencyOracle) answer
//! "how many users hold value `v`?" one value at a time. Apple's Count-Mean-Sketch (Apple
//! Differential Privacy Team, "Learning with Privacy at Scale", 2017 — deployed for keyboard,
//! emoji, and Safari telemetry) instead maintains a `k × m` *sketch matrix* so the whole frequency
//! histogram is queryable, while keeping each client's report `ε`-LDP.
//!
//! Each client with value `d` picks one of the `k` hash rows `j` at random, builds the `±1` vector
//! `v ∈ {−1,+1}^m` that is `+1` only at `h_j(d)`, then flips each coordinate independently with
//! probability `1 / (e^{ε/2} + 1)` (the randomized-response step that provides the privacy) and
//! sends `(v, j)`. The server debiases each report by `c_ε = (e^{ε/2}+1)/(e^{ε/2}−1)` — adding
//! `k·(c_ε·v_i + 1)/2` into row `j` — so the sketch is an unbiased accumulator. To estimate the
//! count of `d` it averages the matched cell across rows and removes the uniform collision floor:
//! `f̂(d) = (m/(m−1)) · ((1/k)·Σ_l M[l, h_l(d)] − N/m)`, which is unbiased for the true count.
//!
//! The randomizer takes a caller-provided RNG so production code can supply a CSPRNG (the privacy
//! guarantee depends on the coin being unpredictable) while tests can seed for reproducibility.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use rand::Rng;

/// Base seed mixed with the row index to derive each row's hash function.
const ROW_SEED_BASE: u64 = 0x436D_7353_6565_6421; // "CmsSeed!"

/// An `ε`-LDP Count-Mean-Sketch over a domain `0..domain`, using `k` hash rows of `m` buckets.
///
/// # Example
/// ```
/// use sketch_oxide::privacy::CountMeanSketch;
/// use rand::{rngs::StdRng, SeedableRng};
///
/// let mut rng = StdRng::seed_from_u64(1);
/// let mut cms = CountMeanSketch::new(32, 256, 4.0).unwrap();
///
/// // 40_000 users truly hold value 7, the rest are spread over a large domain.
/// for i in 0..100_000u64 {
///     let v = if i < 40_000 { 7 } else { (1 + i % 1000) as usize };
///     cms.submit(v, &mut rng);
/// }
/// let est = cms.estimate(7);
/// assert!((est - 40_000.0).abs() < 0.12 * 40_000.0, "estimate {est}");
/// ```
#[derive(Debug, Clone)]
pub struct CountMeanSketch {
    /// Number of hash rows.
    k: usize,
    /// Buckets per row.
    m: usize,
    epsilon: f64,
    /// Debiasing constant `c_ε = (e^{ε/2}+1)/(e^{ε/2}−1)`.
    c_eps: f64,
    /// Flip probability `1 / (e^{ε/2}+1)` for the privatization step.
    flip_prob: f64,
    /// `k × m` debiased accumulator, row-major.
    matrix: Vec<f64>,
    /// Number of clients aggregated.
    n: u64,
}

impl CountMeanSketch {
    /// Creates a sketch with `k` hash rows of `m` buckets at privacy level `epsilon`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `k == 0`, `m < 2`, or `epsilon` is not positive and
    /// finite.
    pub fn new(k: usize, m: usize, epsilon: f64) -> Result<Self> {
        if k == 0 {
            return Err(SketchError::InvalidParameter {
                param: "k".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        if m < 2 {
            return Err(SketchError::InvalidParameter {
                param: "m".to_string(),
                value: m.to_string(),
                constraint: "must be >= 2".to_string(),
            });
        }
        if !epsilon.is_finite() || epsilon <= 0.0 {
            return Err(SketchError::InvalidParameter {
                param: "epsilon".to_string(),
                value: epsilon.to_string(),
                constraint: "must be a positive finite number".to_string(),
            });
        }
        let e_half = (epsilon / 2.0).exp();
        Ok(Self {
            k,
            m,
            epsilon,
            c_eps: (e_half + 1.0) / (e_half - 1.0),
            flip_prob: 1.0 / (e_half + 1.0),
            matrix: vec![0.0; k * m],
            n: 0,
        })
    }

    /// Bucket of `value` under hash row `row` (`0..m`).
    #[inline]
    fn bucket(&self, value: usize, row: usize) -> usize {
        let seed = ROW_SEED_BASE.wrapping_add(row as u64);
        (xxhash(&(value as u64).to_le_bytes(), seed) % self.m as u64) as usize
    }

    /// Client side: returns the privatized report `(vector, row)` for a true `value`.
    ///
    /// Picks a random row `j`, builds the `±1` indicator at `h_j(value)`, and flips each coordinate
    /// with probability `1 / (e^{ε/2}+1)`. The returned vector has length `m`.
    pub fn privatize<R: Rng>(&self, value: usize, rng: &mut R) -> (Vec<i8>, usize) {
        debug_assert!(value < usize::MAX);
        let j = rng.random_range(0..self.k);
        let hit = self.bucket(value, j);
        let mut v = vec![-1i8; self.m];
        v[hit] = 1;
        for coord in v.iter_mut() {
            if rng.random::<f64>() < self.flip_prob {
                *coord = -*coord;
            }
        }
        (v, j)
    }

    /// Server side: ingests one privatized report, debiasing it into the sketch matrix.
    pub fn observe(&mut self, report: (Vec<i8>, usize)) {
        let (v, j) = report;
        debug_assert_eq!(v.len(), self.m);
        debug_assert!(j < self.k);
        let base = j * self.m;
        let k = self.k as f64;
        for (i, &coord) in v.iter().enumerate() {
            // Debiased contribution: k·(c_ε·v_i + 1)/2.
            self.matrix[base + i] += k * (self.c_eps * coord as f64 + 1.0) / 2.0;
        }
        self.n += 1;
    }

    /// Convenience: simulate one user — privatize `true_value` and observe the report.
    pub fn submit<R: Rng>(&mut self, true_value: usize, rng: &mut R) {
        let r = self.privatize(true_value, rng);
        self.observe(r);
    }

    /// Debiased estimate of how many users truly held `value`:
    /// `(m/(m−1)) · ((1/k)·Σ_l M[l, h_l(value)] − N/m)`.
    pub fn estimate(&self, value: usize) -> f64 {
        let m = self.m as f64;
        let mut sum = 0.0;
        for row in 0..self.k {
            sum += self.matrix[row * self.m + self.bucket(value, row)];
        }
        let mean = sum / self.k as f64;
        (m / (m - 1.0)) * (mean - self.n as f64 / m)
    }

    /// Debiased estimated frequency (fraction) of `value`.
    pub fn frequency(&self, value: usize) -> f64 {
        if self.n == 0 {
            0.0
        } else {
            self.estimate(value) / self.n as f64
        }
    }

    /// Number of clients aggregated.
    #[inline]
    pub fn total(&self) -> u64 {
        self.n
    }

    /// Number of hash rows `k`.
    #[inline]
    pub fn rows(&self) -> usize {
        self.k
    }

    /// Buckets per row `m`.
    #[inline]
    pub fn width(&self) -> usize {
        self.m
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
        assert!(CountMeanSketch::new(0, 256, 1.0).is_err());
        assert!(CountMeanSketch::new(8, 1, 1.0).is_err());
        assert!(CountMeanSketch::new(8, 256, 0.0).is_err());
        assert!(CountMeanSketch::new(8, 256, -1.0).is_err());
        assert!(CountMeanSketch::new(8, 256, f64::NAN).is_err());
        assert!(CountMeanSketch::new(8, 256, 1.0).is_ok());
    }

    #[test]
    fn estimates_a_heavy_value() {
        let mut rng = StdRng::seed_from_u64(7);
        let mut cms = CountMeanSketch::new(32, 256, 4.0).unwrap();
        // Heavy value 0; background spreads over 1..1000, so the true count of 0 is exactly 40_000.
        for i in 0..100_000u64 {
            let v = if i < 40_000 {
                0
            } else {
                (1 + i % 1000) as usize
            };
            cms.submit(v, &mut rng);
        }
        let est = cms.estimate(0);
        assert!((est - 40_000.0).abs() < 0.12 * 40_000.0, "estimate {est}");
    }

    #[test]
    fn absent_value_estimates_near_zero() {
        let mut rng = StdRng::seed_from_u64(13);
        let mut cms = CountMeanSketch::new(32, 256, 4.0).unwrap();
        // No user ever holds value 999_999.
        for i in 0..80_000u64 {
            cms.submit((1 + i % 500) as usize, &mut rng);
        }
        let est = cms.estimate(999_999);
        assert!(est.abs() < 0.05 * 80_000.0, "estimate {est}");
    }

    #[test]
    fn frequency_is_a_fraction() {
        let mut rng = StdRng::seed_from_u64(11);
        let mut cms = CountMeanSketch::new(16, 128, 3.0).unwrap();
        for i in 0..60_000usize {
            cms.submit(i % 50, &mut rng);
        }
        // Each of the 50 values appears ~2% of the time.
        let f = cms.frequency(0);
        assert!((f - 0.02).abs() < 0.015, "frequency {f}");
        assert_eq!(cms.total(), 60_000);
    }

    #[test]
    fn no_noise_at_huge_epsilon_is_exact_per_row() {
        // With ε enormous the flip probability ≈ 0 and c_ε ≈ 1, so each client deposits exactly k
        // into its chosen cell. A single client therefore yields an estimate of 1.
        let mut rng = StdRng::seed_from_u64(5);
        let mut cms = CountMeanSketch::new(8, 64, 40.0).unwrap();
        cms.submit(3usize, &mut rng);
        let est = cms.estimate(3);
        assert!((est - 1.0).abs() < 1e-6, "estimate {est}");
    }
}
