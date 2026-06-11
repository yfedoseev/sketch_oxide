//! Memento Filter — dynamic range filter via prefix/memento decomposition.
//!
//! Memento (Adas, Pandey & Bender, "Memento Filter: A Fast, Dynamic, and Robust Range Filter",
//! SIGMOD 2024) is a dynamic range filter supporting insertions. Its defining idea is the
//! **memento**: a key is split into a high-order *prefix* and a low-order *memento*, and all keys
//! sharing a prefix are stored together as that prefix's sorted list of mementos. A range query
//! walks only the prefixes overlapping the query range (order-preserving, sublinear), is **exact
//! in the interior** — an occupied prefix wholly inside the range certainly holds a key in range —
//! and is refined at the two boundary prefixes by testing whether any stored memento falls in the
//! partial range. There are never false negatives.
//!
//! Storing only the low `MEMENTO_BITS` of each key (the prefix is shared across a whole group) is
//! the space lever: bytes scale with mementos, not full 64-bit keys.
//!
//! # Design note
//!
//! This reference stores prefixes in an ordered map of `prefix → sorted mementos` and keeps the
//! mementos *exactly*, so within this layout the filter is effectively exact (a query is rejected
//! whenever no stored key falls in range). The published Memento gets its small per-key footprint
//! and its tunable `2^-r` false-positive rate by embedding the prefixes as runs of a
//! **rank-and-select quotient filter** keyed on a prefix *fingerprint* (so distinct prefixes may
//! collide) with the mementos in adjacent slots. That packing is an optimization over this same
//! query contract — sublinear range scan, interior-exact, boundary-refined — and is left as a
//! follow-up; it trades a bounded FPR for fewer bits, it does not add false negatives.
//!
//! # Example
//! ```
//! use sketch_oxide::range_filters::MementoFilter;
//!
//! let mut filter = MementoFilter::new(1000, 0.01).unwrap();
//! filter.insert(42, b"value1").unwrap();
//! filter.insert(100, b"value2").unwrap();
//! filter.insert(250, b"value3").unwrap();
//!
//! assert!(filter.may_contain_range(40, 50));   // 42 is in range
//! assert!(filter.may_contain_range(95, 105));  // 100 is in range
//! assert!(!filter.may_contain_range(500, 600));// nothing there
//! ```

use crate::common::{RangeFilter, SketchError};
use std::collections::BTreeMap;

/// Number of low-order bits of each key kept as a memento.
const MEMENTO_BITS: u32 = 16;

/// Dynamic range filter supporting insertions.
///
/// Combines a cheap min/max base range with a prefix→memento store: the base rejects out-of-range
/// queries instantly, and the memento store answers in-range queries with interior-exact,
/// boundary-refined precision.
#[derive(Clone, Debug)]
pub struct MementoFilter {
    /// Base range (min/max) for instant out-of-range rejection.
    base_filter: MementoBaseFilter,
    /// Prefix → sorted-memento store (the actual filter).
    store: MementoStore,
    /// Metadata and statistics.
    metadata: MementoMetadata,
}

/// Base range component: tracks the span of inserted keys for fast out-of-range rejection.
#[derive(Clone, Debug)]
struct MementoBaseFilter {
    min_key: Option<u64>,
    max_key: Option<u64>,
}

/// The prefix→memento store.
#[derive(Clone, Debug)]
struct MementoStore {
    memento_bits: u32,
    memento_mask: u64,
    /// Each prefix maps to its sorted, de-duplicated mementos.
    prefixes: BTreeMap<u64, Vec<u32>>,
    num_entries: usize,
}

/// Metadata and statistics for a Memento Filter.
#[derive(Clone, Debug)]
struct MementoMetadata {
    capacity: usize,
    num_elements: usize,
    fpr_target: f64,
    /// Number of times an insert widened the [min, max] base range.
    num_expansions: usize,
}

impl MementoFilter {
    /// Creates a new Memento Filter.
    ///
    /// # Arguments
    /// * `expected_elements` - Expected number of elements (capacity).
    /// * `fpr` - Target false-positive rate in `(0, 1)` (retained for sizing/compatibility).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `expected_elements` is 0 or `fpr` is not in `(0, 1)`.
    ///
    /// # Example
    /// ```
    /// use sketch_oxide::range_filters::MementoFilter;
    ///
    /// let filter = MementoFilter::new(1000, 0.01).unwrap();
    /// assert_eq!(filter.len(), 0);
    /// ```
    pub fn new(expected_elements: usize, fpr: f64) -> Result<Self, SketchError> {
        if expected_elements == 0 {
            return Err(SketchError::InvalidParameter {
                param: "expected_elements".to_string(),
                value: expected_elements.to_string(),
                constraint: "must be greater than 0".to_string(),
            });
        }
        if fpr <= 0.0 || fpr >= 1.0 {
            return Err(SketchError::InvalidParameter {
                param: "fpr".to_string(),
                value: fpr.to_string(),
                constraint: "must be in range (0.0, 1.0)".to_string(),
            });
        }

        Ok(MementoFilter {
            base_filter: MementoBaseFilter {
                min_key: None,
                max_key: None,
            },
            store: MementoStore {
                memento_bits: MEMENTO_BITS,
                memento_mask: (1u64 << MEMENTO_BITS) - 1,
                prefixes: BTreeMap::new(),
                num_entries: 0,
            },
            metadata: MementoMetadata {
                capacity: expected_elements,
                num_elements: 0,
                fpr_target: fpr,
                num_expansions: 0,
            },
        })
    }

