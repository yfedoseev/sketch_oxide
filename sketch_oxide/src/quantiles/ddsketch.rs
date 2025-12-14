//! DDSketch: Quantile estimation with relative error guarantees (VLDB 2019)
//!
//! Production-proven by Datadog, ClickHouse 24.1+, TimescaleDB.
//!
//! # Overview
//!
//! DDSketch provides quantile estimation with **relative error guarantees**, which means
//! the error is proportional to the value being estimated (e.g., 1% of the value).
//! This is more useful than absolute error for metrics spanning orders of magnitude.
//!
//! # Key Features
//!
//! - **Relative accuracy**: Error ≤ α × value (e.g., 1% error)
//! - **Fully mergeable**: Constant time merge operation
//! - **Fast updates**: O(1) insertion
//! - **Fast queries**: O(k) quantile retrieval where k = number of bins
//! - **Handles all values**: Positive, negative, and zero
//!
//! # Use Cases
//!
//! - Latency monitoring (spans milliseconds to seconds)
//! - Request size tracking (bytes to gigabytes)
//! - Financial metrics (cents to millions)
//! - Any metric spanning multiple orders of magnitude
//!
//! # Example
//!
//! ```
//! use sketch_oxide::quantiles::DDSketch;
//! use sketch_oxide::common::Sketch;
//!
//! // Create sketch with 1% relative accuracy
//! let mut dd = DDSketch::new(0.01).unwrap();
//!
//! // Add latency measurements (milliseconds)
//! for i in 1..=1000 {
//!     dd.update(&(i as f64));
//! }
//!
//! // Query quantiles
//! println!("p50: {}", dd.quantile(0.50).unwrap());
//! println!("p99: {}", dd.quantile(0.99).unwrap());
//! ```
//!
//! # References
//!
//! - Paper: "DDSketch: A Fast and Fully-Mergeable Quantile Sketch with Relative-Error Guarantees" (VLDB 2019)
//! - Datadog blog: https://www.datadoghq.com/blog/engineering/computing-accurate-percentiles-with-ddsketch/

use crate::common::{Mergeable, Sketch, SketchError};
use std::collections::HashMap;

/// Store for binned values
///
/// Maintains histogram bins with counts, along with min/max tracking.
#[derive(Debug, Clone)]
struct Store {
    bins: HashMap<i32, u64>, // bin_index -> count
    count: u64,              // total count
    min: f64,                // minimum value
    max: f64,                // maximum value
}

impl Store {
    fn new() -> Self {
        Self {
            bins: HashMap::new(),
            count: 0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        }
    }

    fn add(&mut self, index: i32) {
        *self.bins.entry(index).or_insert(0) += 1;
        self.count += 1;
    }

    fn merge(&mut self, other: &Store) {
        for (&index, &count) in &other.bins {
            *self.bins.entry(index).or_insert(0) += count;
        }
        self.count += other.count;
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }
}

/// DDSketch for quantile estimation with relative error guarantees
///
/// # Algorithm
///
/// DDSketch uses logarithmic binning to achieve relative error guarantees:
///
/// 1. **Mapping**: Values are mapped to bins using: `k = ceil(log_gamma(value))`
/// 2. **Storage**: Each bin stores a count of values
/// 3. **Quantile Query**: Binary search through sorted bins to find target rank
/// 4. **Inverse Mapping**: Bin index mapped back to value: `value ≈ gamma^k`
///
/// where `gamma = (1 + α) / (1 - α)` and α is the relative accuracy parameter.
///
/// # Complexity
///
/// - **Update**: O(1) average, O(log n) worst case (hash map)
/// - **Quantile query**: O(k) where k is number of distinct bins
/// - **Merge**: O(k₁ + k₂) where k₁, k₂ are bin counts
/// - **Space**: O(k) where k ≈ log₁₊α(max/min)
///
/// # Accuracy
///
/// For a quantile q with true value v, the estimated value v' satisfies:
///
/// ```text
/// |v' - v| ≤ α × v
/// ```
///
/// This relative error guarantee holds for all quantiles.
#[derive(Debug, Clone)]
pub struct DDSketch {
    alpha: f64,    // Relative accuracy parameter (e.g., 0.01 = 1%)
    gamma: f64,    // Bin width: (1 + alpha) / (1 - alpha)
    gamma_ln: f64, // ln(gamma) for efficiency
    offset: f64,   // Bias for log mapping

