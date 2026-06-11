//! Double-Anonymous Sketch — top-K-*fair* global top-K across disjoint streams (Zhao, Han, Zhong,
//! Zhang, Yang & Cui, SIGMOD/PACMMOD 2023).
//!
//! When several sites each hold a *disjoint* slice of a key space and we want the **global** top-K
//! frequent items, the usual trick — run a top-K sketch per site and merge the local top-Ks — is
//! biased: top-K sketches (Space-Saving, even *unbiased* ones) systematically **over-estimate the
//! items they select** and under-estimate the rest. So a genuinely hot item that happens to live in a
//! *light* stream can be out-shouted by inflated estimates from *heavy* streams and dropped from the
//! global answer. The paper formalises this as **top-K-fairness** and proves a sufficient condition,
//! **double-anonymity**: keep the *top-K part* (which decides membership) and the *count part* (which
//! estimates frequency) **independent**, so the frequency estimate is uncorrelated with whether an
//! item was selected.
//!
//! This reference implements the **basic double-anonymous sketch**: every item is inserted
//! independently into (a) a **Space-Saving** top-K part that decides candidacy and (b) an **unbiased
//! Count-Mean** sketch that estimates frequency. A query reports the top-K *set* from the Space-Saving
//! part with each frequency taken *only* from the count part. Sketches from different sites
//! [`merge`](DoubleAnonymousSketch::merge) by summing the (linear) count parts and unioning the
//! candidate sets, so the global top-K is fair and unbiased.
//!
//! The paper's two further accuracy optimisations — **hot panning** (record hot items only in the
//! top-K part to drop redundancy) and **early freezing** (a freezing counter that caps accumulating
//! error) — are layered on this basic, double-anonymous core and are noted here as extensions.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

const SEED_MIX: u64 = 0x9E37_79B9_7F4A_7C15;

/// A Space-Saving top-K part: up to `k` `(item, count)` slots.
#[derive(Debug, Clone)]
struct SpaceSaving {
    k: usize,
    entries: Vec<(u64, u64)>,
}

impl SpaceSaving {
    fn new(k: usize) -> Self {
        Self {
            k,
            entries: Vec::with_capacity(k),
        }
    }

    /// Inserts `weight` occurrences of `item` (Space-Saving: a full summary replaces its min slot,
    /// setting the new count to the displaced minimum plus `weight`).
    fn insert(&mut self, item: u64, weight: u64) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.0 == item) {
            e.1 += weight;
            return;
        }
        if self.entries.len() < self.k {
            self.entries.push((item, weight));
            return;
        }
        let (min_idx, min_count) = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| (i, e.1))
            .min_by_key(|&(_, c)| c)
            .unwrap();
        self.entries[min_idx] = (item, min_count + weight);
    }
}

/// A Double-Anonymous Sketch over `u64` item keys.
///
/// # Example
/// ```
/// use sketch_oxide::frequency::DoubleAnonymousSketch;
///
/// // Two disjoint sites. The global top item (1000, freq 3000) lives in the "light" site B.
/// let mut a = DoubleAnonymousSketch::new(4096, 4, 64, 1).unwrap();
/// let mut b = DoubleAnonymousSketch::new(4096, 4, 64, 1).unwrap();
/// for i in 0..1000u64 { a.insert(i); }          // site A: many items once each ...
/// for _ in 0..2000 { a.insert(0); }             // ... plus a heavy item 0 (freq ~2000)
/// for _ in 0..3000 { b.insert(1000); }          // site B: one very hot item (freq 3000)
///
/// // Merge sites and ask for the fair global top-2.
/// a.merge(&b).unwrap();
/// let top = a.top_k(2);
/// assert_eq!(top[0].0, 1000); // global #1, from the light site, not dropped
/// assert_eq!(top[1].0, 0);    // global #2
/// ```
#[derive(Debug, Clone)]
pub struct DoubleAnonymousSketch {
    width: usize,
    depth: usize,
    seed: u64,
    counters: Vec<u64>, // count part: depth × width
    total: u64,
    topk: SpaceSaving,
}

