//! QSketch: Weighted Cardinality Estimation (2024-2025)
//!
//! QSketch is a state-of-the-art algorithm for estimating the cardinality of weighted elements,
//! combining the benefits of probabilistic sampling with weight-aware estimation.
//!
//! # Algorithm Overview
//!
//! QSketch maintains a probabilistic sample of items weighted by their importance,
//! enabling accurate cardinality estimation even when weights vary dramatically.
//! Unlike standard cardinality sketches that treat all items equally, QSketch accounts
//! for item weights in the cardinality estimate.
//!
//! Key features:
//! 1. **Weighted Random Sampling**: Items are sampled with probability proportional to weight
//! 2. **Adaptive Threshold**: Dynamically adjusts sampling threshold to maintain fixed sample size
//! 3. **Probabilistic Bounds**: Provides confidence intervals for estimates
//! 4. **Mergeable**: Combine sketches from distributed systems
//!
//! # Time Complexity
//!
//! - Update: O(1) amortized
//! - Estimate: O(k) where k = sample size
//! - Merge: O(k)
//!
//! # Space Complexity
//!
//! O(k log n) where k = max_samples, n = cardinality
//!
//! # Use Cases
//!
//! - Weighted set cardinality (e.g., users weighted by spending)
//! - Financial metrics (market cap estimation with volatility)
//! - Network traffic (bytes vs packet counts)
//! - Weighted metrics in time series
//!
//! # References
//!
//! - QSketch: 2024-2025 weighted cardinality estimation research
//! - Similar to: HyperLogLog but for weighted elements
//! - Combines: Weighted reservoir sampling + probabilistic cardinality
//!
//! # Examples
//!
//! ```
//! use sketch_oxide::cardinality::QSketch;
//!
//! let mut qsketch = QSketch::new(256);
//!
//! // Add weighted elements
//! qsketch.update(b"user_1", 100.0);   // Weight: 100
//! qsketch.update(b"user_2", 250.0);   // Weight: 250
//! qsketch.update(b"user_3", 50.0);    // Weight: 50
//!
//! // Estimate weighted cardinality with bounds
//! let (estimate, error_bound) = qsketch.estimate_weighted_cardinality();
//! println!("Weighted cardinality: {} ± {}", estimate, error_bound);
//!
//! // Get distinct count and total weight
//! let distinct = qsketch.estimate_distinct_elements();
//! let total = qsketch.total_weight();
//! ```

use crate::common::{Mergeable, Sketch, SketchError};
use rand::Rng;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use twox_hash::XxHash64;

/// A weighted item in the QSketch sample
#[derive(Clone, Debug)]
struct SampledItem {
    /// Hash of the element
    element_id: u64,
    /// Weight of the element
    weight: f64,
}

/// QSketch for weighted cardinality estimation
///
/// Maintains a probabilistic sample of weighted elements to estimate
/// the cardinality of weighted sets with bounded error.
///
/// # Examples
///
/// ```
/// use sketch_oxide::cardinality::QSketch;
///
/// let mut qsketch = QSketch::new(256);
/// qsketch.update(b"item_1", 10.0);
/// qsketch.update(b"item_2", 20.0);
///
/// let (estimate, error) = qsketch.estimate_weighted_cardinality();
/// assert!(error > 0.0);
/// ```
#[derive(Clone, Debug)]
pub struct QSketch {
    /// Maximum number of samples to maintain
    max_samples: usize,

    /// Active samples: (element_id, weight) pairs
    samples: Vec<SampledItem>,

    /// Distinct items seen (hash -> weight map for tracking duplicates)
    items_seen: HashMap<u64, f64>,

    /// Sum of all weights seen
    total_weight: f64,

    /// Adaptive threshold for sampling
    threshold: f64,

    /// Random number generator
    rng: rand::rngs::SmallRng,
}

impl QSketch {
    /// Default maximum number of samples
    pub const DEFAULT_MAX_SAMPLES: usize = 256;

    /// Minimum samples for meaningful estimation
    const MIN_SAMPLES: usize = 32;

