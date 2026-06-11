//! SketchPolymer — per-item *tail* quantile estimation with one sketch (Guo et al., KDD 2023).
//!
//! Most quantile sketches estimate *full* distributions; SketchPolymer targets the **tail** quantile
//! of a *per-item* value distribution (e.g. "the 99th-percentile latency of flow `e`") under tight
//! memory. It pairs two ideas:
//!
//! - **Value Splitting and Sharing (VSS):** every value `t` is replaced by its logarithm class
//!   `T = ⌊log_a t⌋` for a base `a` slightly above 1. Items sharing a class share storage, and a
//!   class is decoded back to a value by `a^T`. This keeps multiplicative (log-space) error small
//!   while collapsing the value range.
//! - **Early Filtration:** real streams are skewed and tail quantiles only make sense for *frequent*
//!   items, so a cheap Count-Min "Filter Stage" gates entry — only after an item's frequency crosses
//!   a threshold `𝒯` do its values reach the quantile machinery.
//!
//! The four stages (paper §3.6, Algorithms 9–10):
//! 1. **Filter Stage** — a Count-Min counting item frequency; items below `𝒯` go no further.
//! 2. **Polymer Stage** — a Count-Min whose buckets carry both a frequency and the **maximum**
//!    logarithm class `T` seen for the item.
//! 3. **Splitting Stage** — a Count-Min over `(item, T)` pairs giving each class's frequency
//!    (8-bit truncated counters: a class count saturates at 255).
//! 4. **Verification Filter** — a Bloom filter over `(item, T)` pairs that suppresses Splitting-Stage
//!    over-counts from classes that were never actually inserted.
//!
//! A `w`-quantile query reads the item's total frequency `f` and top class `T` from the Polymer
//! Stage, sets a budget `m = (1−w)·f` (how many values lie above the quantile), then walks classes
//! **downward** from `T`, subtracting each verified class's frequency from `m`, and returns `a^(T+1)`
//! once `m` is exhausted.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

const FILTER_SEED: u64 = 0x5A07_F11E_0000_0001;
const POLYMER_SEED: u64 = 0x5A07_F11E_0000_0002;
const SPLIT_SEED: u64 = 0x5A07_F11E_0000_0003;
const VERIFY_SEED: u64 = 0x5A07_F11E_0000_0004;
const ROW_STRIDE: u64 = 0x9E37_79B9_7F4A_7C15;

/// A SketchPolymer estimating per-item tail quantiles over positive values.
///
/// # Example
/// ```
/// use sketch_oxide::quantiles::SketchPolymer;
///
/// // depth 4, width 8192 per stage, log base 1.05, frequency threshold 1.
/// let mut sp = SketchPolymer::new(4, 8192, 1.05, 1).unwrap();
/// // Flow "x" sees latencies 1..=1000 (each once).
/// for v in 1..=1000u32 {
///     sp.insert(b"x", v as f64);
/// }
/// // The 0.95-quantile of ~[2,1000] is around 950.
/// let q95 = sp.quantile(b"x", 0.95);
/// assert!((q95 - 950.0).abs() / 950.0 < 0.15, "q95 = {q95}");
/// // An unseen / infrequent item yields 0.
/// assert_eq!(sp.quantile(b"absent", 0.95), 0.0);
/// ```
#[derive(Debug, Clone)]
pub struct SketchPolymer {
    depth: usize,
    width: usize,
    a: f64,
    ln_a: f64,
    threshold: u64,
    /// Filter Stage: item-frequency Count-Min.
    filter: Vec<u64>,
    /// Polymer Stage frequency field.
    polymer_f: Vec<u64>,
    /// Polymer Stage value field: maximum logarithm class `T` (init `i64::MIN`).
    polymer_t: Vec<i64>,
    /// Splitting Stage: per-`(item, T)` frequency, 8-bit truncated.
    split: Vec<u8>,
    /// Verification Filter: per-`(item, T)` membership bits.
    verify: Vec<bool>,
}

