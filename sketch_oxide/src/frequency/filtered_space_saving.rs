//! Filtered Space-Saving — memory-tight top-k heavy hitters (Homem & Carvalho, 2010).
//!
//! Plain [`SpaceSaving`](crate::frequency::SpaceSaving) admits *every* unmonitored item immediately,
//! evicting the current minimum — so under a heavy long tail the monitored set churns and the
//! reported counts inflate. Filtered Space-Saving (the algorithm behind Redis's `TOPK`) interposes a
//! lightweight **filter**: an array of counters indexed by hash. An unmonitored item just bumps its
//! filter cell; it is promoted into the monitored set only once that cell would exceed the current
//! monitored minimum. When a monitored item is evicted its count is parked back in its filter cell, so
//! a recurring heavy hitter re-enters with a good estimate instead of from scratch.
//!
//! The result is a tighter top-k with the same Space-Saving guarantees: every item with true
//! frequency above `N/capacity` is monitored, and each estimate carries an error bound (`true ∈
//! [count − error, count]`). Query the ranked heavy hitters with [`FilteredSpaceSaving::top_k`].

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use std::collections::HashMap;
use std::hash::Hash;

/// Hash seed for filter-cell selection.
const FILTER_SEED: u64 = 0xF117_5E33_5A71_9C0D;

/// A top-k heavy-hitter tracker with a filter layer over a Space-Saving monitored set.
///
/// # Example
/// ```
/// use sketch_oxide::frequency::FilteredSpaceSaving;
///
/// let mut fss = FilteredSpaceSaving::new(32, 256).unwrap();
/// // 0 is heaviest, then 1, then 2, plus a noisy tail.
/// for i in 0..100_000u64 {
///     let e = match i % 100 { 0..=49 => 0, 50..=74 => 1, 75..=84 => 2, _ => 1000 + i };
///     fss.update(e);
/// }
/// let top = fss.top_k(3);
/// assert_eq!(top.iter().map(|(k, _)| *k).collect::<Vec<_>>(), vec![0, 1, 2]);
/// ```
#[derive(Debug, Clone)]
pub struct FilteredSpaceSaving<T: Hash + Eq + Clone> {
    capacity: usize,
    /// Monitored items: `item -> (count, error)`, where `true ∈ [count − error, count]`.
    monitored: HashMap<T, (u64, u64)>,
    /// Filter counters for unmonitored items.
    filter: Vec<u64>,
    n: u64,
}