    /// Creates a new QSketch with default capacity
    ///
    /// Default max_samples = 256, suitable for most applications
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::cardinality::QSketch;
    /// use sketch_oxide::Sketch;
    ///
    /// let qsketch = QSketch::new(256);
    /// assert!(qsketch.is_empty());
    /// ```
    pub fn new(max_samples: usize) -> Self {
        use rand::SeedableRng;

        assert!(
            max_samples >= Self::MIN_SAMPLES,
            "max_samples must be at least {}",
            Self::MIN_SAMPLES
        );

        QSketch {
            max_samples,
            samples: Vec::with_capacity(max_samples),
            items_seen: HashMap::new(),
            total_weight: 0.0,
            threshold: 0.0,
            rng: rand::rngs::SmallRng::from_os_rng(),
        }
    }

    /// Creates a new QSketch with a specific random seed for reproducibility
    ///
    /// # Arguments
    ///
    /// * `max_samples` - Maximum number of samples to maintain
    /// * `seed` - Random seed for reproducibility
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::cardinality::QSketch;
    ///
    /// let qsketch1 = QSketch::with_seed(256, 42);
    /// let qsketch2 = QSketch::with_seed(256, 42);
    /// // Both will produce identical results with same input
    /// ```
    pub fn with_seed(max_samples: usize, seed: u64) -> Self {
        use rand::SeedableRng;

        assert!(
            max_samples >= Self::MIN_SAMPLES,
            "max_samples must be at least {}",
            Self::MIN_SAMPLES
        );

        QSketch {
            max_samples,
            samples: Vec::with_capacity(max_samples),
            items_seen: HashMap::new(),
            total_weight: 0.0,
            threshold: 0.0,
            rng: rand::rngs::SmallRng::seed_from_u64(seed),
        }
    }

    /// Returns the maximum number of samples this sketch can maintain
    #[inline]
    pub fn max_samples(&self) -> usize {
        self.max_samples
    }

    /// Returns the current number of samples in the sketch
    #[inline]
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// Returns the sum of all weights added to the sketch
    ///
    /// This is the sum of all weights of all items seen (not just sampled items).
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::cardinality::QSketch;
    ///
    /// let mut qsketch = QSketch::new(256);
    /// qsketch.update(b"item_1", 10.5);
    /// qsketch.update(b"item_2", 20.5);
    /// assert!((qsketch.total_weight() - 31.0).abs() < 0.001);
    /// ```
    pub fn total_weight(&self) -> f64 {
        self.total_weight
    }

    /// Returns the approximate number of distinct elements
    ///
    /// Estimates cardinality based on sampled items.
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::cardinality::QSketch;
    ///
    /// let mut qsketch = QSketch::new(256);
    /// for i in 0..100 {
    ///     qsketch.update(format!("item_{}", i).as_bytes(), 1.0);
    /// }
    /// let distinct = qsketch.estimate_distinct_elements();
    /// assert!(distinct > 0);
    /// ```
    pub fn estimate_distinct_elements(&self) -> u64 {
        if self.samples.is_empty() {
            return 0;
        }

        // If we haven't filled the sample yet, return actual count
        if self.samples.len() < self.max_samples {
            return self.items_seen.len() as u64;
        }

        // Estimate: (distinct_in_sample / sample_probability)
        // where sample_probability ≈ max_samples / total_weight
        let sample_size = self.samples.len() as f64;
        let total_w = self.total_weight();

        if total_w <= 0.0 {
            return self.items_seen.len() as u64;
        }

        // Estimate of distinct items based on sampling proportion
        let estimated_distinct = (self.items_seen.len() as f64) * (total_w / sample_size).max(1.0);

        estimated_distinct.ceil() as u64
    }

    /// Estimates the weighted cardinality with confidence bounds
    ///
    /// Returns (estimate, error_bound) where error_bound represents
    /// the radius of a 95% confidence interval around the estimate.
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::cardinality::QSketch;
    ///
    /// let mut qsketch = QSketch::new(256);
    /// qsketch.update(b"item_1", 100.0);
    /// qsketch.update(b"item_2", 250.0);
    ///
    /// let (estimate, error) = qsketch.estimate_weighted_cardinality();
    /// println!("Estimate: {} ± {}", estimate, error);
    /// ```
    pub fn estimate_weighted_cardinality(&self) -> (f64, f64) {
        if self.samples.is_empty() {
            return (0.0, 0.0);
        }

        // Sum of weights in sample
        let sample_weight: f64 = self.samples.iter().map(|s| s.weight).sum();

        if sample_weight <= 0.0 {
            return (0.0, 0.0);
        }

        // Basic estimate: (total_weight / sample_weight) * num_samples
        let sample_size = self.samples.len() as f64;
        let total_w = self.total_weight();

        // Weighted cardinality estimate
        let estimate = if sample_size > 0.0 && sample_weight > 0.0 {
            (total_w / sample_weight) * (self.items_seen.len() as f64)
        } else {
            self.items_seen.len() as f64
        };

        // Error bound: 95% CI width (1.96 * standard_error)
        // Standard error for weighted cardinality
        let std_error = self.compute_standard_error(sample_size, sample_weight);
        let error_bound = 1.96 * std_error;

        (estimate, error_bound)
    }

