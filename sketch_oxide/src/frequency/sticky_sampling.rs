//! Sticky Sampling — randomized approximate frequency counting (Manku & Motwani, VLDB 2002).
//!
//! Sticky Sampling is the randomized companion to [`LossyCounting`](crate::frequency::LossyCounting):
//! it answers "which items exceed an `s` fraction of the stream?" in space *independent of the stream
//! length* — `O((1/ε)·log(1/(s·δ)))` expected — at the cost of a `δ` failure probability. It keeps a
//! map of `(element, count)`; an element already in the map is always counted, while a new element is
//! admitted only with the current sampling probability `1/r`. The sampling rate `r` starts at 1 and
//! doubles as the stream grows (after `2t`, `4t`, `8t`, … elements, where `t = ⌈(1/ε)·ln(1/(s·δ))⌉`).
//! On each doubling the existing counts are *diminished* by a coin-tossing sweep — for each entry,
//! toss a fair coin repeatedly, decrementing the count once per tail until the first head — so the
//! retained counts stay consistent with the lower sampling rate.
//!
//! Guarantees (with `N` items seen): counts only ever **underestimate** the true frequency (`f ≤
//! true`, deterministically), and with probability `≥ 1 − δ` every element with true frequency `≥ sN`
//! is reported and no count is below `true − εN`. Compared to Lossy Counting, the space no longer
//! grows with `N`, but the bounds are probabilistic rather than worst-case.

use crate::common::{Result, SketchError};
use rand::Rng;
use std::collections::HashMap;
use std::hash::Hash;

/// Randomized approximate frequency counter over items of type `T`.
///
/// # Example
/// ```
/// use sketch_oxide::frequency::StickySampling;
///
/// let mut ss = StickySampling::with_seed(0.05, 0.01, 0.001, 42).unwrap();
/// // Key 0 is heavy (40%), key 1 moderate (20%), plus a singleton tail.
/// for _ in 0..4000 { ss.insert(0u64); }
/// for _ in 0..2000 { ss.insert(1u64); }
/// for i in 0..4000u64 { ss.insert(1000 + i); }
///
/// let heavy: Vec<_> = ss.query(0.10).into_iter().map(|(k, _)| k).collect();
/// assert!(heavy.contains(&0)); // 40% ≥ 10%
/// assert!(heavy.contains(&1)); // 20% ≥ 10%
/// ```
#[derive(Debug, Clone)]
pub struct StickySampling<T: Hash + Eq + Clone> {
    epsilon: f64,
    /// Threshold `t = ⌈(1/ε)·ln(1/(s·δ))⌉` controlling rate doublings.
    t: u64,
    /// Current sampling rate `r` (admit new items with probability `1/r`).
    r: u64,
    n: u64,
    entries: HashMap<T, u64>,
    rng: rand::rngs::SmallRng,
}

impl<T: Hash + Eq + Clone> StickySampling<T> {
    /// Creates a counter for support `s`, error `epsilon`, failure probability `delta`, seeded from
    /// the OS.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `s` or `delta` is not in `(0, 1)`, or `epsilon` is not in
    /// `(0, s)`.
    pub fn new(s: f64, epsilon: f64, delta: f64) -> Result<Self> {
        use rand::SeedableRng;
        Self::from_rng(s, epsilon, delta, rand::rngs::SmallRng::from_os_rng())
    }

    /// Creates a counter with a fixed RNG seed (reproducible).
    ///
    /// # Errors
    /// As [`StickySampling::new`].
    pub fn with_seed(s: f64, epsilon: f64, delta: f64, seed: u64) -> Result<Self> {
        use rand::SeedableRng;
        Self::from_rng(s, epsilon, delta, rand::rngs::SmallRng::seed_from_u64(seed))
    }