impl<T: Hash + Eq + Clone> FilteredSpaceSaving<T> {
    /// Creates a tracker monitoring up to `capacity` items with a filter of `filter_size` cells.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `capacity` or `filter_size` is 0.
    pub fn new(capacity: usize, filter_size: usize) -> Result<Self> {
        if capacity == 0 {
            return Err(SketchError::InvalidParameter {
                param: "capacity".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        if filter_size == 0 {
            return Err(SketchError::InvalidParameter {
                param: "filter_size".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            capacity,
            monitored: HashMap::new(),
            filter: vec![0u64; filter_size],
            n: 0,
        })
    }

    /// Filter cell of `item`.
    #[inline]
    fn cell(&self, item: &T) -> usize {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        use std::hash::Hasher;
        item.hash(&mut hasher);
        let h = xxhash(&hasher.finish().to_le_bytes(), FILTER_SEED);
        (h % self.filter.len() as u64) as usize
    }

    /// Returns the monitored item with the smallest count (and that count).
    fn argmin(&self) -> Option<(T, u64)> {
        self.monitored
            .iter()
            .min_by_key(|(_, (c, _))| *c)
            .map(|(k, (c, _))| (k.clone(), *c))
    }

    /// Records one occurrence of `item`.
    pub fn update(&mut self, item: T) {
        self.n += 1;
        if let Some((count, _)) = self.monitored.get_mut(&item) {
            *count += 1;
            return;
        }
        let b = self.cell(&item);
        if self.monitored.len() < self.capacity {
            let start = self.filter[b];
            self.monitored.insert(item, (start + 1, start));
            return;
        }
        // Monitored set is full: promote only if the filter cell would beat the current minimum.
        let (min_item, c_min) = self.argmin().expect("non-empty when full");
        if self.filter[b] + 1 > c_min {
            // Park the evicted item's count in its filter cell, promote the newcomer.
            let bmin = self.cell(&min_item);
            self.filter[bmin] = c_min;
            self.monitored.remove(&min_item);
            let start = self.filter[b];
            self.monitored.insert(item, (start + 1, start));
        } else {
            self.filter[b] += 1;
        }
    }

    /// The top `k` monitored items, most frequent first, as `(item, estimated_count)`.
    pub fn top_k(&self, k: usize) -> Vec<(T, u64)> {
        let mut items: Vec<(T, u64)> = self
            .monitored
            .iter()
            .map(|(key, (count, _))| (key.clone(), *count))
            .collect();
        items.sort_by_key(|&(_, count)| std::cmp::Reverse(count));
        items.truncate(k);
        items
    }

    /// Estimated `(count, error)` of `item` if monitored (`true ∈ [count − error, count]`), else
    /// `None`.
    pub fn estimate(&self, item: &T) -> Option<(u64, u64)> {
        self.monitored.get(item).copied()
    }

    /// Number of items observed (`N`).
    #[inline]
    pub fn count(&self) -> u64 {
        self.n
    }

    /// Number of monitored items.
    #[inline]
    pub fn len(&self) -> usize {
        self.monitored.len()
    }

    /// Whether nothing is monitored.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.monitored.is_empty()
    }

    /// Maximum number of monitored items.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap as Map;

    #[test]
    fn rejects_bad_params() {
        assert!(FilteredSpaceSaving::<u64>::new(0, 64).is_err());
        assert!(FilteredSpaceSaving::<u64>::new(32, 0).is_err());
        assert!(FilteredSpaceSaving::<u64>::new(32, 64).is_ok());
    }

    #[test]
    fn recovers_top_k() {
        let mut fss = FilteredSpaceSaving::new(64, 512).unwrap();
        let mut truth: Map<u64, u64> = Map::new();
        for i in 0..200_000u64 {
            // Five planted heavies (0..5) with decreasing weight, plus a churny tail.
            let e = match i % 100 {
                0..=39 => 0,
                40..=64 => 1,
                65..=79 => 2,
                80..=89 => 3,
                90..=94 => 4,
                _ => 10_000 + i,
            };
            *truth.entry(e).or_default() += 1;
            fss.update(e);
        }
        let top: Vec<u64> = fss.top_k(5).into_iter().map(|(k, _)| k).collect();
        assert_eq!(top, vec![0, 1, 2, 3, 4], "top-5 {top:?}");
    }

    #[test]
    fn heavy_estimate_is_close() {
        let mut fss = FilteredSpaceSaving::new(32, 256).unwrap();
        for i in 0..100_000u64 {
            let e = if i % 2 == 0 { 0 } else { 1000 + i }; // 0 is exactly half
            fss.update(e);
        }
        let (count, error) = fss.estimate(&0).unwrap();
        let truth = 50_000u64;
        // Space-Saving invariant: count over-estimates, and true ∈ [count − error, count].
        assert!(count >= truth, "count {count} < truth {truth}");
        assert!(count - error <= truth, "count {count} error {error}");
        // Promoted early, so the over-estimate is small.
        assert!(
            count - truth < 1000,
            "over-estimate {} too large",
            count - truth
        );
    }

    #[test]
    fn empty_and_absent() {
        let fss = FilteredSpaceSaving::<u64>::new(8, 64).unwrap();
        assert!(fss.is_empty());
        assert!(fss.estimate(&5).is_none());
        assert!(fss.top_k(3).is_empty());
    }
}

// --- Capability-trait adoption (fable5 doc 01 F3) ---
use crate::common::capabilities::Update;

impl<T: std::hash::Hash + Eq + Clone> Update<T> for FilteredSpaceSaving<T> {
    fn update(&mut self, item: &T) {
        FilteredSpaceSaving::update(self, item.clone());
    }
}

// PointQuery is intentionally NOT implemented: `estimate` returns
// `Option<(u64, u64)>` (count + error, or `None` when unseen), not a bare `u64`.