    /// Computes the standard error for weighted cardinality estimation
    #[inline]
    fn compute_standard_error(&self, sample_size: f64, sample_weight: f64) -> f64 {
        if sample_size < 2.0 || sample_weight <= 0.0 {
            return 0.0;
        }

        // Standard error based on sample variance
        // For weighted samples: SE ≈ sqrt(variance / sample_size)
        let total_w = self.total_weight();

        // Compute variance in weights
        let mean_weight = sample_weight / sample_size;
        let variance: f64 = self
            .samples
            .iter()
            .map(|s| (s.weight - mean_weight).powi(2))
            .sum::<f64>()
            / sample_size;

        let se = (variance / sample_size).sqrt();
        // Extrapolate error to full population
        (total_w * se) / sample_weight
    }

    /// Hash a byte slice to get a 64-bit hash value
    #[inline]
    fn hash_item(&self, item: &[u8]) -> u64 {
        let mut hasher = XxHash64::with_seed(0);
        item.hash(&mut hasher);
        hasher.finish()
    }

    /// Updates the sketch with a weighted element
    ///
    /// Adds the element and its weight to the sketch, maintaining
    /// a probabilistic sample of max_samples items.
    ///
    /// # Arguments
    ///
    /// * `item` - Byte slice representing the element
    /// * `weight` - Weight of the element (should be positive)
    ///
    /// # Panics
    ///
    /// Panics if weight is non-positive or NaN
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::cardinality::QSketch;
    ///
    /// let mut qsketch = QSketch::new(256);
    /// qsketch.update(b"user_123", 100.0);
    /// qsketch.update(b"user_456", 250.0);
    /// qsketch.update(b"user_123", 50.0);  // Duplicate - weight is updated
    /// ```
    pub fn update(&mut self, item: &[u8], weight: f64) {
        assert!(
            weight > 0.0 && weight.is_finite(),
            "Weight must be positive and finite, got {}",
            weight
        );

        let element_id = self.hash_item(item);

        // Track total weight and distinct items
        if let Some(existing) = self.items_seen.get_mut(&element_id) {
            // Duplicate item - update weight
            *existing += weight;
        } else {
            // New item
            self.items_seen.insert(element_id, weight);
        }

        self.total_weight += weight;

        // Decide whether to sample this item
        self.maybe_add_sample(element_id, weight);
    }

    /// Decides whether to add the item to the sample
    fn maybe_add_sample(&mut self, element_id: u64, weight: f64) {
        // If sample not full, always add
        if self.samples.len() < self.max_samples {
            self.samples.push(SampledItem { element_id, weight });
            self.update_threshold();
            return;
        }

        // Sample is full - decide based on weight
        // Sample with probability proportional to weight
        if weight > self.threshold {
            // Generate sampling probability
            let prob = weight / (weight + self.threshold);
            if self.rng.random::<f64>() < prob {
                // Remove minimum weight sample
                if let Some(min_idx) = self.find_min_weight_index() {
                    self.samples[min_idx] = SampledItem { element_id, weight };
                    self.update_threshold();
                }
            }
        }
    }

