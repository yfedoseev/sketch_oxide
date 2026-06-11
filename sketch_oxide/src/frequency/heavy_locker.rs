//! HeavyLocker — heavy-hitter detection in distributed data streams via dynamic threshold locking
//! (Shi, Li, Zheng, Yang, Wang & Xu, KDD 2025).
//!
//! HeavyLocker exploits a property of real streams the authors call **separability**: heavy hitters
//! and non-heavy items separate cleanly around the *real-time* heavy-hitter threshold
//! `θ · item_num` (a fixed fraction `θ` of the items seen so far). An item that consistently stays
//! above that line is very likely a *final* heavy hitter, so it is worth protecting until the stream
//! ends.
//!
//! The structure is `w` buckets of `d` cells; each cell holds an item key and a count, kept sorted in
//! descending count order, and each bucket carries a **lock bit**. When a bucket is full and even its
//! *smallest* cell exceeds the real-time threshold (so *every* item in the bucket is a real-time
//! heavy hitter), the bucket **locks** — no further replacement is allowed, preserving those items.
//! The lock is re-evaluated on every insert, so a bucket unlocks again if the (growing) threshold
//! overtakes its smallest count. A tunable factor `L ≤ 1` (Optimization 1) lowers the locking line to
//! protect heavy hitters that ramp up slowly. Multi-hashing (Optimization 2) maps each item to
//! several candidate buckets to cut collisions.
//!
//! On insert, a matched key is incremented; an empty cell is filled; otherwise — if the bucket is
//! unlocked — the smallest cell is replaced with the **RAP** policy (probability `1/(min_count+1)`,
//! taking the slot at `min_count+1`). A locked bucket simply drops the new item. Because full keys
//! are stored, the sketch is **invertible** (heavy hitters are read directly) and **mergeable**:
//! co-located buckets from several per-stream HeavyLockers combine by summing per-key counts and
//! keeping the top `d`, yielding global heavy hitters as if measured by one detector.
//!
//! # Randomness
//!
//! The RAP draw `rand < 1/(min+1)` is realised exactly with integer arithmetic as
//! `xxhash(item, seed ⊕ item_num) mod (min+1) == 0` — deterministic and reproducible.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use std::collections::HashMap;

const SEED_HASH: u64 = 0x1EAF_10C0_0000_0001;
const SEED_RAP: u64 = 0x1EAF_10C0_0000_0002;
const ROW_STRIDE: u64 = 0x9E37_79B9_7F4A_7C15;

/// One cell: an item key and its estimated count (`None` key ⇔ empty ⇔ count 0).
#[derive(Debug, Clone)]
struct Cell {
    id: Option<Vec<u8>>,
    cnt: u64,
}

/// One bucket: `d` cells sorted by descending count, plus a lock bit.
#[derive(Debug, Clone)]
struct Bucket {
    cells: Vec<Cell>,
    locked: bool,
}

impl Bucket {
    fn new(d: usize) -> Self {
        Self {
            cells: (0..d).map(|_| Cell { id: None, cnt: 0 }).collect(),
            locked: false,
        }
    }
    /// Smallest count in the bucket (last cell, since sorted descending).
    fn min_cnt(&self) -> u64 {
        self.cells.last().map_or(0, |c| c.cnt)
    }
    /// A bucket is full when even its smallest cell is occupied.
    fn is_full(&self) -> bool {
        self.min_cnt() > 0
    }
    fn resort(&mut self) {
        self.cells.sort_by_key(|c| std::cmp::Reverse(c.cnt));
    }
}

/// A HeavyLocker sketch with `w` buckets of `d` cells, dynamic threshold locking, and merge support.
///
/// # Example
/// ```
/// use sketch_oxide::frequency::HeavyLocker;
///
/// let mut hl = HeavyLocker::new(1024, 4, 0.001, 1.0, 1).unwrap();
/// for _ in 0..50_000 { hl.insert(b"elephant"); }
/// for i in 0..20_000u32 { hl.insert(&i.to_le_bytes()); } // light background flows
///
/// assert!(hl.query(b"elephant") >= 49_000);
/// let hh = hl.heavy_hitters(0.01); // items above 1% of the stream
/// assert_eq!(hh[0].0, b"elephant");
/// ```
#[derive(Debug, Clone)]
pub struct HeavyLocker {
    w: usize,
    d: usize,
    theta: f64,
    lock_tuning: f64,
    num_hashes: usize,
    buckets: Vec<Bucket>,
    item_num: u64,
}

