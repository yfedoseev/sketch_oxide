//! UDDSketch — uniform DDSketch with a bounded number of buckets.
//!
//! Plain [`DDSketch`](super::DDSketch) keeps the `α`-relative-error guarantee only as long
//! as it is allowed unboundedly many buckets. Real deployments cap memory, and the usual
//! DDSketch remedy — *collapsing* the extreme buckets into one — silently destroys the
//! guarantee for the smallest/largest values. UDDSketch (Italiano et al., 2020) fixes this:
//! when it runs out of buckets it performs a **uniform collapse** that merges every adjacent
//! pair of buckets at once, doubling the bin width (`γ → γ²`) and so *uniformly* relaxing
//! the relative accuracy from `α` to `α' = 2α / (1 + α²)`. The relative-error guarantee
//! still holds everywhere — only the (known, queryable) value of `α` has grown.
//!
//! # Guarantee
//!
//! After any number of collapses, for a true quantile value `v` the estimate `v'` satisfies
//! `|v' - v| ≤ α_now · v`, where `α_now` ([`relative_accuracy`](UddSketch::relative_accuracy))
//! is the current, possibly-relaxed accuracy. This is the property plain bucket-collapsing
//! loses.
//!
//! # References
//! - Italiano, Lattanzi, Mirrokni, et al. "UDDSketch: Accurate Tracking of Quantiles in Data
//!   Streams" (IEEE Access, 2020).

use crate::common::{Mergeable, Result, Sketch, SketchError};
use std::collections::HashMap;

/// A bounded-bucket DDSketch variant that preserves its relative-error guarantee under
/// memory pressure via uniform collapse.
///
/// # Example
/// ```
/// use sketch_oxide::quantiles::UddSketch;
///
/// // 1% initial accuracy, at most 128 buckets.
/// let mut udd = UddSketch::new(0.01, 128).unwrap();
/// for i in 1..=10_000u64 {
///     udd.add(i as f64);
/// }
/// let p50 = udd.quantile(0.5).unwrap();
/// // Median of 1..=10000 is ~5000, within the (possibly relaxed) accuracy.
/// assert!((p50 - 5000.0).abs() <= udd.relative_accuracy() * 5000.0 + 1.0);
/// ```
#[derive(Debug, Clone)]
pub struct UddSketch {
    /// Accuracy at construction (before any collapse); fixes the collapse schedule and is
    /// the compatibility key for [`merge`](Mergeable::merge).
    initial_alpha: f64,
    /// Current relative accuracy (grows with each collapse).
    alpha: f64,
    /// Current bin width `γ = (1 + α) / (1 - α)`.
    gamma: f64,
    /// `ln(γ)`, cached for the log mapping.
    gamma_ln: f64,
    /// Maximum number of distinct buckets (positive + negative) before a collapse.
    max_buckets: usize,
    /// Number of uniform collapses performed so far.
    collapses: u32,
    /// Buckets for positive values: `key -> count`.
    positive: HashMap<i32, u64>,
    /// Buckets for negative values (keyed on the absolute value).
    negative: HashMap<i32, u64>,
    /// Count of exact zeros.
    zero_count: u64,
}

