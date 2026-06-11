//! Bubble Sketch — high-performance, memory-efficient top-*k* finder (Cao et al., CIKM 2024).
//!
//! Bubble Sketch detects the top-*k* most frequent items in a stream without a min-heap, beating
//! HeavyKeeper on accuracy by up to two orders of magnitude in the paper's experiments. It keeps
//! two arrays `A₁`, `A₂` of `w` buckets; each bucket holds `B` entries kept **sorted ascending by
//! frequency**, so the top entry (`entry_B`, the *hot entry*) always holds the bucket's heaviest
//! item. The hot entry stores the item's **full key**, while the lower `B-1` *cold* entries store
//! only a compact **fingerprint** — a "ladder" layout that spends bits where they matter.
//!
//! Three ideas make it work (paper §3):
//! 1. **Ladder bucket layout** — full key + frequency for the hot item, fingerprint + frequency
//!    for cold items, so most memory goes to the items actually reported.
//! 2. **Real-time bubble sorting** — on each hit the touched entry bubbles up to its sorted place,
//!    keeping hot items at the top and cold items at the bottom.
//! 3. **Threshold relocation** — when a *cold* item's frequency crosses the dynamic threshold
//!    `Δ = f_max·(1/k)^α` *and* beats the top of its alternate bucket, it is promoted into that
//!    bucket's hot entry (its fingerprint is restored to the full key, which the insert holds),
//!    resolving the "two hot items in one bucket" conflict and preserving its identity for reporting.
//!
//! New items take the first empty entry of either candidate bucket; if both are full, the coldest
//! entry of a (hash-chosen) bucket is decayed by one and replaced only if it hits zero — a cheap
//! count-decay eviction in the spirit of HeavyKeeper. A top-*k* query simply scans every hot entry.
//!
//! # Layout note
//!
//! This is a behaviour-faithful reference layout: entries are explicit `(count, fingerprint)` pairs
//! plus a full key for the hot slot, rather than the paper's word-packed variable-width encoding.
//! It reproduces the same reported items, estimates, fingerprint-collision behaviour, and eviction
//! dynamics; the bit-packing is a space optimisation over this same contract.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

const SEED_H1: u64 = 0xB0BB_1E50_0000_0001;
const SEED_H2: u64 = 0xB0BB_1E50_0000_0002;
const SEED_FP: u64 = 0xB0BB_1E50_0000_0003;
const SEED_DECAY: u64 = 0xB0BB_1E50_0000_0004;

/// One bucket: `B` entries sorted ascending by `counts`; index `B-1` is the hot entry.
#[derive(Debug, Clone)]
struct Bucket {
    /// Frequencies, ascending; `counts[B-1]` is the largest (hot).
    counts: Vec<u64>,
    /// Per-entry fingerprints (also kept for the hot entry, for when it is demoted).
    fps: Vec<u32>,
    /// Full key of the hot entry (index `B-1`); `None` when that entry is empty.
    hot_key: Option<Vec<u8>>,
}

impl Bucket {
    fn new(b: usize) -> Self {
        Self {
            counts: vec![0; b],
            fps: vec![0; b],
            hot_key: None,
        }
    }
}

/// A Bubble Sketch over two `w`-bucket arrays of `B` entries each, tuned for top-`k` detection.
///
/// # Example
/// ```
/// use sketch_oxide::frequency::BubbleSketch;
///
/// let mut bs = BubbleSketch::new(256, 8, 3, 1.0).unwrap();
/// for _ in 0..1000 { bs.insert(b"apple"); }
/// for _ in 0..500 { bs.insert(b"pear"); }
/// for _ in 0..100 { bs.insert(b"plum"); }
/// for i in 0..2000u32 { bs.insert(&i.to_le_bytes()); } // light background noise
///
/// let top = bs.top_k();
/// assert_eq!(top[0].0, b"apple");           // heaviest item reported first
/// assert!(top[0].1 >= 950);                  // frequency recovered closely
/// assert!(bs.estimate(b"pear") >= 450);
/// ```
#[derive(Debug, Clone)]
pub struct BubbleSketch {
    w: usize,
    b: usize,
    k: usize,
    alpha: f64,
    a1: Vec<Bucket>,
    a2: Vec<Bucket>,
    /// Global maximum observed frequency, driving the relocation threshold `Δ`.
    f_max: u64,
}