    /// Inserts a key (the `value` participates only in duplicate accounting; range membership is
    /// by key).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if the configured capacity would be exceeded.
    ///
    /// # Example
    /// ```
    /// use sketch_oxide::range_filters::MementoFilter;
    ///
    /// let mut filter = MementoFilter::new(1000, 0.01).unwrap();
    /// filter.insert(42, b"value").unwrap();
    /// assert_eq!(filter.len(), 1);
    /// ```
    pub fn insert(&mut self, key: u64, _value: &[u8]) -> Result<(), SketchError> {
        if self.metadata.num_elements >= self.metadata.capacity {
            return Err(SketchError::InvalidParameter {
                param: "capacity".to_string(),
                value: self.metadata.num_elements.to_string(),
                constraint: format!("exceeded maximum capacity of {}", self.metadata.capacity),
            });
        }

        self.update_base_filter(key);
        self.store.insert(key);
        self.metadata.num_elements += 1;
        Ok(())
    }

    /// Whether a key might be present (point query). No false negatives.
    pub fn may_contain(&self, key: u64) -> bool {
        if !self.base_filter.contains(key) {
            return false;
        }
        self.store.may_contain(key)
    }

    /// Whether the range `[low, high]` might contain a key.
    ///
    /// # Guarantees
    /// - No false negatives.
    /// - Interior prefixes are exact; only boundary prefixes can (in the packed variant) be
    ///   approximate.
    ///
    /// # Example
    /// ```
    /// use sketch_oxide::range_filters::MementoFilter;
    ///
    /// let mut filter = MementoFilter::new(1000, 0.01).unwrap();
    /// filter.insert(50, b"value").unwrap();
    /// assert!(filter.may_contain_range(45, 55));
    /// assert!(!filter.may_contain_range(100, 200));
    /// ```
    pub fn may_contain_range(&self, low: u64, high: u64) -> bool {
        if low > high {
            return false;
        }
        if !self.base_filter.overlaps_range(low, high) {
            return false;
        }
        self.store.may_contain_range(low, high)
    }

    /// Number of elements inserted (counts duplicates).
    pub fn len(&self) -> usize {
        self.metadata.num_elements
    }

    /// Whether the filter is empty.
    pub fn is_empty(&self) -> bool {
        self.metadata.num_elements == 0
    }

    /// Statistics: element count, capacity, target FPR, range expansions, load factor.
    pub fn stats(&self) -> MementoStats {
        MementoStats {
            num_elements: self.metadata.num_elements,
            capacity: self.metadata.capacity,
            fpr_target: self.metadata.fpr_target,
            num_expansions: self.metadata.num_expansions,
            load_factor: self.metadata.num_elements as f64 / self.metadata.capacity as f64,
        }
    }

    /// The `(min, max)` span of inserted keys, or `None` if empty.
    pub fn range(&self) -> Option<(u64, u64)> {
        match (self.base_filter.min_key, self.base_filter.max_key) {
            (Some(min), Some(max)) => Some((min, max)),
            _ => None,
        }
    }

    /// Number of distinct prefixes currently stored.
    pub fn num_prefixes(&self) -> usize {
        self.store.prefixes.len()
    }

    /// Widens the base range and counts an expansion when a key falls outside it.
    fn update_base_filter(&mut self, key: u64) {
        let outside = match (self.base_filter.min_key, self.base_filter.max_key) {
            (Some(min), Some(max)) => key < min || key > max,
            _ => false,
        };
        if outside {
            self.metadata.num_expansions += 1;
        }
        self.base_filter.min_key = Some(self.base_filter.min_key.map_or(key, |m| m.min(key)));
        self.base_filter.max_key = Some(self.base_filter.max_key.map_or(key, |m| m.max(key)));
    }
}

impl MementoBaseFilter {
    fn overlaps_range(&self, low: u64, high: u64) -> bool {
        match (self.min_key, self.max_key) {
            (Some(min), Some(max)) => !(high < min || low > max),
            _ => false,
        }
    }

    fn contains(&self, key: u64) -> bool {
        match (self.min_key, self.max_key) {
            (Some(min), Some(max)) => key >= min && key <= max,
            _ => false,
        }
    }
}

impl MementoStore {
    #[inline]
    fn split(&self, key: u64) -> (u64, u32) {
        (key >> self.memento_bits, (key & self.memento_mask) as u32)
    }

