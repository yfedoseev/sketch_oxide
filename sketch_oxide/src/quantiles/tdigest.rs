//! T-Digest: Quantile estimation with tail accuracy (Dunning 2019)
//!
//! T-Digest is a streaming quantile algorithm that provides high accuracy
//! at the distribution tails (extreme percentiles like p99, p99.9).
//! Used by Netflix, Microsoft, Elasticsearch, and Prometheus.
//!
//! # Algorithm Overview
//!
//! T-Digest maintains a sorted set of "centroids" where each centroid
//! has a mean and weight. The compression parameter controls accuracy:
//! - Higher compression = more centroids = better accuracy
//! - Lower compression = fewer centroids = less memory
//!
//! Key insight: centroids are smaller at the tails (for better accuracy)
//! and larger in the middle (for space efficiency).
//!
//! # Comparison with DDSketch and REQ
//!
//! | Algorithm | Error Type | Memory | Best For |
//! |-----------|-----------|--------|----------|
//! | DDSketch | Relative | Medium | Uniform accuracy |
//! | REQ | Absolute at tails | Larger | Guaranteed tail |
//! | T-Digest | Relative at tails | Smaller | Extreme percentiles |
//!
//! # Time Complexity
//!
//! - Update: O(log n) amortized with batch compression
//! - Quantile: O(log n)
//! - Merge: O(n log n) where n is number of centroids
//!
//! # Space Complexity
//!
//! O(compression) centroids, typically 100-500
//!
//! # References
//!
//! - Dunning & Ertl "Computing Extremely Accurate Quantiles Using t-Digests" (2019)
//! - https://github.com/tdunning/t-digest
//!
//! # Examples
//!
//! ```
//! use sketch_oxide::quantiles::TDigest;
//!
//! let mut td = TDigest::new(100.0);
//!
//! // Add values
//! for i in 0..10_000 {
//!     td.update(i as f64);
//! }
//!
//! // Get quantiles
//! let median = td.quantile(0.5);
//! let p99 = td.quantile(0.99);
//! let p999 = td.quantile(0.999);
//! println!("Median: {}, P99: {}, P99.9: {}", median, p99, p999);
//! ```

use crate::common::{Mergeable, Sketch, SketchError};
use std::cmp::Ordering;

/// A centroid represents a cluster of values with a mean and count
#[derive(Clone, Debug)]
struct Centroid {
    /// Mean value of this centroid
    mean: f64,
    /// Number of values represented
    weight: f64,
}

impl Centroid {
    fn new(mean: f64, weight: f64) -> Self {
        Centroid { mean, weight }
    }

    /// Adds a value with weight to this centroid
    fn add(&mut self, value: f64, weight: f64) {
        let new_weight = self.weight + weight;
        self.mean += (value - self.mean) * weight / new_weight;
        self.weight = new_weight;
    }
}

/// T-Digest sketch for quantile estimation
///
/// Provides high accuracy at distribution tails with bounded memory.
///
/// # Examples
///
/// ```
/// use sketch_oxide::quantiles::TDigest;
/// use sketch_oxide::Sketch;
///
/// let mut td = TDigest::new(100.0);
/// for i in 0..1000 {
///     td.update(i as f64);
/// }
/// let median = td.quantile(0.5);
/// assert!((median - 500.0).abs() < 50.0);
/// ```
#[derive(Clone, Debug)]
pub struct TDigest {
    /// Compression parameter (higher = more accurate, more memory)
    compression: f64,
    /// Centroids sorted by mean
    centroids: Vec<Centroid>,
    /// Buffer for incoming values before compression
    buffer: Vec<f64>,
    /// Total weight of all centroids
    total_weight: f64,
    /// Minimum value seen
    min: f64,
    /// Maximum value seen
    max: f64,
    /// Buffer size before triggering compression
    buffer_size: usize,
}

impl TDigest {
    /// Default compression parameter
    pub const DEFAULT_COMPRESSION: f64 = 100.0;

