//! Reservoir sampling with Algorithm L — the optimal-skip variant.
//!
//! Algorithm R (the textbook reservoir, [`ReservoirSampling`](crate::sampling::ReservoirSampling))
//! draws a random number for *every* item, so it does `O(n)` RNG work over a stream of length `n`.
//! Algorithm L (Li, "Reservoir-Sampling Algorithms of Time Complexity O(n(1+log(N/n)))", ACM TOMS
//! 1994) keeps the identical uniform guarantee — after seeing `n` items every item is in the
//! reservoir with probability `k/n` — but draws only `O(k·(1 + log(n/k)))` random numbers by
//! computing, after each acceptance, an *exponential jump* over the run of items that are certain
//! to be rejected. For a long stream that is dramatically fewer RNG calls.
//!
//! The public API matches [`ReservoirSampling`](crate::sampling::ReservoirSampling); only the
//! internals skip.

use crate::common::SketchError;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

/// Uniform reservoir sampler using Algorithm L (exponential-jump skipping).
///
/// # Example
/// ```
/// use sketch_oxide::sampling::ReservoirSamplingL;
///
/// let mut r = ReservoirSamplingL::with_seed(100, 42).unwrap();
/// for i in 0..100_000u64 { r.update(i); }
/// assert_eq!(r.len(), 100);          // reservoir filled to k
/// assert_eq!(r.count(), 100_000);    // saw the whole stream
/// ```
#[derive(Debug, Clone)]
pub struct ReservoirSamplingL<T: Clone> {
    k: usize,
    reservoir: Vec<T>,
    count: u64,
    rng: SmallRng,
    /// Algorithm-L jump weight `W`.
    w: f64,
    /// Index (0-based) of the next item that will be accepted into the reservoir.
    next_accept: u64,
}

impl<T: Clone> ReservoirSamplingL<T> {
    /// Creates a sampler with reservoir size `k`, seeded from the OS.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `k` is 0.
    pub fn new(k: usize) -> Result<Self, SketchError> {
        Self::build(k, SmallRng::from_os_rng())
    }

    /// Creates a reproducible sampler with reservoir size `k` and the given `seed`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `k` is 0.
    pub fn with_seed(k: usize, seed: u64) -> Result<Self, SketchError> {
        Self::build(k, SmallRng::seed_from_u64(seed))
    }

    fn build(k: usize, rng: SmallRng) -> Result<Self, SketchError> {
        if k == 0 {
            return Err(SketchError::InvalidParameter {
                param: "k".to_string(),
                value: "0".to_string(),
                constraint: "must be greater than 0".to_string(),
            });
        }
        Ok(Self {
            k,
            reservoir: Vec::with_capacity(k),
            count: 0,
            rng,
            w: 1.0,
            next_accept: 0,
        })
    }

    /// A random number in `(0, 1]` (never 0, so `ln` is finite).
    #[inline]
    fn unit(&mut self) -> f64 {
        let u: f64 = self.rng.random();
        if u > 0.0 { u } else { f64::MIN_POSITIVE }
    }

    /// Computes the jump to the next accepted index after a replacement.
    fn advance_jump(&mut self) {
        self.w *= (self.unit().ln() / self.k as f64).exp();
        let gap = (self.unit().ln() / (1.0 - self.w).ln()).floor();
        // gap is finite and >= 0 because w in (0,1); guard against overflow on absurd values.
        let gap = if gap.is_finite() && gap >= 0.0 {
            gap as u64
        } else {
            0
        };
        self.next_accept = self.next_accept.saturating_add(gap).saturating_add(1);
    }

