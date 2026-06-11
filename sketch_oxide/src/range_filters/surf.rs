//! SuRF — the Succinct Range Filter.
//!
//! SuRF (Zhang, Lim, Andersen, Kaminsky, Keeton & Pavlo, "SuRF: Practical Range Query Filtering
//! with Fast Succinct Tries", SIGMOD 2018 — the filter that gave RocksDB practical range
//! filtering) is a trie over the keys, *pruned* to the shortest prefixes that still tell the
//! keys apart. Because the tails below each branch are dropped, a query for an absent key that
//! shares a stored prefix can yield a false positive — but a present key always walks its prefix
//! to a stored leaf, so there are **no false negatives**. Unlike a Bloom filter, the trie keeps
//! keys in sorted order, which is what lets it answer **range** queries ("is there any key in
//! `[low, high]`?"), not just point lookups.
//!
//! This is SuRF-Base: the pruned trie itself. The accuracy-tuning suffix variants (SuRF-Hash /
//! SuRF-Real, which append a few hash or real key bits per leaf to cut the false-positive rate)
//! layer cleanly on top of the same trie.
//!
//! # Layout note
//!
//! The trie here is an explicit pointer/`BTreeMap` structure — the clear, verifiable reference
//! form with exactly SuRF's query semantics. SuRF's headline contribution, the **LOUDS-DS**
//! succinct encoding (the trie as rank/select bitmaps in ~10 bits/key), is a space optimization
//! over this same contract and is left as a follow-up; it does not change which keys the filter
//! accepts.
//!
//! # Guarantees
//!
//! - **No false negatives**: every inserted key is reported present, and any range containing an
//!   inserted key returns `true`.
//! - **One-sided error**: an absent key (or empty range) may return `true` only through a
//!   truncated-prefix collision; it never wrongly returns `false`.

use crate::common::RangeFilter;
use std::collections::BTreeMap;

/// One trie node: a labelled edge map plus a terminal flag. A terminal node with no children is
/// a *truncated leaf* — it stands for every key extending its prefix.
#[derive(Debug, Clone, Default)]
struct Node {
    children: BTreeMap<u8, Node>,
    terminal: bool,
}

/// A Succinct Range Filter (SuRF-Base) over byte-string keys.
///
/// Range queries treat keys as fixed 8-byte big-endian integers (the `u64` domain shared with
/// this module's other range filters); point queries work on arbitrary byte strings.
///
/// # Example
/// ```
/// use sketch_oxide::range_filters::Surf;
/// use sketch_oxide::common::RangeFilter;
///
/// let surf = Surf::from_u64(&[10, 20, 30, 40, 50]);
/// assert!(surf.contains_u64(30));            // present
/// assert!(surf.may_contain_range(15, 25));   // 20 is in range
/// assert!(!surf.may_contain_range(60, 70));  // nothing there → definitely absent
/// ```
#[derive(Debug, Clone)]
pub struct Surf {
    root: Node,
    len: usize,
}

impl Surf {
    /// Builds a SuRF over arbitrary byte-string keys.
    pub fn build_bytes(keys: &[Vec<u8>]) -> Self {
        let mut root = Node::default();
        let mut len = 0usize;
        for key in keys {
            if Self::insert_full(&mut root, key) {
                len += 1;
            }
        }
        // Prune: collapse every subtree that holds a single key into a one-byte-past-branch leaf.
        Self::prune(&mut root);
        Self { root, len }
    }

    /// Builds a SuRF over `u64` keys (encoded as 8-byte big-endian strings).
    pub fn from_u64(keys: &[u64]) -> Self {
        let encoded: Vec<Vec<u8>> = keys.iter().map(|k| k.to_be_bytes().to_vec()).collect();
        Self::build_bytes(&encoded)
    }

    /// Inserts a full key path; returns `true` if this key was not already terminal.
    fn insert_full(root: &mut Node, key: &[u8]) -> bool {
        let mut node = root;
        for &b in key {
            node = node.children.entry(b).or_default();
        }
        let was_new = !node.terminal;
        node.terminal = true;
        was_new
    }

    /// Returns the number of distinct keys (terminals) in a subtree, collapsing every
    /// single-key subtree into a truncated leaf as it goes.
    fn prune(node: &mut Node) -> usize {
        if node.children.is_empty() {
            return node.terminal as usize;
        }
        let mut total = node.terminal as usize;
        // First recurse so child counts are post-pruned.
        let bytes: Vec<u8> = node.children.keys().copied().collect();
        for b in bytes {
            let child = node.children.get_mut(&b).unwrap();
            let count = Self::prune(child);
            if count == 1 && !child.terminal {
                // The child's subtree holds exactly one key and the child itself is not a key
                // end: truncate it to a single leaf one byte past this branch.
                child.children.clear();
                child.terminal = true;
            }
            total += count;
        }
        total
    }

    /// Point query over a byte-string key. May return a false positive; never a false negative.
    pub fn contains_bytes(&self, key: &[u8]) -> bool {
        let mut node = &self.root;
        for &b in key {
            if node.terminal && node.children.is_empty() {
                // Reached a truncated leaf: every extension of this prefix matches.
                return true;
            }
            match node.children.get(&b) {
                Some(child) => node = child,
                None => return false,
            }
        }
        node.terminal
    }