    /// Default buffer size multiplier
    const BUFFER_FACTOR: usize = 5;

    /// Creates a new T-Digest with specified compression
    ///
    /// # Arguments
    ///
    /// * `compression` - Controls accuracy/memory tradeoff (typically 100-500)
    ///   - 100: Good balance, ~500 centroids max
    ///   - 200: Higher accuracy, ~1000 centroids max
    ///   - 500: Very high accuracy, ~2500 centroids max
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::quantiles::TDigest;
    /// use sketch_oxide::Sketch;
    ///
    /// let td = TDigest::new(100.0);
    /// assert!(td.is_empty());
    /// ```
    pub fn new(compression: f64) -> Self {
        let compression = compression.max(10.0); // Minimum compression
        TDigest {
            compression,
            centroids: Vec::new(),
            buffer: Vec::new(),
            total_weight: 0.0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
            buffer_size: (compression as usize) * Self::BUFFER_FACTOR,
        }
    }

    /// Creates a T-Digest with default compression (100)
    pub fn default_compression() -> Self {
        Self::new(Self::DEFAULT_COMPRESSION)
    }

    /// Returns the compression parameter
    pub fn compression(&self) -> f64 {
        self.compression
    }

    /// Returns the number of centroids
    pub fn centroid_count(&self) -> usize {
        self.centroids.len()
    }

    /// Returns the total weight (number of values added)
    pub fn count(&self) -> f64 {
        self.total_weight + self.buffer.len() as f64
    }

    /// Returns the minimum value seen
    pub fn min(&self) -> f64 {
        self.min
    }

    /// Returns the maximum value seen
    pub fn max(&self) -> f64 {
        self.max
    }

    /// Updates the T-Digest with a single value
    ///
    /// # Arguments
    ///
    /// * `value` - Value to add (must be finite)
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::quantiles::TDigest;
    ///
    /// let mut td = TDigest::new(100.0);
    /// td.update(42.0);
    /// td.update(100.0);
    /// ```
    pub fn update(&mut self, value: f64) {
        if !value.is_finite() {
            return;
        }

        self.min = self.min.min(value);
        self.max = self.max.max(value);
        self.buffer.push(value);

        if self.buffer.len() >= self.buffer_size {
            self.compress();
        }
    }

    /// Updates with a weighted value
    ///
    /// # Arguments
    ///
    /// * `value` - Value to add
    /// * `weight` - Weight of this value (typically frequency)
    pub fn update_weighted(&mut self, value: f64, weight: f64) {
        if !value.is_finite() || weight <= 0.0 {
            return;
        }

        self.min = self.min.min(value);
        self.max = self.max.max(value);

        // For weighted updates, add directly to centroids
        self.add_centroid(value, weight);

        if self.centroids.len() > self.buffer_size {
            self.compress();
        }
    }

    /// Adds a batch of values efficiently
    pub fn update_batch(&mut self, values: &[f64]) {
        for &value in values {
            if value.is_finite() {
                self.min = self.min.min(value);
                self.max = self.max.max(value);
                self.buffer.push(value);
            }
        }

        if self.buffer.len() >= self.buffer_size {
            self.compress();
        }
    }

