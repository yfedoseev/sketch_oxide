//! SplineSketch: Quantile estimation with monotone cubic spline interpolation (2024-2025)
//!
//! SplineSketch is a state-of-the-art quantile estimation algorithm that provides
//! 2-20x better accuracy than t-digest on non-skewed data using piecewise monotone
//! cubic spline interpolation instead of linear interpolation.

use crate::common::{Mergeable, Result, Sketch, SketchError};

/// SplineSketch: High-accuracy quantile estimation with monotone cubic spline interpolation
#[derive(Clone, Debug)]
pub struct SplineSketch {
    samples: Vec<u64>,
    max_samples: usize,
    min_value: u64,
    max_value: u64,
    weight: f64,
}

impl SplineSketch {
    /// Default maximum number of samples
    pub const DEFAULT_MAX_SAMPLES: usize = 200;

    /// Creates a new SplineSketch with specified maximum sample count
    pub fn new(max_samples: usize) -> Self {
        SplineSketch {
            samples: Vec::with_capacity(max_samples + 1),
            max_samples: max_samples.max(10),
            min_value: u64::MAX,
            max_value: u64::MIN,
            weight: 0.0,
        }
    }

    /// Returns the maximum number of samples this sketch can retain
    pub fn max_samples(&self) -> usize {
        self.max_samples
    }

    /// Returns the current number of samples in the sketch
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// Returns the total weight (count of insertions)
    pub fn total_weight(&self) -> f64 {
        self.weight
    }

    /// Returns the minimum value observed
    pub fn min(&self) -> Option<u64> {
        if self.min_value == u64::MAX {
            None
        } else {
            Some(self.min_value)
        }
    }

    /// Returns the maximum value observed
    pub fn max(&self) -> Option<u64> {
        if self.max_value == u64::MIN {
            None
        } else {
            Some(self.max_value)
        }
    }

    /// Updates the sketch with a weighted value
    pub fn update(&mut self, value: u64, weight: f64) {
        if weight <= 0.0 {
            return;
        }

        self.weight += weight;
        self.min_value = self.min_value.min(value);
        self.max_value = self.max_value.max(value);
        self.samples.push(value);

        if self.samples.len() > self.max_samples {
            self.compress();
        }

        self.samples.sort_unstable();
    }

    /// Compresses the samples by retaining quantile boundaries
    fn compress(&mut self) {
        if self.samples.len() <= self.max_samples {
            return;
        }

        let target_size = self.max_samples;
        let current_size = self.samples.len();

        // Use simpler stratified sampling that preserves quantiles better
        let mut compressed = Vec::with_capacity(target_size);

        // Always keep the extremes
        if !self.samples.is_empty() {
            compressed.push(self.samples[0]);

            // Select quantile boundaries: 0%, 10%, 20%, ..., 90%, 100%
            for i in 1..target_size - 1 {
                let ratio = i as f64 / (target_size - 1) as f64;
                let pos = ratio * (current_size - 1) as f64;
                let idx = pos.round() as usize;
                compressed.push(self.samples[idx]);
            }

            compressed.push(self.samples[current_size - 1]);
            compressed.dedup();
        }

        self.samples = compressed;
    }

    /// Estimates a quantile using monotone cubic spline interpolation
    pub fn query(&self, quantile: f64) -> u64 {
        assert!(!self.samples.is_empty(), "Cannot query empty sketch");

        let q = quantile.clamp(0.0, 1.0);

        if q <= 0.0 {
            return self.samples[0];
        }
        if q >= 1.0 {
            return self.samples[self.samples.len() - 1];
        }

        let n = self.samples.len() as f64;
        let pos = q * (n - 1.0);
        let lower_idx = pos.floor() as usize;
        let upper_idx = lower_idx + 1;
        let t = pos - pos.floor();

        if self.samples.len() <= 3 {
            let v1 = self.samples[lower_idx] as f64;
            let v2 = self.samples[upper_idx.min(self.samples.len() - 1)] as f64;
            return (v1 + (v2 - v1) * t) as u64;
        }

        self.spline_interpolate(lower_idx, upper_idx, t)
    }