impl DoubleAnonymousSketch {
    /// Creates a sketch with a `width`×`depth` unbiased count part and a `k`-slot top-K part.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `width < 2`, `depth` is not in `1..=16`, or `k == 0`.
    pub fn new(width: usize, depth: usize, k: usize, seed: u64) -> Result<Self> {
        if width < 2 {
            return Err(SketchError::InvalidParameter {
                param: "width".to_string(),
                value: width.to_string(),
                constraint: "must be >= 2".to_string(),
            });
        }
        if !(1..=16).contains(&depth) {
            return Err(SketchError::InvalidParameter {
                param: "depth".to_string(),
                value: depth.to_string(),
                constraint: "must be in 1..=16".to_string(),
            });
        }
        if k == 0 {
            return Err(SketchError::InvalidParameter {
                param: "k".to_string(),
                value: "0".to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        Ok(Self {
            width,
            depth,
            seed,
            counters: vec![0; width * depth],
            total: 0,
            topk: SpaceSaving::new(k),
        })
    }

    fn col(&self, item: u64, r: usize) -> usize {
        let h = xxhash(
            &item.to_le_bytes(),
            self.seed ^ (r as u64).wrapping_mul(SEED_MIX),
        );
        (h % self.width as u64) as usize
    }

    /// Inserts one occurrence of `item` into both parts independently (double-anonymity).
    pub fn insert(&mut self, item: u64) {
        self.total += 1;
        for r in 0..self.depth {
            let c = self.col(item, r);
            self.counters[r * self.width + c] += 1;
        }
        self.topk.insert(item, 1);
    }

    /// Unbiased Count-Mean frequency estimate of `item` from the count part: the average over rows of
    /// `counter − (N − counter)/(width − 1)`, which removes the expected hash-collision noise.
    pub fn estimate(&self, item: u64) -> f64 {
        let n = self.total as f64;
        let denom = (self.width - 1) as f64;
        let mut sum = 0.0;
        for r in 0..self.depth {
            let a = self.counters[r * self.width + self.col(item, r)] as f64;
            sum += a - (n - a) / denom;
        }
        (sum / self.depth as f64).max(0.0)
    }

    /// The top-K candidate items (from the top-K part only).
    pub fn candidates(&self) -> Vec<u64> {
        self.topk.entries.iter().map(|e| e.0).collect()
    }

    /// Reports up to `k` items as `(item, estimated_frequency)`, most frequent first. Membership comes
    /// from the top-K part; each frequency comes *only* from the count part (the fair, double-anonymous
    /// report).
    pub fn top_k(&self, k: usize) -> Vec<(u64, f64)> {
        let mut out: Vec<(u64, f64)> = self
            .candidates()
            .into_iter()
            .map(|item| (item, self.estimate(item)))
            .collect();
        out.sort_unstable_by(|a, b| b.1.total_cmp(&a.1));
        out.truncate(k);
        out
    }

    /// Total number of items inserted (across both parts is identical; this is the stream length).
    #[inline]
    pub fn total(&self) -> u64 {
        self.total
    }

    /// Merges another site's sketch into this one: sums the linear count parts and unions the candidate
    /// sets. The merged sketch reports the fair global top-K. Both must share `(width, depth, seed)` and
    /// the top-K capacity.
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the parameters differ.
    pub fn merge(&mut self, other: &Self) -> Result<()> {
        if self.width != other.width
            || self.depth != other.depth
            || self.seed != other.seed
            || self.topk.k != other.topk.k
        {
            return Err(SketchError::IncompatibleSketches {
                reason: "DoubleAnonymousSketch instances must share (width, depth, seed, k)"
                    .to_string(),
            });
        }
        for (a, b) in self.counters.iter_mut().zip(&other.counters) {
            *a += *b;
        }
        self.total += other.total;
        for &(item, count) in &other.topk.entries {
            self.topk.insert(item, count);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(DoubleAnonymousSketch::new(1, 4, 8, 1).is_err());
        assert!(DoubleAnonymousSketch::new(64, 0, 8, 1).is_err());
        assert!(DoubleAnonymousSketch::new(64, 17, 8, 1).is_err());
        assert!(DoubleAnonymousSketch::new(64, 4, 0, 1).is_err());
        assert!(DoubleAnonymousSketch::new(64, 4, 8, 1).is_ok());
    }

    #[test]
    fn finds_local_top_k_in_order() {
        let mut s = DoubleAnonymousSketch::new(4096, 4, 16, 7).unwrap();
        for i in 0..2000u64 {
            s.insert(i); // background, once each
        }
        for _ in 0..5000 {
            s.insert(100);
        }
        for _ in 0..3000 {
            s.insert(200);
        }
        let top = s.top_k(2);
        assert_eq!(top[0].0, 100);
        assert_eq!(top[1].0, 200);
        assert!(
            (top[0].1 - 5000.0).abs() / 5000.0 < 0.10,
            "est {}",
            top[0].1
        );
    }

    #[test]
    fn unbiased_estimate_is_accurate_for_heavy_item() {
        let mut s = DoubleAnonymousSketch::new(8192, 5, 32, 3).unwrap();
        for i in 0..5000u64 {
            s.insert(i);
        }
        for _ in 0..10_000 {
            s.insert(42);
        }
        let est = s.estimate(42);
        assert!((est - 10_000.0).abs() / 10_000.0 < 0.08, "estimate {est}");
    }

    #[test]
    fn fair_global_top_k_keeps_hot_item_in_light_stream() {
        // Site A is "heavy": 3000 distinct items, each fairly frequent. Site B is "light": few items,
        // but holds the true global #1. A naive per-site top-k with inflated estimates could drop it;
        // the double-anonymous merge must keep it.
        let mut a = DoubleAnonymousSketch::new(8192, 4, 64, 1).unwrap();
        let mut b = DoubleAnonymousSketch::new(8192, 4, 64, 1).unwrap();
        for i in 0..3000u64 {
            for _ in 0..3 {
                a.insert(i); // heavy site: each item appears 3 times
            }
        }
        a.insert(10); // make item 10 the heavy site's local hot item
        for _ in 0..400 {
            a.insert(10);
        }
        for _ in 0..5000 {
            b.insert(900_000); // light site: one very hot item, the true global #1
        }
        a.merge(&b).unwrap();
        let top = a.top_k(3);
        assert_eq!(
            top[0].0, 900_000,
            "global #1 from the light site dropped: {top:?}"
        );
        assert!(
            (top[0].1 - 5000.0).abs() / 5000.0 < 0.15,
            "est {}",
            top[0].1
        );
    }

    #[test]
    fn merge_rejects_incompatible() {
        let mut a = DoubleAnonymousSketch::new(64, 4, 8, 1).unwrap();
        let b = DoubleAnonymousSketch::new(64, 4, 8, 2).unwrap();
        assert!(a.merge(&b).is_err());
    }
}