impl UddSketch {
    /// Creates a sketch with initial relative accuracy `alpha` and at most `max_buckets`
    /// buckets.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `alpha` is not in `(0, 1)` or `max_buckets < 2`.
    pub fn new(alpha: f64, max_buckets: usize) -> Result<Self> {
        if !(alpha > 0.0 && alpha < 1.0) {
            return Err(SketchError::InvalidParameter {
                param: "alpha".to_string(),
                value: alpha.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }
        if max_buckets < 2 {
            return Err(SketchError::InvalidParameter {
                param: "max_buckets".to_string(),
                value: max_buckets.to_string(),
                constraint: "must be >= 2".to_string(),
            });
        }
        let gamma = (1.0 + alpha) / (1.0 - alpha);
        Ok(Self {
            initial_alpha: alpha,
            alpha,
            gamma,
            gamma_ln: gamma.ln(),
            max_buckets,
            collapses: 0,
            positive: HashMap::new(),
            negative: HashMap::new(),
            zero_count: 0,
        })
    }

    /// The current relative accuracy `α`, which may have grown from the initial value due to
    /// collapses. The error guarantee `|v' - v| ≤ α · v` holds at this `α`.
    #[inline]
    pub fn relative_accuracy(&self) -> f64 {
        self.alpha
    }

    /// Number of uniform collapses performed so far.
    #[inline]
    pub fn collapse_count(&self) -> u32 {
        self.collapses
    }

    /// Current number of distinct buckets (positive + negative).
    #[inline]
    pub fn num_buckets(&self) -> usize {
        self.positive.len() + self.negative.len()
    }

    /// Total number of values recorded.
    pub fn count(&self) -> u64 {
        let pos: u64 = self.positive.values().sum();
        let neg: u64 = self.negative.values().sum();
        pos + neg + self.zero_count
    }

    /// Whether no values have been recorded.
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// Bucket key for a positive magnitude: `⌈log_γ(v)⌉`.
    #[inline]
    fn key(&self, magnitude: f64) -> i32 {
        (magnitude.ln() / self.gamma_ln).ceil() as i32
    }

    /// Representative value for a bucket key: the geometric midpoint `2·γ^k / (γ + 1)`.
    #[inline]
    fn value_of(&self, key: i32) -> f64 {
        2.0 * self.gamma.powi(key) / (self.gamma + 1.0)
    }

    /// Records a value, collapsing if the bucket budget is exceeded.
    pub fn add(&mut self, value: f64) {
        if value > 0.0 {
            let k = self.key(value);
            *self.positive.entry(k).or_insert(0) += 1;
        } else if value < 0.0 {
            let k = self.key(-value);
            *self.negative.entry(k).or_insert(0) += 1;
        } else {
            self.zero_count += 1;
        }
        while self.num_buckets() > self.max_buckets {
            self.collapse();
        }
    }

    /// Performs one uniform collapse: `γ → γ²`, `α → 2α/(1+α²)`, every bucket `k` remapped to
    /// `⌈k/2⌉` (merging adjacent pairs).
    fn collapse(&mut self) {
        self.positive = remap_halve(&self.positive);
        self.negative = remap_halve(&self.negative);
        self.gamma *= self.gamma;
        self.gamma_ln *= 2.0;
        self.alpha = 2.0 * self.alpha / (1.0 + self.alpha * self.alpha);
        self.collapses += 1;
    }

    /// Collapses repeatedly until `collapses == target` (used to align two sketches before a
    /// merge). A no-op if already at or beyond `target`.
    fn collapse_to(&mut self, target: u32) {
        while self.collapses < target {
            self.collapse();
        }
    }

    /// Estimates the value at quantile `q` in `[0, 1]`. Returns `None` if `q` is out of range
    /// or the sketch is empty.
    pub fn quantile(&self, q: f64) -> Option<f64> {
        if !(0.0..=1.0).contains(&q) {
            return None;
        }
        let count = self.count();
        if count == 0 {
            return None;
        }

        let rank = if q == 0.0 {
            1
        } else {
            (q * count as f64).ceil() as u64
        };

        let mut accumulated = 0u64;

        // Negative values: most-negative first => largest key (largest magnitude) first.
        let neg_total: u64 = self.negative.values().sum();
        if rank <= neg_total {
            let mut keys: Vec<i32> = self.negative.keys().copied().collect();
            keys.sort_unstable_by(|a, b| b.cmp(a));
            for k in keys {
                accumulated += self.negative[&k];
                if accumulated >= rank {
                    return Some(-self.value_of(k));
                }
            }
        }
        accumulated = neg_total;

        // Zeros.
        if rank <= accumulated + self.zero_count {
            return Some(0.0);
        }
        accumulated += self.zero_count;

        // Positive values: smallest key first.
        let mut keys: Vec<i32> = self.positive.keys().copied().collect();
        keys.sort_unstable();
        for k in keys {
            accumulated += self.positive[&k];
            if accumulated >= rank {
                return Some(self.value_of(k));
            }
        }
        None
    }
}

/// Remaps a bucket map by merging adjacent pairs: key `k → ⌈k/2⌉`, summing counts.
fn remap_halve(bins: &HashMap<i32, u64>) -> HashMap<i32, u64> {
    let mut out = HashMap::with_capacity(bins.len());
    for (&k, &c) in bins {
        // ⌈k/2⌉ via Euclidean division (correct for negative keys too).
        let nk = (k + 1).div_euclid(2);
        *out.entry(nk).or_insert(0) += c;
    }
    out
}

impl Mergeable for UddSketch {
    /// Merges `other` into `self`. Both must share the same initial accuracy; the
    /// finer-resolution sketch is collapsed to match the coarser before bucket counts are
    /// combined, so the merged guarantee is the coarser of the two.
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the initial accuracies differ.
    fn merge(&mut self, other: &Self) -> Result<()> {
        if (self.initial_alpha - other.initial_alpha).abs() > 1e-12 {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "initial alpha mismatch: {} vs {}",
                    self.initial_alpha, other.initial_alpha
                ),
            });
        }

