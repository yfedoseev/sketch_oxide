//! Conservative Update Count-Min Sketch (Estan & Varghese 2002)
//!
//! A modification of Count-Min Sketch that improves accuracy by up to 10x
//! by only incrementing counters to the minimum necessary value.
//!
//! # Algorithm
//!
//! Standard CM sketch increments all k hash positions by 1.
//! Conservative update only increments to max(current_min + 1, current_value).
//!
//! # Trade-offs
//!
//! | Aspect | Standard CM | Conservative CM |
//! |--------|-------------|-----------------|
//! | Accuracy | Baseline | Up to 10x better |
//! | Deletions | Supported | Not supported |
//! | Point queries | Overestimates | Less overestimation |
//!
//! # Use Cases
//!
//! - When deletions are NOT needed
//! - High-accuracy frequency estimation
//! - Heavy hitter detection
//!
//! # References
//!
//! - Estan & Varghese "New Directions in Traffic Measurement and Accounting" (SIGCOMM 2002)
//!
//! # Examples
//!
//! ```
//! use sketch_oxide::frequency::ConservativeCountMin;
//!
//! let mut cms = ConservativeCountMin::new(0.01, 0.01).unwrap();
//!
//! cms.update(&"apple");
//! cms.update(&"apple");
//! cms.update(&"banana");
//!
//! // Estimates are more accurate than standard Count-Min
//! assert!(cms.estimate(&"apple") >= 2);
//! assert!(cms.estimate(&"banana") >= 1);
//! ```

use crate::common::hash::hash_value;
use crate::common::SketchError;
use std::hash::Hash;

/// Conservative Update Count-Min Sketch
///
/// Provides improved accuracy over standard Count-Min Sketch by using
/// conservative updates that only increment counters to the minimum necessary.
///
/// # Trade-off
///
/// Cannot support deletions or negative updates, but offers significantly
/// better accuracy for point queries.
///
/// # Examples
///
/// ```
/// use sketch_oxide::frequency::ConservativeCountMin;
///
/// let mut cms = ConservativeCountMin::new(0.01, 0.01).unwrap();
/// cms.update(&"hello");
/// assert!(cms.estimate(&"hello") >= 1);
/// ```
#[derive(Clone, Debug)]
pub struct ConservativeCountMin {
    /// Width of each row: w = ⌈e/ε⌉
    width: usize,
    /// Number of rows (hash functions): d = ⌈ln(1/δ)⌉
    depth: usize,
    /// 2D table of counters: depth × width
    table: Vec<Vec<u64>>,
    /// Independent hash seeds for each row
    hash_seeds: Vec<u32>,
    /// Epsilon parameter (error bound)
    epsilon: f64,
    /// Delta parameter (failure probability)
    delta: f64,
    /// Total count of all updates
    total_count: u64,
}

