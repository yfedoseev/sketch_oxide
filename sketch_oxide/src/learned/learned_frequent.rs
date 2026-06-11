//! LA-Misra-Gries — learning-augmented frequency estimation.
//!
//! Classical frequency sketches spread their error uniformly, so even the heaviest items carry the
//! Misra–Gries offset. Learning-augmented estimation (Hsu, Indyk, Katabi & Vakilian, "Learning-Based
//! Frequency Estimation Algorithms", ICLR 2019; Aamand et al.) uses a cheap **oracle** that predicts
//! which keys are heavy and gives those keys their *own exact counters*, routing only the predicted-
//! light tail into a shared [`FrequentItems`](crate::frequency::FrequentItems) (Misra–Gries) sketch.
//! When the oracle is good, the heavy hitters — the answers people actually care about — are exact,
//! and the bounded sketch error falls only on the unimportant tail.
//!
//! The oracle is any [`Oracle`](crate::learned::Oracle): a learned model, a precomputed table, or a
//! closure. A key is treated as heavy when its score is at least `threshold`.

use crate::common::hash::xxhash;
use crate::common::Result;
use crate::frequency::FrequentItems;
use crate::learned::oracle::{Oracle, Score};
use std::collections::HashMap;

/// A learning-augmented frequency estimator: exact counts for predicted-heavy keys, a Misra–Gries
/// sketch for the rest.
///
/// # Example
/// ```
/// use sketch_oxide::learned::{LearnedFrequent, ClosureOracle};
///
/// // Oracle "knows" that keys starting with b'H' are heavy.
/// let oracle = ClosureOracle::new(|k: &[u8]| if k.first() == Some(&b'H') { 1.0 } else { 0.0 });
/// let mut lf = LearnedFrequent::new(oracle, 0.5, 64).unwrap();
///
/// for _ in 0..1000 { lf.update(b"Heavy"); }     // predicted heavy → exact
/// for i in 0..500u64 { lf.update(&i.to_le_bytes()); } // light tail → sketched
///
/// assert_eq!(lf.estimate(b"Heavy"), 1000);       // exact, no Misra–Gries offset
/// ```
#[derive(Debug, Clone)]
pub struct LearnedFrequent<O: Oracle> {
    oracle: O,
    threshold: Score,
    /// Exact counters for predicted-heavy keys.
    heavy: HashMap<Vec<u8>, u64>,
    /// Misra–Gries over a hash of the predicted-light keys.
    tail: FrequentItems<u64>,
    n: u64,
}

impl<O: Oracle> LearnedFrequent<O> {
    /// Creates an estimator using `oracle`, treating a key as heavy when its score is at least
    /// `threshold`, with a Misra–Gries capacity of `tail_capacity` for the light keys.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `tail_capacity` is 0.
    pub fn new(oracle: O, threshold: Score, tail_capacity: usize) -> Result<Self> {
        Ok(Self {
            oracle,
            threshold,
            heavy: HashMap::new(),
            tail: FrequentItems::new(tail_capacity)?,
            n: 0,
        })
    }

    #[inline]
    fn is_heavy(&self, key: &[u8]) -> bool {
        self.oracle.score(key) >= self.threshold
    }

    /// Records one occurrence of `key`.
    pub fn update(&mut self, key: &[u8]) {
        self.n += 1;
        if self.is_heavy(key) {
            *self.heavy.entry(key.to_vec()).or_insert(0) += 1;
        } else {
            self.tail.update(xxhash(key, 0));
        }
    }

    /// Estimated frequency of `key`: exact for predicted-heavy keys, the Misra–Gries estimate for
    /// the rest.
    pub fn estimate(&self, key: &[u8]) -> u64 {
        if self.is_heavy(key) {
            self.heavy.get(key).copied().unwrap_or(0)
        } else {
            self.tail
                .get_estimate(&xxhash(key, 0))
                .map_or(0, |(count, _)| count)
        }
    }

