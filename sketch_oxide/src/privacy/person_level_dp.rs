//! Person-level differential privacy via bounded user contribution (Wilson, Zhang, Lam, Desfontaines,
//! Simmons-Marengo & Gipson, "Differentially Private SQL with Bounded User Contribution", PETS 2020).
//!
//! Standard DP histograms protect a single *record*, but one **person** may contribute many records,
//! so record-level noise leaks at the person level. Person-level (user-level) DP fixes this by
//! **bounding each user's contribution** before aggregating:
//!
//! * **`L0` (cross-group bound)** — a user may influence at most `L0` distinct keys/groups; extra keys
//!   from that user are dropped.
//! * **`L∞` (per-group bound)** — a user's running total for any one key is clipped to `±L∞`.
//!
//! After bounding, one user changes the released histogram by at most `L0` keys, each by at most `L∞`,
//! so its **L1 sensitivity is `L0·L∞`**. Releasing each key's sum with discrete-Laplace noise of scale
//! `L0·L∞/ε` therefore satisfies **`ε`-DP at the person level**.
//!
//! Noise uses the crate's discrete (snapping-safe) [`laplace_mechanism`](crate::privacy::mechanisms)
//! over a **caller-supplied CSPRNG** — never a fast non-cryptographic RNG.

use crate::common::{Result, SketchError};
use crate::privacy::mechanisms::laplace_mechanism;
use rand::Rng;
use std::collections::HashMap;

/// A person-level DP histogram aggregator over `u64` user ids and `u64` keys with `i64` values.
///
/// # Example
/// ```
/// use sketch_oxide::privacy::PersonLevelDp;
/// use rand::{rngs::StdRng, SeedableRng};
///
/// // Each user may touch at most 1 key, contributing at most ±1 to it; ε = 1.0.
/// let mut agg = PersonLevelDp::new(1, 1, 1.0).unwrap();
/// for user in 0..2000u64 {
///     agg.add(user, 7, 1); // 2000 distinct users all vote for key 7
/// }
/// let mut rng = StdRng::seed_from_u64(42);
/// let released = agg.release(&mut rng).unwrap();
/// let (_, noisy) = released.iter().find(|(k, _)| *k == 7).unwrap();
/// // The noisy count is close to the true 2000 (sensitivity L0·L∞ = 1, so noise scale 1/ε = 1).
/// assert!((noisy - 2000).abs() < 40, "noisy {noisy}");
/// ```
#[derive(Debug, Clone)]
pub struct PersonLevelDp {
    l0: usize,
    l_inf: i64,
    epsilon: f64,
    contributions: HashMap<(u64, u64), i64>, // (user, key) -> clipped running sum
    user_key_count: HashMap<u64, usize>,     // user -> number of distinct keys used
}

