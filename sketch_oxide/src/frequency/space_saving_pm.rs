//! Space-Saving± — frequent items under bounded deletions (Double Space-Saving).
//!
//! Plain Space-Saving assumes an insert-only stream. Many real workloads delete too
//! (GDPR erasure, materialized-view maintenance, retractions). Space-Saving± solves the
//! frequent-items / frequency-estimation problem in the **bounded-deletion model** — where
//! at every prefix each item's net count is non-negative and deletions are bounded relative
//! to insertions — with the *Double Space-Saving* construction: run one Space-Saving sketch
//! over the insertion sub-stream and another over the deletion sub-stream, and report the
//! difference of their estimates as the net frequency.
//!
//! Because each sub-sketch keeps the standard Space-Saving error guarantee, the net estimate
//! is accurate whenever deletions are bounded — exactly the regime this model targets. The
//! construction is space-optimal for deterministic bounded-deletion frequent items, and no
//! mainstream library ships it.

use crate::common::Result;
use crate::frequency::SpaceSaving;
use std::hash::Hash;

/// Space-Saving± over a stream of insertions and deletions.
///
/// # Example
/// ```
/// use sketch_oxide::frequency::SpaceSavingPlusMinus;
///
/// let mut ss = SpaceSavingPlusMinus::with_capacity(64).unwrap();
/// for _ in 0..100 { ss.insert("a"); }
/// for _ in 0..30  { ss.delete("a"); }   // net 70
/// for _ in 0..50 { ss.insert("b"); }
///
/// assert!((ss.estimate(&"a") as i64 - 70).abs() <= 2);
/// assert!((ss.estimate(&"b") as i64 - 50).abs() <= 2);
/// ```
#[derive(Debug, Clone)]
pub struct SpaceSavingPlusMinus<T: Hash + Eq + Clone> {
    inserts: SpaceSaving<T>,
    deletes: SpaceSaving<T>,
}

impl<T: Hash + Eq + Clone> SpaceSavingPlusMinus<T> {
    /// Creates a Space-Saving± with per-sub-sketch error parameter `epsilon` (in `(0, 1)`).
    ///
    /// # Errors
    /// Propagates [`SpaceSaving::new`]'s validation error.
    pub fn new(epsilon: f64) -> Result<Self> {
        Ok(Self {
            inserts: SpaceSaving::new(epsilon)?,
            deletes: SpaceSaving::new(epsilon)?,
        })
    }

    /// Creates a Space-Saving± where each sub-sketch tracks up to `capacity` items.
    ///
    /// # Errors
    /// Propagates [`SpaceSaving::with_capacity`]'s validation error.
    pub fn with_capacity(capacity: usize) -> Result<Self> {
        Ok(Self {
            inserts: SpaceSaving::with_capacity(capacity)?,
            deletes: SpaceSaving::with_capacity(capacity)?,
        })
    }

    /// Records one insertion of `item`.
    pub fn insert(&mut self, item: T) {
        self.inserts.update(item);
    }

    /// Records one deletion of `item`.
    pub fn delete(&mut self, item: T) {
        self.deletes.update(item);
    }

    /// Estimated net frequency of `item` (insertions − deletions), clamped at 0.
    pub fn estimate(&self, item: &T) -> u64 {
        let ins = self.inserts.estimate(item).map_or(0, |(_, upper)| upper);
        let del = self.deletes.estimate(item).map_or(0, |(_, upper)| upper);
        ins.saturating_sub(del)
    }

    /// Returns items whose estimated net frequency is at least `min_net`, as
    /// `(item, net_count)`, sorted by net count descending.
    ///
    /// Candidates are the items monitored by the insertion sub-sketch (anything heavy must be
    /// among them); each candidate's net frequency is computed against the deletion sketch.
    pub fn heavy_hitters(&self, min_net: u64) -> Vec<(T, u64)> {
        let mut out: Vec<(T, u64)> = self
            .inserts
            .top_k(self.inserts.capacity())
            .into_iter()
            .filter_map(|(item, _lower, _upper)| {
                let net = self.estimate(&item);
                (net >= min_net).then_some((item, net))
            })
            .collect();
        out.sort_by_key(|e| std::cmp::Reverse(e.1));
        out
    }

    /// Per-sub-sketch capacity.
    pub fn capacity(&self) -> usize {
        self.inserts.capacity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_epsilon_errors() {
        assert!(SpaceSavingPlusMinus::<u64>::new(0.0).is_err());
        assert!(SpaceSavingPlusMinus::<u64>::new(1.0).is_err());
        assert!(SpaceSavingPlusMinus::<u64>::new(0.01).is_ok());
    }

    #[test]
    fn net_frequency_subtracts_deletions() {
        let mut ss = SpaceSavingPlusMinus::with_capacity(64).unwrap();
        for _ in 0..1000 {
            ss.insert("x");
        }
        for _ in 0..400 {
            ss.delete("x");
        }
        let net = ss.estimate(&"x") as i64;
        assert!((net - 600).abs() <= 5, "net estimate {net}");
    }

    #[test]
    fn fully_deleted_item_is_zero() {
        let mut ss = SpaceSavingPlusMinus::with_capacity(64).unwrap();
        for _ in 0..50 {
            ss.insert("gone");
        }
        for _ in 0..50 {
            ss.delete("gone");
        }
        assert_eq!(ss.estimate(&"gone"), 0);
    }

    #[test]
    fn unseen_item_is_zero() {
        let ss = SpaceSavingPlusMinus::<&str>::with_capacity(64).unwrap();
        assert_eq!(ss.estimate(&"never"), 0);
    }

    #[test]
    fn heavy_hitters_reflect_net_counts() {
        let mut ss = SpaceSavingPlusMinus::with_capacity(128).unwrap();
        for _ in 0..1000 {
            ss.insert("heavy");
        }
        for _ in 0..900 {
            ss.delete("heavy"); // net 100
        }
        for _ in 0..500 {
            ss.insert("steady"); // net 500
        }
        let hh = ss.heavy_hitters(200);
        // "steady" (net 500) qualifies; "heavy" (net 100) does not.
        assert!(hh.iter().any(|(k, _)| *k == "steady"));
        assert!(!hh.iter().any(|(k, _)| *k == "heavy"));
        // Sorted descending: top is the largest net.
        assert_eq!(hh.first().map(|(k, _)| *k), Some("steady"));
    }

    #[test]
    fn many_keys_with_deletions() {
        // Capacity comfortably exceeds the 500 distinct keys, so both sub-sketches monitor
        // every key exactly (no eviction error).
        let mut ss = SpaceSavingPlusMinus::with_capacity(1024).unwrap();
        for _ in 0..10 {
            for i in 0..500u64 {
                ss.insert(i);
            }
        }
        for _ in 0..5 {
            for i in (0..500u64).step_by(2) {
                ss.delete(i);
            }
        }
        // Even key: 10 - 5 = 5 net; odd key: 10 net (exact when fully monitored).
        assert_eq!(ss.estimate(&100), 5, "even key net");
        assert_eq!(ss.estimate(&101), 10, "odd key net");
    }
}

// --- Capability-trait adoption (fable5 doc 01 F3) ---
use crate::common::capabilities::{PointQuery, Update};

impl<T: std::hash::Hash + Eq + Clone> Update<T> for SpaceSavingPlusMinus<T> {
    fn update(&mut self, item: &T) {
        self.insert(item.clone());
    }
}

impl<T: std::hash::Hash + Eq + Clone> PointQuery<T> for SpaceSavingPlusMinus<T> {
    fn query(&self, item: &T) -> u64 {
        self.estimate(item)
    }
}