    /// The predicted-heavy keys and their exact counts, sorted by count descending.
    pub fn heavy_hitters(&self) -> Vec<(Vec<u8>, u64)> {
        let mut out: Vec<(Vec<u8>, u64)> =
            self.heavy.iter().map(|(k, &c)| (k.clone(), c)).collect();
        out.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
        out
    }

    /// Number of distinct predicted-heavy keys tracked exactly.
    #[inline]
    pub fn num_heavy(&self) -> usize {
        self.heavy.len()
    }

    /// Total number of updates processed.
    #[inline]
    pub fn total_updates(&self) -> u64 {
        self.n
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learned::ClosureOracle;

    fn heavy_if_prefix_h() -> ClosureOracle<impl Fn(&[u8]) -> Score> {
        ClosureOracle::new(|k: &[u8]| if k.first() == Some(&b'H') { 1.0 } else { 0.0 })
    }

    #[test]
    fn rejects_zero_capacity() {
        assert!(LearnedFrequent::new(heavy_if_prefix_h(), 0.5, 0).is_err());
        assert!(LearnedFrequent::new(heavy_if_prefix_h(), 0.5, 16).is_ok());
    }

    #[test]
    fn predicted_heavy_keys_are_exact() {
        let mut lf = LearnedFrequent::new(heavy_if_prefix_h(), 0.5, 32).unwrap();
        for _ in 0..1234 {
            lf.update(b"Hot");
        }
        // Pollute the tail heavily.
        for i in 0..10_000u64 {
            lf.update(&i.to_le_bytes());
        }
        // The oracle-predicted-heavy key carries no Misra-Gries offset.
        assert_eq!(lf.estimate(b"Hot"), 1234);
    }

    #[test]
    fn light_keys_use_the_sketch() {
        let mut lf = LearnedFrequent::new(heavy_if_prefix_h(), 0.5, 64).unwrap();
        for _ in 0..500 {
            lf.update(b"light"); // not 'H' → tail
        }
        let est = lf.estimate(b"light");
        // Misra-Gries never underestimates by more than the offset and never overestimates.
        assert!(est <= 500 && est >= 400, "tail estimate {est}");
    }

    #[test]
    fn heavy_and_light_do_not_mix() {
        let mut lf = LearnedFrequent::new(heavy_if_prefix_h(), 0.5, 64).unwrap();
        for _ in 0..100 {
            lf.update(b"Heavy1");
            lf.update(b"Heavy2");
        }
        assert_eq!(lf.estimate(b"Heavy1"), 100);
        assert_eq!(lf.estimate(b"Heavy2"), 100);
        assert_eq!(lf.num_heavy(), 2);
        // A light key that was never inserted reads ~0.
        assert!(lf.estimate(b"never") < 50);
    }

    #[test]
    fn heavy_hitters_sorted_exact() {
        let mut lf = LearnedFrequent::new(heavy_if_prefix_h(), 0.5, 64).unwrap();
        for _ in 0..300 {
            lf.update(b"Hbig");
        }
        for _ in 0..50 {
            lf.update(b"Hsmall");
        }
        let hh = lf.heavy_hitters();
        assert_eq!(hh[0], (b"Hbig".to_vec(), 300));
        assert_eq!(hh[1], (b"Hsmall".to_vec(), 50));
    }

    #[test]
    fn beats_plain_misra_gries_on_heavy_key() {
        // With a small tail capacity, a key the oracle flags heavy is exact, whereas pushing it
        // through the tail Misra-Gries (capacity 4) under heavy contention would be lossy.
        let mut lf = LearnedFrequent::new(heavy_if_prefix_h(), 0.5, 4).unwrap();
        for _ in 0..1000 {
            lf.update(b"Hkey");
        }
        for i in 0..5000u64 {
            lf.update(&i.to_le_bytes());
        }
        assert_eq!(
            lf.estimate(b"Hkey"),
            1000,
            "learned heavy estimate must be exact"
        );
    }
}