impl PersonLevelDp {
    /// Creates an aggregator bounding each user to `max_keys_per_user` (`L0`) distinct keys and
    /// `±max_per_key` (`L∞`) per key, releasing with `ε`-DP.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `max_keys_per_user == 0`, `max_per_key <= 0`, or
    /// `epsilon <= 0`.
    pub fn new(max_keys_per_user: usize, max_per_key: i64, epsilon: f64) -> Result<Self> {
        if max_keys_per_user == 0 {
            return Err(SketchError::InvalidParameter {
                param: "max_keys_per_user".to_string(),
                value: "0".to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        if max_per_key <= 0 {
            return Err(SketchError::InvalidParameter {
                param: "max_per_key".to_string(),
                value: max_per_key.to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        if !epsilon.is_finite() || epsilon <= 0.0 {
            return Err(SketchError::InvalidParameter {
                param: "epsilon".to_string(),
                value: epsilon.to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            l0: max_keys_per_user,
            l_inf: max_per_key,
            epsilon,
            contributions: HashMap::new(),
            user_key_count: HashMap::new(),
        })
    }

    /// Adds `value` from `user` toward `key`, applying the `L0` and `L∞` contribution bounds. A new key
    /// from a user already at the `L0` limit is dropped; the per-`(user, key)` total is clipped to
    /// `±L∞`.
    pub fn add(&mut self, user: u64, key: u64, value: i64) {
        match self.contributions.get_mut(&(user, key)) {
            Some(sum) => {
                *sum = sum.saturating_add(value).clamp(-self.l_inf, self.l_inf);
            }
            None => {
                let count = self.user_key_count.entry(user).or_insert(0);
                if *count >= self.l0 {
                    return; // user already influences L0 keys; drop this new key
                }
                *count += 1;
                self.contributions
                    .insert((user, key), value.clamp(-self.l_inf, self.l_inf));
            }
        }
    }

    /// The exact (non-private) bounded histogram, sorted by key. **Not** differentially private — for
    /// testing and accounting only.
    pub fn true_histogram(&self) -> Vec<(u64, i64)> {
        let mut hist: HashMap<u64, i64> = HashMap::new();
        for (&(_, key), &v) in &self.contributions {
            *hist.entry(key).or_insert(0) += v;
        }
        let mut out: Vec<(u64, i64)> = hist.into_iter().collect();
        out.sort_unstable_by_key(|&(k, _)| k);
        out
    }

    /// L1 sensitivity of the release: one user changes at most `L0` keys by at most `L∞` each.
    #[inline]
    pub fn sensitivity(&self) -> u64 {
        (self.l0 as u64) * (self.l_inf as u64)
    }

    /// Releases the `ε`-DP histogram: each key's bounded sum plus independent discrete-Laplace noise of
    /// scale `L0·L∞/ε`. Keys are returned sorted. `rng` must be a CSPRNG (e.g. `StdRng`).
    ///
    /// # Errors
    /// Propagates [`SketchError::InvalidParameter`] from the noise mechanism.
    pub fn release<R: Rng + ?Sized>(&self, rng: &mut R) -> Result<Vec<(u64, i64)>> {
        let sensitivity = self.sensitivity();
        let mut out = Vec::new();
        for (key, sum) in self.true_histogram() {
            let noisy = laplace_mechanism(rng, sum, sensitivity, self.epsilon)?;
            out.push((key, noisy));
        }
        Ok(out)
    }

    /// Number of distinct keys with at least one contribution.
    pub fn num_keys(&self) -> usize {
        self.true_histogram().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    fn rng() -> StdRng {
        StdRng::seed_from_u64(0xD1FF_0BEE_C0DE)
    }

    #[test]
    fn rejects_bad_params() {
        assert!(PersonLevelDp::new(0, 1, 1.0).is_err());
        assert!(PersonLevelDp::new(1, 0, 1.0).is_err());
        assert!(PersonLevelDp::new(1, -3, 1.0).is_err());
        assert!(PersonLevelDp::new(1, 1, 0.0).is_err());
        assert!(PersonLevelDp::new(1, 1, -1.0).is_err());
        assert!(PersonLevelDp::new(2, 5, 1.0).is_ok());
    }

    #[test]
    fn l0_bounds_distinct_keys_per_user() {
        // A user that touches 10 keys is bounded to L0 = 3 of them.
        let mut agg = PersonLevelDp::new(3, 100, 1.0).unwrap();
        for key in 0..10u64 {
            agg.add(1, key, 1);
        }
        assert_eq!(agg.num_keys(), 3, "only L0 distinct keys should survive");
        // Total mass is exactly L0 (each surviving key has value 1).
        let total: i64 = agg.true_histogram().iter().map(|&(_, v)| v).sum();
        assert_eq!(total, 3);
    }

    #[test]
    fn l_inf_clips_per_key_total() {
        let mut agg = PersonLevelDp::new(5, 10, 1.0).unwrap();
        agg.add(1, 7, 1000); // single huge contribution clipped to 10
        agg.add(2, 7, 8);
        agg.add(2, 7, 8); // user 2's running total 16 clipped to 10
        let hist = agg.true_histogram();
        assert_eq!(hist, vec![(7, 20)]); // 10 (user 1) + 10 (user 2)
    }

    #[test]
    fn sensitivity_is_l0_times_linf() {
        let agg = PersonLevelDp::new(4, 25, 1.0).unwrap();
        assert_eq!(agg.sensitivity(), 100);
    }

    #[test]
    fn release_is_close_to_truth_for_large_counts() {
        let mut agg = PersonLevelDp::new(1, 1, 1.0).unwrap();
        for user in 0..3000u64 {
            agg.add(user, 9, 1);
        }
        let released = agg.release(&mut rng()).unwrap();
        let (_, noisy) = released.iter().find(|&&(k, _)| k == 9).unwrap();
        // Sensitivity 1, scale 1/ε = 1 ⇒ noise is tiny relative to the count of 3000.
        assert!((noisy - 3000).abs() < 40, "noisy {noisy} far from 3000");
    }

    #[test]
    fn release_is_deterministic_given_the_rng() {
        let mut agg = PersonLevelDp::new(2, 10, 0.5).unwrap();
        for u in 0..100u64 {
            agg.add(u, u % 5, 3);
        }
        let a = agg.release(&mut rng()).unwrap();
        let b = agg.release(&mut rng()).unwrap();
        assert_eq!(a, b, "same seed ⇒ same released histogram");
    }
}
