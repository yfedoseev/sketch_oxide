//! OpenTelemetry-style base-2 exponential histogram.
//!
//! This is the wire format of modern observability — OTel / Prometheus native histograms.
//! Buckets are exponentially spaced with a `scale` parameter: the bucket boundaries are
//! powers of `base = 2^(2^-scale)`, so a larger `scale` means finer (lower-relative-error)
//! buckets. When the populated bucket span would exceed `max_buckets`, the histogram
//! **downscales** (halving resolution, `scale -= 1`, merging adjacent bucket pairs) exactly
//! as the OTel SDKs do, which keeps it mergeable and bounded.
//!
//! The internal layout mirrors the OTLP `ExponentialHistogramDataPoint`: a `scale`, a
//! `zero_count`, and positive/negative bucket runs each given by an `offset` and a dense
//! `counts` array. The accessors here ([`scale`](OtelExponentialHistogram::scale),
//! [`positive_offset`](OtelExponentialHistogram::positive_offset), …) expose exactly those
//! fields for OTLP encoding.
//!
//! # Mapping
//!
//! A positive value `v` maps to bucket `index = ⌈log2(v) · 2^scale⌉ − 1`, whose range is
//! `(base^index, base^(index+1)]`. Quantile queries return a bucket's geometric midpoint, so
//! the relative error is at most `base − 1 = 2^(2^-scale) − 1`.
//!
//! # References
//! - OpenTelemetry metrics data model — Exponential Histogram.

use crate::common::{Mergeable, Result, Sketch, SketchError};

/// A run of exponential buckets: bucket `offset + i` holds `counts[i]`.
#[derive(Debug, Clone, Default)]
struct BucketRun {
    offset: i32,
    counts: Vec<u64>,
}

impl BucketRun {
    fn is_empty(&self) -> bool {
        self.counts.is_empty()
    }

    fn total(&self) -> u64 {
        self.counts.iter().sum()
    }

    /// Number of dense bucket slots (the index span covered).
    fn span(&self) -> usize {
        self.counts.len()
    }

    /// Adds `n` to bucket `index`, growing/shifting the dense array to cover it.
    fn record(&mut self, index: i32, n: u64) {
        if self.counts.is_empty() {
            self.offset = index;
            self.counts.push(n);
            return;
        }
        if index < self.offset {
            // Prepend zeros down to the new lower index.
            let grow = (self.offset - index) as usize;
            let mut new_counts = Vec::with_capacity(grow + self.counts.len());
            new_counts.resize(grow, 0);
            new_counts.extend_from_slice(&self.counts);
            self.counts = new_counts;
            self.offset = index;
        }
        let pos = (index - self.offset) as usize;
        if pos >= self.counts.len() {
            self.counts.resize(pos + 1, 0);
        }
        self.counts[pos] += n;
    }

    /// Halves resolution `by` times: bucket `i` moves to `i >> by`, summing counts.
    fn downscale(&mut self, by: u32) {
        if self.is_empty() || by == 0 {
            return;
        }
        let mut merged: Vec<(i32, u64)> = Vec::new();
        for (i, &c) in self.counts.iter().enumerate() {
            if c == 0 {
                continue;
            }
            let new_index = (self.offset + i as i32) >> by;
            merged.push((new_index, c));
        }
        if merged.is_empty() {
            self.counts.clear();
            return;
        }
        let lo = merged.iter().map(|&(i, _)| i).min().unwrap();
        let hi = merged.iter().map(|&(i, _)| i).max().unwrap();
        let mut counts = vec![0u64; (hi - lo + 1) as usize];
        for (i, c) in merged {
            counts[(i - lo) as usize] += c;
        }
        self.offset = lo;
        self.counts = counts;
    }

    /// Adds another run (already at the same scale) into this one.
    fn add(&mut self, other: &BucketRun) {
        for (i, &c) in other.counts.iter().enumerate() {
            if c != 0 {
                self.record(other.offset + i as i32, c);
            }
        }
    }
}

/// OpenTelemetry-compatible base-2 exponential histogram with bounded buckets.
///
/// # Example
/// ```
/// use sketch_oxide::quantiles::OtelExponentialHistogram;
///
/// // scale 4 (~4.4% relative error), at most 160 buckets.
/// let mut h = OtelExponentialHistogram::new(4, 160).unwrap();
/// for i in 1..=10_000u64 {
///     h.record(i as f64);
/// }
/// let p50 = h.quantile(0.5).unwrap();
/// assert!((p50 - 5000.0).abs() < 0.10 * 5000.0);
/// ```
#[derive(Debug, Clone)]
pub struct OtelExponentialHistogram {
    scale: i32,
    max_buckets: usize,
    positive: BucketRun,
    negative: BucketRun,
    zero_count: u64,
    count: u64,
    sum: f64,
    min: f64,
    max: f64,
}

