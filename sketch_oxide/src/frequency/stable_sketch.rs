//! Stable-Sketch — a versatile flat sketch for heavy hitters, changers, and persistent items
//! (Li & Patras, WWW 2024, Best Student Paper).
//!
//! Stable-Sketch is a single-layer `m × u` table (rows × buckets) where each bucket stores three
//! fields: the candidate item's **key** `K`, a **value** counter `V` (its statistic, e.g. frequency),
//! and a **bucket stability** `S`. Stability is the sketch's key idea: a bucket whose recorded item
//! keeps re-appearing accrues stability, while a bucket churning through many different items stays
//! unstable. Because real streams are highly skewed, buckets holding *heavy* items tend to be far
//! more stable than those holding light ones, so stability is a strong signal for "this bucket holds
//! something worth protecting."
//!
//! On each arrival of item `f` (paper Algorithm 1), the `m` rows are probed in order:
//! - **Empty bucket** → claim it: `K←f, V←1, S←1`.
//! - **Key match** → reinforce it: `V←V+1, S←S+1`.
//! - Otherwise track the bucket with the **smallest value counter** across the rows.
//!
//! If every probed bucket is occupied by another key (a collision in all rows), Stable-Sketch
//! applies a **stochastic decay-based replacement** to the minimum-value bucket `B(R,M)`: with
//! probability `L(f) = 1 / (V·S + 1)` it decrements `V` by one; if `V` then hits zero the incoming
//! item takes the bucket (`K←f, V←1, S←max(S−1,0)`); otherwise the incoming item is discarded.
//! Because both `V` and `S` grow for items that genuinely persist, `L(f)` shrinks for them, so heavy
//! items become progressively harder to evict — the protection MV-Sketch and friends lack.
//!
//! This yields a **one-sided** estimator: the reported value never exceeds the true frequency
//! (paper Theorem 4.1), since full keys are matched exactly and `V` only ever counts genuine hits.
//!
//! # Randomness
//!
//! The replacement test `rand(0,1) < 1/(V·S+1)` is realised exactly with integer arithmetic as
//! `xxhash(f, seed ⊕ step) mod (V·S+1) == 0`, where `step` is a per-insert counter. This is
//! deterministic and reproducible (no RNG state to thread) while drawing an independent, uniform
//! decision per arrival.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use std::collections::HashMap;

/// Base seed for the per-row hash functions.
const SEED_BASE: u64 = 0x57AB_1E50_0000_0001;
/// Seed for the per-arrival replacement decision.
const SEED_RAND: u64 = 0x57AB_1E50_0000_0002;
/// Golden-ratio odd increment that decorrelates successive row seeds.
const ROW_STRIDE: u64 = 0x9E37_79B9_7F4A_7C15;

/// A Stable-Sketch over `m` rows of `u` buckets, each bucket a `(key, value, stability)` triple.
///
/// # Example
/// ```
/// use sketch_oxide::frequency::StableSketch;
///
/// let mut ss = StableSketch::new(4, 1024).unwrap();
/// for _ in 0..10_000 { ss.insert(b"elephant"); }
/// for i in 0..3_000u32 { ss.insert(&i.to_le_bytes()); } // light background flows
///
/// // One-sided: the estimate never exceeds the truth, and stays close for a heavy item.
/// let est = ss.estimate(b"elephant");
/// assert!(est <= 10_000 && est >= 9_500, "estimate {est}");
///
/// // Heavy-hitter report: items whose estimate exceeds a threshold.
/// let hh = ss.heavy_hitters(5_000);
/// assert_eq!(hh[0].0, b"elephant");
/// ```
#[derive(Debug, Clone)]
pub struct StableSketch {
    rows: usize,
    cols: usize,
    keys: Vec<Option<Vec<u8>>>,
    values: Vec<u64>,
    stability: Vec<u64>,
    /// Per-insert counter that varies the replacement decision draw.
    step: u64,
}

