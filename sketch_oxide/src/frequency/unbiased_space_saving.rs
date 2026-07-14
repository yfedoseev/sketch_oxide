//! Unbiased Space-Saving — heavy hitters with statistically unbiased counts.
//!
//! Classic [`SpaceSaving`](crate::frequency::SpaceSaving) always evicts the minimum counter and
//! hands its value to the newcomer, which systematically *over*-counts tail items. Unbiased
//! Space-Saving (Ting, "Data Sketches for Disaggregated Subset Sum and Frequent Item Estimation",
//! KDD 2018) fixes the bias with one change: when the table is full, it increments the minimum
//! counter and only **probabilistically** — with probability `1/(min+1)` — lets the new item take
//! over that slot. The resulting counts are *unbiased* estimators of the true frequencies
//! (`E[estimate] = true count`), which makes them composable into unbiased subset-sum estimates.
//!
//! Two invariants make it easy to trust:
//! - the counters always sum to exactly the number of updates (no mass is created or lost);
//! - every still-monitored item's count is an unbiased estimate of its true frequency.

use crate::common::SketchError;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{BuildHasherDefault, Hash};

/// A `HashMap` with a fixed-seed hasher so iteration order — and thus the min-slot tie-breaking
/// and top-k ordering — is reproducible run-to-run.
type DetMap<T> = HashMap<T, u64, BuildHasherDefault<DefaultHasher>>;

/// An Unbiased Space-Saving top-k sketch over items of type `T`.
///
/// # Example
/// ```
/// use sketch_oxide::frequency::UnbiasedSpaceSaving;
///
/// let mut uss = UnbiasedSpaceSaving::new(8).unwrap();
/// for _ in 0..1000 { uss.update("elephant".to_string()); }
/// for i in 0..500 { uss.update_owned(format!("mouse{i}")); } // many tail items
///
/// // The heavy hitter is retained, and counter mass is conserved.
/// assert!(uss.estimate(&"elephant".to_string()).unwrap() > 500);
/// assert_eq!(uss.total_count(), uss.stream_length());
/// ```
#[derive(Debug, Clone)]
pub struct UnbiasedSpaceSaving<T: Hash + Eq + Clone> {
    capacity: usize,
    counts: DetMap<T>,
    n: u64,
    rng: SmallRng,
}