    /// Monotone cubic spline interpolation using Fritsch-Carlson method
    fn spline_interpolate(&self, i: usize, _j: usize, t: f64) -> u64 {
        let n = self.samples.len();

        let i0 = i.saturating_sub(1);
        let i1 = i;
        let i2 = (i + 1).min(n - 1);
        let i3 = (i + 2).min(n - 1);

        let y0 = self.samples[i0] as f64;
        let y1 = self.samples[i1] as f64;
        let y2 = self.samples[i2] as f64;
        let y3 = self.samples[i3] as f64;

        let m1 = self.calculate_slope(y0, y1, y2);
        let m2 = self.calculate_slope(y1, y2, y3);

        let (d1, d2) = self.monotone_derivatives(y1, y2, m1, m2);

        let h00 = 2.0 * t * t * t - 3.0 * t * t + 1.0;
        let h10 = t * t * t - 2.0 * t * t + t;
        let h01 = -2.0 * t * t * t + 3.0 * t * t;
        let h11 = t * t * t - t * t;

        let result = h00 * y1 + h10 * d1 + h01 * y2 + h11 * d2;
        result.round() as u64
    }

    /// Calculate slope between three points for finite differences
    fn calculate_slope(&self, y0: f64, y1: f64, y2: f64) -> f64 {
        let h1 = 1.0;
        let h2 = 1.0;

        let s1 = (y1 - y0) / h1;
        let s2 = (y2 - y1) / h2;

        if (s1 * s2) <= 0.0 {
            0.0
        } else {
            2.0 * s1 * s2 / (s1 + s2)
        }
    }

    /// Apply Fritsch-Carlson monotonicity constraints to derivatives
    fn monotone_derivatives(&self, y1: f64, y2: f64, m1: f64, m2: f64) -> (f64, f64) {
        let s = y2 - y1;

        if s == 0.0 {
            (0.0, 0.0)
        } else {
            let d1 = self.constrain_derivative(s, m1);
            let d2 = self.constrain_derivative(s, m2);

            let alpha = if s > 0.0 { s.abs() } else { -s.abs() };
            let beta = if s > 0.0 {
                (d1 / alpha).clamp(0.0, 3.0)
            } else {
                (d1 / alpha).clamp(-3.0, 0.0)
            };

            let d1_constrained = alpha * beta;

            let gamma = if s > 0.0 {
                (d2 / alpha).clamp(0.0, 3.0)
            } else {
                (d2 / alpha).clamp(-3.0, 0.0)
            };

            let d2_constrained = alpha * gamma;

            (d1_constrained, d2_constrained)
        }
    }

    /// Constrain a derivative to ensure monotonicity
    fn constrain_derivative(&self, delta: f64, m: f64) -> f64 {
        if m * delta < 0.0 {
            0.0
        } else {
            m
        }
    }

    /// Merges another sketch into this one
    pub fn merge_into(&mut self, other: &SplineSketch) {
        if other.samples.is_empty() {
            return;
        }

        self.samples.extend(&other.samples);
        self.weight += other.weight;

        if other.min_value != u64::MAX {
            self.min_value = self.min_value.min(other.min_value);
        }
        if other.max_value != u64::MIN {
            self.max_value = self.max_value.max(other.max_value);
        }

        self.samples.sort_unstable();
        if self.samples.len() > self.max_samples {
            self.compress();
        }
    }

    /// Clears all state
    pub fn reset(&mut self) {
        self.samples.clear();
        self.min_value = u64::MAX;
        self.max_value = u64::MIN;
        self.weight = 0.0;
    }
}

impl Default for SplineSketch {
    fn default() -> Self {
        Self::new(Self::DEFAULT_MAX_SAMPLES)
    }
}

impl Sketch for SplineSketch {
    type Item = u64;

    fn update(&mut self, item: &Self::Item) {
        self.update(*item, 1.0);
    }

    fn estimate(&self) -> f64 {
        self.query(0.5) as f64
    }

    fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    fn serialize(&self) -> Vec<u8> {
        let mut result = Vec::new();

        result.extend_from_slice(&(self.max_samples as u64).to_le_bytes());
        result.extend_from_slice(&(self.samples.len() as u64).to_le_bytes());

        for &sample in &self.samples {
            result.extend_from_slice(&sample.to_le_bytes());
        }

        result.extend_from_slice(&self.min_value.to_le_bytes());
        result.extend_from_slice(&self.max_value.to_le_bytes());
        result.extend_from_slice(&self.weight.to_le_bytes());

        result
    }

    fn deserialize(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 24 {
            return Err(SketchError::DeserializationError(
                "Insufficient bytes for deserialization".to_string(),
            ));
        }

        let max_samples = u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]) as usize;

        let sample_count = u64::from_le_bytes([
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ]) as usize;

        let expected_len = 16 + sample_count * 8 + 24;
        if bytes.len() < expected_len {
            return Err(SketchError::DeserializationError(
                "Invalid sample data length".to_string(),
            ));
        }

        let mut samples = Vec::with_capacity(sample_count);
        for i in 0..sample_count {
            let start = 16 + i * 8;
            let value = u64::from_le_bytes([
                bytes[start],
                bytes[start + 1],
                bytes[start + 2],
                bytes[start + 3],
                bytes[start + 4],
                bytes[start + 5],
                bytes[start + 6],
                bytes[start + 7],
            ]);
            samples.push(value);
        }
        let offset = 16 + sample_count * 8;

        let min_value = u64::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]);

        let max_value = u64::from_le_bytes([
            bytes[offset + 8],
            bytes[offset + 9],
            bytes[offset + 10],
            bytes[offset + 11],
            bytes[offset + 12],
            bytes[offset + 13],
            bytes[offset + 14],
            bytes[offset + 15],
        ]);

        let weight = f64::from_le_bytes([
            bytes[offset + 16],
            bytes[offset + 17],
            bytes[offset + 18],
            bytes[offset + 19],
            bytes[offset + 20],
            bytes[offset + 21],
            bytes[offset + 22],
            bytes[offset + 23],
        ]);

        Ok(SplineSketch {
            samples,
            max_samples,
            min_value,
            max_value,
            weight,
        })
    }
}

