//! Persistent Count-Min — *temporal frequency* queries over the entire history via a dyadic time
//! decomposition (the counting analog of the [Persistent Bloom
//! Filter](crate::streaming::PersistentBloomFilter); dyadic-ranges framework of Cormode &
//! Muthukrishnan).
//!
//! A Count-Min sketch answers "how often did `x` occur?"; a Persistent Count-Min answers the
//! **temporal** "how often did `x` occur during `[s, e]`?" — frequency over an arbitrary historical
//! window, for forensic/audit analytics. Over a time domain `[1, T]` (a power of two) with leaf
//! granularity `g`, it builds a binary tree with **one Count-Min sketch per node**, where the node at
//! level `ℓ` summarises the elements seen in its `T/2^ℓ`-length interval.
//!
//! * **Insert** `(x, t)`: increment `x` in the Count-Min sketch at every node on the root→leaf(t) path.
//! * **Query** `(x, [s,e])`: decompose `[s,e]` into its `O(log T)` disjoint dyadic **canonical cover**
//!   pieces and **sum** each piece's Count-Min estimate of `x`.
//!
//! Because the canonical cover tiles `[s,e]` disjointly and each node's Count-Min never under-counts,
//! the summed estimate **never underestimates** the true range frequency (overestimation comes from
//! hash collisions). The dyadic decomposition keeps the query to `O(log T)` sketch probes regardless of
//! the window width.
//!
//! This is the clean dyadic-decomposition construction; Wei et al.'s sampling-based *persistent sketch*
//! (SIGMOD 2015) trades exactness for `O(log)` space per counter and is the more space-efficient
//! alternative.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

const SEED_MIX: u64 = 0x9E37_79B9_7F4A_7C15;

/// A Persistent Count-Min answering temporal frequency over `[1, T]`.
///
/// # Example
/// ```
/// use sketch_oxide::streaming::PersistentCountMin;
///
/// // Time domain [1, 1024], per-timestamp granularity, 512-wide depth-4 node sketches.
/// let mut pcm = PersistentCountMin::new(1024, 1, 512, 4, 7).unwrap();
/// for _ in 0..5 { pcm.insert(42, 10); }    // element 42 occurred 5 times at t=10
/// for _ in 0..3 { pcm.insert(42, 800); }   // and 3 times at t=800
///
/// assert_eq!(pcm.estimate_range(42, 1, 1024), 8); // 8 over the whole history
/// assert_eq!(pcm.estimate_range(42, 1, 100), 5);  // only the t=10 occurrences
/// assert_eq!(pcm.estimate_range(42, 500, 900), 3);
/// assert_eq!(pcm.estimate_range(42, 100, 700), 0); // none in this gap
/// ```
#[derive(Debug, Clone)]
pub struct PersistentCountMin {
    t_max: u64,
    granularity: u64,
    leaves: u64,
    num_nodes: usize,
    width: usize,
    depth: usize,
    seed: u64,
    counters: Vec<u64>, // num_nodes × depth × width
}