impl<T: Hash + Eq + Clone> UnbiasedSpaceSaving<T> {
    /// Creates a sketch monitoring at most `capacity` items.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `capacity` is 0.
    pub fn new(capacity: usize) -> Result<Self, SketchError> {
        if capacity == 0 {
            return Err(SketchError::InvalidParameter {
                param: "capacity".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            capacity,
            counts: DetMap::with_capacity_and_hasher(capacity + 1, BuildHasherDefault::default()),
            n: 0,
            rng: SmallRng::seed_from_u64(0x5A17_9E37_79B9_7C15),
        })
    }

    /// Creates a sketch sized for an `epsilon`-accurate top-k (`capacity = ⌈1/epsilon⌉`).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `epsilon` is not in `(0, 1)`.
    pub fn with_epsilon(epsilon: f64) -> Result<Self, SketchError> {
        if !(epsilon > 0.0 && epsilon < 1.0) {
            return Err(SketchError::InvalidParameter {
                param: "epsilon".to_string(),
                value: epsilon.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }
        Self::new((1.0 / epsilon).ceil() as usize)
    }

    /// Records one occurrence of `item` (by reference; clones only if a new slot is needed).
    pub fn update(&mut self, item: T) {
        self.update_owned(item);
    }

    /// Records one occurrence of `item`, consuming it.
    pub fn update_owned(&mut self, item: T) {
        self.n += 1;
        if let Some(c) = self.counts.get_mut(&item) {
            *c += 1;
            return;
        }
        if self.counts.len() < self.capacity {
            self.counts.insert(item, 1);
            return;
        }
        // Table full: find the minimum-count slot, increment it, and let the newcomer take it over
        // with probability 1/(min+1) — the unbiased-replacement rule.
        let (min_key, min_val) = self
            .counts
            .iter()
            .min_by_key(|&(_, &v)| v)
            .map(|(k, &v)| (k.clone(), v))
            .expect("table is full so non-empty");
        let new_count = min_val + 1;
        self.counts.remove(&min_key);
        if self.rng.random::<f64>() < 1.0 / new_count as f64 {
            self.counts.insert(item, new_count); // newcomer adopts the slot
        } else {
            self.counts.insert(min_key, new_count); // incumbent keeps it, count grows
        }
    }

    /// Unbiased estimate of `item`'s frequency, or `None` if not currently monitored.
    pub fn estimate(&self, item: &T) -> Option<u64> {
        self.counts.get(item).copied()
    }

    /// The monitored items and counts, sorted by count descending (the top-k estimate).
    pub fn top_k(&self, k: usize) -> Vec<(T, u64)> {
        let mut entries: Vec<(T, u64)> = self.counts.iter().map(|(t, &c)| (t.clone(), c)).collect();
        entries.sort_by_key(|&(_, c)| std::cmp::Reverse(c));
        entries.truncate(k);
        entries
    }

    /// Number of updates processed (stream length).
    #[inline]
    pub fn stream_length(&self) -> u64 {
        self.n
    }

    /// Sum of all counters — invariant: always equals [`stream_length`](Self::stream_length).
    pub fn total_count(&self) -> u64 {
        self.counts.values().sum()
    }

    /// Number of monitored items.
    #[inline]
    pub fn num_items(&self) -> usize {
        self.counts.len()
    }

    /// Maximum number of monitored items.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Whether nothing has been recorded.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_capacity() {
        assert!(UnbiasedSpaceSaving::<u64>::new(0).is_err());
        assert!(UnbiasedSpaceSaving::<u64>::new(16).is_ok());
        assert!(UnbiasedSpaceSaving::<u64>::with_epsilon(0.1).is_ok());
        assert!(UnbiasedSpaceSaving::<u64>::with_epsilon(1.5).is_err());
    }

    #[test]
    fn counters_sum_to_stream_length() {
        // The defining invariant of Unbiased Space-Saving: no count mass is created or destroyed.
        let mut uss = UnbiasedSpaceSaving::new(8).unwrap();
        for i in 0..10_000u64 {
            uss.update(i % 500); // 500 distinct items, capacity 8 → lots of replacement
        }
        assert_eq!(uss.total_count(), uss.stream_length());
        assert_eq!(uss.total_count(), 10_000);
    }

    #[test]
    fn below_capacity_is_exact() {
        let mut uss = UnbiasedSpaceSaving::new(16).unwrap();
        for _ in 0..100 {
            uss.update("a");
        }
        for _ in 0..50 {
            uss.update("b");
        }
        assert_eq!(uss.estimate(&"a"), Some(100));
        assert_eq!(uss.estimate(&"b"), Some(50));
    }

    #[test]
    fn heavy_hitter_is_retained_and_ranked_first() {
        let mut uss = UnbiasedSpaceSaving::new(8).unwrap();
        for _ in 0..5000 {
            uss.update(0u64); // the elephant
        }
        for i in 1..2000u64 {
            uss.update(i); // mice
        }
        let top = uss.top_k(1);
        assert_eq!(top[0].0, 0, "elephant should rank first");
        assert!(top[0].1 > 2500, "elephant count {} too eroded", top[0].1);
    }

    #[test]
    fn unbiased_total_across_heavy_and_tail() {
        // Aggregate (subset-sum) estimate over all monitored items must equal n exactly.
        let mut uss = UnbiasedSpaceSaving::new(32).unwrap();
        for i in 0..50_000u64 {
            uss.update(if i % 3 == 0 { 7 } else { i });
        }
        assert_eq!(uss.total_count(), 50_000);
        // The frequent item (1/3 of the stream) should be monitored with a large count.
        assert!(uss.estimate(&7).is_some());
    }

    #[test]
    fn deterministic_with_fixed_seed() {
        let build = || {
            let mut uss = UnbiasedSpaceSaving::new(8).unwrap();
            for i in 0..5000u64 {
                uss.update(i % 100);
            }
            uss
        };
        let a = build();
        let b = build();
        assert_eq!(a.top_k(8), b.top_k(8));
    }
}

// --- Capability-trait adoption (fable5 doc 01 F3) ---
use crate::common::capabilities::Update;

impl<T: std::hash::Hash + Eq + Clone> Update<T> for UnbiasedSpaceSaving<T> {
    fn update(&mut self, item: &T) {
        UnbiasedSpaceSaving::update(self, item.clone());
    }
}

// PointQuery is intentionally NOT implemented: `estimate` returns `Option<u64>`
// (`None` for evicted/unseen items), not the bare `u64` PointQuery requires.