impl OtelExponentialHistogram {
    /// Creates a histogram with the given initial `scale` and `max_buckets` budget.
    ///
    /// `scale` may be negative (coarser than base 2). Typical SDK defaults start at 20 and
    /// downscale as needed; pick a smaller initial scale to skip early downscales.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `max_buckets < 2` or `scale` is outside the OTLP
    /// range `[-10, 20]`.
    pub fn new(scale: i32, max_buckets: usize) -> Result<Self> {
        if !(-10..=20).contains(&scale) {
            return Err(SketchError::InvalidParameter {
                param: "scale".to_string(),
                value: scale.to_string(),
                constraint: "must be in [-10, 20]".to_string(),
            });
        }
        if max_buckets < 2 {
            return Err(SketchError::InvalidParameter {
                param: "max_buckets".to_string(),
                value: max_buckets.to_string(),
                constraint: "must be >= 2".to_string(),
            });
        }
        Ok(Self {
            scale,
            max_buckets,
            positive: BucketRun::default(),
            negative: BucketRun::default(),
            zero_count: 0,
            count: 0,
            sum: 0.0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        })
    }

    /// `2^scale`, the factor in the log mapping.
    #[inline]
    fn scale_factor(&self) -> f64 {
        2.0_f64.powi(self.scale)
    }

    /// Maps a positive magnitude to its bucket index at the current scale.
    #[inline]
    fn index_of(&self, magnitude: f64) -> i32 {
        (magnitude.log2() * self.scale_factor()).ceil() as i32 - 1
    }

    /// Geometric midpoint value of a positive bucket at the current scale.
    #[inline]
    fn value_of(&self, index: i32) -> f64 {
        // midpoint = base^(index + 0.5) = 2^((index + 0.5) / 2^scale)
        2.0_f64.powf((index as f64 + 0.5) / self.scale_factor())
    }

    /// Records a measurement.
    pub fn record(&mut self, value: f64) {
        if !value.is_finite() {
            return;
        }
        self.count += 1;
        self.sum += value;
        self.min = self.min.min(value);
        self.max = self.max.max(value);

        if value == 0.0 {
            self.zero_count += 1;
            return;
        }
        let index = self.index_of(value.abs());
        if value > 0.0 {
            self.positive.record(index, 1);
        } else {
            self.negative.record(index, 1);
        }
        self.rescale_if_needed();
    }

    /// Downscales until the combined positive+negative bucket span fits the budget.
    fn rescale_if_needed(&mut self) {
        while self.positive.span() + self.negative.span() > self.max_buckets && self.scale > -10 {
            self.positive.downscale(1);
            self.negative.downscale(1);
            self.scale -= 1;
        }
    }

    /// Total number of recorded measurements.
    #[inline]
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Whether nothing has been recorded.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Sum of all recorded values (OTLP `sum`).
    #[inline]
    pub fn sum(&self) -> f64 {
        self.sum
    }

    /// Minimum recorded value, if any.
    pub fn min(&self) -> Option<f64> {
        (self.count > 0).then_some(self.min)
    }

    /// Maximum recorded value, if any.
    pub fn max(&self) -> Option<f64> {
        (self.count > 0).then_some(self.max)
    }

    /// Current scale (OTLP `scale`).
    #[inline]
    pub fn scale(&self) -> i32 {
        self.scale
    }

    /// OTLP `zero_count`.
    #[inline]
    pub fn zero_count(&self) -> u64 {
        self.zero_count
    }

    /// OTLP positive-bucket `offset`.
    #[inline]
    pub fn positive_offset(&self) -> i32 {
        self.positive.offset
    }

    /// OTLP positive-bucket `bucket_counts`.
    #[inline]
    pub fn positive_counts(&self) -> &[u64] {
        &self.positive.counts
    }

    /// OTLP negative-bucket `offset`.
    #[inline]
    pub fn negative_offset(&self) -> i32 {
        self.negative.offset
    }

    /// OTLP negative-bucket `bucket_counts`.
    #[inline]
    pub fn negative_counts(&self) -> &[u64] {
        &self.negative.counts
    }

