//! q-digest — a deterministic, mergeable quantile summary over a bounded integer universe.
//!
//! The q-digest (Shrivastava, Buragohain, Agrawal, Suri, "Medians and Beyond: New Aggregation
//! Techniques for Sensor Networks", SenSys 2004) summarizes a multiset drawn from a fixed integer
//! universe `[0, 2^L)`. It overlays a complete binary tree on that universe — the root covers the
//! whole range, each child half of its parent's, leaves single values — and keeps a count at a
//! sparse set of nodes. A *compression* invariant bounds the size to `O(k)` nodes: no light node may
//! retain its own count when it, its sibling, and its parent together hold at most `⌊N/k⌋` items —
//! such triples are merged upward into the parent, which then represents a coarser value range.
//!
//! Querying a rank or quantile walks the retained nodes in value order, charging each node's count to
//! the right end of the range it covers. The resulting rank error is at most `(L/k)·N`, and the
//! structure is **mergeable**: summaries from many sources add node-wise and re-compress, with the
//! same error bound — the property that makes it the canonical sensor-network quantile sketch.
//!
//! Unlike the comparison-based [`GreenwaldKhanna`](crate::quantiles::GreenwaldKhanna) or
//! [`KllSketch`](crate::quantiles::KllSketch), q-digest needs a bounded integer domain but in return
//! is fully deterministic and merges in closed form.

use crate::common::{Result, SketchError};
use std::collections::HashMap;

/// A q-digest over the universe `[0, 2^levels)` with compression parameter `compression`.
///
/// # Example
/// ```
/// use sketch_oxide::quantiles::QDigest;
///
/// let mut qd = QDigest::new(16, 1000).unwrap(); // universe [0, 65536), k = 1000
/// for v in 0..10_000u64 {
///     qd.insert(v).unwrap();
/// }
/// // Median of a 0..10000 ramp is ≈ 5000.
/// let median = qd.quantile(0.5).unwrap();
/// assert!((median as i64 - 5000).abs() < 500, "median {median}");
/// ```
#[derive(Debug, Clone)]
pub struct QDigest {
    /// Universe is `[0, 2^levels)`.
    levels: u32,
    /// Compression parameter `k`: larger ⇒ more accurate, more nodes.
    compression: u64,
    /// Total items inserted.
    n: u64,
    /// Sparse node counts, keyed by heap-style node id (root = 1, children `2i`, `2i+1`).
    nodes: HashMap<u64, u64>,
}