    /// Point query over a `u64` key.
    pub fn contains_u64(&self, key: u64) -> bool {
        self.contains_bytes(&key.to_be_bytes())
    }

    /// Number of keys inserted.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the filter is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The closed key interval `[lo, hi]` covered by a prefix of `depth` bytes whose big-endian
    /// value is `prefix_val`, interpreting the unspecified low bytes as ranging over all values.
    #[inline]
    fn prefix_interval(prefix_val: u64, depth: u32) -> (u64, u64) {
        let shift = (8 - depth) * 8;
        let lo = prefix_val << shift;
        let hi = if shift == 0 {
            lo
        } else {
            lo | ((1u64 << shift) - 1)
        };
        (lo, hi)
    }

    /// DFS for an inserted prefix whose covered interval intersects `[low, high]`.
    fn range_dfs(node: &Node, prefix_val: u64, depth: u32, low: u64, high: u64) -> bool {
        if node.terminal {
            let (lo, hi) = Self::prefix_interval(prefix_val, depth);
            if lo <= high && low <= hi {
                return true;
            }
        }
        for (&b, child) in &node.children {
            let cdepth = depth + 1;
            let cval = (prefix_val << 8) | b as u64;
            let (clo, chi) = Self::prefix_interval(cval, cdepth);
            // Only descend where the child's possible key interval can intersect the query.
            if clo <= high && low <= chi && Self::range_dfs(child, cval, cdepth, low, high) {
                return true;
            }
        }
        false
    }
}

impl RangeFilter for Surf {
    fn may_contain_range(&self, low: u64, high: u64) -> bool {
        if self.is_empty() || low > high {
            return false;
        }
        Self::range_dfs(&self.root, 0, 0, low, high)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_filter_finds_nothing() {
        let surf = Surf::from_u64(&[]);
        assert!(surf.is_empty());
        assert!(!surf.contains_u64(5));
        assert!(!surf.may_contain_range(0, u64::MAX));
    }

    #[test]
    fn no_false_negatives_point() {
        let keys: Vec<u64> = (0..5000u64)
            .map(|i| i.wrapping_mul(2_654_435_761))
            .collect();
        let surf = Surf::from_u64(&keys);
        for &k in &keys {
            assert!(surf.contains_u64(k), "false negative for {k}");
        }
    }

    #[test]
    fn no_false_negatives_range() {
        let keys: Vec<u64> = vec![100, 5_000, 5_001, 900_000, 1_000_000, u64::MAX / 2];
        let surf = Surf::from_u64(&keys);
        for &k in &keys {
            assert!(surf.may_contain_range(k, k), "range missed exact key {k}");
            assert!(
                surf.may_contain_range(k.saturating_sub(3), k + 3),
                "range missed near {k}"
            );
        }
        // A range spanning several keys.
        assert!(surf.may_contain_range(4_000, 6_000));
    }

    #[test]
    fn empty_gaps_are_rejected() {
        let keys: Vec<u64> = vec![1_000, 2_000, 3_000];
        let surf = Surf::from_u64(&keys);
        // Far away from every key and its truncated prefix: must be a definite no.
        assert!(!surf.may_contain_range(10_000_000, 20_000_000));
        assert!(!surf.may_contain_range(0, 100));
    }

    #[test]
    fn distinguishes_present_from_far_absent() {
        let keys: Vec<u64> = (0..1000u64).map(|i| i * 1_000_000 + 12_345).collect();
        let surf = Surf::from_u64(&keys);
        for &k in &keys {
            assert!(surf.contains_u64(k));
        }
        // Keys placed far from any stored value (offset chosen to avoid prefix collisions).
        let mut far_hits = 0;
        for i in 0..1000u64 {
            if surf.contains_u64(i * 1_000_000 + 777_777) {
                far_hits += 1;
            }
        }
        // Some prefix collisions are expected (it's a filter), but most should be rejected.
        assert!(far_hits < 500, "too many false positives: {far_hits}/1000");
    }

    #[test]
    fn string_keys_point_query() {
        let keys: Vec<Vec<u8>> = ["apple", "application", "apply", "banana", "band"]
            .iter()
            .map(|s| s.as_bytes().to_vec())
            .collect();
        let surf = Surf::build_bytes(&keys);
        for k in &keys {
            assert!(surf.contains_bytes(k), "false negative for {:?}", k);
        }
        // "cherry" shares no stored prefix branch → definite reject.
        assert!(!surf.contains_bytes(b"cherry"));
    }

    #[test]
    fn shared_prefix_keys_distinguished() {
        // Keys differ only in the last bytes; the trie must retain the distinguishing branch.
        let keys: Vec<u64> = vec![
            0x1122_3344_5566_7700,
            0x1122_3344_5566_7701,
            0x1122_3344_5566_7702,
        ];
        let surf = Surf::from_u64(&keys);
        for &k in &keys {
            assert!(surf.contains_u64(k));
        }
        assert!(surf.may_contain_range(0x1122_3344_5566_7700, 0x1122_3344_5566_7702));
    }

    #[test]
    fn len_counts_distinct_keys() {
        let surf = Surf::from_u64(&[7, 7, 7, 8, 9]);
        assert_eq!(surf.len(), 3);
    }
}