impl StableSketch {
    /// Creates a Stable-Sketch with `rows` rows (`m ≥ 1`, pairwise-independent hashes) and `cols`
    /// buckets per row (`u ≥ 1`). Total memory is `rows · cols` buckets. The paper fixes `m = 4`
    /// and sizes `u` to the memory budget.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `rows == 0` or `cols == 0`.
    pub fn new(rows: usize, cols: usize) -> Result<Self> {
        if rows == 0 {
            return Err(SketchError::InvalidParameter {
                param: "rows".to_string(),
                value: rows.to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        if cols == 0 {
            return Err(SketchError::InvalidParameter {
                param: "cols".to_string(),
                value: cols.to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        let n = rows * cols;
        Ok(Self {
            rows,
            cols,
            keys: vec![None; n],
            values: vec![0; n],
            stability: vec![0; n],
            step: 0,
        })
    }

    /// Hash of `item` for row `i`, mapped to a bucket index in `0..cols`.
    fn bucket(&self, item: &[u8], i: usize) -> usize {
        let seed = SEED_BASE.wrapping_add((i as u64).wrapping_mul(ROW_STRIDE));
        (xxhash(item, seed) % self.cols as u64) as usize
    }

    /// Inserts one occurrence of `item` (paper Algorithm 1).
    pub fn insert(&mut self, item: &[u8]) {
        self.step = self.step.wrapping_add(1);
        let mut min_v = u64::MAX;
        let mut min_idx = 0usize;

        for i in 0..self.rows {
            let idx = i * self.cols + self.bucket(item, i);
            match &self.keys[idx] {
                None => {
                    // Case 1: empty bucket — claim it.
                    self.keys[idx] = Some(item.to_vec());
                    self.values[idx] = 1;
                    self.stability[idx] = 1;
                    return;
                }
                Some(k) if k.as_slice() == item => {
                    // Case 2: key match — reinforce.
                    self.values[idx] += 1;
                    self.stability[idx] += 1;
                    return;
                }
                Some(_) => {
                    // Occupied by another key: remember the minimum-value bucket.
                    if self.values[idx] < min_v {
                        min_v = self.values[idx];
                        min_idx = idx;
                    }
                }
            }
        }

        // Case 3: collision in every row — stochastic decay-based replacement of B(R,M).
        let v = self.values[min_idx];
        let s = self.stability[min_idx];
        let denom = v.saturating_mul(s).saturating_add(1);
        let draw = xxhash(item, SEED_RAND ^ self.step) % denom;
        if draw != 0 {
            return; // failed the replacement test — discard the new item
        }
        self.values[min_idx] -= 1;
        if self.values[min_idx] == 0 {
            self.keys[min_idx] = Some(item.to_vec());
            self.values[min_idx] = 1;
            self.stability[min_idx] = s.saturating_sub(1);
        }
    }

    /// Estimated frequency of `item` — the largest value counter among the buckets holding its key,
    /// or `0` if it is not tracked. One-sided: never exceeds the true frequency (Theorem 4.1).
    pub fn estimate(&self, item: &[u8]) -> u64 {
        let mut best = 0;
        for i in 0..self.rows {
            let idx = i * self.cols + self.bucket(item, i);
            if let Some(k) = &self.keys[idx] {
                if k.as_slice() == item {
                    best = best.max(self.values[idx]);
                }
            }
        }
        best
    }

    /// Returns every tracked item whose estimated frequency is strictly greater than `threshold`,
    /// as `(key, estimate)` sorted by descending estimate (ties broken by key). Set `threshold` to
    /// `⌊θ·N⌋` for the standard "frequency exceeds a fraction θ of the stream" heavy-hitter query.
    pub fn heavy_hitters(&self, threshold: u64) -> Vec<(Vec<u8>, u64)> {
        let mut best: HashMap<&[u8], u64> = HashMap::new();
        for idx in 0..self.keys.len() {
            if let Some(k) = &self.keys[idx] {
                let e = best.entry(k.as_slice()).or_insert(0);
                *e = (*e).max(self.values[idx]);
            }
        }
        let mut out: Vec<(Vec<u8>, u64)> = best
            .into_iter()
            .filter(|&(_, v)| v > threshold)
            .map(|(k, v)| (k.to_vec(), v))
            .collect();
        out.sort_by(|x, y| y.1.cmp(&x.1).then_with(|| x.0.cmp(&y.0)));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn rejects_bad_params() {
        assert!(StableSketch::new(0, 16).is_err());
        assert!(StableSketch::new(4, 0).is_err());
        assert!(StableSketch::new(4, 16).is_ok());
    }

    #[test]
    fn empty_reports_nothing() {
        let ss = StableSketch::new(4, 64).unwrap();
        assert_eq!(ss.estimate(b"x"), 0);
        assert!(ss.heavy_hitters(0).is_empty());
    }

    #[test]
    fn isolated_item_is_exact() {
        // With no competing items, a flow always re-matches its own bucket: V counts every arrival.
        let mut ss = StableSketch::new(4, 256).unwrap();
        for _ in 0..1000 {
            ss.insert(b"solo");
        }
        assert_eq!(ss.estimate(b"solo"), 1000);
    }

    #[test]
    fn estimate_is_one_sided() {
        // Theorem 4.1: the estimate never over-counts, even under heavy collision pressure.
        let mut ss = StableSketch::new(4, 128).unwrap();
        // True counts we control.
        let heavy = b"heavy";
        let mut true_heavy = 0u64;
        for round in 0..20_000u32 {
            ss.insert(heavy);
            true_heavy += 1;
            // Lots of distinct noise to stress the buckets.
            ss.insert(&round.to_le_bytes());
        }
        let est = ss.estimate(heavy);
        assert!(
            est <= true_heavy,
            "estimate {est} must not exceed {true_heavy}"
        );
        assert!(est > 0, "a persistent heavy item should survive");
    }

    #[test]
    fn dominant_item_estimate_is_monotonic() {
        // An item that keeps matching its bucket only ever increments — never decremented.
        let mut ss = StableSketch::new(4, 256).unwrap();
        let mut prev = 0;
        for n in 1..=5_000u64 {
            ss.insert(b"king");
            if n % 250 == 0 {
                let e = ss.estimate(b"king");
                assert!(e >= prev, "estimate dropped: {e} < {prev}");
                prev = e;
            }
        }
    }

    #[test]
    fn zipf_heavy_hitter_f1() {
        // Zipf-like stream: item r appears ~ C/r times. Detect items above a frequency threshold
        // and compare the reported set against the truth via F1.
        let universe = 2_000usize;
        let mut ss = StableSketch::new(4, 1024).unwrap();
        let mut freqs: Vec<(u32, u64)> = Vec::new();
        let mut total = 0u64;
        for r in 1..=universe {
            let f = (200_000u64 / r as u64).max(1);
            total += f;
            let key = (r as u32).to_le_bytes();
            freqs.push((r as u32, f));
            for _ in 0..f {
                ss.insert(&key);
            }
        }
        // Heavy hitter = frequency above θ·N with θ = 0.1%.
        let threshold = total / 1000;
        let truth: HashSet<u32> = freqs
            .iter()
            .filter(|&&(_, f)| f > threshold)
            .map(|&(r, _)| r)
            .collect();
        let reported: HashSet<u32> = ss
            .heavy_hitters(threshold)
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
        assert!(
            f1 >= 0.85,
            "F1 {f1:.3} (precision {precision:.3}, recall {recall:.3}) too low"
        );
    }
}
