//! Local differential privacy — frequency estimation via Optimized Local Hashing (OLH).
//!
//! Generalized Randomized Response ([`GrrFrequencyOracle`](crate::privacy::GrrFrequencyOracle))
//! reports into the *full* domain, so its debiasing variance grows linearly with the domain size
//! `d`: for large `d` almost every report is noise. Optimized Local Hashing (Wang, Blocki, Li,
//! Jha, "Locally Differentially Private Protocols for Frequency Estimation", USENIX Security 2017)
//! removes that dependence. Each user first hashes their value into a **small** range of `g`
//! buckets with a privately-chosen random hash function, then runs randomized response over those
//! `g` buckets only. Choosing `g = ⌊e^ε⌉ + 1` minimizes the estimator variance, which becomes
//! `O(e^ε / (e^ε − 1)²) · n` — independent of `d`.
//!
//! Each user transmits `(seed, y)`: the seed of their hash function and the (possibly perturbed)
//! bucket. The server, to estimate how many users truly held value `a`, counts the **support** of
//! `a` — the reports whose hash of `a` lands on the reported bucket, `H_seed(a) = y` — then
//! debiases: with `p* = e^ε / (e^ε + g − 1)` (a true holder supports `a`) and `q* = 1/g` (a
//! non-holder supports `a` by chance), `n̂_a = (support_a − n·q*) / (p* − q*)` is unbiased.
//!
//! The randomizer takes a caller-provided RNG so production code can supply a CSPRNG (the privacy
//! guarantee depends on the coin — and the hash seed — being unpredictable) while tests can seed
//! for reproducibility. See [`GrrFrequencyOracle`](crate::privacy::GrrFrequencyOracle) for the
//! direct-encoding oracle that wins on small domains.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use rand::Rng;

/// An `ε`-LDP frequency oracle over a domain `0..domain` using Optimized Local Hashing.
///
/// # Example
/// ```
/// use sketch_oxide::privacy::OlhFrequencyOracle;
/// use rand::{rngs::StdRng, SeedableRng};
///
/// let mut rng = StdRng::seed_from_u64(1);
/// let mut oracle = OlhFrequencyOracle::new(1024, 4.0).unwrap();
///
/// // 40_000 users truly hold value 7, the rest are spread over a large domain.
/// for i in 0..100_000u64 {
///     let v = if i < 40_000 { 7 } else { (1 + i % 1000) as usize };
///     oracle.submit(v, &mut rng);
/// }
/// let est = oracle.estimate(7);
/// assert!((est - 40_000.0).abs() < 0.1 * 40_000.0, "estimate {est}");
/// ```
#[derive(Debug, Clone)]
pub struct OlhFrequencyOracle {
    domain: usize,
    epsilon: f64,
    /// Hash range `g = ⌊e^ε⌉ + 1` (the variance-optimal choice).
    g: u64,
    /// Pr[a true holder of `a` supports `a`] `= e^ε / (e^ε + g − 1)`.
    p_star: f64,
    /// Pr[a non-holder supports `a` by chance] `= 1/g`.
    q_star: f64,
    /// Per-user reports: `(hash_seed, reported_bucket)`.
    reports: Vec<(u64, u32)>,
}

