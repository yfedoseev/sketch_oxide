//! Count-Min Sketch implementation for frequency estimation
//!
//! Count-Min Sketch (Cormode & Muthukrishnan, 2003) is the standard algorithm
//! for point query frequency estimation. It provides probabilistic guarantees:
//! - Never underestimates (only overestimates)
//! - Error bounded by ε with probability 1-δ
//! - Space: O((e/ε) * ln(1/δ))
//! - Time: O(ln(1/δ)) per operation
//!
//! # Optimizations
//! - **Single-hash-derive pattern**: Hash item once, derive d positions by mixing state
//! - **Power-of-2 width with bitmask**: Use `& mask` instead of `% width`
//! - **Flat table layout**: Better cache locality than Vec<Vec<>>
//!
//! # References
//! - Cormode, G., & Muthukrishnan, S. (2003). "An improved data stream summary:
//!   the count-min sketch and its applications"
//!
//! # Production Use
//! - Redis (RedisBloom module)
//! - Network traffic monitoring
//! - Database query optimization
//! - Real-time analytics systems

use crate::common::{validation, Mergeable, Sketch, SketchError};
use std::hash::{Hash, Hasher};
use twox_hash::XxHash64;

/// Count-Min Sketch for frequency estimation
///
/// A space-efficient probabilistic data structure for estimating item frequencies
/// in a data stream. The sketch guarantees:
/// - Never underestimates (always returns count >= true count)
/// - Error bounded by εN with probability 1-δ (where N is total stream size)
///
/// # Type Parameters
/// The sketch works with any type that implements `Hash`.
///
/// # Examples
/// ```
/// use sketch_oxide::frequency::CountMinSketch;
///
/// let mut cms = CountMinSketch::new(0.01, 0.01).unwrap();
///
/// // Update with items
/// cms.update(&"apple");
/// cms.update(&"apple");
/// cms.update(&"banana");
///
/// // Query frequencies (never underestimates)
/// assert!(cms.estimate(&"apple") >= 2);
/// assert!(cms.estimate(&"banana") >= 1);
/// assert_eq!(cms.estimate(&"cherry"), 0);
/// ```
#[derive(Clone, Debug)]
pub struct CountMinSketch {
    /// Width of each row (power of 2 for fast modulo)
    width: usize,
    /// Bitmask for fast modulo: width - 1
    mask: usize,
    /// Number of rows (hash functions): d = ⌈ln(1/δ)⌉
    depth: usize,
    /// Flat table of counters: depth × width (row-major for cache locality)
    table: Vec<u64>,
    /// Epsilon parameter (error bound)
    epsilon: f64,
    /// Delta parameter (failure probability)
    delta: f64,
}

