//! Morris counter — approximate counting in `O(log log n)` bits.
//!
//! The Morris counter (Robert Morris, "Counting Large Numbers of Events in Small Registers", CACM
//! 1978 — the first streaming algorithm) counts up to `n` events while storing only a small register
//! `c ≈ log_b n`. Instead of incrementing on every event, it increments the register *probabilistically*
//! with probability `b^{−c}`, so the register tracks the logarithm of the count. The estimate
//! `(b^c − 1)/(b − 1)` is an **unbiased** estimator of the number of events, for any base `b > 1`.
//!
//! The base trades accuracy for space: the relative variance is exactly `(b − 1)/2`, so a base near 1
//! is accurate but lets `c` grow larger, while the classic base `b = 2` (estimate `2^c − 1`) keeps `c`
//! tiny at the cost of ~71% relative standard deviation per counter. The register takes a
//! caller-seedable RNG for reproducibility.
//!
//! Unlike the distinct-count sketches in [`cardinality`](crate::cardinality), Morris counts *total*
//! events (the first moment `F1`); it complements the [`AmsSketch`](crate::statistics::AmsSketch)
//! (`F2`) and [`PStableLpSketch`](crate::statistics::PStableLpSketch) (`Lp`).

use crate::common::{Result, SketchError};
use rand::Rng;

/// A probabilistic approximate counter with base `b`.
///
/// # Example
/// ```
/// use sketch_oxide::statistics::MorrisCounter;
///
/// // A near-1 base is accurate; average many independent counters to see the unbiasedness.
/// let trials = 400;
/// let mut sum = 0.0;
/// for seed in 0..trials {
///     let mut m = MorrisCounter::with_seed(1.1, seed).unwrap();
///     for _ in 0..2000 { m.increment(); }
///     sum += m.estimate();
/// }
/// let mean = sum / trials as f64;
/// assert!((mean - 2000.0).abs() < 0.06 * 2000.0, "mean {mean}");
/// ```
#[derive(Debug, Clone)]
pub struct MorrisCounter {
    /// Logarithmic register.
    c: u32,
    /// Base `b > 1`.
    base: f64,
    rng: rand::rngs::SmallRng,
}

impl MorrisCounter {
    /// Creates a counter with base `base`, seeded from the OS.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `base` is not `> 1` and finite.
    pub fn new(base: f64) -> Result<Self> {
        use rand::SeedableRng;
        Self::from_rng(base, rand::rngs::SmallRng::from_os_rng())
    }

    /// Creates a counter with base `base` and a fixed RNG seed (reproducible).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `base` is not `> 1` and finite.
    pub fn with_seed(base: f64, seed: u64) -> Result<Self> {
        use rand::SeedableRng;
        Self::from_rng(base, rand::rngs::SmallRng::seed_from_u64(seed))
    }

    fn from_rng(base: f64, rng: rand::rngs::SmallRng) -> Result<Self> {
        if !(base.is_finite() && base > 1.0) {
            return Err(SketchError::InvalidParameter {
                param: "base".to_string(),
                value: base.to_string(),
                constraint: "must be a finite number > 1".to_string(),
            });
        }
        Ok(Self { c: 0, base, rng })
    }

    /// Records one event: increments the register with probability `b^{−c}`.
    pub fn increment(&mut self) {
        let p = self.base.powi(-(self.c as i32));
        if self.rng.random::<f64>() < p {
            self.c += 1;
        }
    }

    /// Unbiased estimate of the number of events recorded: `(b^c − 1)/(b − 1)`.
    pub fn estimate(&self) -> f64 {
        (self.base.powi(self.c as i32) - 1.0) / (self.base - 1.0)
    }

    /// The raw logarithmic register value `c`.
    #[inline]
    pub fn register(&self) -> u32 {
        self.c
    }

    /// Base `b`.
    #[inline]
    pub fn base(&self) -> f64 {
        self.base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_base() {
        assert!(MorrisCounter::new(1.0).is_err());
        assert!(MorrisCounter::new(0.5).is_err());
        assert!(MorrisCounter::new(f64::INFINITY).is_err());
        assert!(MorrisCounter::new(2.0).is_ok());
    }

    #[test]
    fn zero_events_estimates_zero() {
        let m = MorrisCounter::with_seed(2.0, 1).unwrap();
        assert_eq!(m.estimate(), 0.0);
        assert_eq!(m.register(), 0);
    }

    #[test]
    fn register_grows_logarithmically() {
        // After 100_000 base-2 increments the register is ≈ log2(100_000) ≈ 17, far below 100_000.
        let mut m = MorrisCounter::with_seed(2.0, 7).unwrap();
        for _ in 0..100_000 {
            m.increment();
        }
        assert!(m.register() < 25, "register {}", m.register());
        // The estimate should still be in the right ballpark for a single base-2 run (~71% std).
        let est = m.estimate();
        assert!(est > 20_000.0 && est < 500_000.0, "estimate {est}");
    }

    #[test]
    fn unbiased_over_many_seeds() {
        let trials = 400u64;
        let n = 3000;
        let mut sum = 0.0;
        for seed in 0..trials {
            let mut m = MorrisCounter::with_seed(2.0, seed).unwrap();
            for _ in 0..n {
                m.increment();
            }
            sum += m.estimate();
        }
        let mean = sum / trials as f64;
        assert!((mean - n as f64).abs() < 0.08 * n as f64, "mean {mean}");
    }

    #[test]
    fn smaller_base_reduces_variance() {
        // Relative variance is (b−1)/2, so a base near 1 is far more accurate than base 2.
        fn empirical_rel_std(base: f64, n: usize, trials: u64) -> f64 {
            let mut ests = Vec::new();
            for seed in 0..trials {
                let mut m = MorrisCounter::with_seed(base, seed + 1000).unwrap();
                for _ in 0..n {
                    m.increment();
                }
                ests.push(m.estimate());
            }
            let mean = ests.iter().sum::<f64>() / trials as f64;
            let var = ests.iter().map(|e| (e - mean).powi(2)).sum::<f64>() / trials as f64;
            var.sqrt() / mean
        }
        let std_big = empirical_rel_std(2.0, 4000, 300);
        let std_small = empirical_rel_std(1.05, 4000, 300);
        assert!(
            std_small < std_big,
            "small-base std {std_small} should beat base-2 std {std_big}"
        );
    }
}
