//! Lossy Counting — deterministic approximate frequency counting (Manku & Motwani, VLDB 2002).
//!
//! Lossy Counting answers "which items appear in more than a `s` fraction of the stream, and roughly
//! how often?" with **deterministic** error guarantees and `O((1/ε)·log(εN))` space. The stream is
//! conceptually cut into buckets of `w = ⌈1/ε⌉ items`. Each tracked element carries an exact-since
//! count `f` and a maximum-possible-error `Δ` (the bucket boundary at which it was first seen). At
//! every bucket boundary the structure prunes any element whose `f + Δ` has fallen to or below the
//! current bucket id — these cannot be frequent, so they are dropped, bounding the memory.
//!
//! The guarantees, with `N` items seen so far:
//! - **No false negatives.** Every element with true frequency `≥ sN` is reported by `query(s)`.
//! - **Bounded over-reporting.** No element with true frequency `< (s − ε)N` is reported.
//! - **Underestimation only, by `≤ εN`.** The stored `f` satisfies `true − εN ≤ f ≤ true`.
//!
//! Unlike the randomized sketches in this module ([`CountMinSketch`](crate::frequency::CountMinSketch),
//! [`HeavyKeeper`](crate::frequency::HeavyKeeper)), Lossy Counting's bounds are worst-case
//! deterministic — there is no failure probability.

use crate::common::{Result, SketchError};
use std::collections::HashMap;
use std::hash::Hash;

/// A per-element entry: count since insertion and the maximum error from before insertion.
#[derive(Debug, Clone, Copy)]
struct Entry {
    /// Occurrences counted since this element was inserted.
    f: u64,
    /// Maximum number of occurrences that could have been missed before insertion (`Δ`).
    delta: u64,
}

/// Deterministic approximate frequency counter over items of type `T`.
///
/// # Example
/// ```
/// use sketch_oxide::frequency::LossyCounting;
///
/// let mut lc = LossyCounting::<String>::new(0.01).unwrap(); // ε = 1%
/// // A heavy stream: 'a' dominates, 'b' moderate, plus a long tail of singletons.
/// for _ in 0..5000 { lc.insert("a".to_string()); }
/// for _ in 0..2000 { lc.insert("b".to_string()); }
/// for i in 0..3000u32 { lc.insert(format!("x{i}")); } // 3000 distinct singletons
///
/// // N = 10_000. Query items in > 30% of the stream: only 'a' (50%) qualifies, not 'b' (20%).
/// let heavy = lc.query(0.30);
/// assert!(heavy.iter().any(|(k, _)| k.as_str() == "a"));
/// assert!(!heavy.iter().any(|(k, _)| k.as_str() == "b"));
/// ```
#[derive(Debug, Clone)]
pub struct LossyCounting<T: Hash + Eq + Clone> {
    epsilon: f64,
    /// Bucket width `w = ⌈1/ε⌉`.
    width: u64,
    /// Total items seen.
    n: u64,
    entries: HashMap<T, Entry>,
}