    store_positive: Store, // Positive values
    store_negative: Store, // Negative values (stored as absolute values)
    zero_count: u64,       // Count of zero values
}

impl DDSketch {
    /// Creates a new DDSketch with specified relative accuracy
    ///
    /// # Arguments
    ///
    /// * `relative_accuracy` - Relative error bound (e.g., 0.01 for 1% error)
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if alpha ≤ 0 or alpha ≥ 1
    ///
    /// # Example
    ///
    /// ```
    /// use sketch_oxide::quantiles::DDSketch;
    ///
    /// let dd = DDSketch::new(0.01).unwrap();  // 1% relative error
    /// let dd2 = DDSketch::new(0.001).unwrap(); // 0.1% relative error (more accurate)
    /// ```
    ///
    /// # Accuracy vs Space Trade-off
    ///
    /// - α = 0.1 (10%): ~50 bins for 6 orders of magnitude
    /// - α = 0.01 (1%): ~500 bins for 6 orders of magnitude
    /// - α = 0.001 (0.1%): ~5000 bins for 6 orders of magnitude
    pub fn new(relative_accuracy: f64) -> Result<Self, SketchError> {
        if relative_accuracy <= 0.0 || relative_accuracy >= 1.0 {
            return Err(SketchError::InvalidParameter {
                param: "relative_accuracy".to_string(),
                value: relative_accuracy.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }

        let gamma = (1.0 + relative_accuracy) / (1.0 - relative_accuracy);
        let gamma_ln = gamma.ln();
        let offset = 0.0; // Can be adjusted for numerical stability

        Ok(Self {
            alpha: relative_accuracy,
            gamma,
            gamma_ln,
            offset,
            store_positive: Store::new(),
            store_negative: Store::new(),
            zero_count: 0,
        })
    }

    /// Updates the sketch with a new value
    ///
    /// # Time Complexity
    ///
    /// O(1) average case (hash map insertion)
    ///
    /// # Example
    ///
    /// ```
    /// use sketch_oxide::quantiles::DDSketch;
    ///
    /// let mut dd = DDSketch::new(0.01).unwrap();
    /// dd.add(42.0);
    /// dd.add(-10.5);
    /// dd.add(0.0);
    /// ```
    pub fn add(&mut self, value: f64) {
        if value > 0.0 {
            let index = self.key(value);
            self.store_positive.add(index);
            self.store_positive.min = self.store_positive.min.min(value);
            self.store_positive.max = self.store_positive.max.max(value);
        } else if value < 0.0 {
            let index = self.key(-value);
            self.store_negative.add(index);
            self.store_negative.min = self.store_negative.min.min(-value);
            self.store_negative.max = self.store_negative.max.max(-value);
        } else {
            self.zero_count += 1;
        }
    }

    /// Maps a value to its bin index using logarithmic binning
    ///
    /// Formula: k = ceil(log_gamma(value)) + offset
    ///
    /// This ensures that values within a factor of gamma are mapped to the same bin,
    /// which provides the relative error guarantee.
    fn key(&self, value: f64) -> i32 {
        // DDSketch mapping: k = ceil(log_gamma(value)) + offset
        ((value.ln() / self.gamma_ln + self.offset).ceil()) as i32
    }

    /// Maps a bin index back to a representative value
    ///
    /// Formula: value = 2 * gamma^(k-1) / (1 + gamma^(-1))
    ///
    /// This returns the geometric midpoint of the bin's range [gamma^(k-1), gamma^k],
    /// which provides better accuracy than using just the upper bound.
    fn value(&self, index: i32) -> f64 {
        // Geometric midpoint: 2 * gamma^(k-1) / (1 + 1/gamma)
        // Simplified: 2 * gamma^(k-1) * gamma / (gamma + 1)
        // Which equals: 2 * gamma^k / (gamma + 1)
        let exponent = (index as f64) - self.offset;
        let gamma_k = self.gamma.powf(exponent);
        2.0 * gamma_k / (self.gamma + 1.0)
    }

    /// Returns the total count of values
    ///
    /// # Example
    ///
    /// ```
    /// use sketch_oxide::quantiles::DDSketch;
    ///
    /// let mut dd = DDSketch::new(0.01).unwrap();
    /// dd.add(1.0);
    /// dd.add(2.0);
    /// assert_eq!(dd.count(), 2);
    /// ```
    pub fn count(&self) -> u64 {
        self.store_positive.count + self.zero_count + self.store_negative.count
    }

    /// Returns the quantile for a given rank (0.0 to 1.0)
    ///
    /// # Arguments
    ///
    /// * `q` - Quantile rank in [0.0, 1.0], where 0.5 = median, 0.99 = p99, etc.
    ///
    /// # Returns
    ///
    /// - `Some(value)` - The estimated quantile value
    /// - `None` - If q is out of range or sketch is empty
    ///
    /// # Time Complexity
    ///
    /// O(k) where k is the number of distinct bins
    ///
    /// # Example
    ///
    /// ```
    /// use sketch_oxide::quantiles::DDSketch;
    ///
    /// let mut dd = DDSketch::new(0.01).unwrap();
    /// for i in 1..=100 {
    ///     dd.add(i as f64);
    /// }
    ///
    /// let median = dd.quantile(0.5).unwrap();   // p50
    /// let p99 = dd.quantile(0.99).unwrap();     // p99
    /// ```
    pub fn quantile(&self, q: f64) -> Option<f64> {
        if !(0.0..=1.0).contains(&q) {
            return None;
        }

        let count = self.count();
        if count == 0 {
            return None;
        }

        // Calculate target rank (1-indexed)
        // Special case: q=0.0 should give first element (rank 1)
        let rank = if q == 0.0 {
            1
        } else {
            (q * count as f64).ceil() as u64
        };

        // Search through stores: negative (reverse) → zeros → positive
        let mut accumulated = 0u64;

        // Negative values (in reverse order, largest negative first)
        if rank <= self.store_negative.count {
            let mut indices: Vec<_> = self.store_negative.bins.keys().copied().collect();
            indices.sort_by(|a, b| b.cmp(a)); // Reverse for negatives

            for index in indices {
                accumulated += self.store_negative.bins[&index];
                if accumulated >= rank {
                    return Some(-self.value(index));
                }
            }
        }
        accumulated += self.store_negative.count;

        // Zeros
        if rank <= accumulated + self.zero_count {
            return Some(0.0);
        }
        accumulated += self.zero_count;

        // Positive values
        let mut indices: Vec<_> = self.store_positive.bins.keys().copied().collect();
        indices.sort();

        for index in indices {
            accumulated += self.store_positive.bins[&index];
            if accumulated >= rank {
                return Some(self.value(index));
            }
        }

        None
    }

    /// Returns minimum value seen
    ///
    /// # Example
    ///
    /// ```
    /// use sketch_oxide::quantiles::DDSketch;
    ///
    /// let mut dd = DDSketch::new(0.01).unwrap();
    /// dd.add(10.0);
    /// dd.add(5.0);
    /// dd.add(20.0);
    ///
    /// assert!(dd.min().unwrap() < 5.5);
    /// ```
    pub fn min(&self) -> Option<f64> {
        if self.count() == 0 {
            return None;
        }

        let mut result = f64::INFINITY;

        if self.store_negative.count > 0 {
            result = result.min(-self.store_negative.max);
        }

        if self.zero_count > 0 {
            result = result.min(0.0);
        }

        if self.store_positive.count > 0 {
            result = result.min(self.store_positive.min);
        }

        Some(result)
    }

    /// Returns maximum value seen
    ///
    /// # Example
    ///
    /// ```
    /// use sketch_oxide::quantiles::DDSketch;
    ///
    /// let mut dd = DDSketch::new(0.01).unwrap();
    /// dd.add(10.0);
    /// dd.add(5.0);
    /// dd.add(20.0);
    ///
    /// assert!(dd.max().unwrap() > 19.5);
    /// ```
    pub fn max(&self) -> Option<f64> {
        if self.count() == 0 {
            return None;
        }

        let mut result = f64::NEG_INFINITY;

        if self.store_positive.count > 0 {
            result = result.max(self.store_positive.max);
        }

        if self.zero_count > 0 {
            result = result.max(0.0);
        }

        if self.store_negative.count > 0 {
            result = result.max(-self.store_negative.min);
        }

        Some(result)
    }

    /// Returns the relative accuracy parameter (alpha)
    ///
    /// This is the same value passed to `DDSketch::new()`.
    ///
    /// # Example
    ///
    /// ```
    /// use sketch_oxide::quantiles::DDSketch;
    ///
    /// let dd = DDSketch::new(0.01).unwrap();
    /// assert!((dd.alpha() - 0.01).abs() < 1e-10);
    /// ```
    pub fn alpha(&self) -> f64 {
        self.alpha
    }
}

impl Sketch for DDSketch {
    type Item = f64;

    fn update(&mut self, item: &Self::Item) {
        self.add(*item);
    }

    fn estimate(&self) -> f64 {
        self.count() as f64
    }

    fn is_empty(&self) -> bool {
        self.count() == 0
    }

    fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Header: alpha, gamma, gamma_ln, offset, zero_count
        bytes.extend_from_slice(&self.alpha.to_le_bytes());
        bytes.extend_from_slice(&self.gamma.to_le_bytes());
        bytes.extend_from_slice(&self.gamma_ln.to_le_bytes());
        bytes.extend_from_slice(&self.offset.to_le_bytes());
        bytes.extend_from_slice(&self.zero_count.to_le_bytes());

        // Positive store
        bytes.extend_from_slice(&self.store_positive.count.to_le_bytes());
        bytes.extend_from_slice(&self.store_positive.min.to_le_bytes());
        bytes.extend_from_slice(&self.store_positive.max.to_le_bytes());
        bytes.extend_from_slice(&(self.store_positive.bins.len() as u64).to_le_bytes());
        for (&index, &count) in &self.store_positive.bins {
            bytes.extend_from_slice(&index.to_le_bytes());
            bytes.extend_from_slice(&count.to_le_bytes());
        }

        // Negative store
        bytes.extend_from_slice(&self.store_negative.count.to_le_bytes());
        bytes.extend_from_slice(&self.store_negative.min.to_le_bytes());
        bytes.extend_from_slice(&self.store_negative.max.to_le_bytes());
        bytes.extend_from_slice(&(self.store_negative.bins.len() as u64).to_le_bytes());
        for (&index, &count) in &self.store_negative.bins {
            bytes.extend_from_slice(&index.to_le_bytes());
            bytes.extend_from_slice(&count.to_le_bytes());
        }

        bytes
    }

    fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        if bytes.len() < 104 {
            return Err(SketchError::DeserializationError(
                "Insufficient data for DDSketch header".to_string(),
            ));
        }

        let alpha = f64::from_le_bytes(bytes[0..8].try_into().unwrap());
        let gamma = f64::from_le_bytes(bytes[8..16].try_into().unwrap());
        let gamma_ln = f64::from_le_bytes(bytes[16..24].try_into().unwrap());
        let offset = f64::from_le_bytes(bytes[24..32].try_into().unwrap());
        let zero_count = u64::from_le_bytes(bytes[32..40].try_into().unwrap());

        let mut pos = 40;

        // Read positive store
        let pos_count = u64::from_le_bytes(bytes[pos..pos + 8].try_into().unwrap());
        pos += 8;
        let pos_min = f64::from_le_bytes(bytes[pos..pos + 8].try_into().unwrap());
        pos += 8;
        let pos_max = f64::from_le_bytes(bytes[pos..pos + 8].try_into().unwrap());
        pos += 8;
        let pos_bins_len = u64::from_le_bytes(bytes[pos..pos + 8].try_into().unwrap()) as usize;
        pos += 8;

        let mut pos_bins = HashMap::new();
        for _ in 0..pos_bins_len {
            let index = i32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap());
            pos += 4;
            let count = u64::from_le_bytes(bytes[pos..pos + 8].try_into().unwrap());
            pos += 8;
            pos_bins.insert(index, count);
        }