impl OlhFrequencyOracle {
    /// Creates an oracle over `domain` distinct values at privacy level `epsilon`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `domain < 2` or `epsilon` is not positive and finite.
    pub fn new(domain: usize, epsilon: f64) -> Result<Self> {
        if domain < 2 {
            return Err(SketchError::InvalidParameter {
                param: "domain".to_string(),
                value: domain.to_string(),
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
        let e = epsilon.exp();
        // Variance-optimal hash range; at least 2 buckets so randomized response is meaningful.
        let g = (e.round() as u64 + 1).max(2);
        let denom = e + g as f64 - 1.0;
        Ok(Self {
            domain,
            epsilon,
            g,
            p_star: e / denom,
            q_star: 1.0 / g as f64,
            reports: Vec::new(),
        })
    }

    /// Hashes `value` into `0..g` under hash function `seed`.
    #[inline]
    fn bucket(&self, value: usize, seed: u64) -> u32 {
        (xxhash(&(value as u64).to_le_bytes(), seed) % self.g) as u32
    }

    /// Client side: returns the randomized report `(seed, bucket)` for a true `value`.
    ///
    /// Draws a fresh hash seed, hashes `value` into `0..g`, then keeps that bucket with probability
    /// `p* = e^ε / (e^ε + g − 1)` and otherwise reports a uniformly random *other* bucket. `value`
    /// must be `< domain`.
    pub fn privatize<R: Rng>(&self, value: usize, rng: &mut R) -> (u64, u32) {
        debug_assert!(value < self.domain);
        let seed = rng.random::<u64>();
        let x = self.bucket(value, seed);
        let y = if rng.random::<f64>() < self.p_star {
            x
        } else {
            // Uniform over the other g − 1 buckets.
            let mut other = rng.random_range(0..self.g as u32 - 1);
            if other >= x {
                other += 1;
            }
            other
        };
        (seed, y)
    }

    /// Server side: ingests one randomized report.
    pub fn observe(&mut self, report: (u64, u32)) {
        self.reports.push(report);
    }

    /// Convenience: simulate one user — privatize `true_value` and observe the report.
    pub fn submit<R: Rng>(&mut self, true_value: usize, rng: &mut R) {
        let r = self.privatize(true_value, rng);
        self.observe(r);
    }

    /// Support of `value`: number of reports whose hash of `value` equals the reported bucket.
    pub fn support(&self, value: usize) -> u64 {
        self.reports
            .iter()
            .filter(|&&(seed, y)| self.bucket(value, seed) == y)
            .count() as u64
    }

    /// Debiased estimate of how many users truly held `value`:
    /// `(support − n·q*) / (p* − q*)`.
    pub fn estimate(&self, value: usize) -> f64 {
        if value >= self.domain {
            return 0.0;
        }
        let n = self.reports.len() as f64;
        (self.support(value) as f64 - n * self.q_star) / (self.p_star - self.q_star)
    }

    /// Debiased estimated frequency (fraction) of `value`.
    pub fn frequency(&self, value: usize) -> f64 {
        if self.reports.is_empty() {
            0.0
        } else {
            self.estimate(value) / self.reports.len() as f64
        }
    }

    /// Number of reports observed.
    #[inline]
    pub fn total(&self) -> u64 {
        self.reports.len() as u64
    }

    /// Hash range `g = ⌊e^ε⌉ + 1`.
    #[inline]
    pub fn hash_range(&self) -> u64 {
        self.g
    }

    /// Privacy parameter `ε`.
    #[inline]
    pub fn epsilon(&self) -> f64 {
        self.epsilon
    }

    /// Domain size.
    #[inline]
    pub fn domain(&self) -> usize {
        self.domain
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn rejects_bad_params() {
        assert!(OlhFrequencyOracle::new(1, 1.0).is_err());
        assert!(OlhFrequencyOracle::new(10, 0.0).is_err());
        assert!(OlhFrequencyOracle::new(10, -1.0).is_err());
        assert!(OlhFrequencyOracle::new(10, f64::NAN).is_err());
        assert!(OlhFrequencyOracle::new(10, 1.0).is_ok());
    }

    #[test]
    fn optimal_hash_range() {
        // g = round(e^ε) + 1.
        let o = OlhFrequencyOracle::new(1000, 1.0).unwrap();
        assert_eq!(o.hash_range(), 1f64.exp().round() as u64 + 1); // e ≈ 2.718 → 3 + 1 = 4
        let o = OlhFrequencyOracle::new(1000, 4.0).unwrap();
        assert_eq!(o.hash_range(), 4f64.exp().round() as u64 + 1); // 54.6 → 55 + 1 = 56
    }

    #[test]
    fn estimates_a_heavy_value_on_large_domain() {
        let mut rng = StdRng::seed_from_u64(7);
        let mut oracle = OlhFrequencyOracle::new(1024, 4.0).unwrap();
        // Heavy value 0; background uses 1..1000 so the true count of 0 is exactly 40_000.
        for i in 0..100_000u64 {
            let v = if i < 40_000 {
                0
            } else {
                (1 + i % 1000) as usize
            };
            oracle.submit(v, &mut rng);
        }
        let est = oracle.estimate(0);
        assert!((est - 40_000.0).abs() < 0.1 * 40_000.0, "estimate {est}");
    }

    #[test]
    fn absent_value_estimates_near_zero() {
        let mut rng = StdRng::seed_from_u64(13);
        let mut oracle = OlhFrequencyOracle::new(1024, 4.0).unwrap();
        // No user ever holds value 500.
        for i in 0..80_000u64 {
            oracle.submit((1 + i % 200) as usize, &mut rng);
        }
        let est = oracle.estimate(500);
        // Debiased estimate of an absent value is ~0 (within sampling noise).
        assert!(est.abs() < 0.05 * 80_000.0, "estimate {est}");
    }

    #[test]
    fn frequency_is_a_fraction() {
        let mut rng = StdRng::seed_from_u64(11);
        let mut oracle = OlhFrequencyOracle::new(256, 3.0).unwrap();
        for i in 0..50_000usize {
            oracle.submit(i % 50, &mut rng);
        }
        // Each of the 50 values appears ~2% of the time.
        let f = oracle.frequency(0);
        assert!((f - 0.02).abs() < 0.01, "frequency {f}");
        assert_eq!(oracle.total(), 50_000);
    }
}