impl<T: Hash + Eq + Clone> LossyCounting<T> {
    /// Creates a counter with error parameter `epsilon` (the per-element underestimate is `≤ εN`).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `epsilon` is not in `(0, 1)`.
    pub fn new(epsilon: f64) -> Result<Self> {
        if !(epsilon > 0.0 && epsilon < 1.0) {
            return Err(SketchError::InvalidParameter {
                param: "epsilon".to_string(),
                value: epsilon.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }
        Ok(Self {
            epsilon,
            width: (1.0 / epsilon).ceil() as u64,
            n: 0,
            entries: HashMap::new(),
        })
    }

    /// Current bucket id `⌈n / w⌉`.
    #[inline]
    fn current_bucket(&self) -> u64 {
        self.n.div_ceil(self.width)
    }

    /// Records one occurrence of `item`.
    pub fn insert(&mut self, item: T) {
        self.n += 1;
        let bucket = self.current_bucket();
        match self.entries.get_mut(&item) {
            Some(e) => e.f += 1,
            None => {
                self.entries.insert(
                    item,
                    Entry {
                        f: 1,
                        delta: bucket - 1,
                    },
                );
            }
        }
        // At each bucket boundary, prune entries that cannot become frequent.
        if self.n.is_multiple_of(self.width) {
            self.entries.retain(|_, e| e.f + e.delta > bucket);
        }
    }

    /// Estimated frequency of `item` (its stored `f`, or 0 if not tracked). Underestimates the true
    /// frequency by at most `εN`.
    pub fn estimate(&self, item: &T) -> u64 {
        self.entries.get(item).map_or(0, |e| e.f)
    }

    /// All elements with estimated frequency `≥ (s − ε)·N`, the Lossy-Counting answer to "which items
    /// appear in more than an `s` fraction of the stream?".
    ///
    /// Guarantees: every element with true frequency `≥ sN` is included (no false negatives), and no
    /// element with true frequency `< (s − ε)N` is included.
    pub fn query(&self, s: f64) -> Vec<(T, u64)> {
        let threshold = ((s - self.epsilon) * self.n as f64).max(0.0);
        self.entries
            .iter()
            .filter(|(_, e)| e.f as f64 >= threshold)
            .map(|(k, e)| (k.clone(), e.f))
            .collect()
    }

    /// Total number of items observed (`N`).
    #[inline]
    pub fn count(&self) -> u64 {
        self.n
    }

    /// Number of elements currently tracked.
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no elements are currently tracked.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Error parameter `ε`.
    #[inline]
    pub fn epsilon(&self) -> f64 {
        self.epsilon
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn rejects_bad_epsilon() {
        assert!(LossyCounting::<u64>::new(0.0).is_err());
        assert!(LossyCounting::<u64>::new(1.0).is_err());
        assert!(LossyCounting::<u64>::new(-0.1).is_err());
        assert!(LossyCounting::<u64>::new(0.01).is_ok());
    }

    #[test]
    fn underestimates_by_at_most_epsilon_n() {
        let mut lc = LossyCounting::new(0.01).unwrap();
        let mut truth: HashMap<u64, u64> = HashMap::new();
        // Mixed stream: a few heavy keys plus a long singleton tail.
        for i in 0..20_000u64 {
            let key = match i % 100 {
                0..=40 => 0,   // ~41% heavy
                41..=60 => 1,  // ~20%
                61..=70 => 2,  // ~10%
                _ => 1000 + i, // singleton tail
            };
            *truth.entry(key).or_default() += 1;
            lc.insert(key);
        }
        let n = lc.count();
        let eps_n = (0.01 * n as f64) as u64;
        for &k in &[0u64, 1, 2] {
            let est = lc.estimate(&k);
            let t = truth[&k];
            assert!(est <= t, "estimate {est} exceeds truth {t} for {k}");
            assert!(
                t - est <= eps_n,
                "underestimate {} > εN {} for {k}",
                t - est,
                eps_n
            );
        }
    }

    #[test]
    fn reports_all_heavy_hitters_no_false_negatives() {
        let mut lc = LossyCounting::new(0.005).unwrap();
        let mut truth: HashMap<u64, u64> = HashMap::new();
        for i in 0..40_000u64 {
            let key = match i % 100 {
                0..=49 => 0,     // 50%
                50..=74 => 1,    // 25%
                75..=84 => 2,    // 10%
                _ => 10_000 + i, // tail
            };
            *truth.entry(key).or_default() += 1;
            lc.insert(key);
        }
        let n = lc.count();
        let s = 0.08;
        let reported: std::collections::HashSet<u64> =
            lc.query(s).into_iter().map(|(k, _)| k).collect();
        // Every truly-≥ sN item must be reported.
        for (&k, &t) in &truth {
            if t as f64 >= s * n as f64 {
                assert!(reported.contains(&k), "missed heavy hitter {k} (count {t})");
            }
        }
        // The three planted heavies are present; no singleton is.
        assert!(reported.contains(&0) && reported.contains(&1) && reported.contains(&2));
        assert!(reported.iter().all(|&k| k < 3), "a tail item was reported");
    }

    #[test]
    fn pruning_bounds_tracked_entries() {
        // A stream of all-distinct items: nothing is frequent, so the table stays small (≤ ~1/ε).
        let mut lc = LossyCounting::new(0.01).unwrap();
        for i in 0..50_000u64 {
            lc.insert(i);
        }
        // Lossy Counting bounds the table to O((1/ε)·log(εN)); for ε=0.01 that is comfortably under
        // a few thousand, far below the 50_000 distinct items.
        assert!(lc.len() < 2000, "table grew to {}", lc.len());
        assert_eq!(lc.count(), 50_000);
    }

    #[test]
    fn empty_and_absent() {
        let lc = LossyCounting::<u64>::new(0.01).unwrap();
        assert!(lc.is_empty());
        assert_eq!(lc.estimate(&42), 0);
        assert!(lc.query(0.1).is_empty());
    }
}

// --- Capability-trait adoption (fable5 doc 01 F3) ---
use crate::common::capabilities::{PointQuery, Update};

impl<T: std::hash::Hash + Eq + Clone> Update<T> for LossyCounting<T> {
    fn update(&mut self, item: &T) {
        self.insert(item.clone());
    }
}

impl<T: std::hash::Hash + Eq + Clone> PointQuery<T> for LossyCounting<T> {
    fn query(&self, item: &T) -> u64 {
        self.estimate(item)
    }
}