impl HeavyLocker {
    /// Creates a HeavyLocker with `w` buckets, `d` cells per bucket, real-time heavy-hitter fraction
    /// `theta` (`0 < θ < 1`), lock-tuning factor `lock_tuning` (`0 < L ≤ 1`; the paper lowers the
    /// locking line below the real-time threshold), and `num_hashes` candidate buckets per item
    /// (`≥ 1`; multi-hashing reduces collisions).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if any constraint above is violated.
    pub fn new(
        w: usize,
        d: usize,
        theta: f64,
        lock_tuning: f64,
        num_hashes: usize,
    ) -> Result<Self> {
        let err = |param: &str, value: String, constraint: &str| {
            Err(SketchError::InvalidParameter {
                param: param.to_string(),
                value,
                constraint: constraint.to_string(),
            })
        };
        if w < 1 {
            return err("w", w.to_string(), "must be >= 1");
        }
        if d < 1 {
            return err("d", d.to_string(), "must be >= 1");
        }
        if !(theta > 0.0 && theta < 1.0) {
            return err("theta", theta.to_string(), "must be in (0, 1)");
        }
        if !(lock_tuning > 0.0 && lock_tuning <= 1.0) {
            return err("lock_tuning", lock_tuning.to_string(), "must be in (0, 1]");
        }
        if num_hashes < 1 {
            return err("num_hashes", num_hashes.to_string(), "must be >= 1");
        }
        Ok(Self {
            w,
            d,
            theta,
            lock_tuning,
            num_hashes,
            buckets: (0..w).map(|_| Bucket::new(d)).collect(),
            item_num: 0,
        })
    }

    /// Number of items processed.
    #[inline]
    pub fn item_num(&self) -> u64 {
        self.item_num
    }

    /// Candidate bucket indices for `item`.
    fn candidates(&self, item: &[u8]) -> Vec<usize> {
        (0..self.num_hashes)
            .map(|h| {
                let seed = SEED_HASH.wrapping_add((h as u64).wrapping_mul(ROW_STRIDE));
                (xxhash(item, seed) % self.w as u64) as usize
            })
            .collect()
    }

    /// Current real-time locking threshold.
    fn lock_threshold(&self) -> u64 {
        (self.item_num as f64 * self.theta * self.lock_tuning) as u64
    }

    /// Re-evaluates a bucket's lock bit against the current threshold.
    fn update_lock(&mut self, bi: usize, threshold: u64) {
        let b = &mut self.buckets[bi];
        b.locked = b.is_full() && b.min_cnt() >= threshold;
    }

    /// Inserts one occurrence of `item` (paper Algorithm 1, with both optimizations).
    pub fn insert(&mut self, item: &[u8]) {
        self.item_num += 1;
        let threshold = self.lock_threshold();
        let cands = self.candidates(item);

        // 1) Already present in a candidate bucket: increment and re-sort.
        for &bi in &cands {
            self.update_lock(bi, threshold);
            if let Some(j) = self.buckets[bi]
                .cells
                .iter()
                .position(|c| c.cnt > 0 && c.id.as_deref() == Some(item))
            {
                self.buckets[bi].cells[j].cnt += 1;
                self.buckets[bi].resort();
                self.update_lock(bi, threshold);
                return;
            }
        }

        // 2) An empty cell in a candidate bucket: claim it.
        for &bi in &cands {
            if let Some(j) = self.buckets[bi].cells.iter().position(|c| c.cnt == 0) {
                self.buckets[bi].cells[j] = Cell {
                    id: Some(item.to_vec()),
                    cnt: 1,
                };
                self.buckets[bi].resort();
                self.update_lock(bi, threshold);
                return;
            }
        }

        // 3) All candidate buckets full: RAP-replace the smallest cell of the most promising
        //    *unlocked* candidate (the one whose smallest count is lowest).
        let target = cands
            .iter()
            .copied()
            .filter(|&bi| !self.buckets[bi].locked)
            .min_by_key(|&bi| self.buckets[bi].min_cnt());
        if let Some(bi) = target {
            let min_cnt = self.buckets[bi].min_cnt();
            let draw = xxhash(item, SEED_RAP ^ self.item_num) % (min_cnt + 1);
            if draw == 0 {
                let last = self.d - 1;
                self.buckets[bi].cells[last] = Cell {
                    id: Some(item.to_vec()),
                    cnt: min_cnt + 1,
                };
                self.buckets[bi].resort();
                self.update_lock(bi, threshold);
            }
        }
        // Otherwise every candidate is locked → the item is dropped.
    }

