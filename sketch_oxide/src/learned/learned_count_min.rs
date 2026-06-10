//! Learned Count-Min — an oracle-augmented frequency sketch.
//!
//! Plain Count-Min's error is dominated by collisions with *heavy* keys. If a model can
//! predict which keys are heavy, those keys can be given **exact** counters while everything
//! else shares a smaller Count-Min, removing the heavy-key collision error and sharpening
//! the tail (Hsu, Indyk, Katabi & Vakilian, ICLR 2019; Aamand et al.). The prediction is
//! supplied through the shared [`Oracle`](crate::learned::Oracle) interface.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use crate::learned::{Oracle, Score};
use std::collections::HashMap;

/// A Count-Min sketch with an exact side-table for oracle-predicted heavy keys.
///
/// # Example
/// ```
/// use sketch_oxide::learned::{LearnedCountMin, ClosureOracle};
///
/// // Oracle marks the single key "hot" as heavy (score 1.0), others 0.0.
/// let oracle = ClosureOracle::new(|k: &[u8]| if k == b"hot" { 1.0 } else { 0.0 });
/// let mut cm = LearnedCountMin::new(4, 256, 0.5, 1024).unwrap();
///
/// cm.update(b"hot", 1000, &oracle);
/// for i in 0..500u64 { cm.update(&i.to_le_bytes(), 1, &oracle); }
///
/// // "hot" is counted exactly — no collision inflation.
/// assert_eq!(cm.estimate(b"hot"), 1000);
/// ```
#[derive(Debug, Clone)]
pub struct LearnedCountMin {
    depth: usize,
    width: usize,
    counters: Vec<u64>,
    /// Exact counts for predicted-heavy keys.
    heavy: HashMap<Vec<u8>, u64>,
    /// Oracle score at/above which a key is treated as heavy.
    threshold: Score,
    /// Maximum number of distinct heavy keys to store exactly.
    heavy_capacity: usize,
}

impl LearnedCountMin {
    /// Creates a learned Count-Min.
    ///
    /// * `depth`, `width` — the Count-Min back-end for non-heavy keys.
    /// * `threshold` — oracle score at/above which a key is routed to the exact table.
    /// * `heavy_capacity` — cap on distinct exact (heavy) keys.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `depth` or `width` is 0.
    pub fn new(
        depth: usize,
        width: usize,
        threshold: Score,
        heavy_capacity: usize,
    ) -> Result<Self> {
        if depth == 0 || width == 0 {
            return Err(SketchError::InvalidParameter {
                param: if depth == 0 { "depth" } else { "width" }.to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            depth,
            width,
            counters: vec![0u64; depth * width],
            heavy: HashMap::new(),
            threshold,
            heavy_capacity,
        })
    }

    #[inline]
    fn column(&self, item: &[u8], r: usize) -> usize {
        (xxhash(item, r as u64) % self.width as u64) as usize
    }

    fn cm_update(&mut self, item: &[u8], count: u64) {
        for r in 0..self.depth {
            let c = self.column(item, r);
            self.counters[r * self.width + c] += count;
        }
    }

    fn cm_estimate(&self, item: &[u8]) -> u64 {
        (0..self.depth)
            .map(|r| self.counters[r * self.width + self.column(item, r)])
            .min()
            .unwrap_or(0)
    }

    /// Adds `count` occurrences of `item`, routing it to the exact table if the oracle
    /// predicts it heavy (and capacity allows), else to the Count-Min back-end.
    pub fn update<O: Oracle + ?Sized>(&mut self, item: &[u8], count: u64, oracle: &O) {
        let heavy = oracle.score(item) >= self.threshold;
        if heavy && (self.heavy.contains_key(item) || self.heavy.len() < self.heavy_capacity) {
            *self.heavy.entry(item.to_vec()).or_insert(0) += count;
        } else {
            self.cm_update(item, count);
        }
    }

    /// Estimates the count of `item`: exact if it is a stored heavy key, otherwise the
    /// Count-Min estimate. No oracle call is needed — heavy keys are looked up directly.
    pub fn estimate(&self, item: &[u8]) -> u64 {
        if let Some(&c) = self.heavy.get(item) {
            c
        } else {
            self.cm_estimate(item)
        }
    }

    /// Number of keys stored exactly.
    pub fn heavy_len(&self) -> usize {
        self.heavy.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learned::ClosureOracle;

    fn heavy_oracle(heavy_keys: &'static [&'static [u8]]) -> ClosureOracle<impl Fn(&[u8]) -> f64> {
        ClosureOracle::new(
            move |k: &[u8]| {
                if heavy_keys.contains(&k) {
                    1.0
                } else {
                    0.0
                }
            },
        )
    }

    #[test]
    fn rejects_zero_dims() {
        assert!(LearnedCountMin::new(0, 10, 0.5, 100).is_err());
        assert!(LearnedCountMin::new(4, 0, 0.5, 100).is_err());
        assert!(LearnedCountMin::new(4, 10, 0.5, 100).is_ok());
    }

    #[test]
    fn heavy_key_is_exact() {
        let oracle = heavy_oracle(&[b"hot"]);
        let mut cm = LearnedCountMin::new(4, 64, 0.5, 1024).unwrap();
        cm.update(b"hot", 5000, &oracle);
        // Pile collisions onto the small Count-Min; "hot" stays exact.
        for i in 0..2000u64 {
            cm.update(&i.to_le_bytes(), 3, &oracle);
        }
        assert_eq!(cm.estimate(b"hot"), 5000);
        assert_eq!(cm.heavy_len(), 1);
    }

    #[test]
    fn improves_over_plain_count_min_for_heavy_keys() {
        // With heavy keys exact, a cold key's estimate is not inflated by the heavy keys.
        let oracle = heavy_oracle(&[b"h1", b"h2"]);
        let mut cm = LearnedCountMin::new(4, 256, 0.5, 1024).unwrap();
        cm.update(b"h1", 100_000, &oracle);
        cm.update(b"h2", 100_000, &oracle);
        cm.update(b"cold", 1, &oracle);
        // cold's estimate is at most a few (no heavy-key collision inflation).
        assert!(
            cm.estimate(b"cold") < 50,
            "cold estimate {}",
            cm.estimate(b"cold")
        );
    }

    #[test]
    fn non_heavy_keys_use_count_min() {
        let oracle = heavy_oracle(&[]); // nothing is heavy
        let mut cm = LearnedCountMin::new(5, 2048, 0.5, 1024).unwrap();
        for _ in 0..200 {
            cm.update(b"x", 1, &oracle);
        }
        assert_eq!(cm.heavy_len(), 0);
        let e = cm.estimate(b"x");
        assert!((200..220).contains(&e), "estimate {e}");
    }

    #[test]
    fn heavy_capacity_overflow_falls_back_to_cm() {
        let oracle = heavy_oracle(&[b"a", b"b", b"c"]);
        let mut cm = LearnedCountMin::new(4, 256, 0.5, 2).unwrap(); // cap 2
        cm.update(b"a", 10, &oracle);
        cm.update(b"b", 10, &oracle);
        cm.update(b"c", 10, &oracle); // over capacity -> Count-Min
        assert_eq!(cm.heavy_len(), 2);
        // "c" still estimable (via CM), at least its true count.
        assert!(cm.estimate(b"c") >= 10);
    }
}