        // Align collapse levels: bring both to the coarser (larger collapse count).
        let target = self.collapses.max(other.collapses);
        self.collapse_to(target);
        let mut other = other.clone();
        other.collapse_to(target);

        for (&k, &c) in &other.positive {
            *self.positive.entry(k).or_insert(0) += c;
        }
        for (&k, &c) in &other.negative {
            *self.negative.entry(k).or_insert(0) += c;
        }
        self.zero_count += other.zero_count;

        while self.num_buckets() > self.max_buckets {
            self.collapse();
        }
        Ok(())
    }
}

impl Sketch for UddSketch {
    type Item = f64;

    fn update(&mut self, item: &Self::Item) {
        self.add(*item);
    }

    /// Returns the median (p50) as the single representative estimate, or 0 if empty.
    fn estimate(&self) -> f64 {
        self.quantile(0.5).unwrap_or(0.0)
    }

    fn is_empty(&self) -> bool {
        self.count() == 0
    }

    fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.initial_alpha.to_le_bytes());
        bytes.extend_from_slice(&(self.max_buckets as u64).to_le_bytes());
        bytes.extend_from_slice(&(self.collapses as u64).to_le_bytes());
        bytes.extend_from_slice(&self.zero_count.to_le_bytes());
        for (label, bins) in [(0u8, &self.positive), (1u8, &self.negative)] {
            bytes.extend_from_slice(&(bins.len() as u64).to_le_bytes());
            for (&k, &c) in bins {
                bytes.push(label);
                bytes.extend_from_slice(&k.to_le_bytes());
                bytes.extend_from_slice(&c.to_le_bytes());
            }
        }
        bytes
    }

    fn deserialize(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 32 {
            return Err(SketchError::DeserializationError(
                "UddSketch header too short".to_string(),
            ));
        }
        let initial_alpha = f64::from_le_bytes(bytes[0..8].try_into().unwrap());
        let max_buckets = u64::from_le_bytes(bytes[8..16].try_into().unwrap()) as usize;
        let collapses = u64::from_le_bytes(bytes[16..24].try_into().unwrap());
        let zero_count = u64::from_le_bytes(bytes[24..32].try_into().unwrap());

        // `collapses` is attacker-controlled and drives `collapse_to`, which
        // loops `collapses` times (each pass remaps both bucket maps). A crafted
        // value near u64::MAX would spin for billions of iterations — a pure-CPU
        // DoS. Cap it: `gamma` (which squares each collapse) saturates to
        // infinity after ~20 collapses, so no legitimately-serialized sketch has
        // more than a handful; 4096 is far beyond any real value.
        const MAX_COLLAPSES: u64 = 4096;
        if collapses > MAX_COLLAPSES {
            return Err(SketchError::DeserializationError(
                "collapse count out of range".to_string(),
            ));
        }
        let collapses = collapses as u32;

        let mut sketch = UddSketch::new(initial_alpha, max_buckets.max(2))?;
        // Replay collapses to reach the stored accuracy/gamma.
        sketch.collapse_to(collapses);
        sketch.zero_count = zero_count;

        // Each serialized bucket entry is 13 bytes: [label:1][key:4][count:8].
        const BUCKET_ENTRY_LEN: usize = 13;
        let mut off = 32;
        for store in [0u8, 1u8] {
            let _ = store;
            let n = u64::from_le_bytes(
                bytes
                    .get(off..off + 8)
                    .ok_or_else(|| SketchError::DeserializationError("truncated".into()))?
                    .try_into()
                    .unwrap(),
            ) as usize;
            off += 8;

            // `n` is attacker-controlled. Validate that the declared buckets
            // actually fit in the remaining bytes BEFORE the loop, so the
            // per-entry slicing below cannot panic (out-of-bounds index) or spin
            // over a huge count. `checked_mul` guards the size computation itself.
            let needed = n.checked_mul(BUCKET_ENTRY_LEN).ok_or_else(|| {
                SketchError::DeserializationError("bucket count overflow".to_string())
            })?;
            if needed > bytes.len() - off {
                return Err(SketchError::DeserializationError(
                    "declared more buckets than remaining bytes".to_string(),
                ));
            }

            for _ in 0..n {
                let label = bytes[off];
                let k = i32::from_le_bytes(bytes[off + 1..off + 5].try_into().unwrap());
                let c = u64::from_le_bytes(bytes[off + 5..off + 13].try_into().unwrap());
                off += BUCKET_ENTRY_LEN;
                if label == 0 {
                    sketch.positive.insert(k, c);
                } else {
                    sketch.negative.insert(k, c);
                }
            }
        }
        Ok(sketch)
    }
}

