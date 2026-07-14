//! Differentially private continual counting (binary-tree mechanism).
//!
//! Releasing a running count after *every* event — a continual observation — would, with the
//! naive Laplace mechanism, require noise growing linearly in the number of releases. The
//! **binary-tree mechanism** (Dwork, Naor, Pitassi & Rothblum 2010; Chan, Shi & Song 2011)
//! reduces that to polylogarithmic error: it maintains `O(log T)` partial sums over a binary
//! tree of time, so each event affects only `O(log T)` of them and each prefix-sum query
//! reads only `O(log T)`. Adding discrete-Laplace noise of scale `log2(T) / ε` to each
//! partial sum yields an `ε`-DP stream of running counts with error `O((log T)^1.5 / ε)`.
//!
//! This is the streaming form: [`insert`](DpContinualCounter::insert) one event at a time and
//! read the noisy running total with [`count`](DpContinualCounter::count).

use crate::common::{Result, SketchError};
use crate::privacy::mechanisms::discrete_laplace;
use rand::Rng;

/// An `ε`-differentially private running counter over a stream of up to `T` events.
///
/// # Example
/// ```
/// use sketch_oxide::privacy::DpContinualCounter;
/// use rand::{rngs::StdRng, SeedableRng};
///
/// let mut rng = StdRng::seed_from_u64(1);
/// let mut counter = DpContinualCounter::new(1.0, 1 << 16).unwrap();
/// for _ in 0..10_000 {
///     counter.insert(1, &mut rng).unwrap();
/// }
/// // Running count is ~10000, off only by polylogarithmic DP noise.
/// assert!((counter.count() - 10_000).abs() < 500);
/// ```
#[derive(Debug, Clone)]
pub struct DpContinualCounter {
    epsilon: f64,
    max_steps: u64,
    noise_scale: f64,
    /// Current number of events inserted.
    t: u64,
    /// Noisy partial sums, one slot per binary-tree level.
    noisy: Vec<i64>,
    /// True partial sums (needed to roll lower levels up into a completing level).
    exact: Vec<i64>,
}

impl DpContinualCounter {
    /// Creates a counter for up to `max_steps` events with privacy budget `epsilon`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `epsilon <= 0` or `max_steps < 2`.
    pub fn new(epsilon: f64, max_steps: u64) -> Result<Self> {
        if !epsilon.is_finite() || epsilon <= 0.0 {
            return Err(SketchError::InvalidParameter {
                param: "epsilon".to_string(),
                value: epsilon.to_string(),
                constraint: "must be a finite value > 0".to_string(),
            });
        }
        if max_steps < 2 {
            return Err(SketchError::InvalidParameter {
                param: "max_steps".to_string(),
                value: max_steps.to_string(),
                constraint: "must be >= 2".to_string(),
            });
        }
        // Number of tree levels = ceil(log2(max_steps)); each event touches one level.
        let levels = (64 - (max_steps - 1).leading_zeros()) as usize;
        let noise_scale = levels as f64 / epsilon;
        Ok(Self {
            epsilon,
            max_steps,
            noise_scale,
            t: 0,
            noisy: vec![0; levels + 1],
            exact: vec![0; levels + 1],
        })
    }

    /// Inserts one event with the given integer `value` (use `1` to count occurrences),
    /// updating the noisy partial sums. The RNG **must** be a CSPRNG for the DP guarantee.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if more than `max_steps` events are inserted, or the
    /// noise sampler fails.
    pub fn insert<R: Rng + ?Sized>(&mut self, value: i64, rng: &mut R) -> Result<()> {
        if self.t >= self.max_steps {
            return Err(SketchError::InvalidParameter {
                param: "steps".to_string(),
                value: (self.t + 1).to_string(),
                constraint: format!("exceeds max_steps {}", self.max_steps),
            });
        }
        self.t += 1;
        // The completing level is the position of the lowest set bit of t.
        let i = self.t.trailing_zeros() as usize;
        // Roll all lower-level partial sums (plus this value) up into level i.
        let mut psum = value;
        for j in 0..i {
            psum += self.exact[j];
            self.exact[j] = 0;
            self.noisy[j] = 0;
        }
        self.exact[i] = psum;
        self.noisy[i] = psum + discrete_laplace(rng, self.noise_scale)?;
        Ok(())
    }