    /// Returns the estimated quantile value
    ///
    /// # Arguments
    ///
    /// * `q` - Quantile to estimate (0.0 to 1.0)
    ///
    /// # Returns
    ///
    /// Estimated value at the given quantile
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::quantiles::TDigest;
    ///
    /// let mut td = TDigest::new(100.0);
    /// for i in 0..1000 {
    ///     td.update(i as f64);
    /// }
    ///
    /// let p50 = td.quantile(0.5);  // Median
    /// let p90 = td.quantile(0.9);
    /// let p99 = td.quantile(0.99);
    /// ```
    pub fn quantile(&mut self, q: f64) -> f64 {
        self.flush();

        if self.centroids.is_empty() {
            return 0.0;
        }

        let q = q.clamp(0.0, 1.0);

        if q == 0.0 {
            return self.min;
        }
        if q == 1.0 {
            return self.max;
        }

        let target = q * self.total_weight;
        let mut cumulative = 0.0;

        // Find the two centroids that bound the target
        for i in 0..self.centroids.len() {
            let centroid = &self.centroids[i];
            let next_cumulative = cumulative + centroid.weight;

            if next_cumulative >= target {
                // Interpolate within this centroid
                if i == 0 {
                    // First centroid - interpolate from min
                    let delta = centroid.mean - self.min;
                    let fraction = (target - cumulative) / centroid.weight;
                    return self.min + delta * fraction.min(1.0);
                } else {
                    // Interpolate between centroids
                    let prev = &self.centroids[i - 1];
                    let prev_cumulative = cumulative;

                    // Weight midpoints
                    let lower = prev_cumulative + prev.weight / 2.0;
                    let upper = cumulative + centroid.weight / 2.0;

                    if target <= lower {
                        return prev.mean;
                    }
                    if target >= upper {
                        return centroid.mean;
                    }

                    let fraction = (target - lower) / (upper - lower);
                    return prev.mean + (centroid.mean - prev.mean) * fraction;
                }
            }

            cumulative = next_cumulative;
        }

        self.max
    }

    /// Returns the CDF value (rank) for a given value
    ///
    /// # Arguments
    ///
    /// * `value` - Value to find rank for
    ///
    /// # Returns
    ///
    /// Estimated fraction of values <= given value (0.0 to 1.0)
    pub fn cdf(&mut self, value: f64) -> f64 {
        self.flush();

        if self.centroids.is_empty() {
            return 0.0;
        }

        if value <= self.min {
            return 0.0;
        }
        if value >= self.max {
            return 1.0;
        }

        let mut cumulative = 0.0;

        for i in 0..self.centroids.len() {
            let centroid = &self.centroids[i];

            if value < centroid.mean {
                if i == 0 {
                    // Before first centroid
                    let fraction = (value - self.min) / (centroid.mean - self.min);
                    return (cumulative + centroid.weight * fraction / 2.0) / self.total_weight;
                } else {
                    // Between centroids
                    let prev = &self.centroids[i - 1];
                    let fraction = (value - prev.mean) / (centroid.mean - prev.mean);
                    let weight_so_far = cumulative - prev.weight / 2.0;
                    let weight_span = prev.weight / 2.0 + centroid.weight / 2.0;
                    return (weight_so_far + weight_span * fraction) / self.total_weight;
                }
            }

            cumulative += centroid.weight;
        }

        1.0
    }

    /// Returns the trimmed mean between two quantiles
    ///
    /// # Arguments
    ///
    /// * `low` - Lower quantile bound (e.g., 0.1)
    /// * `high` - Upper quantile bound (e.g., 0.9)
    ///
    /// # Returns
    ///
    /// Mean of values between the two quantiles
    pub fn trimmed_mean(&mut self, low: f64, high: f64) -> f64 {
        let low_val = self.quantile(low);
        let high_val = self.quantile(high);

        self.flush();

        if self.centroids.is_empty() {
            return 0.0;
        }

        let mut sum = 0.0;
        let mut weight = 0.0;

        for centroid in &self.centroids {
            if centroid.mean >= low_val && centroid.mean <= high_val {
                sum += centroid.mean * centroid.weight;
                weight += centroid.weight;
            }
        }

        if weight > 0.0 {
            sum / weight
        } else {
            (low_val + high_val) / 2.0
        }
    }

    /// Flushes buffer and compresses centroids
    fn flush(&mut self) {
        if !self.buffer.is_empty() {
            self.compress();
        }
    }