impl ConservativeCountMin {
    /// Creates a new Conservative Update Count-Min Sketch
    ///
    /// # Arguments
    ///
    /// * `epsilon` - Error bound (ε): estimates are within εN of true value
    /// * `delta` - Failure probability (δ): guarantee holds with probability 1-δ
    ///
    /// # Errors
    ///
    /// Returns error if parameters are invalid
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::frequency::ConservativeCountMin;
    ///
    /// let cms = ConservativeCountMin::new(0.01, 0.01).unwrap();
    /// ```
    pub fn new(epsilon: f64, delta: f64) -> Result<Self, SketchError> {
        if epsilon <= 0.0 || epsilon >= 1.0 {
            return Err(SketchError::InvalidParameter {
                param: "epsilon".to_string(),
                value: epsilon.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }

        if delta <= 0.0 || delta >= 1.0 {
            return Err(SketchError::InvalidParameter {
                param: "delta".to_string(),
                value: delta.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }

        // Width: w = ⌈e/ε⌉
        const E: f64 = std::f64::consts::E;
        let width = (E / epsilon).ceil() as usize;
        let width = width.max(2);

        // Depth: d = ⌈ln(1/δ)⌉
        let depth = (1.0 / delta).ln().ceil() as usize;
        let depth = depth.max(1);

        let table = vec![vec![0u64; width]; depth];

        let hash_seeds: Vec<u32> = (0..depth)
            .map(|i| (i as u32).wrapping_mul(0x9e3779b9))
            .collect();

        Ok(ConservativeCountMin {
            width,
            depth,
            table,
            hash_seeds,
            epsilon,
            delta,
            total_count: 0,
        })
    }

    /// Creates a sketch with specific dimensions
    ///
    /// # Arguments
    ///
    /// * `width` - Width of the table
    /// * `depth` - Number of hash functions
    pub fn with_dimensions(width: usize, depth: usize) -> Result<Self, SketchError> {
        if width == 0 {
            return Err(SketchError::InvalidParameter {
                param: "width".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }

        if depth == 0 {
            return Err(SketchError::InvalidParameter {
                param: "depth".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }

        let table = vec![vec![0u64; width]; depth];

        let hash_seeds: Vec<u32> = (0..depth)
            .map(|i| (i as u32).wrapping_mul(0x9e3779b9))
            .collect();

        // Calculate theoretical epsilon and delta from dimensions
        const E: f64 = std::f64::consts::E;
        let epsilon = E / width as f64;
        let delta = (-(depth as f64)).exp();

        Ok(ConservativeCountMin {
            width,
            depth,
            table,
            hash_seeds,
            epsilon,
            delta,
            total_count: 0,
        })
    }

    /// Updates the sketch with an item using conservative update
    ///
    /// Unlike standard Count-Min, only increments counters to the minimum
    /// necessary value, reducing overestimation.
    ///
    /// # Arguments
    ///
    /// * `item` - The item to add
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::frequency::ConservativeCountMin;
    ///
    /// let mut cms = ConservativeCountMin::new(0.01, 0.01).unwrap();
    /// cms.update(&"apple");
    /// cms.update(&"apple");
    /// ```
    pub fn update<T: Hash>(&mut self, item: &T) {
        self.update_count(item, 1);
    }

    /// Updates with a specific count
    ///
    /// # Arguments
    ///
    /// * `item` - The item to add
    /// * `count` - Number of occurrences to add
    pub fn update_count<T: Hash>(&mut self, item: &T, count: u64) {
        if count == 0 {
            return;
        }

        self.total_count += count;

        // First pass: find current minimum estimate
        let indices: Vec<usize> = self
            .hash_seeds
            .iter()
            .map(|&seed| {
                let hash = hash_value(item, seed);
                (hash as usize) % self.width
            })
            .collect();

        let current_min = indices
            .iter()
            .enumerate()
            .map(|(row, &col)| self.table[row][col])
            .min()
            .unwrap_or(0);

        // Second pass: conservative update
        // Set each counter to max(current_value, current_min + count)
        let new_value = current_min.saturating_add(count);

        for (row, &col) in indices.iter().enumerate() {
            if self.table[row][col] < new_value {
                self.table[row][col] = new_value;
            }
        }
    }

    /// Estimates the frequency of an item
    ///
    /// # Arguments
    ///
    /// * `item` - The item to query
    ///
    /// # Returns
    ///
    /// Estimated frequency (always >= true frequency)
    pub fn estimate<T: Hash>(&self, item: &T) -> u64 {
        self.hash_seeds
            .iter()
            .enumerate()
            .map(|(row, &seed)| {
                let hash = hash_value(item, seed);
                let col = (hash as usize) % self.width;
                self.table[row][col]
            })
            .min()
            .unwrap_or(0)
    }

    /// Returns the width of the table
    pub fn width(&self) -> usize {
        self.width
    }

    /// Returns the depth (number of hash functions)
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Returns the epsilon parameter
    pub fn epsilon(&self) -> f64 {
        self.epsilon
    }

    /// Returns the delta parameter
    pub fn delta(&self) -> f64 {
        self.delta
    }

    /// Returns the total count of all updates
    pub fn total_count(&self) -> u64 {
        self.total_count
    }

    /// Clears all counters
    pub fn clear(&mut self) {
        for row in &mut self.table {
            row.fill(0);
        }
        self.total_count = 0;
    }

    /// Returns memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.width * self.depth * std::mem::size_of::<u64>()
            + self.depth * std::mem::size_of::<u32>()
    }

    /// Serializes the sketch to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&(self.width as u64).to_le_bytes());
        bytes.extend_from_slice(&(self.depth as u64).to_le_bytes());
        bytes.extend_from_slice(&self.epsilon.to_le_bytes());
        bytes.extend_from_slice(&self.delta.to_le_bytes());
        bytes.extend_from_slice(&self.total_count.to_le_bytes());

        for seed in &self.hash_seeds {
            bytes.extend_from_slice(&seed.to_le_bytes());
        }

        for row in &self.table {
            for &val in row {
                bytes.extend_from_slice(&val.to_le_bytes());
            }
        }

        bytes
    }

    /// Deserializes a sketch from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SketchError> {
        if bytes.len() < 40 {
            return Err(SketchError::DeserializationError(
                "Insufficient data for ConservativeCountMin header".to_string(),
            ));
        }

        let width = u64::from_le_bytes(bytes[0..8].try_into().unwrap()) as usize;
        let depth = u64::from_le_bytes(bytes[8..16].try_into().unwrap()) as usize;
        let epsilon = f64::from_le_bytes(bytes[16..24].try_into().unwrap());
        let delta = f64::from_le_bytes(bytes[24..32].try_into().unwrap());
        let total_count = u64::from_le_bytes(bytes[32..40].try_into().unwrap());

        let expected_len = 40 + depth * 4 + width * depth * 8;
        if bytes.len() < expected_len {
            return Err(SketchError::DeserializationError(format!(
                "Expected {} bytes, got {}",
                expected_len,
                bytes.len()
            )));
        }

        let mut offset = 40;
        let mut hash_seeds = Vec::with_capacity(depth);
        for _ in 0..depth {
            hash_seeds.push(u32::from_le_bytes(
                bytes[offset..offset + 4].try_into().unwrap(),
            ));
            offset += 4;
        }

        let mut table = vec![vec![0u64; width]; depth];
        for row in &mut table {
            for val in row.iter_mut() {
                *val = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
                offset += 8;
            }
        }

        Ok(ConservativeCountMin {
            width,
            depth,
            table,
            hash_seeds,
            epsilon,
            delta,
            total_count,
        })
    }

    /// Merges another sketch into this one
    ///
    /// Takes the maximum of corresponding counters.
    ///
    /// # Errors
    ///
    /// Returns error if dimensions don't match
    pub fn merge(&mut self, other: &Self) -> Result<(), SketchError> {
        if self.width != other.width || self.depth != other.depth {
            return Err(SketchError::InvalidParameter {
                param: "dimensions".to_string(),
                value: format!(
                    "{}x{} vs {}x{}",
                    self.width, self.depth, other.width, other.depth
                ),
                constraint: "must have same dimensions to merge".to_string(),
            });
        }

        for (row_idx, (self_row, other_row)) in
            self.table.iter_mut().zip(other.table.iter()).enumerate()
        {
            for (col_idx, (self_val, &other_val)) in
                self_row.iter_mut().zip(other_row.iter()).enumerate()
            {
                // Conservative merge: take maximum
                *self_val = (*self_val).max(other_val);
                let _ = (row_idx, col_idx); // Silence unused variable warning
            }
        }

        self.total_count += other.total_count;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let cms = ConservativeCountMin::new(0.01, 0.01).unwrap();
        assert!(cms.width() > 0);
        assert!(cms.depth() > 0);
    }

    #[test]
    fn test_invalid_params() {
        assert!(ConservativeCountMin::new(0.0, 0.01).is_err());
        assert!(ConservativeCountMin::new(1.0, 0.01).is_err());
        assert!(ConservativeCountMin::new(0.01, 0.0).is_err());
        assert!(ConservativeCountMin::new(0.01, 1.0).is_err());
    }

    #[test]
    fn test_with_dimensions() {
        let cms = ConservativeCountMin::with_dimensions(100, 5).unwrap();
        assert_eq!(cms.width(), 100);
        assert_eq!(cms.depth(), 5);
    }

    #[test]
    fn test_update_and_estimate() {
        let mut cms = ConservativeCountMin::new(0.01, 0.01).unwrap();

        cms.update(&"apple");
        cms.update(&"apple");
        cms.update(&"banana");

        assert!(cms.estimate(&"apple") >= 2);
        assert!(cms.estimate(&"banana") >= 1);
        assert_eq!(cms.estimate(&"cherry"), 0);
    }

    #[test]
    fn test_update_count() {
        let mut cms = ConservativeCountMin::new(0.01, 0.01).unwrap();

        cms.update_count(&"apple", 5);
        cms.update_count(&"banana", 3);

        assert!(cms.estimate(&"apple") >= 5);
        assert!(cms.estimate(&"banana") >= 3);
    }

    #[test]
    fn test_total_count() {
        let mut cms = ConservativeCountMin::new(0.01, 0.01).unwrap();

        cms.update(&"a");
        cms.update(&"b");
        cms.update_count(&"c", 5);

        assert_eq!(cms.total_count(), 7);
    }

    #[test]
    fn test_clear() {
        let mut cms = ConservativeCountMin::new(0.01, 0.01).unwrap();

        cms.update(&"apple");
        cms.update(&"banana");

        cms.clear();

        assert_eq!(cms.estimate(&"apple"), 0);
        assert_eq!(cms.estimate(&"banana"), 0);
        assert_eq!(cms.total_count(), 0);
    }

    #[test]
    fn test_serialization() {
        let mut cms = ConservativeCountMin::new(0.01, 0.01).unwrap();

        cms.update(&"apple");
        cms.update(&"apple");
        cms.update(&"banana");

        let bytes = cms.to_bytes();
        let restored = ConservativeCountMin::from_bytes(&bytes).unwrap();

        assert_eq!(cms.width(), restored.width());
        assert_eq!(cms.depth(), restored.depth());
        assert_eq!(cms.estimate(&"apple"), restored.estimate(&"apple"));
        assert_eq!(cms.estimate(&"banana"), restored.estimate(&"banana"));
    }

    #[test]
    fn test_merge() {
        let mut cms1 = ConservativeCountMin::new(0.01, 0.01).unwrap();
        let mut cms2 = ConservativeCountMin::new(0.01, 0.01).unwrap();

        cms1.update(&"apple");
        cms1.update(&"apple");
        cms2.update(&"banana");
        cms2.update(&"banana");

        cms1.merge(&cms2).unwrap();

        assert!(cms1.estimate(&"apple") >= 2);
        assert!(cms1.estimate(&"banana") >= 2);
    }

    #[test]
    fn test_conservative_vs_standard_accuracy() {
        // Conservative update should be more accurate
        let mut cms = ConservativeCountMin::with_dimensions(100, 5).unwrap();

        // Insert same item many times
        for _ in 0..100 {
            cms.update(&"frequent");
        }

        // Insert many different items to create collisions
        for i in 0..1000 {
            cms.update(&format!("item_{}", i));
        }

        let estimate = cms.estimate(&"frequent");

        // Conservative update should keep estimate closer to 100
        // Standard CM would have much higher overestimation
        assert!(estimate >= 100, "Estimate {} should be >= 100", estimate);
        assert!(
            estimate < 200,
            "Estimate {} should be < 200 due to conservative update",
            estimate
        );
    }

    #[test]
    fn test_memory_usage() {
        let cms = ConservativeCountMin::new(0.01, 0.01).unwrap();
        assert!(cms.memory_usage() > 0);
    }
}