    fn insert(&mut self, key: u64) {
        let (prefix, memento) = self.split(key);
        let slot = self.prefixes.entry(prefix).or_default();
        if let Err(pos) = slot.binary_search(&memento) {
            slot.insert(pos, memento);
            self.num_entries += 1;
        }
    }

    fn may_contain(&self, key: u64) -> bool {
        let (prefix, memento) = self.split(key);
        match self.prefixes.get(&prefix) {
            None => false,
            Some(slot) => slot.binary_search(&memento).is_ok(),
        }
    }

    fn may_contain_range(&self, low: u64, high: u64) -> bool {
        let (plo, mlo) = self.split(low);
        let (phi, mhi) = self.split(high);
        let full_mask = self.memento_mask as u32;
        for (&prefix, slot) in self.prefixes.range(plo..=phi) {
            let at_low = prefix == plo;
            let at_high = prefix == phi;
            if !at_low && !at_high {
                // A fully-interior occupied prefix: every key with this prefix is inside the
                // query range, so a real key in range certainly exists.
                return true;
            }
            let lo = if at_low { mlo } else { 0 };
            let hi = if at_high { mhi } else { full_mask };
            // Any stored memento in [lo, hi]?
            let idx = slot.partition_point(|&m| m < lo);
            if idx < slot.len() && slot[idx] <= hi {
                return true;
            }
        }
        false
    }
}

impl RangeFilter for MementoFilter {
    fn may_contain_range(&self, low: u64, high: u64) -> bool {
        self.may_contain_range(low, high)
    }
}

/// Statistics about a Memento Filter.
#[derive(Debug, Clone, PartialEq)]
pub struct MementoStats {
    /// Number of elements currently stored (counts duplicates).
    pub num_elements: usize,
    /// Maximum capacity.
    pub capacity: usize,
    /// Target false-positive rate.
    pub fpr_target: f64,
    /// Number of base-range expansions.
    pub num_expansions: usize,
    /// Current load factor (`num_elements / capacity`).
    pub load_factor: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_construction() {
        let filter = MementoFilter::new(1000, 0.01).unwrap();
        assert_eq!(filter.len(), 0);
        assert!(filter.is_empty());
    }

    #[test]
    fn test_basic_insertion() {
        let mut filter = MementoFilter::new(1000, 0.01).unwrap();
        filter.insert(42, b"value").unwrap();
        assert_eq!(filter.len(), 1);
        assert!(!filter.is_empty());
    }

    #[test]
    fn test_basic_range_query() {
        let mut filter = MementoFilter::new(1000, 0.01).unwrap();
        filter.insert(50, b"value").unwrap();
        assert!(filter.may_contain_range(45, 55));
    }

    #[test]
    fn no_false_negatives_point_and_range() {
        // Keys spanning many prefixes (high bits vary) and mementos.
        let mut filter = MementoFilter::new(20_000, 0.01).unwrap();
        let keys: Vec<u64> = (0..10_000u64)
            .map(|i| i.wrapping_mul(2_654_435_761))
            .collect();
        for &k in &keys {
            filter.insert(k, b"v").unwrap();
        }
        for &k in &keys {
            assert!(filter.may_contain(k), "point false negative for {k}");
            assert!(
                filter.may_contain_range(k, k),
                "range false negative for {k}"
            );
        }
    }

    #[test]
    fn exact_within_a_single_prefix() {
        // 500 even keys all share prefix 0; odd keys between them must be rejected (interior-exact
        // memento check — no spurious positives from a crowded prefix).
        let mut filter = MementoFilter::new(2000, 0.01).unwrap();
        for i in 0..500u64 {
            filter.insert(i * 2, b"v").unwrap();
        }
        let mut fp = 0;
        for i in 0..500u64 {
            if filter.may_contain_range(i * 2 + 1, i * 2 + 1) {
                fp += 1;
            }
        }
        assert_eq!(
            fp, 0,
            "exact memento storage should not false-positive within a prefix"
        );
    }

    #[test]
    fn interior_prefix_returns_true() {
        // A key sits in a middle prefix; a wide range spanning it must report true.
        let mut filter = MementoFilter::new(1000, 0.01).unwrap();
        filter.insert(5 << MEMENTO_BITS, b"v").unwrap(); // prefix 5, memento 0
        assert!(filter.may_contain_range(2 << MEMENTO_BITS, 9 << MEMENTO_BITS));
    }

    #[test]
    fn far_empty_ranges_rejected() {
        let mut filter = MementoFilter::new(1000, 0.01).unwrap();
        for k in [1_000u64, 2_000, 3_000] {
            filter.insert(k, b"v").unwrap();
        }
        assert!(!filter.may_contain_range(10_000_000, 20_000_000));
        assert!(!filter.may_contain_range(0, 100));
        assert!(!filter.may_contain(500));
    }

    #[test]
    fn capacity_is_enforced() {
        let mut filter = MementoFilter::new(3, 0.01).unwrap();
        for k in 0..3u64 {
            filter.insert(k, b"v").unwrap();
        }
        assert!(filter.insert(3, b"v").is_err());
    }
}