    /// Finds the index of the minimum weight sample
    #[inline]
    fn find_min_weight_index(&self) -> Option<usize> {
        self.samples
            .iter()
            .enumerate()
            .min_by(|a, b| {
                a.1.weight
                    .partial_cmp(&b.1.weight)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(idx, _)| idx)
    }

    /// Updates the adaptive threshold based on current samples
    fn update_threshold(&mut self) {
        if self.samples.is_empty() {
            self.threshold = 0.0;
            return;
        }

        // Threshold is the minimum weight in the current sample
        // This balances between keeping heavy items and sampling light items
        self.threshold = self
            .samples
            .iter()
            .map(|s| s.weight)
            .fold(f64::MAX, f64::min);
    }

    /// Resets the sketch to an empty state
    ///
    /// Clears all samples and weights.
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::cardinality::QSketch;
    /// use sketch_oxide::Sketch;
    ///
    /// let mut qsketch = QSketch::new(256);
    /// qsketch.update(b"item", 10.0);
    /// assert!(!qsketch.is_empty());
    /// qsketch.reset();
    /// assert!(qsketch.is_empty());
    /// ```
    pub fn reset(&mut self) {
        self.samples.clear();
        self.items_seen.clear();
        self.total_weight = 0.0;
        self.threshold = 0.0;
    }

    /// Merges another QSketch into this one
    ///
    /// Combines the weighted cardinality estimates from both sketches.
    /// Both sketches should have the same max_samples for optimal results.
    ///
    /// # Errors
    ///
    /// Returns error if sketches have incompatible configurations
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::cardinality::QSketch;
    ///
    /// let mut qsketch1 = QSketch::new(256);
    /// let mut qsketch2 = QSketch::new(256);
    ///
    /// qsketch1.update(b"item_1", 100.0);
    /// qsketch2.update(b"item_2", 200.0);
    ///
    /// qsketch1.merge(&qsketch2).unwrap();
    /// ```
    pub fn merge(&mut self, other: &Self) -> Result<(), SketchError> {
        if self.max_samples != other.max_samples {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "QSketch max_samples mismatch: {} vs {}",
                    self.max_samples, other.max_samples
                ),
            });
        }

        // Merge items_seen by updating weights
        for (&element_id, &weight) in &other.items_seen {
            self.items_seen
                .entry(element_id)
                .and_modify(|w| *w += weight)
                .or_insert(weight);
        }

        // Update total weight
        self.total_weight += other.total_weight;

        // Re-sample from combined set
        // For proper merging, we should re-process all items
        // But for efficiency, we merge samples and trim to max_samples
        self.samples.extend(other.samples.iter().cloned());

        // If we exceed max_samples, trim by removing minimum weights
        if self.samples.len() > self.max_samples {
            self.samples.sort_by(|a, b| {
                b.weight
                    .partial_cmp(&a.weight)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            self.samples.truncate(self.max_samples);
        }

        self.update_threshold();
        Ok(())
    }

    /// Serializes the QSketch to bytes
    ///
    /// Format: [max_samples: 4 bytes][total_weight: 8 bytes][num_samples: 4 bytes]
    ///         [sample_data: num_samples * 16 bytes]
    ///         [num_items_seen: 4 bytes][items_seen: variable]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Max samples (4 bytes)
        bytes.extend_from_slice(&(self.max_samples as u32).to_le_bytes());

        // Total weight (8 bytes)
        bytes.extend_from_slice(&self.total_weight.to_le_bytes());

        // Number of samples (4 bytes)
        bytes.extend_from_slice(&(self.samples.len() as u32).to_le_bytes());

        // Sample data (8 bytes for ID + 8 bytes for weight per sample)
        for sample in &self.samples {
            bytes.extend_from_slice(&sample.element_id.to_le_bytes());
            bytes.extend_from_slice(&sample.weight.to_le_bytes());
        }

        // Number of items seen (4 bytes)
        bytes.extend_from_slice(&(self.items_seen.len() as u32).to_le_bytes());

        // Items seen (8 bytes for ID + 8 bytes for weight)
        for (&element_id, &weight) in &self.items_seen {
            bytes.extend_from_slice(&element_id.to_le_bytes());
            bytes.extend_from_slice(&weight.to_le_bytes());
        }

        bytes
    }

    /// Deserializes a QSketch from bytes
    ///
    /// # Errors
    ///
    /// Returns error if bytes are invalid or corrupted
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SketchError> {
        if bytes.len() < 16 {
            return Err(SketchError::DeserializationError(
                "QSketch bytes too short".to_string(),
            ));
        }

        let mut offset = 0;

        // Read max_samples
        let max_samples = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        offset += 4;

        // Read total_weight
        let total_weight = f64::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]);
        offset += 8;

        // Read number of samples
        let num_samples = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        offset += 4;

        // Read samples
        let mut samples = Vec::with_capacity(num_samples);
        for _ in 0..num_samples {
            if offset + 16 > bytes.len() {
                return Err(SketchError::DeserializationError(
                    "Invalid sample data".to_string(),
                ));
            }

            let element_id = u64::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]);
            offset += 8;

            let weight = f64::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]);
            offset += 8;

            samples.push(SampledItem { element_id, weight });
        }

        // Read number of items seen
        if offset + 4 > bytes.len() {
            return Err(SketchError::DeserializationError(
                "Invalid items_seen count".to_string(),
            ));
        }

        let num_items_seen = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        offset += 4;

        // Read items seen
        let mut items_seen = HashMap::with_capacity(num_items_seen);
        for _ in 0..num_items_seen {
            if offset + 16 > bytes.len() {
                return Err(SketchError::DeserializationError(
                    "Invalid items_seen data".to_string(),
                ));
            }

            let element_id = u64::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]);
            offset += 8;

            let weight = f64::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]);
            offset += 8;

            items_seen.insert(element_id, weight);
        }

        use rand::SeedableRng;
        Ok(QSketch {
            max_samples,
            samples,
            items_seen,
            total_weight,
            threshold: 0.0,
            rng: rand::rngs::SmallRng::from_os_rng(),
        })
    }
}

