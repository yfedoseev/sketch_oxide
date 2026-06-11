//! Persistent Bloom Filter (PBF) — membership testing over the *entire history* (Peng, Guo, Li, Qian
//! & Zhou, SIGMOD 2018).
//!
//! A plain Bloom filter answers "has `x` ever appeared?"; a Persistent Bloom Filter answers the
//! **temporal** question "did `x` appear during time range `[s, e]`?" — e.g. "did host A contact this
//! server between 9:30 and 9:40?". The naive fix (one membership test per timestamp in `[s,e]`) costs
//! `O(e−s)` time and raises the false-positive probability to the `(e−s)`-th power. PBF-1 fixes both
//! with a **dyadic (segment-tree) decomposition of time**.
//!
//! Over a time domain `[1, T]` (with `T` a power of two) and leaf granularity `g`, PBF-1 builds a
//! binary tree with `L = log₂(T/g) + 1` levels; level `ℓ` has `2^ℓ` nodes each covering a contiguous
//! `T/2^ℓ`-length interval, and **every node owns one Bloom filter** holding the elements seen in its
//! interval.
//!
//! * **Insert** `(x, t)`: walk the root→leaf path for timestamp `t` and add `x` to the Bloom filter at
//!   every node on the path (`L` filters).
//! * **Query** `(x, [s,e])`: compute the **canonical cover** of `[s,e]` (its `O(log T)` dyadic pieces),
//!   test `x` against each piece's Bloom filter, and answer **yes if any** says yes. Crucially the
//!   cover never uses the root for a strict sub-range, which is what gives temporal precision.
//!
//! No false negatives (each Bloom filter has none, and the cover tiles `[s,e]`); false positives come
//! from Bloom collisions and the OR over the cover. The paper sizes filters per level and adds a small
//! sub-granularity helper filter; this reference uses a uniform per-node filter size (documented).

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

const SEED: u64 = 0x9B1F_0F11_0000_0001;

/// A Persistent Bloom Filter answering temporal membership over `[1, T]`.
///
/// # Example
/// ```
/// use sketch_oxide::streaming::PersistentBloomFilter;
///
/// // Time domain [1, 1024], per-timestamp granularity, 4096-bit node filters, 4 hashes.
/// let mut pbf = PersistentBloomFilter::new(1024, 1, 4096, 4).unwrap();
/// pbf.insert(100, 50);   // element 100 appeared at time 50
/// pbf.insert(100, 200);
///
/// // It was present at 50 (any range covering 50), but not in [300, 400].
/// assert!(pbf.query(100, 40, 60));
/// assert!(pbf.query(100, 200, 200));
/// assert!(!pbf.query(100, 300, 400));
/// ```
#[derive(Debug, Clone)]
pub struct PersistentBloomFilter {
    t_max: u64,
    granularity: u64,
    leaves: u64, // T / g (a power of two)
    num_nodes: usize,
    bits: usize,  // bits per node filter
    words: usize, // u64 words per node filter
    num_hashes: u32,
    table: Vec<u64>, // num_nodes × words bitsets
}