// Capability-trait adoptions (fable5 doc 01 F3): delegate to inherent methods.
mod capability_impls {
    use super::*;
    use crate::common::capabilities::{QuantileQuery, Serializable, Update};

    impl Update<f64> for UddSketch {
        fn update(&mut self, item: &f64) {
            self.add(*item);
        }
    }

    // `quantile(&self, ..) -> Option<f64>` is immutable, so `QuantileQuery` fits.
    impl QuantileQuery for UddSketch {
        fn quantile(&self, rank: f64) -> Option<f64> {
            UddSketch::quantile(self, rank)
        }
    }

    impl Serializable for UddSketch {
        fn to_bytes(&self) -> crate::common::Result<Vec<u8>> {
            Ok(<Self as crate::common::Sketch>::serialize(self))
        }
        fn from_bytes(bytes: &[u8]) -> crate::common::Result<Self> {
            <Self as crate::common::Sketch>::deserialize(bytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(UddSketch::new(0.0, 100).is_err());
        assert!(UddSketch::new(1.0, 100).is_err());
        assert!(UddSketch::new(0.01, 1).is_err());
        assert!(UddSketch::new(0.01, 100).is_ok());
    }

    #[test]
    fn quantiles_within_accuracy_no_collapse() {
        let mut udd = UddSketch::new(0.01, 4096).unwrap();
        for i in 1..=10_000u64 {
            udd.add(i as f64);
        }
        assert_eq!(
            udd.collapse_count(),
            0,
            "should not collapse with ample budget"
        );
        for &(q, truth) in &[(0.5, 5000.0), (0.9, 9000.0), (0.99, 9900.0)] {
            let est = udd.quantile(q).unwrap();
            assert!(
                (est - truth).abs() <= udd.relative_accuracy() * truth,
                "q={q}: est {est} vs truth {truth}"
            );
        }
    }

    #[test]
    fn collapse_keeps_guarantee_and_bounds_buckets() {
        // Tiny budget over a wide value range forces several collapses.
        let mut udd = UddSketch::new(0.01, 16).unwrap();
        for i in 1..=100_000u64 {
            udd.add(i as f64);
        }
        assert!(udd.num_buckets() <= 16, "buckets {}", udd.num_buckets());
        assert!(udd.collapse_count() > 0, "expected collapses");
        assert!(udd.relative_accuracy() > 0.01, "alpha should have grown");

        // The relaxed guarantee must still hold at the current alpha.
        let truth = 50_000.0;
        let est = udd.quantile(0.5).unwrap();
        assert!(
            (est - truth).abs() <= udd.relative_accuracy() * truth,
            "p50 {est} vs {truth} at alpha {}",
            udd.relative_accuracy()
        );
    }

    #[test]
    fn handles_zero_and_negative() {
        let mut udd = UddSketch::new(0.05, 256).unwrap();
        for i in -100..=100 {
            udd.add(i as f64);
        }
        assert_eq!(udd.count(), 201);
        let med = udd.quantile(0.5).unwrap();
        assert!(med.abs() < 5.0, "median near 0, got {med}");
        assert!(udd.quantile(0.0).unwrap() < 0.0);
        assert!(udd.quantile(1.0).unwrap() > 0.0);
    }

    #[test]
    fn merge_combines_distributions() {
        let mut a = UddSketch::new(0.01, 2048).unwrap();
        let mut b = UddSketch::new(0.01, 2048).unwrap();
        for i in 1..=5_000u64 {
            a.add(i as f64);
        }
        for i in 5_001..=10_000u64 {
            b.add(i as f64);
        }
        a.merge(&b).unwrap();
        assert_eq!(a.count(), 10_000);
        let p50 = a.quantile(0.5).unwrap();
        assert!(
            (p50 - 5000.0).abs() <= a.relative_accuracy() * 5000.0,
            "merged p50 {p50}"
        );
    }

    #[test]
    fn merge_requires_same_initial_alpha() {
        let mut a = UddSketch::new(0.01, 256).unwrap();
        let b = UddSketch::new(0.02, 256).unwrap();
        assert!(a.merge(&b).is_err());
    }

    #[test]
    fn deserialize_rejects_malicious_headers_without_panic() {
        // Helper to build a header: [alpha:8][max_buckets:8][collapses:8][zero:8].
        fn header(collapses: u64, tail: &[u8]) -> Vec<u8> {
            let mut b = Vec::new();
            b.extend_from_slice(&0.01f64.to_le_bytes());
            b.extend_from_slice(&256u64.to_le_bytes());
            b.extend_from_slice(&collapses.to_le_bytes());
            b.extend_from_slice(&0u64.to_le_bytes());
            b.extend_from_slice(tail);
            b
        }

        // Oversized collapse count would spin `collapse_to` for billions of
        // iterations (CPU DoS) — must be rejected.
        assert!(UddSketch::deserialize(&header(u64::MAX, &[])).is_err());

        // Oversized positive-bucket count with a short tail previously indexed
        // out of bounds (panic). First store-count = u64::MAX, no bucket bytes.
        let mut bytes = header(0, &u64::MAX.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 4]); // fewer than one 13-byte entry
        assert!(
            UddSketch::deserialize(&bytes).is_err(),
            "must error, not panic/OOM"
        );

        // Large-but-non-overflowing bucket count exceeding the tail.
        let mut bytes = header(0, &1_000_000u64.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 13]); // only one real entry present
        assert!(UddSketch::deserialize(&bytes).is_err());

        // Truncated header.
        assert!(UddSketch::deserialize(&[0u8; 16]).is_err());
    }

    #[test]
    fn serde_round_trip() {
        let mut udd = UddSketch::new(0.01, 32).unwrap();
        for i in 1..=50_000u64 {
            udd.add(i as f64);
        }
        let restored = UddSketch::deserialize(&udd.serialize()).unwrap();
        assert_eq!(restored.count(), udd.count());
        assert_eq!(restored.collapse_count(), udd.collapse_count());
        assert!((restored.relative_accuracy() - udd.relative_accuracy()).abs() < 1e-12);
        assert_eq!(restored.quantile(0.5), udd.quantile(0.5));
    }
}