    /// The current `ε`-DP running count (noisy prefix sum over all events so far).
    pub fn count(&self) -> i64 {
        // Sum the noisy partial sums at the set-bit levels of t.
        let mut total = 0i64;
        let mut bits = self.t;
        while bits != 0 {
            let j = bits.trailing_zeros() as usize;
            total += self.noisy[j];
            bits &= bits - 1;
        }
        total
    }

    /// The current count clamped to be non-negative.
    pub fn count_clamped(&self) -> i64 {
        self.count().max(0)
    }

    /// Number of events inserted so far.
    #[inline]
    pub fn steps(&self) -> u64 {
        self.t
    }

    /// The privacy budget.
    #[inline]
    pub fn epsilon(&self) -> f64 {
        self.epsilon
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    fn rng() -> StdRng {
        StdRng::seed_from_u64(0xABCDEF)
    }

    #[test]
    fn rejects_bad_params() {
        assert!(DpContinualCounter::new(0.0, 1024).is_err());
        assert!(DpContinualCounter::new(-1.0, 1024).is_err());
        assert!(DpContinualCounter::new(1.0, 1).is_err());
        assert!(DpContinualCounter::new(1.0, 1024).is_ok());
    }

    #[test]
    fn running_count_tracks_truth() {
        let mut r = rng();
        let mut c = DpContinualCounter::new(1.0, 1 << 16).unwrap();
        for _ in 0..10_000 {
            c.insert(1, &mut r).unwrap();
        }
        assert_eq!(c.steps(), 10_000);
        // Polylog error: |noisy - 10000| should be small relative to 10000.
        assert!((c.count() - 10_000).abs() < 500, "count {}", c.count());
    }

    #[test]
    fn count_is_monotone_in_expectation() {
        let mut r = rng();
        let mut c = DpContinualCounter::new(2.0, 1 << 12).unwrap();
        for _ in 0..100 {
            c.insert(1, &mut r).unwrap();
        }
        let mid = c.count();
        for _ in 0..900 {
            c.insert(1, &mut r).unwrap();
        }
        let end = c.count();
        assert!(end > mid, "running count should grow: {mid} -> {end}");
        assert!((end - 1000).abs() < 200, "final count {end}");
    }

    #[test]
    fn weighted_inserts() {
        let mut r = rng();
        let mut c = DpContinualCounter::new(1.0, 1 << 12).unwrap();
        for _ in 0..1000 {
            c.insert(5, &mut r).unwrap();
        }
        assert!((c.count() - 5000).abs() < 500, "count {}", c.count());
    }

    #[test]
    fn errors_past_max_steps() {
        let mut r = rng();
        let mut c = DpContinualCounter::new(1.0, 4).unwrap();
        for _ in 0..4 {
            c.insert(1, &mut r).unwrap();
        }
        assert!(c.insert(1, &mut r).is_err());
    }

    #[test]
    fn noise_is_present() {
        // Two runs with different seeds give different noisy counts for the same data.
        let mut a = DpContinualCounter::new(0.5, 1 << 16).unwrap();
        let mut b = DpContinualCounter::new(0.5, 1 << 16).unwrap();
        let mut ra = StdRng::seed_from_u64(1);
        let mut rb = StdRng::seed_from_u64(2);
        for _ in 0..5000 {
            a.insert(1, &mut ra).unwrap();
            b.insert(1, &mut rb).unwrap();
        }
        assert_ne!(a.count(), b.count(), "independent noise should differ");
    }
}