impl CountMinSketch {
    /// Create a new Count-Min Sketch with specified error bounds
    ///
    /// # Arguments
    /// * `epsilon` - Error bound (ε): estimates are within εN of true value
    /// * `delta` - Failure probability (δ): guarantee holds with probability 1-δ
    ///
    /// # Returns
    /// A new `CountMinSketch` or an error if parameters are invalid
    ///
    /// # Errors
    /// Returns `InvalidParameter` if:
    /// - `epsilon` <= 0 or >= 1
    /// - `delta` <= 0 or >= 1
    ///
    /// # Space Complexity
    /// O((e/ε) * ln(1/δ)) where e ≈ 2.71828
    ///
    /// # Examples
    /// ```
    /// use sketch_oxide::frequency::CountMinSketch;
    ///
    /// // 1% error bound, 1% failure probability
    /// let cms = CountMinSketch::new(0.01, 0.01).unwrap();
    /// ```
    pub fn new(epsilon: f64, delta: f64) -> Result<Self, SketchError> {
        // Validate epsilon: must be in (0, 1)
        if epsilon <= 0.0 || epsilon >= 1.0 {
            return Err(SketchError::InvalidParameter {
                param: "epsilon".to_string(),
                value: epsilon.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }

        // Validate delta: must be in (0, 1)
        if delta <= 0.0 || delta >= 1.0 {
            return Err(SketchError::InvalidParameter {
                param: "delta".to_string(),
                value: delta.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }

        // Calculate dimensions based on practical bounds
        // Width: w = ⌈2/ε⌉, then round up to power of 2
        // Using 2/ε instead of e/ε gives ~26% smaller tables with similar guarantees
        let width_min = (2.0 / epsilon).ceil() as usize;
        let width = width_min.next_power_of_two(); // Round up to power of 2
        let mask = width - 1; // Bitmask for fast modulo

        // Depth: d = ⌈ln(1/δ)⌉
        let depth = (1.0 / delta).ln().ceil() as usize;

        // Ensure minimum dimensions
        let depth = depth.max(1);

        // Initialize flat table with zeros (better cache locality)
        let table = vec![0u64; depth * width];

        Ok(CountMinSketch {
            width,
            mask,
            depth,
            table,
            epsilon,
            delta,
        })
    }

    /// Update the sketch with an item
    ///
    /// Increments the counters for the item in all rows using derived hash functions.
    ///
    /// # Arguments
    /// * `item` - The item to add (must implement `Hash`)
    ///
    /// # Time Complexity
    /// O(d) where d = ⌈ln(1/δ)⌉
    ///
    /// # Examples
    /// ```
    /// use sketch_oxide::frequency::CountMinSketch;
    ///
    /// let mut cms = CountMinSketch::new(0.01, 0.01).unwrap();
    /// cms.update(&"item");
    /// cms.update(&42u64);
    /// ```
    #[inline]
    pub fn update<T: Hash>(&mut self, item: &T) {
        // Hash item once
        let mut hasher = XxHash64::with_seed(0);
        item.hash(&mut hasher);

        let width = self.width;
        let mask = self.mask;
        let depth = self.depth;

        // Derive d positions from single hash
        for row_idx in 0..depth {
            let hash = hasher.finish();
            let col_idx = (hash as usize) & mask;
            let idx = row_idx * width + col_idx;

            // SAFETY: idx is always in bounds due to mask operation
            unsafe {
                *self.table.get_unchecked_mut(idx) =
                    self.table.get_unchecked(idx).saturating_add(1);
            }

            // Mix state for next row
            hasher.write(&[0x7B]);
        }
    }

    /// Estimate the frequency of an item
    ///
    /// Returns the minimum counter value across all hash functions.
    /// This guarantees the estimate never underestimates the true count.
    ///
    /// # Arguments
    /// * `item` - The item to query
    ///
    /// # Returns
    /// Estimated frequency (always >= true frequency)
    ///
    /// # Guarantee
    /// With probability at least 1-δ:
    /// - estimate >= true_count (always)
    /// - estimate <= true_count + ε*N (where N is total stream size)
    ///
    /// # Time Complexity
    /// O(d) where d = ⌈ln(1/δ)⌉
    ///
    /// # Examples
    /// ```
    /// use sketch_oxide::frequency::CountMinSketch;
    ///
    /// let mut cms = CountMinSketch::new(0.01, 0.01).unwrap();
    /// cms.update(&"apple");
    /// cms.update(&"apple");
    ///
    /// assert!(cms.estimate(&"apple") >= 2);
    /// assert_eq!(cms.estimate(&"banana"), 0);
    /// ```
    #[inline]
    pub fn estimate<T: Hash>(&self, item: &T) -> u64 {
        // Hash item once
        let mut hasher = XxHash64::with_seed(0);
        item.hash(&mut hasher);

        let width = self.width;
        let mask = self.mask;
        let depth = self.depth;
        let mut min_count = u64::MAX;

        // Derive d positions from single hash
        for row_idx in 0..depth {
            let hash = hasher.finish();
            let col_idx = (hash as usize) & mask;
            let idx = row_idx * width + col_idx;

            // SAFETY: idx is always in bounds due to mask operation
            let count = unsafe { *self.table.get_unchecked(idx) };
            min_count = min_count.min(count);

            // Mix state for next row
            hasher.write(&[0x7B]);
        }

        // If all counters are still u64::MAX, the sketch is empty
        if min_count == u64::MAX {
            0
        } else {
            min_count
        }
    }

    /// Get the width of the sketch
    ///
    /// # Returns
    /// The number of counters per row (power of 2)
    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }

    /// Get the depth of the sketch
    ///
    /// # Returns
    /// The number of rows (d = ⌈ln(1/δ)⌉)
    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Get the epsilon parameter
    ///
    /// # Returns
    /// The error bound (ε)
    #[inline]
    pub fn epsilon(&self) -> f64 {
        self.epsilon
    }

    /// Get the delta parameter
    ///
    /// # Returns
    /// The failure probability (δ)
    #[inline]
    pub fn delta(&self) -> f64 {
        self.delta
    }
}

impl Sketch for CountMinSketch {
    /// Item type (Count-Min Sketch works with any hashable type)
    /// We use u64 as the nominal type, but update/estimate are generic
    type Item = u64;

    /// Update is not part of the Sketch trait API for frequency estimation
    ///
    /// Use the generic `update<T: Hash>(&mut self, item: &T)` method instead.
    fn update(&mut self, item: &Self::Item) {
        CountMinSketch::update(self, item);
    }

    /// Estimate returns 0 as Count-Min Sketch requires a query item
    ///
    /// This is a placeholder to satisfy the Sketch trait.
    /// Use `estimate<T: Hash>(&self, item: &T) -> u64` instead.
    fn estimate(&self) -> f64 {
        0.0
    }

    /// Check if the sketch is empty (all counters are zero)
    fn is_empty(&self) -> bool {
        self.table.iter().all(|&count| count == 0)
    }

    /// Serialize the sketch to bytes
    fn serialize(&self) -> Vec<u8> {
        // Format: [width:8][depth:8][epsilon:8][delta:8][table]
        let mut bytes = Vec::new();

        // Dimensions
        bytes.extend_from_slice(&self.width.to_le_bytes());
        bytes.extend_from_slice(&self.depth.to_le_bytes());

        // Parameters
        bytes.extend_from_slice(&self.epsilon.to_le_bytes());
        bytes.extend_from_slice(&self.delta.to_le_bytes());

        // Table data
        for &count in &self.table {
            bytes.extend_from_slice(&count.to_le_bytes());
        }

        bytes
    }

    /// Deserialize a sketch from bytes
    fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        // Validate total size and minimum header
        validation::validate_byte_size(bytes.len())?;
        validation::validate_min_size(bytes.len(), 32)?;

        let mut offset = 0;

        // Read dimensions
        let width = usize::from_le_bytes(
            bytes[offset..offset + 8]
                .try_into()
                .map_err(|_| SketchError::DeserializationError("invalid width".to_string()))?,
        );
        offset += 8;

        let depth = usize::from_le_bytes(
            bytes[offset..offset + 8]
                .try_into()
                .map_err(|_| SketchError::DeserializationError("invalid depth".to_string()))?,
        );
        offset += 8;

        // Validate width and depth dimensions
        validation::validate_width_depth(width as u32, depth as u32)?;

        // Read parameters
        let epsilon = f64::from_le_bytes(
            bytes[offset..offset + 8]
                .try_into()
                .map_err(|_| SketchError::DeserializationError("invalid epsilon".to_string()))?,
        );
        offset += 8;

        let delta = f64::from_le_bytes(
            bytes[offset..offset + 8]
                .try_into()
                .map_err(|_| SketchError::DeserializationError("invalid delta".to_string()))?,
        );
        offset += 8;

        // Validate probability parameters (epsilon and delta must be in (0.0, 1.0))
        validation::validate_probability(epsilon, "epsilon")?;
        validation::validate_probability(delta, "delta")?;

        // Compute mask
        let mask = width - 1;

        // Read table with validated size
        let expected_table_size = depth * width * 8;
        if bytes.len() < offset + expected_table_size {
            return Err(SketchError::DeserializationError(
                "insufficient bytes for table".to_string(),
            ));
        }

        let mut table = Vec::with_capacity(depth * width);
        for _ in 0..(depth * width) {
            let count = u64::from_le_bytes(
                bytes[offset..offset + 8]
                    .try_into()
                    .map_err(|_| SketchError::DeserializationError("invalid count".to_string()))?,
            );
            table.push(count);
            offset += 8;
        }

        Ok(CountMinSketch {
            width,
            mask,
            depth,
            table,
            epsilon,
            delta,
        })
    }
}

impl Mergeable for CountMinSketch {
    /// Merge another Count-Min Sketch into this one
    ///
    /// After merging, this sketch represents the union of both streams.
    /// The merge operation is element-wise addition of all counters.
    ///
    /// # Arguments
    /// * `other` - The sketch to merge (must have identical parameters)
    ///
    /// # Returns
    /// `Ok(())` if merge succeeded, error if sketches are incompatible
    ///
    /// # Errors
    /// Returns `IncompatibleSketches` if:
    /// - Sketches have different width (different epsilon)
    /// - Sketches have different depth (different delta)
    ///
    /// # Properties
    /// - Commutative: A.merge(B) ≡ B.merge(A)
    /// - Associative: (A.merge(B)).merge(C) ≡ A.merge(B.merge(C))
    ///
    /// # Examples
    /// ```
    /// use sketch_oxide::frequency::CountMinSketch;
    /// use sketch_oxide::Mergeable;
    ///
    /// let mut cms1 = CountMinSketch::new(0.01, 0.01).unwrap();
    /// let mut cms2 = CountMinSketch::new(0.01, 0.01).unwrap();
    ///
    /// cms1.update(&"item");
    /// cms2.update(&"item");
    ///
    /// cms1.merge(&cms2).unwrap();
    /// assert!(cms1.estimate(&"item") >= 2);
    /// ```
    fn merge(&mut self, other: &Self) -> Result<(), SketchError> {
        // Verify compatibility: width must match (same epsilon)
        if self.width != other.width {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "width mismatch: {} vs {} (different epsilon parameters)",
                    self.width, other.width
                ),
            });
        }