impl SketchPolymer {
    /// Creates a SketchPolymer with `depth` rows and `width` cells per row in each stage, logarithm
    /// base `a` (`a > 1`, typically close to 1 such as `1.05`), and frequency threshold `threshold`
    /// (`≥ 1`) gating an item into the quantile stages.
    ///
    /// Smaller `a` lowers the multiplicative error but needs more classes; larger `width`/`depth`
    /// lowers Count-Min collision error. The paper tunes the four stages separately; using one
    /// `depth × width` for all is a sound default.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `depth == 0`, `width == 0`, `threshold == 0`, or `a` is
    /// not a finite number greater than 1.
    pub fn new(depth: usize, width: usize, a: f64, threshold: u64) -> Result<Self> {
        if depth == 0 {
            return Err(SketchError::InvalidParameter {
                param: "depth".to_string(),
                value: depth.to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        if width == 0 {
            return Err(SketchError::InvalidParameter {
                param: "width".to_string(),
                value: width.to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        if threshold == 0 {
            return Err(SketchError::InvalidParameter {
                param: "threshold".to_string(),
                value: threshold.to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        if !(a.is_finite() && a > 1.0) {
            return Err(SketchError::InvalidParameter {
                param: "a".to_string(),
                value: a.to_string(),
                constraint: "must be a finite number > 1".to_string(),
            });
        }
        let n = depth * width;
        Ok(Self {
            depth,
            width,
            a,
            ln_a: a.ln(),
            threshold,
            filter: vec![0; n],
            polymer_f: vec![0; n],
            polymer_t: vec![i64::MIN; n],
            split: vec![0; n],
            verify: vec![false; n],
        })
    }

    /// Cell index for a single-key hash in row `i`.
    fn idx_key(&self, key: &[u8], seed: u64, i: usize) -> usize {
        let h = xxhash(key, seed ^ (i as u64).wrapping_mul(ROW_STRIDE));
        i * self.width + (h % self.width as u64) as usize
    }

    /// Logarithm class `T = ⌊log_a value⌋`.
    fn class(&self, value: f64) -> i64 {
        (value.ln() / self.ln_a).floor() as i64
    }

    /// `(item ‖ T)` byte buffer used to hash `(item, T)` pairs.
    fn pair_buf(item: &[u8], t: i64) -> Vec<u8> {
        let mut buf = Vec::with_capacity(item.len() + 8);
        buf.extend_from_slice(item);
        buf.extend_from_slice(&t.to_le_bytes());
        buf
    }

    /// Filter Stage frequency of `item` (min over rows).
    fn filter_query(&self, item: &[u8]) -> u64 {
        let mut f = u64::MAX;
        for i in 0..self.depth {
            f = f.min(self.filter[self.idx_key(item, FILTER_SEED, i)]);
        }
        f
    }

    /// Inserts one occurrence of `item` with positive `value` (paper Algorithm 9).
    ///
    /// Non-positive or non-finite values cannot be log-binned and are ignored.
    pub fn insert(&mut self, item: &[u8], value: f64) {
        if !(value.is_finite() && value > 0.0) {
            return;
        }
        // Early Filtration: until the item is frequent, only the Filter Stage records it.
        if self.filter_query(item) < self.threshold {
            for i in 0..self.depth {
                let idx = self.idx_key(item, FILTER_SEED, i);
                self.filter[idx] += 1;
            }
            return;
        }

        let t = self.class(value);
        // Polymer Stage: frequency + running maximum class.
        for i in 0..self.depth {
            let idx = self.idx_key(item, POLYMER_SEED, i);
            self.polymer_f[idx] += 1;
            if self.polymer_t[idx] < t {
                self.polymer_t[idx] = t;
            }
        }
        let buf = Self::pair_buf(item, t);
        // Splitting Stage: per-class frequency, saturating at the 8-bit ceiling (counter truncation).
        for i in 0..self.depth {
            let idx = self.idx_key(&buf, SPLIT_SEED, i);
            self.split[idx] = self.split[idx].saturating_add(1);
        }
        // Verification Filter: mark the class present.
        for i in 0..self.depth {
            let idx = self.idx_key(&buf, VERIFY_SEED, i);
            self.verify[idx] = true;
        }
    }

    /// Polymer Stage query: `(frequency, maximum class T)` for `item`.
    fn polymer_query(&self, item: &[u8]) -> (u64, i64) {
        let mut f = u64::MAX;
        let mut t = i64::MAX;
        for i in 0..self.depth {
            let idx = self.idx_key(item, POLYMER_SEED, i);
            f = f.min(self.polymer_f[idx]);
            t = t.min(self.polymer_t[idx]);
        }
        (f, t)
    }

    /// Splitting Stage query: frequency of class `(item, t)` (min over rows).
    fn split_query(&self, buf: &[u8]) -> u64 {
        let mut f = u64::MAX;
        for i in 0..self.depth {
            f = f.min(self.split[self.idx_key(buf, SPLIT_SEED, i)] as u64);
        }
        f
    }

    /// Verification Filter query: whether class `(item, t)` was inserted (AND over rows).
    fn verify_query(&self, buf: &[u8]) -> bool {
        for i in 0..self.depth {
            if !self.verify[self.idx_key(buf, VERIFY_SEED, i)] {
                return false;
            }
        }
        true
    }

    /// Estimates the `w`-quantile (`0 ≤ w ≤ 1`) of `item`'s value distribution (paper Algorithm 10).
    ///
    /// Returns `0.0` for an item that never became frequent. `w` is clamped to `[0, 1]`.
    pub fn quantile(&self, item: &[u8], w: f64) -> f64 {
        let w = w.clamp(0.0, 1.0);
        let (f, t_max) = self.polymer_query(item);
        if f == 0 || t_max == i64::MIN {
            return 0.0;
        }
        // Budget: how many values lie strictly above the w-quantile.
        let mut m = (1.0 - w) * f as f64;
        let mut t = t_max;
        // Walk classes downward; the descent is bounded by the item's logarithmic value range.
        let floor = t_max - 1_000_000;
        while m > 0.0 && t > floor {
            let buf = Self::pair_buf(item, t);
            if self.verify_query(&buf) {
                m -= self.split_query(&buf) as f64;
            }
            t -= 1;
        }
        self.a.powf((t + 1) as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(SketchPolymer::new(0, 64, 1.05, 1).is_err());
        assert!(SketchPolymer::new(4, 0, 1.05, 1).is_err());
        assert!(SketchPolymer::new(4, 64, 1.05, 0).is_err());
        assert!(SketchPolymer::new(4, 64, 1.0, 1).is_err());
        assert!(SketchPolymer::new(4, 64, f64::INFINITY, 1).is_err());
        assert!(SketchPolymer::new(4, 64, 1.05, 1).is_ok());
    }

    #[test]
    fn unknown_or_infrequent_item_is_zero() {
        let mut sp = SketchPolymer::new(4, 1024, 1.05, 5).unwrap();
        // Only one occurrence: stays below threshold 5, never enters quantile stages.
        sp.insert(b"rare", 100.0);
        assert_eq!(sp.quantile(b"rare", 0.5), 0.0);
        assert_eq!(sp.quantile(b"never", 0.9), 0.0);
    }

    /// Empirical w-quantile of a sorted slice (nearest-rank).
    fn true_quantile(sorted: &[f64], w: f64) -> f64 {
        if sorted.is_empty() {
            return 0.0;
        }
        let rank = ((w * sorted.len() as f64).floor() as usize).min(sorted.len() - 1);
        sorted[rank]
    }

    #[test]
    fn matches_uniform_distribution() {
        // A single heavy flow with values 1..=2000 (each once). Threshold 1 loses only value 1.
        let mut sp = SketchPolymer::new(4, 16384, 1.03, 1).unwrap();
        let n = 2000u32;
        for v in 1..=n {
            sp.insert(b"flow", v as f64);
        }
        // Captured set is [2, 2000].
        let captured: Vec<f64> = (2..=n).map(|v| v as f64).collect();
        for &w in &[0.5, 0.9, 0.95, 0.99] {
            let truth = true_quantile(&captured, w);
            let est = sp.quantile(b"flow", w);
            let rel = (est - truth).abs() / truth;
            assert!(
                rel < 0.12,
                "w={w}: est {est} vs truth {truth} (rel {rel:.3})"
            );
        }
    }

    #[test]
    fn tail_exceeds_median() {
        let mut sp = SketchPolymer::new(4, 16384, 1.03, 1).unwrap();
        for v in 1..=5000u32 {
            sp.insert(b"f", v as f64);
        }
        let q50 = sp.quantile(b"f", 0.5);
        let q90 = sp.quantile(b"f", 0.9);
        let q99 = sp.quantile(b"f", 0.99);
        assert!(q50 < q90, "median {q50} should be below p90 {q90}");
        assert!(q90 < q99, "p90 {q90} should be below p99 {q99}");
    }

    #[test]
    fn separates_two_flows() {
        // Two flows with separated, wide-ranging value distributions yield well-separated quantiles.
        // (Wide ranges keep each log-class well under the 8-bit Splitting-Stage counter ceiling.)
        let mut sp = SketchPolymer::new(4, 16384, 1.03, 1).unwrap();
        for v in 1..=1000u32 {
            sp.insert(b"low", v as f64); // values in [1, 1000]
            sp.insert(b"high", (v as f64) * 10_000.0); // values in [10_000, 10_000_000]
        }
        let low95 = sp.quantile(b"low", 0.95);
        let high05 = sp.quantile(b"high", 0.05);
        assert!(low95 < 1000.0, "low p95 {low95} should stay under 1000");
        assert!(high05 > 8000.0, "high p05 {high05} should stay above 8000");
    }
}