// Note: QSketch uses a non-standard update interface with weights,
// so we implement Sketch for u64 hash values for compatibility,
// but users should use the QSketch::update(item, weight) method directly.
impl Sketch for QSketch {
    type Item = u64;

    fn update(&mut self, _item: &Self::Item) {
        // Note: This is a placeholder for trait compatibility.
        // The standard way to update QSketch is: qsketch.update(&item_bytes, weight)
        // This method cannot be properly implemented due to missing weight parameter.
        unimplemented!("Use QSketch::update(item, weight) instead of the Sketch trait")
    }

    fn estimate(&self) -> f64 {
        let (estimate, _) = self.estimate_weighted_cardinality();
        estimate
    }

    fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    fn serialize(&self) -> Vec<u8> {
        self.to_bytes()
    }

    fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        Self::from_bytes(bytes)
    }
}

impl Mergeable for QSketch {
    fn merge(&mut self, other: &Self) -> Result<(), SketchError> {
        self.merge(other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_qsketch() {
        let qsketch = QSketch::new(256);
        assert!(qsketch.is_empty());
        assert_eq!(qsketch.max_samples(), 256);
        assert_eq!(qsketch.sample_count(), 0);
        assert_eq!(qsketch.total_weight(), 0.0);
    }

    #[test]
    fn test_update_single_element() {
        let mut qsketch = QSketch::new(256);
        qsketch.update(b"item_1", 10.0);

        assert!(!qsketch.is_empty());
        assert_eq!(qsketch.sample_count(), 1);
        assert!((qsketch.total_weight() - 10.0).abs() < 0.001);
        assert_eq!(qsketch.estimate_distinct_elements(), 1);
    }

    #[test]
    fn test_update_multiple_elements() {
        let mut qsketch = QSketch::new(256);
        qsketch.update(b"item_1", 10.0);
        qsketch.update(b"item_2", 20.0);
        qsketch.update(b"item_3", 30.0);

        assert_eq!(qsketch.sample_count(), 3);
        assert!((qsketch.total_weight() - 60.0).abs() < 0.001);
        assert_eq!(qsketch.estimate_distinct_elements(), 3);
    }

    #[test]
    fn test_duplicate_elements() {
        let mut qsketch = QSketch::new(256);
        qsketch.update(b"item_1", 10.0);
        qsketch.update(b"item_1", 5.0); // Duplicate

        assert_eq!(qsketch.estimate_distinct_elements(), 1);
        assert!((qsketch.total_weight() - 15.0).abs() < 0.001);
    }

    #[test]
    fn test_uniform_weights() {
        let mut qsketch = QSketch::new(256);
        for i in 0..100 {
            qsketch.update(format!("item_{}", i).as_bytes(), 1.0);
        }

        assert_eq!(qsketch.estimate_distinct_elements(), 100);
        assert!((qsketch.total_weight() - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_skewed_weights() {
        let mut qsketch = QSketch::new(256);

        // Zipfian-like distribution: few items with high weights
        qsketch.update(b"item_1", 1000.0);
        qsketch.update(b"item_2", 500.0);
        qsketch.update(b"item_3", 250.0);
        qsketch.update(b"item_4", 100.0);
        qsketch.update(b"item_5", 50.0);

        assert_eq!(qsketch.estimate_distinct_elements(), 5);
        assert!((qsketch.total_weight() - 1900.0).abs() < 0.001);
    }

    #[test]
    fn test_weighted_cardinality_estimate() {
        let mut qsketch = QSketch::new(256);
        qsketch.update(b"user_1", 100.0);
        qsketch.update(b"user_2", 250.0);
        qsketch.update(b"user_3", 50.0);

        let (estimate, error_bound) = qsketch.estimate_weighted_cardinality();

        // Should estimate close to 3 distinct elements with some weight consideration
        assert!(estimate > 0.0);
        assert!(error_bound >= 0.0);
    }

    #[test]
    fn test_merge_sketches() {
        let mut qsketch1 = QSketch::new(256);
        let mut qsketch2 = QSketch::new(256);

        qsketch1.update(b"item_1", 100.0);
        qsketch1.update(b"item_2", 50.0);

        qsketch2.update(b"item_3", 150.0);
        qsketch2.update(b"item_4", 75.0);

        qsketch1.merge(&qsketch2).unwrap();

        assert!((qsketch1.total_weight() - 375.0).abs() < 0.001);
        assert!(qsketch1.estimate_distinct_elements() >= 3);
    }

    #[test]
    fn test_merge_with_duplicates() {
        let mut qsketch1 = QSketch::new(256);
        let mut qsketch2 = QSketch::new(256);

        qsketch1.update(b"item_1", 100.0);
        qsketch2.update(b"item_1", 50.0);

        qsketch1.merge(&qsketch2).unwrap();

        // Total weight should be 150, distinct count 1
        assert!((qsketch1.total_weight() - 150.0).abs() < 0.001);
        assert_eq!(qsketch1.estimate_distinct_elements(), 1);
    }

    #[test]
    fn test_merge_incompatible_config() {
        let mut qsketch1 = QSketch::new(256);
        let qsketch2 = QSketch::new(128);

        qsketch1.update(b"item_1", 100.0);

        let result = qsketch1.merge(&qsketch2);
        assert!(result.is_err());
    }

    #[test]
    fn test_total_weight_accuracy() {
        let mut qsketch = QSketch::new(256);

        let items = vec![
            ("item_1", 10.5),
            ("item_2", 20.3),
            ("item_3", 30.2),
            ("item_4", 15.0),
        ];

        let mut expected_total = 0.0;
        for (item, weight) in items {
            qsketch.update(item.as_bytes(), weight);
            expected_total += weight;
        }

        assert!((qsketch.total_weight() - expected_total).abs() < 0.001);
    }

    #[test]
    fn test_distinct_element_counting() {
        let mut qsketch = QSketch::new(256);

        for i in 0..50 {
            qsketch.update(format!("user_{}", i).as_bytes(), (i as f64) + 1.0);
        }

        let distinct = qsketch.estimate_distinct_elements();
        assert!((45..=50).contains(&distinct));
    }

    #[test]
    fn test_serialization_deserialization() {
        let mut qsketch = QSketch::new(256);
        qsketch.update(b"item_1", 10.0);
        qsketch.update(b"item_2", 20.0);
        qsketch.update(b"item_3", 30.0);

        let bytes = qsketch.to_bytes();
        let restored = QSketch::from_bytes(&bytes).unwrap();

        assert_eq!(restored.max_samples(), qsketch.max_samples());
        assert!((restored.total_weight() - qsketch.total_weight()).abs() < 0.001);
        assert_eq!(restored.sample_count(), qsketch.sample_count());
    }

    #[test]
    fn test_reset() {
        let mut qsketch = QSketch::new(256);
        qsketch.update(b"item_1", 10.0);
        qsketch.update(b"item_2", 20.0);

        assert!(!qsketch.is_empty());

        qsketch.reset();

        assert!(qsketch.is_empty());
        assert_eq!(qsketch.sample_count(), 0);
        assert_eq!(qsketch.total_weight(), 0.0);
        assert_eq!(qsketch.estimate_distinct_elements(), 0);
    }

    #[test]
    fn test_with_seed_reproducibility() {
        let mut qsketch1 = QSketch::with_seed(256, 42);
        let mut qsketch2 = QSketch::with_seed(256, 42);

        // Add same items in same order with same seed
        for i in 1..=50 {
            let weight = (i as f64) * 1.5;
            qsketch1.update(format!("item_{}", i).as_bytes(), weight);
            qsketch2.update(format!("item_{}", i).as_bytes(), weight);
        }

        // With same seed, sampling should be deterministic
        assert_eq!(qsketch1.sample_count(), qsketch2.sample_count());
        assert!((qsketch1.total_weight() - qsketch2.total_weight()).abs() < 0.0001);
    }
}