impl PersistentCountMin {
    /// Creates a Persistent Count-Min over `[1, t_max]` with leaf `granularity` and `width`×`depth`
    /// Count-Min sketches per tree node.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `t_max`/`granularity` are not powers of two with
    /// `1 ≤ granularity ≤ t_max`, `width == 0`, or `depth` is not in `1..=16`.
    pub fn new(
        t_max: u64,
        granularity: u64,
        width: usize,
        depth: usize,
        seed: u64,
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
        if width == 0 {
            return Err(SketchError::InvalidParameter {
                param: "width".to_string(),
                value: "0".to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        if !(1..=16).contains(&depth) {
            return Err(SketchError::InvalidParameter {
                param: "depth".to_string(),
                value: depth.to_string(),
                constraint: "must be in 1..=16".to_string(),
            });
        }
        let leaves = t_max / granularity;
        let num_nodes = (2 * leaves - 1) as usize;
        Ok(Self {
            t_max,
            granularity,
            leaves,
            num_nodes,
            width,
            depth,
            seed,
            counters: vec![0; num_nodes * depth * width],
        })
    }

    fn leaf_node(&self, t: u64) -> usize {
        (self.leaves - 1 + (t - 1) / self.granularity) as usize
    }

    fn col(&self, item: u64, r: usize) -> usize {
        let h = xxhash(
            &item.to_le_bytes(),
            self.seed ^ (r as u64).wrapping_mul(SEED_MIX),
        );
        (h % self.width as u64) as usize
    }

    /// Increments `item`'s Count-Min counters in node `node`.
    fn bump(&mut self, node: usize, item: u64) {
        let base = node * self.depth * self.width;
        for r in 0..self.depth {
            let c = self.col(item, r);
            self.counters[base + r * self.width + c] += 1;
        }
    }

    /// Count-Min estimate of `item` in node `node` (min over rows).
    fn node_estimate(&self, node: usize, item: u64) -> u64 {
        let base = node * self.depth * self.width;
        (0..self.depth)
            .map(|r| self.counters[base + r * self.width + self.col(item, r)])
            .min()
            .unwrap_or(0)
    }

    /// Records one occurrence of `item` at `timestamp` (clamped to `[1, t_max]`).
    pub fn insert(&mut self, item: u64, timestamp: u64) {
        let t = timestamp.clamp(1, self.t_max);
        let mut node = self.leaf_node(t);
        loop {
            self.bump(node, item);
            if node == 0 {
                break;
            }
            node = (node - 1) / 2;
        }
    }

    fn cover(&self, node: usize, lo: u64, hi: u64, s: u64, e: u64, out: &mut Vec<usize>) {
        if e < lo || hi < s {
            return;
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

    /// Estimated number of occurrences of `item` during `[start, end]` (never an underestimate of the
    /// true range frequency).
    pub fn estimate_range(&self, item: u64, start: u64, end: u64) -> u64 {
        let s = start.clamp(1, self.t_max);
        let e = end.clamp(1, self.t_max);
        if s > e {
            return 0;
        }
        let mut cover = Vec::new();
        self.cover(0, 1, self.t_max, s, e, &mut cover);
        cover
            .iter()
            .map(|&node| self.node_estimate(node, item))
            .sum()
    }

    /// The time domain upper bound `T`.
    #[inline]
    pub fn t_max(&self) -> u64 {
        self.t_max
    }

    /// Number of node Count-Min sketches in the tree.
    #[inline]
    pub fn num_sketches(&self) -> usize {
        self.num_nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(PersistentCountMin::new(1000, 1, 256, 4, 1).is_err()); // T not power of two
        assert!(PersistentCountMin::new(1024, 3, 256, 4, 1).is_err()); // g not power of two
        assert!(PersistentCountMin::new(1024, 2048, 256, 4, 1).is_err()); // g > T
        assert!(PersistentCountMin::new(1024, 1, 0, 4, 1).is_err());
        assert!(PersistentCountMin::new(1024, 1, 256, 17, 1).is_err());
        assert!(PersistentCountMin::new(1024, 1, 256, 4, 1).is_ok());
    }

    #[test]
    fn never_underestimates_range_frequency() {
        let mut pcm = PersistentCountMin::new(1024, 1, 4096, 5, 9).unwrap();
        // Insert element 7 a known number of times in [200, 300]; plus noise elsewhere.
        let mut truth = 0u64;
        for t in 200..=300u64 {
            for _ in 0..2 {
                pcm.insert(7, t);
                truth += 1;
            }
        }
        for i in 0..5000u64 {
            pcm.insert(i, (i % 1024) + 1);
        }
        let est = pcm.estimate_range(7, 200, 300);
        assert!(est >= truth, "estimate {est} underestimated truth {truth}");
        assert!(
            est < truth + truth / 5 + 50,
            "estimate {est} much larger than truth {truth}"
        );
    }

    #[test]
    fn temporal_decomposition_is_exact_without_collisions() {
        // With a wide sketch and few elements, estimates are exact.
        let mut pcm = PersistentCountMin::new(1024, 1, 8192, 4, 3).unwrap();
        for _ in 0..5 {
            pcm.insert(42, 10);
        }
        for _ in 0..3 {
            pcm.insert(42, 800);
        }
        assert_eq!(pcm.estimate_range(42, 1, 1024), 8);
        assert_eq!(pcm.estimate_range(42, 1, 100), 5);
        assert_eq!(pcm.estimate_range(42, 500, 900), 3);
        assert_eq!(pcm.estimate_range(42, 100, 700), 0);
        assert_eq!(pcm.estimate_range(42, 10, 10), 5);
    }

    #[test]
    fn absent_element_estimates_zero_with_wide_sketch() {
        let mut pcm = PersistentCountMin::new(512, 1, 8192, 5, 5).unwrap();
        for t in 1..=400u64 {
            pcm.insert(t, t); // each value once at its own time
        }
        // A value that was never inserted reads ~0 over any window.
        assert_eq!(pcm.estimate_range(999_999, 1, 512), 0);
    }

    #[test]
    fn granularity_groups_timestamps() {
        let mut pcm = PersistentCountMin::new(1024, 16, 4096, 4, 1).unwrap();
        for _ in 0..4 {
            pcm.insert(5, 20); // leaf block [17, 32]
        }
        assert_eq!(pcm.estimate_range(5, 17, 32), 4);
        assert_eq!(pcm.estimate_range(5, 25, 25), 4); // same block, partial leaf
        assert_eq!(pcm.estimate_range(5, 100, 120), 0);
    }
}
