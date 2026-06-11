//! Local differential privacy — frequency estimation via Generalized Randomized Response.
//!
//! Under *local* differential privacy each user perturbs their value **before** it leaves their
//! device, so the server never sees raw data and no trust is required. Generalized Randomized
//! Response (GRR; Warner's randomized response generalized to a domain of size `d`, the building
//! block of Google's RAPPOR and Apple's analytics) is the canonical `ε`-LDP frequency oracle: a
//! user reports their true value `v` with probability `p = e^ε / (e^ε + d − 1)` and a uniformly
//! random *other* value with the remaining probability `q = 1 / (e^ε + d − 1)` each. The server
//! counts the noisy reports and **debiases**: `n̂_v = (count_v − n·q) / (p − q)` is an unbiased
//! estimate of how many users truly held `v`.
//!
//! The randomizer takes a caller-provided RNG so production code can supply a CSPRNG (the privacy
//! guarantee depends on the coin being unpredictable) while tests can seed for reproducibility.

use crate::common::{Result, SketchError};
use rand::Rng;

/// An `ε`-LDP frequency oracle over a domain `0..domain` using Generalized Randomized Response.
///
/// # Example
/// ```
/// use sketch_oxide::privacy::GrrFrequencyOracle;
/// use rand::{rngs::StdRng, SeedableRng};
///
/// let mut rng = StdRng::seed_from_u64(1);
/// let mut oracle = GrrFrequencyOracle::new(20, 3.0).unwrap();
///
/// // 40_000 users truly hold value 3, the rest are spread over the domain.
/// for i in 0..100_000u64 {
///     let v = if i < 40_000 { 3 } else { (i % 20) as usize };
///     oracle.submit(v, &mut rng);
/// }
/// let est = oracle.estimate(3);
/// assert!((est - 40_000.0).abs() < 0.1 * 40_000.0, "estimate {est}");
/// ```
#[derive(Debug, Clone)]
pub struct GrrFrequencyOracle {
    domain: usize,
    epsilon: f64,
    p: f64,
    q: f64,
    counts: Vec<u64>,
    n: u64,
}

impl GrrFrequencyOracle {
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
        let denom = e + domain as f64 - 1.0;
        Ok(Self {
            domain,
            epsilon,
            p: e / denom,
            q: 1.0 / denom,
            counts: vec![0u64; domain],
            n: 0,
        })
    }

    /// Client side: returns the randomized report for a true `value` (an ε-LDP message).
    ///
    /// Reports `value` with probability `p`, otherwise a uniformly random other value. `value` must
    /// be `< domain`.
    pub fn privatize<R: Rng>(&self, value: usize, rng: &mut R) -> usize {
        debug_assert!(value < self.domain);
        if rng.random::<f64>() < self.p {
            value
        } else {
            // Uniform over the other domain − 1 values.
            let mut other = rng.random_range(0..self.domain - 1);
            if other >= value {
                other += 1;
            }
            other
        }
    }

    /// Server side: ingests one randomized report.
    pub fn observe(&mut self, reported: usize) {
        if reported < self.domain {
            self.counts[reported] += 1;
            self.n += 1;
        }
    }

    /// Convenience: simulate one user — privatize `true_value` and observe the report.
    pub fn submit<R: Rng>(&mut self, true_value: usize, rng: &mut R) {
        let r = self.privatize(true_value, rng);
        self.observe(r);
    }

    /// Debiased estimate of how many users truly held `value`: `(count − n·q) / (p − q)`.
    pub fn estimate(&self, value: usize) -> f64 {
        if value >= self.domain {
            return 0.0;
        }
        (self.counts[value] as f64 - self.n as f64 * self.q) / (self.p - self.q)
    }

    /// Debiased estimated frequency (fraction) of `value`.
    pub fn frequency(&self, value: usize) -> f64 {
        if self.n == 0 {
            0.0
        } else {
            self.estimate(value) / self.n as f64
        }
    }

    /// Number of reports observed.
    #[inline]
    pub fn total(&self) -> u64 {
        self.n
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
    use rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn rejects_bad_params() {
        assert!(GrrFrequencyOracle::new(1, 1.0).is_err());
        assert!(GrrFrequencyOracle::new(10, 0.0).is_err());
        assert!(GrrFrequencyOracle::new(10, -1.0).is_err());
        assert!(GrrFrequencyOracle::new(10, 1.0).is_ok());
    }

    #[test]
    fn estimates_a_heavy_value() {
        let mut rng = StdRng::seed_from_u64(7);
        let mut oracle = GrrFrequencyOracle::new(20, 3.0).unwrap();
        // Heavy value 0; background uses 1..19 so the true count of 0 is exactly 40_000.
        for i in 0..100_000u64 {
            let v = if i < 40_000 { 0 } else { (1 + i % 19) as usize };
            oracle.submit(v, &mut rng);
        }
        let est = oracle.estimate(0);
        assert!((est - 40_000.0).abs() < 0.1 * 40_000.0, "estimate {est}");
    }

    #[test]
    fn frequency_sums_reasonably() {
        let mut rng = StdRng::seed_from_u64(11);
        let mut oracle = GrrFrequencyOracle::new(10, 2.5).unwrap();
        // Uniform input: each value should estimate ~10% frequency.
        for i in 0..50_000usize {
            oracle.submit(i % 10, &mut rng);
        }
        for v in 0..10 {
            let f = oracle.frequency(v);
            assert!((f - 0.1).abs() < 0.03, "value {v}: frequency {f}");
        }
    }

    #[test]
    fn rare_value_estimates_near_zero() {
        let mut rng = StdRng::seed_from_u64(3);
        let mut oracle = GrrFrequencyOracle::new(50, 2.0).unwrap();
        for _ in 0..100_000 {
            oracle.submit(0, &mut rng); // everyone holds value 0
        }
        // A value nobody holds debiases to ~0 (small relative to n).
        let est = oracle.estimate(25);
        assert!(est.abs() < 0.05 * 100_000.0, "rare estimate {est}");
    }

    #[test]
    fn privatize_stays_in_domain() {
        let mut rng = StdRng::seed_from_u64(99);
        let oracle = GrrFrequencyOracle::new(8, 1.0).unwrap();
        for v in 0..8 {
            for _ in 0..1000 {
                let r = oracle.privatize(v, &mut rng);
                assert!(r < 8);
            }
        }
    }

    #[test]
    fn higher_epsilon_is_more_accurate() {
        // Lower noise (higher ε) should give a tighter estimate. Average over seeds so the
        // comparison reflects variance, not a single noisy draw.
        let truth = 30_000.0;
        let mean_err = |eps: f64| -> f64 {
            let mut total = 0.0;
            for seed in 0..12u64 {
                let mut rng = StdRng::seed_from_u64(seed);
                let mut o = GrrFrequencyOracle::new(10, eps).unwrap();
                // Heavy value 0; background uses 1..9 so 0's true count is exactly 30_000.
                for i in 0..100_000usize {
                    o.submit(if i < 30_000 { 0 } else { 1 + i % 9 }, &mut rng);
                }
                total += (o.estimate(0) - truth).abs();
            }
            total / 12.0
        };
        let lo = mean_err(0.5);
        let hi = mean_err(4.0);
        assert!(
            hi < lo,
            "hi-eps mean err {hi} should be < lo-eps mean err {lo}"
        );
    }
}
