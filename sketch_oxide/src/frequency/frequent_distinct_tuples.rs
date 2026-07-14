//! Frequent Distinct Tuples — keys ranked by how many *distinct* values they associate with.
//!
//! Ordinary heavy-hitter sketches rank keys by occurrence count. Frequent Distinct Tuples (FDT, part
//! of the Apache DataSketches framework) answers a different question: *which keys are associated with
//! the most distinct values?* — e.g. which source IPs contact the most distinct destinations
//! (super-spreader detection), or which users touch the most distinct resources. It composes a
//! Space-Saving-style monitored set with a per-key [`HyperLogLog`](crate::cardinality::HyperLogLog):
//! each monitored key owns an HLL counting its distinct associated values, and when the monitored set
//! is full the key with the smallest distinct estimate is evicted to make room.
//!
//! Because the per-key cardinality is an HLL estimate, ranking is approximate, but keys whose distinct
//! cardinalities differ by more than the HLL error are ordered reliably. Query the leaders with
//! [`FrequentDistinctTuples::top_k`].

use crate::cardinality::HyperLogLog;
use crate::common::{Result, Sketch, SketchError};
use std::collections::HashMap;
use std::hash::Hash;

/// Tracks up to `capacity` keys, each with an HLL over its distinct associated values.
///
/// # Example
/// ```
/// use sketch_oxide::frequency::FrequentDistinctTuples;
///
/// let mut fdt = FrequentDistinctTuples::new(64, 12).unwrap();
/// // Key 1 sees 2000 distinct values, key 2 sees 500, plus a tail of small keys.
/// for v in 0..2000u64 { fdt.update(1u64, &v); }
/// for v in 0..500u64 { fdt.update(2u64, &v); }
/// for k in 100..1000u64 { fdt.update(k, &0u64); } // many keys, 1 distinct value each
///
/// let top = fdt.top_k(2);
/// assert_eq!(top[0].0, 1); // most distinct values
/// assert_eq!(top[1].0, 2);
/// ```
#[derive(Debug, Clone)]
pub struct FrequentDistinctTuples<K: Hash + Eq + Clone> {
    capacity: usize,
    precision: u8,
    monitored: HashMap<K, HyperLogLog>,
}

impl<K: Hash + Eq + Clone> FrequentDistinctTuples<K> {
    /// Creates a tracker monitoring up to `capacity` keys, each with an HLL of the given `precision`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `capacity` is 0; propagates HLL precision validation.
    pub fn new(capacity: usize, precision: u8) -> Result<Self> {
        if capacity == 0 {
            return Err(SketchError::InvalidParameter {
                param: "capacity".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        // Validate precision once via a throwaway sketch.
        HyperLogLog::new(precision)?;
        Ok(Self {
            capacity,
            precision,
            monitored: HashMap::new(),
        })
    }

    /// Records that `key` is associated with `value`.
    pub fn update<V: Hash>(&mut self, key: K, value: &V) {
        if let Some(hll) = self.monitored.get_mut(&key) {
            hll.update(value);
            return;
        }
        if self.monitored.len() < self.capacity {
            let mut hll = HyperLogLog::new(self.precision).expect("precision validated");
            hll.update(value);
            self.monitored.insert(key, hll);
            return;
        }
        // Full: evict the key with the smallest distinct estimate.
        let min_key = self
            .monitored
            .iter()
            .min_by(|a, b| a.1.estimate().total_cmp(&b.1.estimate()))
            .map(|(k, _)| k.clone())
            .expect("non-empty when full");
        self.monitored.remove(&min_key);
        let mut hll = HyperLogLog::new(self.precision).expect("precision validated");
        hll.update(value);
        self.monitored.insert(key, hll);
    }

    /// Estimated number of distinct values associated with `key` (0 if not monitored).
    pub fn distinct_estimate(&self, key: &K) -> f64 {
        self.monitored.get(key).map_or(0.0, |h| h.estimate())
    }

    /// Whether `key` is currently monitored.
    pub fn is_monitored(&self, key: &K) -> bool {
        self.monitored.contains_key(key)
    }

    /// The top `k` keys by estimated distinct-value count, most distinct first.
    pub fn top_k(&self, k: usize) -> Vec<(K, f64)> {
        let mut items: Vec<(K, f64)> = self
            .monitored
            .iter()
            .map(|(key, hll)| (key.clone(), hll.estimate()))
            .collect();
        items.sort_by(|a, b| b.1.total_cmp(&a.1));
        items.truncate(k);
        items
    }

    /// Number of monitored keys.
    #[inline]
    pub fn len(&self) -> usize {
        self.monitored.len()
    }

    /// Whether no keys are monitored.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.monitored.is_empty()
    }

    /// Maximum number of monitored keys.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(FrequentDistinctTuples::<u64>::new(0, 12).is_err());
        assert!(FrequentDistinctTuples::<u64>::new(64, 12).is_ok());
    }

    #[test]
    fn ranks_keys_by_distinct_count() {
        let mut fdt = FrequentDistinctTuples::new(64, 12).unwrap();
        // Five "super-spreaders" with decreasing distinct fan-out, interleaved from the start.
        let spreads = [(1u64, 4000u64), (2, 3000), (3, 2000), (4, 1000), (5, 500)];
        let max = spreads.iter().map(|&(_, c)| c).max().unwrap();
        for step in 0..max {
            for &(key, count) in &spreads {
                if step < count {
                    fdt.update(key, &step);
                }
            }
            // A churny tail: many keys, each with a single distinct value.
            fdt.update(1000 + step, &0u64);
        }
        let top: Vec<u64> = fdt.top_k(5).into_iter().map(|(k, _)| k).collect();
        assert_eq!(top, vec![1, 2, 3, 4, 5], "top-5 {top:?}");
    }

    #[test]
    fn distinct_estimate_is_close() {
        let mut fdt = FrequentDistinctTuples::new(16, 14).unwrap();
        for v in 0..5000u64 {
            fdt.update("hot", &v);
        }
        let est = fdt.distinct_estimate(&"hot");
        assert!((est - 5000.0).abs() < 0.05 * 5000.0, "estimate {est}");
    }

    #[test]
    fn empty_and_absent() {
        let fdt = FrequentDistinctTuples::<u64>::new(8, 12).unwrap();
        assert!(fdt.is_empty());
        assert_eq!(fdt.distinct_estimate(&5), 0.0);
        assert!(fdt.top_k(3).is_empty());
    }
}

// --- Capability-trait adoption (fable5 doc 01 F3) ---
// No capability trait matches cleanly:
//  * ingest is `update<V: Hash>(&mut self, key: K, value: &V)` — a two-argument
//    (key, value) tuple ingest, not the single-item `Update<T>` shape.
//  * `distinct_estimate` returns `f64` (an HLL cardinality per key), not the
//    `u64` frequency PointQuery describes.
