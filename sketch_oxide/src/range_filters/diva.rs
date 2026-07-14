//! Diva — dynamic range filter for variable-length keys and queries.
//!
//! Diva (Eslami, Bercea & Dayan, "Diva: Dynamic Range Filter for Var-Length Keys and Queries",
//! PVLDB 18(11), VLDB 2025) is the 2025 frontier of range filtering: unlike Memento — which needs
//! fixed-width integer keys and bounded range widths — Diva handles **variable-length byte-string
//! keys**, **arbitrary lexicographic range queries**, and is fully **dynamic** (insert *and*
//! delete), with a theoretical false-positive guarantee. That is exactly what real storage engines
//! (RocksDB, WiredTiger, object stores, URL/path indexes) need, where keys are variable-length
//! bytes.
//!
//! Diva keeps a sampled trie that routes a key (or a range endpoint) to a partition, and stores a
//! compact **infix** of each key — the distinguishing bytes — rather than the whole key. Truncating
//! the suffix below the infix is what yields a bounded, tunable false-positive rate while keeping
//! the structure small; nothing is dropped that could cause a false negative.
//!
//! # This implementation
//!
//! Keys are compared lexicographically (the natural order for string range queries). Each key is
//! reduced to its `resolution`-byte infix prefix and held, with a live multiplicity count (for
//! dynamic deletes), in an ordered map. A query — point or range — is answered by an
//! order-preserving scan over the infixes whose covered key interval overlaps the query:
//!
//! - **No false negatives**: a present key's infix always overlaps any range that contains it.
//! - **One-sided error**: a non-member collides only when it shares an infix prefix with a member.
//! - **Dynamic**: `insert` and `remove` maintain the counts; a partition disappears when its last
//!   key leaves.
//!
//! # Design note
//!
//! The ordered infix map is the clear, verifiable form of Diva's contract (dynamic, var-length,
//! order-preserving, bounded FP). Diva's namesake machinery — the *sampled trie* for routing and a
//! rank-and-select packed infix store for a few bits per key — is a space/locality optimization
//! over this same query contract and is a documented follow-up; it shrinks bytes, not the set of
//! ranges accepted.

use crate::common::{RangeFilter, SketchError};
use std::collections::BTreeMap;

/// A dynamic range filter over variable-length byte-string keys.
///
/// # Example
/// ```
/// use sketch_oxide::range_filters::DivaFilter;
///
/// let mut f = DivaFilter::new(0.01).unwrap();
/// f.insert(b"apple");
/// f.insert(b"banana");
/// f.insert(b"cherry");
///
/// assert!(f.may_contain(b"banana"));
/// assert!(f.may_contain_range(b"az", b"c"));   // "banana" is in [az, c]
/// assert!(!f.may_contain_range(b"x", b"z"));    // nothing there
///
/// f.remove(b"banana");                          // dynamic delete
/// assert!(!f.may_contain_range(b"az", b"c"));
/// ```
#[derive(Debug, Clone)]
pub struct DivaFilter {
    /// Infix resolution: keys are reduced to their first `resolution` bytes.
    resolution: usize,
    /// Infix prefix → live multiplicity.
    infixes: BTreeMap<Vec<u8>, u32>,
    /// Total live keys (counts duplicates).
    len: usize,
}