impl QDigest {
    /// Creates an empty q-digest over `[0, 2^levels)` with compression parameter `compression`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `levels` is 0 or above 62, or `compression` is 0.
    pub fn new(levels: u32, compression: u64) -> Result<Self> {
        if levels == 0 || levels > 62 {
            return Err(SketchError::InvalidParameter {
                param: "levels".to_string(),
                value: levels.to_string(),
                constraint: "must be in 1..=62".to_string(),
            });
        }
        if compression == 0 {
            return Err(SketchError::InvalidParameter {
                param: "compression".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            levels,
            compression,
            n: 0,
            nodes: HashMap::new(),
        })
    }

    /// Universe size `2^levels`.
    #[inline]
    fn universe(&self) -> u64 {
        1u64 << self.levels
    }

    /// Right endpoint (max value) of the universe range covered by node `id`.
    #[inline]
    fn node_vmax(&self, id: u64) -> u64 {
        let depth = id.ilog2();
        let pos = id - (1u64 << depth);
        let span = 1u64 << (self.levels - depth);
        pos * span + span - 1
    }

    /// Records one occurrence of `value`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `value >= 2^levels`.
    pub fn insert(&mut self, value: u64) -> Result<()> {
        if value >= self.universe() {
            return Err(SketchError::InvalidParameter {
                param: "value".to_string(),
                value: value.to_string(),
                constraint: format!("must be < 2^levels = {}", self.universe()),
            });
        }
        let leaf = self.universe() + value;
        *self.nodes.entry(leaf).or_insert(0) += 1;
        self.n += 1;
        // Amortized compaction keeps the node count bounded during a long stream.
        if self.nodes.len() as u64 > 8 * self.compression {
            self.compress();
        }
        Ok(())
    }

    /// Enforces the q-digest compression invariant, merging light sibling/parent triples upward.
    fn compress(&mut self) {
        let cap = self.n / self.compression;
        // Process from the leaves up so merged counts can cascade toward the root.
        for depth in (1..=self.levels).rev() {
            // Parents (at depth-1) whose children live at this depth.
            let mut parents: Vec<u64> = self
                .nodes
                .keys()
                .filter(|&&id| id > 1 && id.ilog2() == depth)
                .map(|&id| id >> 1)
                .collect();
            parents.sort_unstable();
            parents.dedup();

            for p in parents {
                let left = 2 * p;
                let right = 2 * p + 1;
                let cl = self.nodes.get(&left).copied().unwrap_or(0);
                let cr = self.nodes.get(&right).copied().unwrap_or(0);
                let cp = self.nodes.get(&p).copied().unwrap_or(0);
                let total = cl + cr + cp;
                if total <= cap {
                    // The triple is too light to be frequent: fold the children into the parent.
                    self.nodes.remove(&left);
                    self.nodes.remove(&right);
                    if total > 0 {
                        self.nodes.insert(p, total);
                    }
                }
            }
        }
    }

    /// Estimated `phi`-quantile (`phi` in `[0, 1]`): the value whose rank is about `phi·N`.
    /// Returns `None` if empty.
    pub fn quantile(&mut self, phi: f64) -> Option<u64> {
        if self.n == 0 {
            return None;
        }
        self.compress();
        let target = (phi.clamp(0.0, 1.0) * self.n as f64).ceil() as u64;
        let target = target.max(1);
        // Walk retained nodes in order of the right end of their range, accumulating counts.
        let mut ranges: Vec<(u64, u64)> = self
            .nodes
            .iter()
            .map(|(&id, &c)| (self.node_vmax(id), c))
            .collect();
        ranges.sort_unstable();
        let mut cum = 0u64;
        for (vmax, c) in ranges.iter() {
            cum += c;
            if cum >= target {
                return Some(*vmax);
            }
        }
        ranges.last().map(|&(vmax, _)| vmax)
    }

    /// Estimated number of items with value `≤ value` (the rank of `value`).
    pub fn rank(&self, value: u64) -> u64 {
        self.nodes
            .iter()
            .filter(|&(&id, _)| self.node_vmax(id) <= value)
            .map(|(_, &c)| c)
            .sum()
    }

    /// Merges `other` into `self` (node-wise add, then re-compress).
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the universes (`levels`) differ.
    pub fn merge(&mut self, other: &Self) -> Result<()> {
        if self.levels != other.levels {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "different universes: 2^{} vs 2^{}",
                    self.levels, other.levels
                ),
            });
        }
        for (&id, &c) in &other.nodes {
            *self.nodes.entry(id).or_insert(0) += c;
        }
        self.n += other.n;
        self.compress();
        Ok(())
    }

    /// Total number of items inserted (`N`).
    #[inline]
    pub fn count(&self) -> u64 {
        self.n
    }

    /// Number of nodes currently retained.
    #[inline]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether nothing has been inserted.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Universe bit-width `L` (universe is `[0, 2^L)`).
    #[inline]
    pub fn levels(&self) -> u32 {
        self.levels
    }
}