    /// Estimated count of `item` (0 if untracked).
    pub fn query(&self, item: &[u8]) -> u64 {
        let mut best = 0;
        for bi in self.candidates(item) {
            for c in &self.buckets[bi].cells {
                if c.cnt > 0 && c.id.as_deref() == Some(item) {
                    best = best.max(c.cnt);
                }
            }
        }
        best
    }

    /// Returns every tracked item whose count exceeds `phi · item_num`, as `(key, count)` sorted by
    /// descending count (ties by key). HeavyLocker is invertible, so this reads keys directly.
    pub fn heavy_hitters(&self, phi: f64) -> Vec<(Vec<u8>, u64)> {
        let threshold = (phi * self.item_num as f64) as u64;
        let mut best: HashMap<&[u8], u64> = HashMap::new();
        for b in &self.buckets {
            for c in &b.cells {
                if c.cnt > 0 {
                    if let Some(id) = &c.id {
                        let e = best.entry(id.as_slice()).or_insert(0);
                        *e = (*e).max(c.cnt);
                    }
                }
            }
        }
        let mut out: Vec<(Vec<u8>, u64)> = best
            .into_iter()
            .filter(|&(_, cnt)| cnt > threshold)
            .map(|(k, cnt)| (k.to_vec(), cnt))
            .collect();
        out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        out
    }