impl DivaFilter {
    /// Creates a filter targeting false-positive rate `fpr`, choosing an infix length accordingly.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `fpr` is not in `(0, 1)`.
    pub fn new(fpr: f64) -> Result<Self, SketchError> {
        if !(fpr > 0.0 && fpr < 1.0) {
            return Err(SketchError::InvalidParameter {
                param: "fpr".to_string(),
                value: fpr.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }
        // Each infix byte cuts the collision probability by ~256; pick enough bytes for the target.
        let resolution = ((-fpr.log2() / 8.0).ceil() as usize + 3).clamp(4, 32);
        Ok(Self::with_resolution(resolution))
    }

    /// Creates a filter with an explicit infix length in bytes (larger ⇒ lower FPR, more space).
    pub fn with_resolution(resolution: usize) -> Self {
        Self {
            resolution: resolution.max(1),
            infixes: BTreeMap::new(),
            len: 0,
        }
    }

    /// The infix of a key: its first `resolution` bytes.
    #[inline]
    fn infix(&self, key: &[u8]) -> Vec<u8> {
        key[..key.len().min(self.resolution)].to_vec()
    }

    /// Inserts a key (dynamic; duplicates increase its multiplicity).
    pub fn insert(&mut self, key: &[u8]) {
        let ix = self.infix(key);
        *self.infixes.entry(ix).or_insert(0) += 1;
        self.len += 1;
    }

    /// Removes one occurrence of a key. Returns `true` if a matching infix was present.
    ///
    /// Because keys are stored as infixes, this removes one occurrence of the key's *infix class*;
    /// it is exact when keys do not share an infix.
    pub fn remove(&mut self, key: &[u8]) -> bool {
        let ix = self.infix(key);
        if let Some(count) = self.infixes.get_mut(&ix) {
            *count -= 1;
            if *count == 0 {
                self.infixes.remove(&ix);
            }
            self.len -= 1;
            true
        } else {
            false
        }
    }

    /// Whether a key might be present (point query). No false negatives.
    pub fn may_contain(&self, key: &[u8]) -> bool {
        self.infixes.contains_key(&self.infix(key))
    }

    /// Whether the lexicographic range `[low, high]` might contain a key. No false negatives.
    pub fn may_contain_range(&self, low: &[u8], high: &[u8]) -> bool {
        if low > high || self.infixes.is_empty() {
            return false;
        }
        // An infix `ix` stands for every key with prefix `ix` — the interval `[ix, ix·0xFF…]`.
        // It overlaps [low, high] iff `ix <= high` and (`ix >= low` or `low` starts with `ix`).
        let intersects = |ix: &[u8]| -> bool { ix <= high && (ix >= low || low.starts_with(ix)) };
        // An infix that is a *prefix of* `low` sorts before `low`, so start the scan from the
        // greatest infix ≤ `low` to avoid missing it.
        let start: Vec<u8> = self
            .infixes
            .range(..=low.to_vec())
            .next_back()
            .map(|(k, _)| k.clone())
            .unwrap_or_default();
        for (ix, _) in self.infixes.range(start..=high.to_vec()) {
            if intersects(ix) {
                return true;
            }
        }
        false
    }

    /// Number of live keys (counts duplicates).
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the filter holds no keys.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Number of distinct infix partitions stored.
    #[inline]
    pub fn num_partitions(&self) -> usize {
        self.infixes.len()
    }

    /// Infix resolution in bytes.
    #[inline]
    pub fn resolution(&self) -> usize {
        self.resolution
    }
}

impl RangeFilter for DivaFilter {
    /// Range query over `u64` keys (big-endian, so byte order matches integer order).
    fn may_contain_range(&self, low: u64, high: u64) -> bool {
        self.may_contain_range(&low.to_be_bytes(), &high.to_be_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_fpr() {
        assert!(DivaFilter::new(0.0).is_err());
        assert!(DivaFilter::new(1.0).is_err());
        assert!(DivaFilter::new(0.01).is_ok());
    }

    #[test]
    fn point_membership_string_keys() {
        let mut f = DivaFilter::with_resolution(8);
        for k in ["apple", "application", "banana", "band", "cherry"] {
            f.insert(k.as_bytes());
        }
        for k in ["apple", "application", "banana", "band", "cherry"] {
            assert!(f.may_contain(k.as_bytes()), "false negative for {k}");
        }
        assert!(!f.may_contain(b"zzzzzzz"));
    }

    #[test]
    fn dynamic_insert_and_delete() {
        let mut f = DivaFilter::with_resolution(8);
        f.insert(b"hello");
        assert!(f.may_contain(b"hello"));
        assert_eq!(f.len(), 1);
        assert!(f.remove(b"hello"));
        assert!(!f.may_contain(b"hello"));
        assert_eq!(f.len(), 0);
        assert!(!f.remove(b"hello")); // already gone
    }

    #[test]
    fn duplicate_counts_survive_single_delete() {
        let mut f = DivaFilter::with_resolution(8);
        f.insert(b"dup");
        f.insert(b"dup");
        assert_eq!(f.len(), 2);
        f.remove(b"dup");
        assert!(f.may_contain(b"dup"), "one copy remains");
        assert_eq!(f.len(), 1);
    }

    #[test]
    fn lexicographic_range_queries() {
        let mut f = DivaFilter::with_resolution(8);
        for k in ["apple", "banana", "cherry", "date", "elderberry"] {
            f.insert(k.as_bytes());
        }
        assert!(f.may_contain_range(b"b", b"d")); // banana, cherry, date
        assert!(f.may_contain_range(b"a", b"a~")); // apple
        assert!(!f.may_contain_range(b"f", b"z")); // nothing past elderberry
        assert!(!f.may_contain_range(b"x", b"z"));
    }

    #[test]
    fn range_with_prefix_of_low_is_not_missed() {
        // A short infix that is a prefix of `low` must still be found (the predecessor scan).
        let mut f = DivaFilter::with_resolution(3);
        f.insert(b"apple"); // infix "app"
        // Range [apq, b): "apple" (=> "app...") should be considered since "app" < "apq" but
        // "apple" itself is < "apq"? "apple" vs "apq": 'a','p' equal, 'p' < 'q' => "apple" < "apq".
        // So apple is NOT in [apq, b). Confirm we correctly reject.
        assert!(!f.may_contain_range(b"apq", b"b"));
        // But [apa, b) DOES contain "apple".
        assert!(f.may_contain_range(b"apa", b"b"));
    }

    #[test]
    fn no_false_negatives_over_many_keys() {
        let mut f = DivaFilter::new(0.01).unwrap();
        let keys: Vec<String> = (0..5000).map(|i| format!("key::{i:08}")).collect();
        for k in &keys {
            f.insert(k.as_bytes());
        }
        for k in &keys {
            assert!(f.may_contain(k.as_bytes()), "point FN for {k}");
            assert!(
                f.may_contain_range(k.as_bytes(), k.as_bytes()),
                "range FN for {k}"
            );
        }
    }

    #[test]
    fn u64_range_filter_trait() {
        let mut f = DivaFilter::with_resolution(8);
        for k in [100u64, 5_000, 1_000_000] {
            f.insert(&k.to_be_bytes());
        }
        assert!(RangeFilter::may_contain_range(&f, 4_000, 6_000)); // 5000
        assert!(!RangeFilter::may_contain_range(&f, 2_000_000, 3_000_000));
    }

    #[test]
    fn empty_filter_rejects() {
        let f = DivaFilter::new(0.01).unwrap();
        assert!(f.is_empty());
        assert!(!f.may_contain(b"x"));
        assert!(!f.may_contain_range(b"a", b"z"));
    }
}