impl PersistentBloomFilter {
    /// Creates a PBF over the time domain `[1, t_max]` with leaf `granularity`, `bits_per_filter` bits
    /// per node, and `num_hashes` hash functions.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `t_max`/`granularity` are not powers of two with
    /// `1 ≤ granularity ≤ t_max`, `bits_per_filter == 0`, or `num_hashes` is not in `1..=16`.
    pub fn new(
        t_max: u64,
        granularity: u64,
        bits_per_filter: usize,
        num_hashes: u32,
    ) -> Result<Self> {
        let pow2 = |x: u64| x > 0 && x.is_power_of_two();
        if !pow2(t_max) || t_max < 2 {
            return Err(SketchError::InvalidParameter {
                param: "t_max".to_string(),
                value: t_max.to_string(),
                constraint: "must be a power of two >= 2".to_string(),
            });
        }
        if !pow2(granularity) || granularity > t_max {
            return Err(SketchError::InvalidParameter {
                param: "granularity".to_string(),
                value: granularity.to_string(),
                constraint: "must be a power of two in 1..=t_max".to_string(),
            });
        }
        if bits_per_filter == 0 {
            return Err(SketchError::InvalidParameter {
                param: "bits_per_filter".to_string(),
                value: "0".to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        if !(1..=16).contains(&num_hashes) {
            return Err(SketchError::InvalidParameter {
                param: "num_hashes".to_string(),
                value: num_hashes.to_string(),
                constraint: "must be in 1..=16".to_string(),
            });
        }
        let leaves = t_max / granularity; // power of two
        let num_nodes = (2 * leaves - 1) as usize; // full binary tree
        let words = bits_per_filter.div_ceil(64);
        Ok(Self {
            t_max,
            granularity,
            leaves,
            num_nodes,
            bits: bits_per_filter,
            words,
            num_hashes,
            table: vec![0; num_nodes * words],
        })
    }

    /// Array index (level-order/heap layout) of the leaf node containing timestamp `t`.
    fn leaf_node(&self, t: u64) -> usize {
        let within = (t - 1) / self.granularity; // 0-based leaf position
        (self.leaves - 1 + within) as usize
    }

    /// Sets `element`'s bits in node `node`'s filter.
    fn set_bits(&mut self, node: usize, element: u64) {
        let base = node * self.words;
        for j in 0..self.num_hashes {
            let h = xxhash(
                &element.to_le_bytes(),
                SEED ^ (j as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15),
            );
            let bit = (h % self.bits as u64) as usize;
            self.table[base + bit / 64] |= 1u64 << (bit % 64);
        }
    }

    /// Whether all of `element`'s bits are set in node `node`'s filter.
    fn test_bits(&self, node: usize, element: u64) -> bool {
        let base = node * self.words;
        for j in 0..self.num_hashes {
            let h = xxhash(
                &element.to_le_bytes(),
                SEED ^ (j as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15),
            );
            let bit = (h % self.bits as u64) as usize;
            if self.table[base + bit / 64] & (1u64 << (bit % 64)) == 0 {
                return false;
            }
        }
        true
    }

    /// Records that `element` appeared at `timestamp` (clamped to `[1, t_max]`): adds it to every node
    /// on the root→leaf path.
    pub fn insert(&mut self, element: u64, timestamp: u64) {
        let t = timestamp.clamp(1, self.t_max);
        let mut node = self.leaf_node(t);
        loop {
            self.set_bits(node, element);
            if node == 0 {
                break;
            }
            node = (node - 1) / 2;
        }
    }

    /// Collects the canonical cover of `[s, e]` into `out`: the dyadic tree nodes that exactly tile the
    /// range (leaf nodes are taken whole even on partial overlap, never adding false negatives).
    fn cover(&self, node: usize, lo: u64, hi: u64, s: u64, e: u64, out: &mut Vec<usize>) {
        if e < lo || hi < s {
            return; // disjoint
        }
        let is_leaf = hi - lo + 1 == self.granularity;
        if (s <= lo && hi <= e) || is_leaf {
            out.push(node);
            return;
        }
        let mid = lo + (hi - lo) / 2;
        self.cover(2 * node + 1, lo, mid, s, e, out);
        self.cover(2 * node + 2, mid + 1, hi, s, e, out);
    }

    /// Returns `true` if `element` may have appeared during `[start, end]` (no false negatives), `false`
    /// if it certainly did not.
    pub fn query(&self, element: u64, start: u64, end: u64) -> bool {
        let s = start.clamp(1, self.t_max);
        let e = end.clamp(1, self.t_max);
        if s > e {
            return false;
        }
        let mut cover = Vec::new();
        self.cover(0, 1, self.t_max, s, e, &mut cover);
        cover.iter().any(|&node| self.test_bits(node, element))
    }

    /// The time domain upper bound `T`.
    #[inline]
    pub fn t_max(&self) -> u64 {
        self.t_max
    }

    /// Number of node Bloom filters in the tree.
    #[inline]
    pub fn num_filters(&self) -> usize {
        self.num_nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(PersistentBloomFilter::new(1000, 1, 1024, 4).is_err()); // T not power of two
        assert!(PersistentBloomFilter::new(1024, 3, 1024, 4).is_err()); // g not power of two
        assert!(PersistentBloomFilter::new(1024, 2048, 1024, 4).is_err()); // g > T
        assert!(PersistentBloomFilter::new(1024, 1, 0, 4).is_err());
        assert!(PersistentBloomFilter::new(1024, 1, 1024, 17).is_err());
        assert!(PersistentBloomFilter::new(1024, 1, 1024, 4).is_ok());
    }

    #[test]
    fn no_false_negatives() {
        let mut pbf = PersistentBloomFilter::new(1024, 1, 8192, 5).unwrap();
        let pairs = [(7u64, 3u64), (7, 500), (42, 1000), (99, 1), (99, 999)];
        for &(x, t) in &pairs {
            pbf.insert(x, t);
        }
        for &(x, t) in &pairs {
            assert!(pbf.query(x, t, t), "false negative for ({x}, {t})");
            assert!(
                pbf.query(x, t.saturating_sub(5).max(1), (t + 5).min(1024)),
                "false negative for ({x}) around {t}"
            );
        }
    }

    #[test]
    fn temporal_precision_excludes_other_times() {
        // Element 100 appears only at time 50. Ranges not covering 50 must reject it (the dyadic cover
        // never falls back to the all-encompassing root).
        let mut pbf = PersistentBloomFilter::new(1024, 1, 16384, 6).unwrap();
        pbf.insert(100, 50);
        assert!(pbf.query(100, 50, 50));
        assert!(pbf.query(100, 33, 64));
        assert!(!pbf.query(100, 51, 60), "should be absent just after 50");
        assert!(!pbf.query(100, 1, 49), "should be absent before 50");
        assert!(!pbf.query(100, 300, 400), "should be absent far from 50");
    }

    #[test]
    fn bounded_false_positive_rate_for_absent_element() {
        let mut pbf = PersistentBloomFilter::new(4096, 4, 8192, 6).unwrap();
        // Populate the history with many elements at scattered times.
        for i in 0..2000u64 {
            pbf.insert(i, (i * 7) % 4096 + 1);
        }
        // Query elements that were never inserted over a mid-sized range.
        let trials = 5000u64;
        let fps = (1_000_000..1_000_000 + trials)
            .filter(|&x| pbf.query(x, 1000, 1200))
            .count();
        let fpr = fps as f64 / trials as f64;
        assert!(fpr < 0.05, "false positive rate {fpr} too high");
    }

    #[test]
    fn granularity_groups_timestamps() {
        // With granularity 16, timestamps in the same leaf block are indistinguishable.
        let mut pbf = PersistentBloomFilter::new(1024, 16, 4096, 5).unwrap();
        pbf.insert(5, 20); // leaf block [17, 32]
                           // Anything in the same block queries positive; a different block does not.
        assert!(pbf.query(5, 17, 32));
        assert!(pbf.query(5, 25, 25)); // same block, partial leaf
        assert!(!pbf.query(5, 100, 120), "different block should reject");
    }
}