    fn from_rng(s: f64, epsilon: f64, delta: f64, rng: rand::rngs::SmallRng) -> Result<Self> {
        if !(s > 0.0 && s < 1.0) {
            return Err(SketchError::InvalidParameter {
                param: "s".to_string(),
                value: s.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }
        if !(epsilon > 0.0 && epsilon < s) {
            return Err(SketchError::InvalidParameter {
                param: "epsilon".to_string(),
                value: epsilon.to_string(),
                constraint: "must be in (0, s)".to_string(),
            });
        }
        if !(delta > 0.0 && delta < 1.0) {
            return Err(SketchError::InvalidParameter {
                param: "delta".to_string(),
                value: delta.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }
        let t = ((1.0 / epsilon) * (1.0 / (s * delta)).ln()).ceil() as u64;
        Ok(Self {
            epsilon,
            t: t.max(1),
            r: 1,
            n: 0,
            entries: HashMap::new(),
            rng,
        })
    }

    /// Sampling rate `r` appropriate to having seen `n` elements: 1 for `n ≤ 2t`, then doubling at
    /// each of `4t, 8t, 16t, …`.
    fn rate_for(&self, n: u64) -> u64 {
        if n <= 2 * self.t {
            return 1;
        }
        let mut r = 2u64;
        let n = n as u128;
        let t = self.t as u128;
        while (2 * r) as u128 * t < n {
            r *= 2;
        }
        r
    }

    /// Diminishes every stored count by a fair-coin sweep (one decrement per tail until the first
    /// head); entries that reach zero are dropped.
    fn sweep(&mut self) {
        let keys: Vec<T> = self.entries.keys().cloned().collect();
        for k in keys {
            let f = self.entries[&k];
            // Count tails before the first head.
            let mut tails = 0u64;
            while !self.rng.random::<bool>() {
                tails += 1;
                if tails >= f {
                    break;
                }
            }
            if tails >= f {
                self.entries.remove(&k);
            } else {
                self.entries.insert(k, f - tails);
            }
        }
    }

    /// Records one occurrence of `item`.
    pub fn insert(&mut self, item: T) {
        self.n += 1;
        // Advance the sampling rate, diminishing counts on each doubling.
        let target = self.rate_for(self.n);
        while self.r < target {
            self.sweep();
            self.r *= 2;
        }
        if let Some(f) = self.entries.get_mut(&item) {
            *f += 1;
        } else if self.r == 1 || self.rng.random_range(0..self.r) == 0 {
            // Admit a new element with probability 1/r.
            self.entries.insert(item, 1);
        }
    }

    /// Estimated frequency of `item` (0 if not tracked). Never exceeds the true frequency.
    pub fn estimate(&self, item: &T) -> u64 {
        self.entries.get(item).copied().unwrap_or(0)
    }

    /// All elements with estimated frequency `≥ (s − ε)·N` — the answer to "which items exceed an `s`
    /// fraction of the stream?".
    pub fn query(&self, s: f64) -> Vec<(T, u64)> {
        let threshold = ((s - self.epsilon) * self.n as f64).max(0.0);
        self.entries
            .iter()
            .filter(|(_, &f)| f as f64 >= threshold)
            .map(|(k, &f)| (k.clone(), f))
            .collect()
    }

    /// Total number of items observed (`N`).
    #[inline]
    pub fn count(&self) -> u64 {
        self.n
    }

    /// Number of elements currently tracked.
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no elements are currently tracked.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Current sampling rate `r`.
    #[inline]
    pub fn sampling_rate(&self) -> u64 {
        self.r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap as Map;

    #[test]
    fn rejects_bad_params() {
        assert!(StickySampling::<u64>::new(0.0, 0.01, 0.01).is_err());
        assert!(StickySampling::<u64>::new(1.0, 0.01, 0.01).is_err());
        assert!(StickySampling::<u64>::new(0.05, 0.05, 0.01).is_err()); // epsilon >= s
        assert!(StickySampling::<u64>::new(0.05, 0.0, 0.01).is_err());
        assert!(StickySampling::<u64>::new(0.05, 0.01, 0.0).is_err());
        assert!(StickySampling::<u64>::new(0.05, 0.01, 0.01).is_ok());
    }

    #[test]
    fn counts_never_overestimate() {
        let mut ss = StickySampling::with_seed(0.02, 0.005, 0.001, 7).unwrap();
        let mut truth: Map<u64, u64> = Map::new();
        for i in 0..50_000u64 {
            let key = match i % 100 {
                0..=39 => 0,
                40..=59 => 1,
                60..=69 => 2,
                _ => 1000 + i,
            };
            *truth.entry(key).or_default() += 1;
            ss.insert(key);
        }
        // Sticky Sampling counts a subset, so every estimate underestimates the truth.
        for (&k, &f) in &truth {
            assert!(
                ss.estimate(&k) <= f,
                "key {k}: est {} > truth {f}",
                ss.estimate(&k)
            );
        }
    }

    #[test]
    fn finds_all_heavy_hitters() {
        let mut ss = StickySampling::with_seed(0.05, 0.01, 0.0001, 99).unwrap();
        let mut truth: Map<u64, u64> = Map::new();
        for i in 0..60_000u64 {
            let key = match i % 100 {
                0..=39 => 0,  // 40%
                40..=59 => 1, // 20%
                60..=69 => 2, // 10%
                _ => 5000 + i,
            };
            *truth.entry(key).or_default() += 1;
            ss.insert(key);
        }
        let n = ss.count();
        let reported: std::collections::HashSet<u64> =
            ss.query(0.05).into_iter().map(|(k, _)| k).collect();
        // With δ tiny and a fixed seed, every true-≥ sN item is reported.
        for (&k, &f) in &truth {
            if f as f64 >= 0.05 * n as f64 {
                assert!(reported.contains(&k), "missed heavy hitter {k} (count {f})");
            }
        }
        assert!(reported.contains(&0) && reported.contains(&1) && reported.contains(&2));
    }

    #[test]
    fn sampling_rate_grows() {
        let mut ss = StickySampling::with_seed(0.1, 0.02, 0.01, 5).unwrap();
        for i in 0..200_000u64 {
            ss.insert(i % 7); // a few keys, long stream
        }
        // The sampling rate must have doubled past 1 on such a long stream.
        assert!(ss.sampling_rate() > 1, "rate {}", ss.sampling_rate());
    }

    #[test]
    fn empty_and_absent() {
        let ss = StickySampling::<u64>::with_seed(0.05, 0.01, 0.01, 1).unwrap();
        assert!(ss.is_empty());
        assert_eq!(ss.estimate(&42), 0);
        assert!(ss.query(0.1).is_empty());
    }
}