        // Read negative store
        let neg_count = u64::from_le_bytes(bytes[pos..pos + 8].try_into().unwrap());
        pos += 8;
        let neg_min = f64::from_le_bytes(bytes[pos..pos + 8].try_into().unwrap());
        pos += 8;
        let neg_max = f64::from_le_bytes(bytes[pos..pos + 8].try_into().unwrap());
        pos += 8;
        let neg_bins_len = u64::from_le_bytes(bytes[pos..pos + 8].try_into().unwrap()) as usize;
        pos += 8;

        let mut neg_bins = HashMap::new();
        for _ in 0..neg_bins_len {
            let index = i32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap());
            pos += 4;
            let count = u64::from_le_bytes(bytes[pos..pos + 8].try_into().unwrap());
            pos += 8;
            neg_bins.insert(index, count);
        }

        Ok(DDSketch {
            alpha,
            gamma,
            gamma_ln,
            offset,
            store_positive: Store {
                bins: pos_bins,
                count: pos_count,
                min: pos_min,
                max: pos_max,
            },
            store_negative: Store {
                bins: neg_bins,
                count: neg_count,
                min: neg_min,
                max: neg_max,
            },
            zero_count,
        })
    }
}

impl Mergeable for DDSketch {
    /// Merges another DDSketch into this one
    ///
    /// # Errors
    ///
    /// Returns `IncompatibleSketches` if the accuracy parameters differ
    ///
    /// # Time Complexity
    ///
    /// O(k₁ + k₂) where k₁, k₂ are the number of bins in each sketch
    ///
    /// # Example
    ///
    /// ```
    /// use sketch_oxide::quantiles::DDSketch;
    /// use sketch_oxide::common::Mergeable;
    ///
    /// let mut dd1 = DDSketch::new(0.01).unwrap();
    /// let mut dd2 = DDSketch::new(0.01).unwrap();
    ///
    /// for i in 1..=500 {
    ///     dd1.add(i as f64);
    /// }
    ///
    /// for i in 501..=1000 {
    ///     dd2.add(i as f64);
    /// }
    ///
    /// dd1.merge(&dd2).unwrap();
    /// assert_eq!(dd1.count(), 1000);
    /// ```
    fn merge(&mut self, other: &Self) -> Result<(), SketchError> {
        // Check compatibility
        if (self.alpha - other.alpha).abs() > 1e-10 {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "Cannot merge sketches with different alpha: {} vs {}",
                    self.alpha, other.alpha
                ),
            });
        }

        self.store_positive.merge(&other.store_positive);
        self.store_negative.merge(&other.store_negative);
        self.zero_count += other.zero_count;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_value_inverse() {
        let dd = DDSketch::new(0.01).unwrap();

        // Test that key and value are approximate inverses
        for i in 1..=20 {
            let original = 2.0_f64.powi(i);
            let index = dd.key(original);
            let recovered = dd.value(index);

            // Should be within relative error
            let relative_error = (recovered - original).abs() / original;
            assert!(
                relative_error <= 0.02,
                "key/value not inverse: {} -> {} -> {}, error: {}",
                original,
                index,
                recovered,
                relative_error
            );
        }
    }

    #[test]
    fn test_gamma_calculation() {
        let dd = DDSketch::new(0.01).unwrap();
        let expected_gamma = 1.01 / 0.99; // (1 + 0.01) / (1 - 0.01)
        assert!((dd.gamma - expected_gamma).abs() < 1e-10);
    }
}