impl BubbleSketch {
    /// Creates a Bubble Sketch with `w` buckets per array, `b` entries per bucket (`b ≥ 2`), a
    /// target of `k` top items, and Zipf skew `alpha` (`α > 0`; the paper's default is `1.0`).
    ///
    /// The relocation threshold is `Δ = f_max·(1/k)^α`; larger `alpha` lowers `Δ` and evicts more
    /// aggressively. Total memory is `≈ 2·w·b` entries.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `w == 0`, `b < 2`, `k == 0`, or `alpha` is not a
    /// finite positive number.
    pub fn new(w: usize, b: usize, k: usize, alpha: f64) -> Result<Self> {
        if w == 0 {
            return Err(SketchError::InvalidParameter {
                param: "w".to_string(),
                value: w.to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        if b < 2 {
            return Err(SketchError::InvalidParameter {
                param: "b".to_string(),
                value: b.to_string(),
                constraint: "must be >= 2".to_string(),
            });
        }
        if k == 0 {
            return Err(SketchError::InvalidParameter {
                param: "k".to_string(),
                value: k.to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        if !(alpha.is_finite() && alpha > 0.0) {
            return Err(SketchError::InvalidParameter {
                param: "alpha".to_string(),
                value: alpha.to_string(),
                constraint: "must be a finite positive number".to_string(),
            });
        }
        Ok(Self {
            w,
            b,
            k,
            alpha,
            a1: vec![Bucket::new(b); w],
            a2: vec![Bucket::new(b); w],
            f_max: 0,
        })
    }

    /// Locates `item` (by full key in the hot entry, by fingerprint in cold entries); returns its
    /// entry index, or `None` if absent.
    fn find(bk: &Bucket, item: &[u8], fp: u32, b: usize) -> Option<usize> {
        if bk.counts[b - 1] > 0 {
            if let Some(key) = &bk.hot_key {
                if key.as_slice() == item {
                    return Some(b - 1);
                }
            }
        }
        (0..b - 1).find(|&i| bk.counts[i] > 0 && bk.fps[i] == fp)
    }

    /// Inserts one occurrence of `item`.
    pub fn insert(&mut self, item: &[u8]) {
        let b = self.b;
        let h1 = (xxhash(item, SEED_H1) % self.w as u64) as usize;
        let h2 = (xxhash(item, SEED_H2) % self.w as u64) as usize;
        let fp = xxhash(item, SEED_FP) as u32;

        // Look for an existing entry in either candidate bucket.
        let matched = Self::find(&self.a1[h1], item, fp, b)
            .map(|pos| (0usize, pos))
            .or_else(|| Self::find(&self.a2[h2], item, fp, b).map(|pos| (1usize, pos)));

        match matched {
            Some((origin, pos)) => self.hit(origin, h1, h2, pos, item, fp),
            None => self.insert_new(h1, h2, item, fp),
        }
    }

    /// Increments a matched entry, bubble-sorts it up, and applies threshold relocation if a cold
    /// item has grown hot enough to claim the alternate bucket's hot slot.
    fn hit(&mut self, origin: usize, h1: usize, h2: usize, pos: usize, item: &[u8], fp: u32) {
        let b = self.b;

        // Increment in place, then bubble the touched entry up to its sorted position.
        let (became_hot, idx, count) = {
            let bk = if origin == 0 {
                &mut self.a1[h1]
            } else {
                &mut self.a2[h2]
            };
            bk.counts[pos] += 1;
            let count = bk.counts[pos];
            let mut i = pos;
            while i + 1 < b && bk.counts[i] > bk.counts[i + 1] {
                bk.counts.swap(i, i + 1);
                bk.fps.swap(i, i + 1);
                i += 1;
            }
            if i == b - 1 {
                // The item reached the hot slot: record its full key (the demoted former-hot item
                // keeps only its fingerprint, already swapped down).
                bk.hot_key = Some(item.to_vec());
                bk.fps[b - 1] = fp;
            }
            (i == b - 1, i, count)
        };

        if count > self.f_max {
            self.f_max = count;
        }
        if became_hot {
            return;
        }

        // Relocation: a cold item that crosses Δ and beats the alternate bucket's hot item is
        // promoted into that bucket's hot entry.
        let delta = self.f_max as f64 * (1.0 / self.k as f64).powf(self.alpha);
        let (alt_arr, alt_idx) = if origin == 0 {
            (1usize, h2)
        } else {
            (0usize, h1)
        };
        let alt_hot = if alt_arr == 0 {
            self.a1[alt_idx].counts[b - 1]
        } else {
            self.a2[alt_idx].counts[b - 1]
        };
        if (count as f64) < delta || count <= alt_hot {
            return;
        }

        // Remove the item from its origin bucket; the freed (zero) entry bubbles to the bottom.
        {
            let bk = if origin == 0 {
                &mut self.a1[h1]
            } else {
                &mut self.a2[h2]
            };
            bk.counts[idx] = 0;
            bk.fps[idx] = 0;
            let mut j = idx;
            while j > 0 && bk.counts[j] < bk.counts[j - 1] {
                bk.counts.swap(j, j - 1);
                bk.fps.swap(j, j - 1);
                j -= 1;
            }
        }
        // Shift the alternate bucket down (discarding its coldest entry) and install the item as
        // the new hot entry.
        {
            let bk = if alt_arr == 0 {
                &mut self.a1[alt_idx]
            } else {
                &mut self.a2[alt_idx]
            };
            for j in 0..b - 1 {
                bk.counts[j] = bk.counts[j + 1];
                bk.fps[j] = bk.fps[j + 1];
            }
            bk.counts[b - 1] = count;
            bk.fps[b - 1] = fp;
            bk.hot_key = Some(item.to_vec());
        }
    }

    /// Handles an item absent from both candidate buckets: fill an empty entry, else count-decay
    /// the coldest entry of a hash-chosen bucket and replace it only if it reaches zero.
    fn insert_new(&mut self, h1: usize, h2: usize, item: &[u8], fp: u32) {
        let b = self.b;
        // Fill the first empty entry (scanning from the hot slot down) in either candidate bucket.
        for &(arr, idx) in &[(0usize, h1), (1usize, h2)] {
            let bk = if arr == 0 {
                &mut self.a1[idx]
            } else {
                &mut self.a2[idx]
            };
            for slot in (0..b).rev() {
                if bk.counts[slot] == 0 {
                    bk.counts[slot] = 1;
                    bk.fps[slot] = fp;
                    if slot == b - 1 {
                        bk.hot_key = Some(item.to_vec());
                    }
                    if self.f_max == 0 {
                        self.f_max = 1;
                    }
                    return;
                }
            }
        }
        // Both buckets full: decay the coldest entry (index 0) of a hash-chosen bucket.
        let r = (xxhash(item, SEED_DECAY) & 1) as usize;
        let idx = if r == 0 { h1 } else { h2 };
        let bk = if r == 0 {
            &mut self.a1[idx]
        } else {
            &mut self.a2[idx]
        };
        bk.counts[0] -= 1;
        if bk.counts[0] == 0 {
            bk.counts[0] = 1;
            bk.fps[0] = fp;
        }
    }

    /// Estimated frequency of `item` (0 if it is not tracked). May under-count after eviction or
    /// over-count on a fingerprint collision.
    pub fn estimate(&self, item: &[u8]) -> u64 {
        let b = self.b;
        let h1 = (xxhash(item, SEED_H1) % self.w as u64) as usize;
        let h2 = (xxhash(item, SEED_H2) % self.w as u64) as usize;
        let fp = xxhash(item, SEED_FP) as u32;
        let mut best = 0;
        for bk in [&self.a1[h1], &self.a2[h2]] {
            if bk.counts[b - 1] > 0 {
                if let Some(key) = &bk.hot_key {
                    if key.as_slice() == item {
                        best = best.max(bk.counts[b - 1]);
                    }
                }
            }
            for i in 0..b - 1 {
                if bk.counts[i] > 0 && bk.fps[i] == fp {
                    best = best.max(bk.counts[i]);
                }
            }
        }
        best
    }

    /// Returns up to `k` heaviest items as `(key, frequency)`, sorted by descending frequency
    /// (ties broken by key). Reads every bucket's hot entry, the only entries with full keys.
    pub fn top_k(&self) -> Vec<(Vec<u8>, u64)> {
        use std::collections::HashMap;
        let b = self.b;
        let mut best: HashMap<Vec<u8>, u64> = HashMap::new();
        for bk in self.a1.iter().chain(self.a2.iter()) {
            if bk.counts[b - 1] > 0 {
                if let Some(key) = &bk.hot_key {
                    let e = best.entry(key.clone()).or_insert(0);
                    *e = (*e).max(bk.counts[b - 1]);
                }
            }
        }
        let mut v: Vec<(Vec<u8>, u64)> = best.into_iter().collect();
        v.sort_by(|x, y| y.1.cmp(&x.1).then_with(|| x.0.cmp(&y.0)));
        v.truncate(self.k);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(BubbleSketch::new(0, 4, 8, 1.0).is_err());
        assert!(BubbleSketch::new(64, 1, 8, 1.0).is_err());
        assert!(BubbleSketch::new(64, 4, 0, 1.0).is_err());
        assert!(BubbleSketch::new(64, 4, 8, 0.0).is_err());
        assert!(BubbleSketch::new(64, 4, 8, f64::NAN).is_err());
        assert!(BubbleSketch::new(64, 4, 8, 1.0).is_ok());
    }

    #[test]
    fn empty_top_k_is_empty() {
        let bs = BubbleSketch::new(64, 4, 8, 1.0).unwrap();
        assert!(bs.top_k().is_empty());
        assert_eq!(bs.estimate(b"absent"), 0);
    }

    #[test]
    fn ranks_distinct_heavy_hitters() {
        // Plenty of room: a handful of well-separated items should be reported in exact order.
        let mut bs = BubbleSketch::new(512, 8, 5, 1.0).unwrap();
        let freqs = [10_000u64, 8_000, 6_000, 4_000, 2_000];
        for (i, &f) in freqs.iter().enumerate() {
            let key = format!("heavy-{i}");
            for _ in 0..f {
                bs.insert(key.as_bytes());
            }
        }
        let top = bs.top_k();
        assert_eq!(top.len(), 5);
        for (i, (key, count)) in top.iter().enumerate() {
            assert_eq!(key.as_slice(), format!("heavy-{i}").as_bytes(), "rank {i}");
            // Exact: no decay (room to spare), relocation preserves the count.
            assert_eq!(*count, freqs[i], "rank {i} frequency");
        }
    }

    #[test]
    fn estimate_close_for_heavy_item() {
        let mut bs = BubbleSketch::new(256, 8, 10, 1.0).unwrap();
        for _ in 0..50_000u64 {
            bs.insert(b"whale");
        }
        // Background noise that should not displace the dominant item.
        for i in 0..5_000u32 {
            bs.insert(&i.to_le_bytes());
        }
        let est = bs.estimate(b"whale");
        assert!(
            (est as i64 - 50_000).abs() < 500,
            "estimate {est} should be ~50000"
        );
    }

    #[test]
    fn zipf_top_k_precision() {
        // A Zipf-like stream: item r appears ~ TOTAL/r times. The true top-k are ranks 1..=k.
        let universe = 2_000usize;
        let mut bs = BubbleSketch::new(512, 8, 20, 1.0).unwrap();
        let mut true_freq: Vec<(usize, u64)> = Vec::new();
        for r in 1..=universe {
            let f = (2_000_000u64 / r as u64).max(1);
            true_freq.push((r, f));
            let key = (r as u32).to_le_bytes();
            for _ in 0..f {
                bs.insert(&key);
            }
        }
        true_freq.sort_by(|a, b| b.1.cmp(&a.1));
        let k = 20;
        let truth: std::collections::HashSet<u32> =
            true_freq.iter().take(k).map(|&(r, _)| r as u32).collect();

        let reported: std::collections::HashSet<u32> = bs
            .top_k()
            .iter()
            .map(|(key, _)| {
                let mut a = [0u8; 4];
                a.copy_from_slice(key);
                u32::from_le_bytes(a)
            })
            .collect();

        let hits = truth.intersection(&reported).count();
        // Bubble Sketch reports ~100% precision on skewed data; require a strong majority.
        assert!(
            hits as f64 / k as f64 >= 0.85,
            "precision {hits}/{k} too low"
        );
    }

    #[test]
    fn duplicates_and_monotonicity() {
        let mut bs = BubbleSketch::new(128, 4, 4, 1.0).unwrap();
        let mut prev = 0;
        for n in 1..=2_000u64 {
            bs.insert(b"x");
            if n % 100 == 0 {
                let e = bs.estimate(b"x");
                assert!(e >= prev, "estimate must not decrease: {e} < {prev}");
                prev = e;
            }
        }
        assert_eq!(bs.estimate(b"x"), 2_000);
    }
}