// Capability-trait adoptions (fable5 doc 01 F3): no clean match, documented.
// Update not implemented: the only ingest, `insert(u64) -> Result<()>`, is
// fallible (rejects out-of-range values), and `Update::update` has no error
// channel — swallowing the error would repeat the silent-drop anti-pattern this
// capability layer exists to resolve.
// QuantileQuery not implemented: quantile takes &mut self and returns
// `Option<u64>`, not `Option<f64>`.
// Serializable not implemented: QDigest has no `Sketch::serialize`/`deserialize`.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(QDigest::new(0, 100).is_err());
        assert!(QDigest::new(63, 100).is_err());
        assert!(QDigest::new(16, 0).is_err());
        assert!(QDigest::new(16, 100).is_ok());
    }

    #[test]
    fn rejects_out_of_range_value() {
        let mut qd = QDigest::new(4, 100).unwrap(); // universe [0, 16)
        assert!(qd.insert(15).is_ok());
        assert!(qd.insert(16).is_err());
    }

    #[test]
    fn exact_quantiles_without_compression() {
        // Huge compression ⇒ cap ≈ 0 ⇒ no merging ⇒ exact leaf counts.
        let mut qd = QDigest::new(14, 10_000_000).unwrap();
        for v in 0..1000u64 {
            qd.insert(v).unwrap();
        }
        assert_eq!(qd.quantile(0.5).unwrap(), 499);
        assert_eq!(qd.quantile(0.1).unwrap(), 99);
        assert_eq!(qd.quantile(0.9).unwrap(), 899);
    }

    #[test]
    fn approximate_quantiles_within_error_bound() {
        let mut qd = QDigest::new(16, 1000).unwrap();
        for v in 0..10_000u64 {
            qd.insert(v).unwrap();
        }
        // Rank error ≤ (L/k)·N = 16·10000/1000 = 160; allow generous slack.
        for (phi, truth) in [(0.1, 1000.0), (0.5, 5000.0), (0.9, 9000.0), (0.99, 9900.0)] {
            let q = qd.quantile(phi).unwrap() as f64;
            assert!(
                (q - truth).abs() < 600.0,
                "phi {phi}: got {q}, want ~{truth}"
            );
        }
    }

    #[test]
    fn compression_bounds_node_count() {
        let mut qd = QDigest::new(20, 500).unwrap();
        for v in 0..100_000u64 {
            qd.insert(v % (1 << 20)).unwrap();
        }
        qd.quantile(0.5); // forces a final compress
        // q-digest keeps O(k) nodes; comfortably under a small multiple of k.
        assert!(qd.len() < 10 * 500, "node count {}", qd.len());
        assert_eq!(qd.count(), 100_000);
    }

    #[test]
    fn rank_is_monotonic() {
        let mut qd = QDigest::new(14, 5000).unwrap();
        for v in 0..5000u64 {
            qd.insert(v).unwrap();
        }
        qd.quantile(0.5);
        assert!(qd.rank(1000) <= qd.rank(2000));
        assert!(qd.rank(2000) <= qd.rank(4999));
        assert_eq!(qd.rank(u64::MAX >> 2), 5000); // everything is ≤ a huge bound
    }

    #[test]
    fn merge_combines_distributions() {
        let mut a = QDigest::new(16, 2000).unwrap();
        let mut b = QDigest::new(16, 2000).unwrap();
        for v in 0..5000u64 {
            a.insert(v).unwrap();
        }
        for v in 5000..10_000u64 {
            b.insert(v).unwrap();
        }
        a.merge(&b).unwrap();
        assert_eq!(a.count(), 10_000);
        let median = a.quantile(0.5).unwrap() as f64;
        assert!((median - 5000.0).abs() < 600.0, "merged median {median}");
    }

    #[test]
    fn merge_rejects_different_universe() {
        let mut a = QDigest::new(16, 1000).unwrap();
        let b = QDigest::new(14, 1000).unwrap();
        assert!(a.merge(&b).is_err());
    }

    #[test]
    fn empty_quantile_is_none() {
        let mut qd = QDigest::new(8, 100).unwrap();
        assert!(qd.is_empty());
        assert!(qd.quantile(0.5).is_none());
    }
}