    /// Compresses the digest by merging centroids
    fn compress(&mut self) {
        // Add buffer values as unit-weight centroids
        for &value in &self.buffer {
            self.centroids.push(Centroid::new(value, 1.0));
            self.total_weight += 1.0;
        }
        self.buffer.clear();

        if self.centroids.len() <= 1 {
            return;
        }

        // Sort centroids by mean
        self.centroids
            .sort_by(|a, b| a.mean.partial_cmp(&b.mean).unwrap_or(Ordering::Equal));

        // Merge using scale function (more aggressive at tails)
        let mut merged = Vec::with_capacity(self.centroids.len());
        let mut current = self.centroids[0].clone();
        let mut cumulative = 0.0;

        for centroid in self.centroids.iter().skip(1) {
            let projected_weight = current.weight + centroid.weight;

            // Scale function: smaller centroids at the tails
            let q = (cumulative + projected_weight / 2.0) / self.total_weight;
            let k = self.compression * q * (1.0 - q) * 4.0; // Scale limit

            if projected_weight <= k.max(1.0) {
                // Merge into current centroid
                current.add(centroid.mean, centroid.weight);
            } else {
                // Start new centroid
                cumulative += current.weight;
                merged.push(current);
                current = centroid.clone();
            }
        }

        merged.push(current);
        self.centroids = merged;
    }

    /// Adds a new centroid, maintaining sort order
    fn add_centroid(&mut self, mean: f64, weight: f64) {
        let centroid = Centroid::new(mean, weight);

        // Binary search for insertion point
        let idx = self
            .centroids
            .binary_search_by(|c| c.mean.partial_cmp(&mean).unwrap_or(Ordering::Equal))
            .unwrap_or_else(|i| i);

        self.centroids.insert(idx, centroid);
        self.total_weight += weight;
    }

    /// Serializes the T-Digest to bytes
    pub fn to_bytes(&mut self) -> Vec<u8> {
        self.flush();

        let mut bytes = Vec::new();

        // Header: compression (8), count (8), min (8), max (8), num_centroids (4)
        bytes.extend_from_slice(&self.compression.to_le_bytes());
        bytes.extend_from_slice(&self.total_weight.to_le_bytes());
        bytes.extend_from_slice(&self.min.to_le_bytes());
        bytes.extend_from_slice(&self.max.to_le_bytes());
        bytes.extend_from_slice(&(self.centroids.len() as u32).to_le_bytes());

        // Centroids: mean (8) + weight (8) each
        for centroid in &self.centroids {
            bytes.extend_from_slice(&centroid.mean.to_le_bytes());
            bytes.extend_from_slice(&centroid.weight.to_le_bytes());
        }

        bytes
    }

    /// Deserializes a T-Digest from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SketchError> {
        if bytes.len() < 36 {
            return Err(SketchError::DeserializationError(
                "Insufficient data for T-Digest header".to_string(),
            ));
        }

        let compression = f64::from_le_bytes(bytes[0..8].try_into().unwrap());
        let total_weight = f64::from_le_bytes(bytes[8..16].try_into().unwrap());
        let min = f64::from_le_bytes(bytes[16..24].try_into().unwrap());
        let max = f64::from_le_bytes(bytes[24..32].try_into().unwrap());
        let num_centroids = u32::from_le_bytes(bytes[32..36].try_into().unwrap()) as usize;

        let expected_len = 36 + num_centroids * 16;
        if bytes.len() < expected_len {
            return Err(SketchError::DeserializationError(format!(
                "Expected {} bytes, got {}",
                expected_len,
                bytes.len()
            )));
        }

        let mut centroids = Vec::with_capacity(num_centroids);
        for i in 0..num_centroids {
            let offset = 36 + i * 16;
            let mean = f64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
            let weight = f64::from_le_bytes(bytes[offset + 8..offset + 16].try_into().unwrap());
            centroids.push(Centroid::new(mean, weight));
        }

        Ok(TDigest {
            compression,
            centroids,
            buffer: Vec::new(),
            total_weight,
            min,
            max,
            buffer_size: (compression as usize) * Self::BUFFER_FACTOR,
        })
    }
}