    /// Feeds one item to the sampler.
    pub fn update(&mut self, item: T) {
        let i = self.count;
        self.count += 1;

        if (self.reservoir.len()) < self.k {
            self.reservoir.push(item);
            if self.reservoir.len() == self.k {
                // Reservoir just filled (indices 0..k-1). Initialise the first jump.
                self.w = (self.unit().ln() / self.k as f64).exp();
                self.next_accept = self.k as u64;
                let gap = (self.unit().ln() / (1.0 - self.w).ln()).floor();
                let gap = if gap.is_finite() && gap >= 0.0 {
                    gap as u64
                } else {
                    0
                };
                self.next_accept = self.next_accept.saturating_add(gap);
            }
            return;
        }

        if i == self.next_accept {
            let j = self.rng.random_range(0..self.k);
            self.reservoir[j] = item;
            self.advance_jump();
        }
        // Otherwise this item is in a skipped run: discarded in O(1) with no RNG.
    }

    /// The current sample (up to `k` items).
    pub fn sample(&self) -> &[T] {
        &self.reservoir
    }

    /// Consumes the sampler and returns the sampled items.
    pub fn into_sample(self) -> Vec<T> {
        self.reservoir
    }

    /// Whether the reservoir is empty.
    pub fn is_empty(&self) -> bool {
        self.reservoir.is_empty()
    }

    /// Number of items currently held (≤ `k`).
    pub fn len(&self) -> usize {
        self.reservoir.len()
    }

    /// Reservoir capacity `k`.
    pub fn capacity(&self) -> usize {
        self.k
    }

    /// Total number of items seen.
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Probability that the most recent item is currently in the reservoir (`k/n`).
    pub fn inclusion_probability(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            (self.k as f64 / self.count as f64).min(1.0)
        }
    }

    /// Clears the reservoir and counters (keeps the RNG state).
    pub fn clear(&mut self) {
        self.reservoir.clear();
        self.count = 0;
        self.w = 1.0;
        self.next_accept = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_k() {
        assert!(ReservoirSamplingL::<u64>::new(0).is_err());
        assert!(ReservoirSamplingL::<u64>::new(10).is_ok());
    }

    #[test]
    fn fills_to_k_and_counts_all() {
        let mut r = ReservoirSamplingL::with_seed(100, 7).unwrap();
        for i in 0..100_000u64 {
            r.update(i);
        }
        assert_eq!(r.len(), 100);
        assert_eq!(r.count(), 100_000);
    }

    #[test]
    fn holds_everything_below_capacity() {
        let mut r = ReservoirSamplingL::with_seed(50, 1).unwrap();
        for i in 0..30u64 {
            r.update(i);
        }
        assert_eq!(r.len(), 30);
        let mut s = r.into_sample();
        s.sort_unstable();
        assert_eq!(s, (0..30).collect::<Vec<_>>());
    }

    #[test]
    fn deterministic_with_seed() {
        let run = || {
            let mut r = ReservoirSamplingL::with_seed(20, 99).unwrap();
            for i in 0..10_000u64 {
                r.update(i);
            }
            r.into_sample()
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn sample_is_approximately_uniform() {
        // Mean of a uniform sample of 0..N should be near N/2. Averaged over several seeds to
        // keep the test stable while still exercising the jump arithmetic.
        let n = 200_000u64;
        let mut means = Vec::new();
        for seed in 0..8u64 {
            let mut r = ReservoirSamplingL::with_seed(2000, seed).unwrap();
            for i in 0..n {
                r.update(i);
            }
            let mean: f64 = r.sample().iter().map(|&x| x as f64).sum::<f64>() / r.len() as f64;
            means.push(mean);
        }
        let avg = means.iter().sum::<f64>() / means.len() as f64;
        let expected = (n as f64 - 1.0) / 2.0;
        // Within 8% of the stream mean — uniform sampling, no positional bias.
        assert!(
            (avg - expected).abs() < 0.08 * expected,
            "sample mean {avg} far from expected {expected}"
        );
    }

    #[test]
    fn clear_resets() {
        let mut r = ReservoirSamplingL::with_seed(10, 3).unwrap();
        for i in 0..1000u64 {
            r.update(i);
        }
        r.clear();
        assert!(r.is_empty());
        assert_eq!(r.count(), 0);
    }
}