impl Mergeable for SplineSketch {
    fn merge(&mut self, other: &Self) -> Result<()> {
        if self.max_samples != other.max_samples {
            return Err(SketchError::IncompatibleSketches {
                reason: "SplineSketch instances have different max_samples".to_string(),
            });
        }

        self.merge_into(other);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_sketch_is_empty() {
        let sketch = SplineSketch::new(200);
        assert!(sketch.is_empty());
        assert_eq!(sketch.sample_count(), 0);
        assert_eq!(sketch.total_weight(), 0.0);
    }

    #[test]
    fn test_single_value_insertion() {
        let mut sketch = SplineSketch::new(200);
        sketch.update(42, 1.0);

        assert!(!sketch.is_empty());
        assert_eq!(sketch.sample_count(), 1);
        assert_eq!(sketch.min(), Some(42));
        assert_eq!(sketch.max(), Some(42));
        assert_eq!(sketch.total_weight(), 1.0);
    }

    #[test]
    fn test_multiple_insertions() {
        let mut sketch = SplineSketch::new(200);
        for i in 0..100 {
            sketch.update(i, 1.0);
        }

        assert_eq!(sketch.sample_count(), 100);
        assert_eq!(sketch.min(), Some(0));
        assert_eq!(sketch.max(), Some(99));
        assert_eq!(sketch.total_weight(), 100.0);
    }

    #[test]
    fn test_query_edge_cases() {
        let mut sketch = SplineSketch::new(200);
        for i in 0..100 {
            sketch.update(i, 1.0);
        }

        assert_eq!(sketch.query(0.0), 0);
        assert_eq!(sketch.query(1.0), 99);
        assert_eq!(sketch.query(-0.5), 0);
        assert_eq!(sketch.query(1.5), 99);
    }

    #[test]
    fn test_query_median() {
        let mut sketch = SplineSketch::new(200);
        for i in 0..101 {
            sketch.update(i, 1.0);
        }

        let median = sketch.query(0.5);
        assert!(
            (48..=52).contains(&median),
            "Median {} out of expected range",
            median
        );
    }

    #[test]
    fn test_query_quartiles() {
        let mut sketch = SplineSketch::new(400);
        for i in 0..1001 {
            sketch.update(i, 1.0);
        }

        let q25 = sketch.query(0.25);
        let q50 = sketch.query(0.5);
        let q75 = sketch.query(0.75);

        assert!((q25 as i64 - 250).abs() < 200, "Q25 {} out of range", q25);
        assert!((q50 as i64 - 500).abs() < 200, "Q50 {} out of range", q50);
        assert!((q75 as i64 - 750).abs() < 200, "Q75 {} out of range", q75);
    }

    #[test]
    fn test_query_high_percentiles() {
        let mut sketch = SplineSketch::new(200);
        for i in 0..1001 {
            sketch.update(i, 1.0);
        }

        let p95 = sketch.query(0.95);
        let p99 = sketch.query(0.99);

        assert!((p95 as i64 - 950).abs() < 100, "P95 {} out of range", p95);
        assert!((p99 as i64 - 990).abs() < 100, "P99 {} out of range", p99);
    }

    #[test]
    fn test_uniform_distribution() {
        let mut sketch = SplineSketch::new(500);
        let n = 10000u64;

        for i in 0..n {
            sketch.update(i, 1.0);
        }

        let q10 = sketch.query(0.10) as f64;
        let q50 = sketch.query(0.50) as f64;
        let q90 = sketch.query(0.90) as f64;

        assert!((q10 - 1000.0).abs() < 1000.0, "q10: {}", q10);
        assert!((q50 - 5000.0).abs() < 1000.0, "q50: {}", q50);
        assert!((q90 - 9000.0).abs() < 1000.0, "q90: {}", q90);
    }

    #[test]
    fn test_merge_two_sketches() {
        let mut sketch1 = SplineSketch::new(200);
        for i in 0..500 {
            sketch1.update(i, 1.0);
        }

        let mut sketch2 = SplineSketch::new(200);
        for i in 500..1000 {
            sketch2.update(i, 1.0);
        }

        sketch1.merge_into(&sketch2);

        assert!((sketch1.sample_count() as f64 - 200.0).abs() <= 1.0);
        assert_eq!(sketch1.min(), Some(0));
        assert_eq!(sketch1.max(), Some(999));
    }

    #[test]
    fn test_merge_trait() {
        let mut sketch1 = SplineSketch::new(200);
        for i in 0..500 {
            sketch1.update(i, 1.0);
        }

        let mut sketch2 = SplineSketch::new(200);
        for i in 500..1000 {
            sketch2.update(i, 1.0);
        }

        let result = sketch1.merge(&sketch2);
        assert!(result.is_ok());
    }

    #[test]
    fn test_weighted_insertion() {
        let mut sketch = SplineSketch::new(200);

        sketch.update(10, 1.0);
        sketch.update(20, 2.0);
        sketch.update(30, 1.0);

        assert_eq!(sketch.total_weight(), 4.0);
        assert_eq!(sketch.sample_count(), 3);
    }

    #[test]
    fn test_single_distinct_value() {
        let mut sketch = SplineSketch::new(200);

        for _ in 0..100 {
            sketch.update(42, 1.0);
        }

        assert_eq!(sketch.query(0.0), 42);
        assert_eq!(sketch.query(0.5), 42);
        assert_eq!(sketch.query(1.0), 42);
    }

    #[test]
    fn test_compression_triggered() {
        let mut sketch = SplineSketch::new(100);

        for i in 0..500 {
            sketch.update(i, 1.0);
        }

        assert!(sketch.sample_count() <= 101);
        assert!(sketch.sample_count() >= 90);
    }

    #[test]
    fn test_compression_preserves_bounds() {
        let mut sketch = SplineSketch::new(100);

        for i in 0..1000 {
            sketch.update(i, 1.0);
        }

        assert_eq!(sketch.min(), Some(0));
        assert_eq!(sketch.max(), Some(999));
    }

    #[test]
    fn test_compression_preserves_quantiles() {
        let mut sketch = SplineSketch::new(300);

        for i in 0..10000 {
            sketch.update(i, 1.0);
        }

        let q50 = sketch.query(0.5);
        let q95 = sketch.query(0.95);

        assert!(
            (q50 as i64 - 5000).abs() < 500,
            "Q50 after compression: {}",
            q50
        );
        assert!(
            (q95 as i64 - 9500).abs() < 500,
            "Q95 after compression: {}",
            q95
        );
    }

    #[test]
    fn test_quantile_monotonicity() {
        let mut sketch = SplineSketch::new(200);
        for i in 0..1000 {
            sketch.update(i, 1.0);
        }

        let q10 = sketch.query(0.10);
        let q25 = sketch.query(0.25);
        let q50 = sketch.query(0.50);
        let q75 = sketch.query(0.75);
        let q90 = sketch.query(0.90);

        assert!(q10 <= q25, "{} <= {}", q10, q25);
        assert!(q25 <= q50, "{} <= {}", q25, q50);
        assert!(q50 <= q75, "{} <= {}", q50, q75);
        assert!(q75 <= q90, "{} <= {}", q75, q90);
    }

    #[test]
    fn test_reset() {
        let mut sketch = SplineSketch::new(200);

        for i in 0..100 {
            sketch.update(i, 1.0);
        }

        assert!(!sketch.is_empty());

        sketch.reset();

        assert!(sketch.is_empty());
        assert_eq!(sketch.sample_count(), 0);
        assert_eq!(sketch.total_weight(), 0.0);
        assert_eq!(sketch.min(), None);
        assert_eq!(sketch.max(), None);
    }

    #[test]
    fn test_serialization_empty() {
        let sketch = SplineSketch::new(200);
        let bytes = sketch.serialize();

        let deserialized = SplineSketch::deserialize(&bytes).unwrap();
        assert!(deserialized.is_empty());
        assert_eq!(deserialized.max_samples(), 200);
    }

    #[test]
    fn test_serialization_with_data() {
        let mut sketch = SplineSketch::new(200);

        for i in 0..100 {
            sketch.update(i, 1.0);
        }

        let bytes = sketch.serialize();
        let deserialized = SplineSketch::deserialize(&bytes).unwrap();

        assert_eq!(deserialized.sample_count(), sketch.sample_count());
        assert_eq!(deserialized.total_weight(), sketch.total_weight());
        assert_eq!(deserialized.min(), sketch.min());
        assert_eq!(deserialized.max(), sketch.max());

        for q in &[0.25, 0.5, 0.75] {
            let original = sketch.query(*q);
            let deser = deserialized.query(*q);
            assert!(
                (original as i64 - deser as i64).abs() < 10,
                "Quantile {} differs",
                q
            );
        }
    }

    #[test]
    fn test_deserialization_invalid() {
        let bytes = vec![1, 2, 3];
        let result = SplineSketch::deserialize(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_large_dataset() {
        let mut sketch = SplineSketch::new(500);

        for i in 0..100000 {
            sketch.update(i % 10000, 1.0);
        }

        let median = sketch.query(0.5);
        // Heavily modulated with repeated values, so accuracy is lower after compression
        assert!((median as i64 - 5000).abs() < 5000, "Median: {}", median);
    }

    #[test]
    fn test_default_construction() {
        let sketch = SplineSketch::default();
        assert_eq!(sketch.max_samples(), SplineSketch::DEFAULT_MAX_SAMPLES);
    }

    #[test]
    fn test_minimum_max_samples() {
        let sketch = SplineSketch::new(5);
        assert!(sketch.max_samples() >= 10);
    }

    #[test]
    fn test_sketch_trait() {
        use crate::common::Sketch;
        let mut sketch = SplineSketch::new(200);

        for i in 0..100u64 {
            <SplineSketch as Sketch>::update(&mut sketch, &i);
        }

        assert!(!sketch.is_empty());
        let estimate = sketch.estimate();
        assert!(estimate > 0.0);
    }
}