impl Default for TDigest {
    fn default() -> Self {
        Self::default_compression()
    }
}

impl Sketch for TDigest {
    type Item = f64;

    fn update(&mut self, item: &Self::Item) {
        self.update(*item);
    }

    /// Returns the median as the primary estimate
    fn estimate(&self) -> f64 {
        let mut td = self.clone();
        td.quantile(0.5)
    }

    fn is_empty(&self) -> bool {
        self.centroids.is_empty() && self.buffer.is_empty()
    }

    fn serialize(&self) -> Vec<u8> {
        let mut td = self.clone();
        td.to_bytes()
    }

    fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        Self::from_bytes(bytes)
    }
}

impl Mergeable for TDigest {
    /// Merges another T-Digest into this one
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::quantiles::TDigest;
    /// use sketch_oxide::Mergeable;
    ///
    /// let mut td1 = TDigest::new(100.0);
    /// let mut td2 = TDigest::new(100.0);
    ///
    /// for i in 0..500 {
    ///     td1.update(i as f64);
    /// }
    /// for i in 500..1000 {
    ///     td2.update(i as f64);
    /// }
    ///
    /// td1.merge(&td2).unwrap();
    /// ```
    fn merge(&mut self, other: &Self) -> Result<(), SketchError> {
        // Flush both digests
        let mut other_clone = other.clone();
        self.flush();
        other_clone.flush();

        // Update min/max
        self.min = self.min.min(other_clone.min);
        self.max = self.max.max(other_clone.max);

        // Add all centroids from other
        for centroid in other_clone.centroids {
            self.centroids.push(centroid);
        }
        self.total_weight += other_clone.total_weight;

        // Compress
        self.compress();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tdigest() {
        let td = TDigest::new(100.0);
        assert!(td.is_empty());
        assert_eq!(td.compression(), 100.0);
    }

    #[test]
    fn test_update() {
        let mut td = TDigest::new(100.0);
        td.update(42.0);
        assert!(!td.is_empty());
    }

    #[test]
    fn test_quantile_single() {
        let mut td = TDigest::new(100.0);
        td.update(100.0);
        assert!((td.quantile(0.5) - 100.0).abs() < 1.0);
    }

    #[test]
    fn test_quantile_uniform() {
        let mut td = TDigest::new(100.0);
        for i in 0..1000 {
            td.update(i as f64);
        }

        let p50 = td.quantile(0.5);
        assert!(
            (p50 - 500.0).abs() < 50.0,
            "Median {} too far from 500",
            p50
        );

        let p90 = td.quantile(0.9);
        assert!((p90 - 900.0).abs() < 50.0, "P90 {} too far from 900", p90);
    }

    #[test]
    fn test_min_max() {
        let mut td = TDigest::new(100.0);
        td.update(10.0);
        td.update(100.0);
        td.update(50.0);

        assert_eq!(td.min(), 10.0);
        assert_eq!(td.max(), 100.0);
    }

    #[test]
    fn test_merge() {
        let mut td1 = TDigest::new(100.0);
        let mut td2 = TDigest::new(100.0);

        for i in 0..500 {
            td1.update(i as f64);
        }
        for i in 500..1000 {
            td2.update(i as f64);
        }

        td1.merge(&td2).unwrap();

        let median = td1.quantile(0.5);
        assert!(
            (median - 500.0).abs() < 100.0,
            "Merged median {} unexpected",
            median
        );
    }

    #[test]
    fn test_serialization() {
        let mut td = TDigest::new(100.0);
        for i in 0..1000 {
            td.update(i as f64);
        }

        let bytes = td.to_bytes();
        let mut restored = TDigest::from_bytes(&bytes).unwrap();

        assert_eq!(td.compression(), restored.compression());
        assert!((td.quantile(0.5) - restored.quantile(0.5)).abs() < 1.0);
    }
}