        // Verify compatibility: depth must match (same delta)
        if self.depth != other.depth {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "depth mismatch: {} vs {} (different delta parameters)",
                    self.depth, other.depth
                ),
            });
        }

        // Element-wise addition of all counters
        for (a, &b) in self.table.iter_mut().zip(other.table.iter()) {
            *a = a.saturating_add(b);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_construction() {
        let cms = CountMinSketch::new(0.01, 0.01).unwrap();
        assert!(cms.width() > 0);
        assert!(cms.depth() > 0);
        // Width should be power of 2
        assert!(cms.width().is_power_of_two());
    }

    #[test]
    fn test_dimension_calculation() {
        let cms = CountMinSketch::new(0.01, 0.01).unwrap();

        // With ε=0.01: width = ⌈2/0.01⌉ = 200, rounded to 256 (power of 2)
        assert_eq!(cms.width(), 256);

        // With δ=0.01: depth = ⌈ln(100)⌉ = ⌈4.605⌉ = 5
        assert_eq!(cms.depth(), 5);
    }

    #[test]
    fn test_update_and_estimate() {
        let mut cms = CountMinSketch::new(0.01, 0.01).unwrap();
        cms.update(&"test");
        assert_eq!(cms.estimate(&"test"), 1);
    }

    #[test]
    fn test_never_underestimates() {
        let mut cms = CountMinSketch::new(0.01, 0.01).unwrap();

        for _ in 0..100 {
            cms.update(&"item");
        }

        let estimate = cms.estimate(&"item");
        assert!(estimate >= 100);
    }

    #[test]
    fn test_merge_basic() {
        let mut cms1 = CountMinSketch::new(0.01, 0.01).unwrap();
        let mut cms2 = CountMinSketch::new(0.01, 0.01).unwrap();

        cms1.update(&"a");
        cms2.update(&"a");

        cms1.merge(&cms2).unwrap();

        assert!(cms1.estimate(&"a") >= 2);
    }

    #[test]
    fn test_merge_incompatible() {
        let mut cms1 = CountMinSketch::new(0.01, 0.01).unwrap();
        let cms2 = CountMinSketch::new(0.001, 0.01).unwrap();

        let result = cms1.merge(&cms2);
        assert!(result.is_err());
    }
}