    /// Merges `other` into `self` (paper Algorithm 3): co-located buckets combine by summing
    /// per-key counts and keeping the top `d`, giving heavy hitters over the union of both streams.
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the two sketches differ in shape or configuration.
    pub fn merge(&mut self, other: &Self) -> Result<()> {
        if self.w != other.w
            || self.d != other.d
            || self.num_hashes != other.num_hashes
            || self.theta != other.theta
            || self.lock_tuning != other.lock_tuning
        {
            return Err(SketchError::IncompatibleSketches {
                reason: "HeavyLockers must share w, d, num_hashes, theta, and lock_tuning"
                    .to_string(),
            });
        }
        self.item_num += other.item_num;
        let threshold = self.lock_threshold();
        for bi in 0..self.w {
            // Stream-Summary-style combine of co-located buckets: sum counts per key.
            let mut agg: HashMap<Vec<u8>, u64> = HashMap::new();
            for src in [&self.buckets[bi], &other.buckets[bi]] {
                for c in &src.cells {
                    if c.cnt > 0 {
                        if let Some(id) = &c.id {
                            *agg.entry(id.clone()).or_insert(0) += c.cnt;
                        }
                    }
                }
            }
            let mut items: Vec<(Vec<u8>, u64)> = agg.into_iter().collect();
            items.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            items.truncate(self.d);
            let mut cells: Vec<Cell> = items
                .into_iter()
                .map(|(id, cnt)| Cell { id: Some(id), cnt })
                .collect();
            while cells.len() < self.d {
                cells.push(Cell { id: None, cnt: 0 });
            }
            self.buckets[bi].cells = cells;
            self.update_lock(bi, threshold);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn rejects_bad_params() {
        assert!(HeavyLocker::new(0, 4, 0.001, 1.0, 1).is_err());
        assert!(HeavyLocker::new(64, 0, 0.001, 1.0, 1).is_err());
        assert!(HeavyLocker::new(64, 4, 0.0, 1.0, 1).is_err());
        assert!(HeavyLocker::new(64, 4, 0.001, 1.5, 1).is_err());
        assert!(HeavyLocker::new(64, 4, 0.001, 1.0, 0).is_err());
        assert!(HeavyLocker::new(64, 4, 0.001, 1.0, 1).is_ok());
    }

    #[test]
    fn empty_reports_nothing() {
        let hl = HeavyLocker::new(64, 4, 0.001, 1.0, 1).unwrap();
        assert_eq!(hl.query(b"x"), 0);
        assert!(hl.heavy_hitters(0.0).is_empty());
    }

    #[test]
    fn isolated_item_is_exact() {
        let mut hl = HeavyLocker::new(64, 4, 0.001, 1.0, 1).unwrap();
        for _ in 0..1000 {
            hl.insert(b"solo");
        }
        assert_eq!(hl.query(b"solo"), 1000);
        assert_eq!(hl.item_num(), 1000);
    }

    #[test]
    fn lock_protects_heavy_item_from_eviction() {
        // A heavy item built up early should survive a long tail of distinct cold items that hash
        // around it — the lock prevents its bucket from being replaced away.
        let mut hl = HeavyLocker::new(256, 4, 0.001, 1.0, 2).unwrap();
        for _ in 0..20_000 {
            hl.insert(b"whale");
        }
        let after_build = hl.query(b"whale");
        for i in 0..200_000u32 {
            hl.insert(&i.to_le_bytes());
        }
        let after_noise = hl.query(b"whale");
        assert!(after_build >= 19_000, "build estimate {after_build}");
        // The protected heavy hitter is not evicted by the cold tail.
        assert!(
            after_noise >= 19_000,
            "heavy item lost to cold tail: {after_noise}"
        );
    }

    #[test]
    fn zipf_heavy_hitter_f1() {
        let universe = 2_000usize;
        let mut hl = HeavyLocker::new(1024, 4, 0.0005, 1.0, 2).unwrap();
        let mut freqs: Vec<(u32, u64)> = Vec::new();
        let mut total = 0u64;
        for r in 1..=universe {
            let f = (200_000u64 / r as u64).max(1);
            total += f;
            freqs.push((r as u32, f));
            let key = (r as u32).to_le_bytes();
            for _ in 0..f {
                hl.insert(&key);
            }
        }
        let phi = 0.001;
        let threshold = (phi * total as f64) as u64;
        let truth: HashSet<u32> = freqs
            .iter()
            .filter(|&&(_, f)| f > threshold)
            .map(|&(r, _)| r)
            .collect();
        let reported: HashSet<u32> = hl
            .heavy_hitters(phi)
            .iter()
            .map(|(key, _)| {
                let mut a = [0u8; 4];
                a.copy_from_slice(key);
                u32::from_le_bytes(a)
            })
            .collect();
        let tp = truth.intersection(&reported).count() as f64;
        let precision = if reported.is_empty() {
            1.0
        } else {
            tp / reported.len() as f64
        };
        let recall = if truth.is_empty() {
            1.0
        } else {
            tp / truth.len() as f64
        };
        let f1 = if precision + recall == 0.0 {
            0.0
        } else {
            2.0 * precision * recall / (precision + recall)
        };
        assert!(f1 >= 0.85, "F1 {f1:.3} (p {precision:.3}, r {recall:.3})");
    }

    #[test]
    fn merge_recovers_global_heavy_hitters() {
        // Two disjoint per-stream HeavyLockers; a couple of items are globally heavy across both.
        let mut a = HeavyLocker::new(512, 4, 0.001, 1.0, 2).unwrap();
        let mut b = HeavyLocker::new(512, 4, 0.001, 1.0, 2).unwrap();
        // Global heavy hitters split across the two streams.
        for _ in 0..30_000 {
            a.insert(b"global-1");
        }
        for _ in 0..25_000 {
            b.insert(b"global-2");
        }
        // Background noise in each.
        for i in 0..40_000u32 {
            a.insert(&i.to_le_bytes());
            b.insert(&(i + 1_000_000).to_le_bytes());
        }
        a.merge(&b).unwrap();
        let hh: HashSet<Vec<u8>> = a.heavy_hitters(0.05).into_iter().map(|(k, _)| k).collect();
        assert!(
            hh.contains(b"global-1".as_slice()),
            "global-1 missing: {hh:?}"
        );
        assert!(hh.contains(b"global-2".as_slice()), "global-2 missing");
        assert!(a.query(b"global-1") >= 29_000);
    }

    #[test]
    fn merge_rejects_mismatched_config() {
        let mut a = HeavyLocker::new(64, 4, 0.001, 1.0, 1).unwrap();
        let b = HeavyLocker::new(128, 4, 0.001, 1.0, 1).unwrap();
        assert!(a.merge(&b).is_err());
    }
}