    /// Estimates the value at quantile `q` in `[0, 1]`.
    pub fn quantile(&self, q: f64) -> Option<f64> {
        if !(0.0..=1.0).contains(&q) || self.count == 0 {
            return None;
        }
        let rank = if q == 0.0 {
            1
        } else {
            (q * self.count as f64).ceil() as u64
        };

        let mut acc = 0u64;

        // Negative buckets: most negative first (largest magnitude = highest index).
        let neg_total = self.negative.total();
        if rank <= neg_total {
            for i in (0..self.negative.counts.len()).rev() {
                acc += self.negative.counts[i];
                if acc >= rank {
                    return Some(-self.value_of(self.negative.offset + i as i32));
                }
            }
        }
        acc = neg_total;

        if rank <= acc + self.zero_count {
            return Some(0.0);
        }
        acc += self.zero_count;

        for (i, &c) in self.positive.counts.iter().enumerate() {
            acc += c;
            if acc >= rank {
                return Some(self.value_of(self.positive.offset + i as i32));
            }
        }
        None
    }
}

impl Mergeable for OtelExponentialHistogram {
    /// Merges `other` in, aligning to the coarser (smaller) scale first.
    fn merge(&mut self, other: &Self) -> Result<()> {
        let mut other = other.clone();
        // Bring both to the same (coarser) scale.
        if self.scale > other.scale {
            let by = (self.scale - other.scale) as u32;
            self.positive.downscale(by);
            self.negative.downscale(by);
            self.scale = other.scale;
        } else if other.scale > self.scale {
            let by = (other.scale - self.scale) as u32;
            other.positive.downscale(by);
            other.negative.downscale(by);
            other.scale = self.scale;
        }

        self.positive.add(&other.positive);
        self.negative.add(&other.negative);
        self.zero_count += other.zero_count;
        self.count += other.count;
        self.sum += other.sum;
        if other.count > 0 {
            self.min = self.min.min(other.min);
            self.max = self.max.max(other.max);
        }
        self.rescale_if_needed();
        Ok(())
    }
}

impl Sketch for OtelExponentialHistogram {
    type Item = f64;

    fn update(&mut self, item: &Self::Item) {
        self.record(*item);
    }

    fn estimate(&self) -> f64 {
        self.quantile(0.5).unwrap_or(0.0)
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }

    fn serialize(&self) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&self.scale.to_le_bytes());
        b.extend_from_slice(&(self.max_buckets as u64).to_le_bytes());
        b.extend_from_slice(&self.zero_count.to_le_bytes());
        b.extend_from_slice(&self.count.to_le_bytes());
        b.extend_from_slice(&self.sum.to_le_bytes());
        b.extend_from_slice(&self.min.to_le_bytes());
        b.extend_from_slice(&self.max.to_le_bytes());
        for run in [&self.positive, &self.negative] {
            b.extend_from_slice(&run.offset.to_le_bytes());
            b.extend_from_slice(&(run.counts.len() as u64).to_le_bytes());
            for &c in &run.counts {
                b.extend_from_slice(&c.to_le_bytes());
            }
        }
        b
    }

    fn deserialize(bytes: &[u8]) -> Result<Self> {
        let err = || SketchError::DeserializationError("OtelExponentialHistogram".into());
        let rd_i32 = |o: usize| -> Result<i32> {
            Ok(i32::from_le_bytes(
                bytes.get(o..o + 4).ok_or_else(err)?.try_into().unwrap(),
            ))
        };
        let rd_u64 = |o: usize| -> Result<u64> {
            Ok(u64::from_le_bytes(
                bytes.get(o..o + 8).ok_or_else(err)?.try_into().unwrap(),
            ))
        };
        let rd_f64 = |o: usize| -> Result<f64> {
            Ok(f64::from_le_bytes(
                bytes.get(o..o + 8).ok_or_else(err)?.try_into().unwrap(),
            ))
        };

        let scale = rd_i32(0)?;
        let max_buckets = rd_u64(4)? as usize;
        let mut h = Self::new(scale, max_buckets.max(2))?;
        h.zero_count = rd_u64(12)?;
        h.count = rd_u64(20)?;
        h.sum = rd_f64(28)?;
        h.min = rd_f64(36)?;
        h.max = rd_f64(44)?;

        let mut off = 52;
        for which in 0..2 {
            let offset = rd_i32(off)?;
            let len = rd_u64(off + 4)? as usize;
            off += 12;
            // Validate the attacker-controlled `len` against the bytes actually
            // remaining before reserving — otherwise a crafted huge `len` drives
            // an unbounded `Vec::with_capacity` (OOM) even though the read loop
            // would eventually error (fable5 doc 01 F2).
            let needed = len
                .checked_mul(8)
                .ok_or_else(|| SketchError::DeserializationError("run length overflow".into()))?;
            if off.checked_add(needed).is_none_or(|end| end > bytes.len()) {
                return Err(SketchError::DeserializationError(
                    "bucket run length exceeds input".into(),
                ));
            }
            let mut counts = Vec::with_capacity(len);
            for _ in 0..len {
                counts.push(rd_u64(off)?);
                off += 8;
            }
            let run = BucketRun { offset, counts };
            if which == 0 {
                h.positive = run;
            } else {
                h.negative = run;
            }
        }
        Ok(h)
    }
}

// Capability-trait adoptions (fable5 doc 01 F3): delegate to inherent methods.
mod capability_impls {
    use super::*;
    use crate::common::capabilities::{QuantileQuery, Serializable, Update};

    impl Update<f64> for OtelExponentialHistogram {
        fn update(&mut self, item: &f64) {
            self.record(*item);
        }
    }

    // `quantile(&self, ..) -> Option<f64>` is immutable, so `QuantileQuery` fits.
    impl QuantileQuery for OtelExponentialHistogram {
        fn quantile(&self, rank: f64) -> Option<f64> {
            OtelExponentialHistogram::quantile(self, rank)
        }
    }

    impl Serializable for OtelExponentialHistogram {
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
        assert!(OtelExponentialHistogram::new(21, 100).is_err());
        assert!(OtelExponentialHistogram::new(-11, 100).is_err());
        assert!(OtelExponentialHistogram::new(4, 1).is_err());
        assert!(OtelExponentialHistogram::new(4, 100).is_ok());
    }

    #[test]
    fn quantiles_within_relative_error() {
        let mut h = OtelExponentialHistogram::new(6, 4096).unwrap(); // ~1.1% error
        for i in 1..=10_000u64 {
            h.record(i as f64);
        }
        let base = 2.0_f64.powf(2.0_f64.powi(-h.scale()));
        let rel = base - 1.0;
        for &(q, truth) in &[(0.5, 5000.0), (0.9, 9000.0), (0.99, 9900.0)] {
            let est = h.quantile(q).unwrap();
            assert!(
                (est - truth).abs() <= rel * truth * 1.5 + 1.0,
                "q={q}: est {est} truth {truth} rel {rel}"
            );
        }
    }

    #[test]
    fn downscales_to_fit_budget() {
        let mut h = OtelExponentialHistogram::new(10, 32).unwrap();
        for i in 1..=1_000_000u64 {
            if i % 7 == 0 {
                h.record(i as f64);
            }
        }
        assert!(
            h.positive_counts().len() <= 32,
            "{}",
            h.positive_counts().len()
        );
        assert!(
            h.scale() < 10,
            "should have downscaled, scale={}",
            h.scale()
        );
        // The true median (~500k) must fall within the estimate's (now coarse) bucket: the
        // bucket midpoint is within a factor of sqrt(base) of any value it contains.
        let est = h.quantile(0.5).unwrap();
        let half_width = 2.0_f64.powf(2.0_f64.powi(-h.scale())).sqrt();
        assert!(
            est * half_width >= 500_000.0 && est / half_width <= 500_000.0,
            "p50 {est} should bracket true median 500000 at scale {} (factor {half_width})",
            h.scale()
        );
    }

    #[test]
    fn handles_zero_and_negative() {
        let mut h = OtelExponentialHistogram::new(4, 512).unwrap();
        for i in -1000..=1000 {
            h.record(i as f64);
        }
        assert_eq!(h.count(), 2001);
        assert_eq!(h.zero_count(), 1);
        assert!(h.quantile(0.5).unwrap().abs() < 50.0);
        assert!(h.quantile(0.01).unwrap() < 0.0);
        assert!(h.quantile(0.99).unwrap() > 0.0);
    }

    #[test]
    fn merge_aligns_scales() {
        let mut a = OtelExponentialHistogram::new(8, 64).unwrap();
        let mut b = OtelExponentialHistogram::new(8, 64).unwrap();
        for i in 1..=20_000u64 {
            a.record(i as f64);
        }
        for i in 20_001..=40_000u64 {
            b.record(i as f64);
        }
        a.merge(&b).unwrap();
        assert_eq!(a.count(), 40_000);
        let p50 = a.quantile(0.5).unwrap();
        assert!((p50 - 20_000.0).abs() < 0.1 * 20_000.0, "p50 {p50}");
    }

    #[test]
    fn serde_round_trip() {
        let mut h = OtelExponentialHistogram::new(6, 128).unwrap();
        for i in 1..=50_000u64 {
            h.record(i as f64);
        }
        let restored = OtelExponentialHistogram::deserialize(&h.serialize()).unwrap();
        assert_eq!(restored.count(), h.count());
        assert_eq!(restored.scale(), h.scale());
        assert_eq!(restored.positive_counts(), h.positive_counts());
        assert_eq!(restored.quantile(0.5), h.quantile(0.5));
    }
}
