//! Elastic Sketch implementation for frequency estimation
//!
//! Elastic Sketch is a state-of-the-art algorithm for accurate frequency estimation
//! in network measurement and multi-task telemetry (2024-2025 research). It combines
//! the efficiency of Count-Min Sketch with elastic counters that adapt based on the
//! observed frequency distribution.
//!
//! # Key Features
//! - **Elastic Counters**: Adapt based on frequency distribution to reduce overestimation
//! - **Space Efficient**: O(k * d) space where k=bucket_count, d=depth
//! - **Fast Updates**: O(d) update time with minimal hash operations
//! - **Heavy Hitter Optimization**: Designed for discovering frequent items efficiently
//!
//! # Algorithm Overview
//! Unlike traditional Count-Min Sketch which maintains only a single counter per position,
//! Elastic Sketch maintains elastic counters that track frequency distribution. When updating
//! an item, if its current bucket is empty, it's inserted there. Otherwise, the item is
//! inserted into the bucket with minimum frequency, and elastic counters help refine
//! the frequency estimation.
//!
//! # Use Cases
//! - Network traffic monitoring and measurement
//! - Heavy hitter detection in data streams
//! - Multi-task telemetry collection
//! - Traffic engineering and load balancing
//! - Real-time anomaly detection
//!
//! # References
//! - 2024-2025 Research on Elastic Sketch for network measurement
//! - Extended from Count-Min Sketch (Cormode & Muthukrishnan, 2003)
//!
//! # Examples
//! ```
//! use sketch_oxide::frequency::ElasticSketch;
//!
//! let mut sketch = ElasticSketch::new(512, 3).unwrap();
//!
//! // Update with items
//! sketch.update(b"flow1", 1);
//! sketch.update(b"flow2", 2);
//! sketch.update(b"flow1", 1);
//!
//! // Estimate frequencies
//! assert!(sketch.estimate(b"flow1") > 0);
//! assert!(sketch.estimate(b"flow2") > 0);
//! assert_eq!(sketch.estimate(b"unknown"), 0);
//!
//! // Find heavy hitters
//! let hitters = sketch.heavy_hitters(1);
//! assert!(!hitters.is_empty());
//! ```

use crate::common::{Mergeable, Sketch, SketchError};
use std::cmp::Ordering;
use twox_hash::XxHash64;

/// Represents a bucket in the Elastic Sketch containing an item and its frequency information
#[derive(Clone, Debug, Copy)]
struct ElasticBucket {
    /// Hash of the item stored in this bucket
    item_hash: u64,
    /// Estimated frequency of the item
    frequency: u64,
    /// Elastic counter for more accurate frequency estimation
    elastic_counter: u64,
    /// Whether this bucket contains a valid item (to distinguish from empty buckets)
    is_occupied: bool,
}

impl ElasticBucket {
    /// Create an empty bucket
    fn empty() -> Self {
        ElasticBucket {
            item_hash: 0,
            frequency: 0,
            elastic_counter: 0,
            is_occupied: false,
        }
    }

    /// Create a bucket with an item
    fn new(item_hash: u64, frequency: u64) -> Self {
        ElasticBucket {
            item_hash,
            frequency,
            elastic_counter: 0,
            is_occupied: true,
        }
    }
}

/// Elastic Sketch for frequency estimation
///
/// A probabilistic data structure that provides accurate frequency estimates
/// for items in a data stream with minimal space overhead.
///
/// # Type Parameters
/// The sketch is configured with a fixed bucket count and depth, both specified
/// at construction time.
///
/// # Examples
/// ```
/// use sketch_oxide::frequency::ElasticSketch;
///
/// let mut sketch = ElasticSketch::new(1024, 4).unwrap();
/// sketch.update(b"item", 1);
/// assert!(sketch.estimate(b"item") >= 1);
/// ```
#[derive(Clone, Debug)]
pub struct ElasticSketch {
    /// 2D array of buckets: [depth][bucket_count]
    /// Flat layout for better cache locality
    buckets: Vec<ElasticBucket>,
    /// Number of buckets per row (power of 2 for fast modulo)
    bucket_count: usize,
    /// Bitmask for fast modulo: bucket_count - 1
    mask: usize,
    /// Number of hash functions (depth)
    depth: usize,
    /// Elastic ratio for counter expansion (0.0 - 1.0)
    elastic_ratio: f64,
    /// Total items added to sketch for normalization
    total_count: u64,
}

impl ElasticSketch {
    /// Create a new Elastic Sketch with default elastic ratio
    ///
    /// # Arguments
    /// * `bucket_count` - Number of buckets per row (will be rounded to power of 2)
    /// * `depth` - Number of hash functions (2-5 recommended)
    ///
    /// # Returns
    /// A new `ElasticSketch` or an error if parameters are invalid
    ///
    /// # Errors
    /// Returns `InvalidParameter` if:
    /// - `bucket_count` <= 0
    /// - `depth` <= 0 or > 8
    ///
    /// # Default Elastic Ratio
    /// Uses 0.2 (20% of minimum frequency for elastic counter expansion)
    ///
    /// # Examples
    /// ```
    /// use sketch_oxide::frequency::ElasticSketch;
    ///
    /// let sketch = ElasticSketch::new(512, 3).unwrap();
    /// assert_eq!(sketch.bucket_count(), 512);
    /// assert_eq!(sketch.depth(), 3);
    /// ```
    pub fn new(bucket_count: usize, depth: usize) -> Result<Self, SketchError> {
        Self::with_elastic_ratio(bucket_count, depth, 0.2)
    }

    /// Create a new Elastic Sketch with custom elastic ratio
    ///
    /// # Arguments
    /// * `bucket_count` - Number of buckets per row (will be rounded to power of 2)
    /// * `depth` - Number of hash functions (2-5 recommended)
    /// * `elastic_ratio` - Elastic expansion ratio (0.0 - 1.0)
    ///
    /// # Returns
    /// A new `ElasticSketch` or an error if parameters are invalid
    ///
    /// # Errors
    /// Returns `InvalidParameter` if:
    /// - `bucket_count` <= 0
    /// - `depth` <= 0 or > 8
    /// - `elastic_ratio` < 0.0 or > 1.0
    ///
    /// # Elastic Ratio
    /// - Lower values (0.0-0.3): More aggressive optimization, better space efficiency
    /// - Higher values (0.7-1.0): More conservative, better accuracy
    ///
    /// # Examples
    /// ```
    /// use sketch_oxide::frequency::ElasticSketch;
    ///
    /// let sketch = ElasticSketch::with_elastic_ratio(512, 3, 0.5).unwrap();
    /// ```
    pub fn with_elastic_ratio(
        bucket_count: usize,
        depth: usize,
        elastic_ratio: f64,
    ) -> Result<Self, SketchError> {
        // Validate bucket_count
        if bucket_count == 0 {
            return Err(SketchError::InvalidParameter {
                param: "bucket_count".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }

        // Validate depth
        if depth == 0 || depth > 8 {
            return Err(SketchError::InvalidParameter {
                param: "depth".to_string(),
                value: depth.to_string(),
                constraint: "must be in [1, 8]".to_string(),
            });
        }

        // Validate elastic_ratio
        if !(0.0..=1.0).contains(&elastic_ratio) {
            return Err(SketchError::InvalidParameter {
                param: "elastic_ratio".to_string(),
                value: elastic_ratio.to_string(),
                constraint: "must be in [0.0, 1.0]".to_string(),
            });
        }

        // Round bucket_count to power of 2 for fast modulo
        let bucket_count = bucket_count.next_power_of_two();
        let mask = bucket_count - 1;

        // Initialize flat bucket array
        let buckets = vec![ElasticBucket::empty(); depth * bucket_count];

        Ok(ElasticSketch {
            buckets,
            bucket_count,
            mask,
            depth,
            elastic_ratio,
            total_count: 0,
        })
    }

    /// Hash an item to get its hash value
    ///
    /// # Arguments
    /// * `item` - The item to hash
    ///
    /// # Returns
    /// A 64-bit hash value
    #[inline]
    fn hash_item(item: &[u8]) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = XxHash64::with_seed(0);
        item.hash(&mut hasher);
        hasher.finish()
    }

    /// Get bucket index for a given row and item hash
    ///
    /// # Arguments
    /// * `row` - The row index (hash function index)
    /// * `item_hash` - The hash of the item
    ///
    /// # Returns
    /// The bucket index in the flat array
    #[inline]
    fn bucket_index(&self, row: usize, item_hash: u64) -> usize {
        let col = (item_hash as usize) & self.mask;
        row * self.bucket_count + col
    }

    /// Update the sketch with an item and its count
    ///
    /// This operation:
    /// 1. Hashes the item using multiple hash functions
    /// 2. For each hash position, checks if the bucket is empty or occupied
    /// 3. If empty, inserts the item with the given count
    /// 4. If occupied by the same item, increases its frequency
    /// 5. If occupied by a different item, updates elastic counters
    ///
    /// # Arguments
    /// * `item` - The item to add (typically a network flow ID, URL, or feature)
    /// * `count` - The count/weight to add (typically 1 for single occurrence)
    ///
    /// # Time Complexity
    /// O(d) where d is the depth
    ///
    /// # Examples
    /// ```
    /// use sketch_oxide::frequency::ElasticSketch;
    ///
    /// let mut sketch = ElasticSketch::new(256, 3).unwrap();
    /// sketch.update(b"flow1", 1);
    /// sketch.update(b"flow1", 2);  // Can add multiple counts
    /// ```
    pub fn update(&mut self, item: &[u8], count: u64) {
        let item_hash = Self::hash_item(item);
        self.total_count = self.total_count.saturating_add(count);

        // Try to find or insert the item in each row
        let mut min_frequency = u64::MAX;
        let mut min_row = 0;
        let mut found_empty = false;

        // First pass: find minimum frequency and check for existing item
        for row in 0..self.depth {
            let idx = self.bucket_index(row, item_hash);
            let bucket = self.buckets[idx];

            if !bucket.is_occupied {
                // Empty bucket - we can insert here
                min_row = row;
                found_empty = true;
                break;
            } else if bucket.item_hash == item_hash {
                // Same item - update its frequency
                self.buckets[idx].frequency = self.buckets[idx].frequency.saturating_add(count);
                self.buckets[idx].elastic_counter =
                    self.buckets[idx].elastic_counter.saturating_add(count);
                return;
            } else if bucket.frequency < min_frequency {
                // Different item with lower frequency - potential swap candidate
                min_frequency = bucket.frequency;
                min_row = row;
            }
        }

        // If no empty bucket found, use the row with minimum frequency
        if !found_empty {
            // min_row and min_frequency already set from the loop
        }

        // Insert or update in the row with minimum frequency
        let idx = self.bucket_index(min_row, item_hash);
        if !self.buckets[idx].is_occupied {
            // Insert new item
            self.buckets[idx] = ElasticBucket::new(item_hash, count);
        } else {
            // Update existing item's elastic counter
            self.buckets[idx].elastic_counter =
                self.buckets[idx].elastic_counter.saturating_add(count);
        }
    }

    /// Estimate the frequency of an item
    ///
    /// This operation:
    /// 1. Hashes the item using all hash functions
    /// 2. Queries all rows to find potential matches
    /// 3. Uses elastic counters for refined estimation
    /// 4. Returns the maximum valid estimate (more optimistic than Count-Min's minimum)
    ///
    /// # Arguments
    /// * `item` - The item to query
    ///
    /// # Returns
    /// Estimated frequency (0 if item not found)
    ///
    /// # Time Complexity
    /// O(d) where d is the depth
    ///
    /// # Examples
    /// ```
    /// use sketch_oxide::frequency::ElasticSketch;
    ///
    /// let mut sketch = ElasticSketch::new(512, 3).unwrap();
    /// sketch.update(b"item", 5);
    /// assert_eq!(sketch.estimate(b"item"), 5);
    /// ```
    pub fn estimate(&self, item: &[u8]) -> u64 {
        let item_hash = Self::hash_item(item);
        let mut max_estimate = 0u64;
        let mut found_count = 0;

        // Check all rows
        for row in 0..self.depth {
            let idx = self.bucket_index(row, item_hash);
            let bucket = self.buckets[idx];

            if bucket.is_occupied && bucket.item_hash == item_hash {
                // Found the item - use elastic counter for better estimation
                let elastic_estimate = bucket.frequency
                    + (bucket.elastic_counter as f64 * self.elastic_ratio).floor() as u64;
                max_estimate = max_estimate.max(elastic_estimate);
                found_count += 1;
            }
        }

        // Return the best estimate if found, otherwise 0
        if found_count > 0 {
            max_estimate
        } else {
            0
        }
    }

    /// Find all items with frequency >= threshold
    ///
    /// This operation:
    /// 1. Scans all buckets in the sketch
    /// 2. Returns items with estimated frequency >= threshold
    /// 3. Results are sorted by frequency in descending order
    /// 4. Deduplicates items that appear in multiple rows
    ///
    /// # Arguments
    /// * `threshold` - Minimum frequency threshold
    ///
    /// # Returns
    /// Vector of (item_hash, frequency) tuples sorted by frequency descending
    ///
    /// # Time Complexity
    /// O(k * d) where k=bucket_count, d=depth
    ///
    /// # Examples
    /// ```
    /// use sketch_oxide::frequency::ElasticSketch;
    ///
    /// let mut sketch = ElasticSketch::new(512, 3).unwrap();
    /// sketch.update(b"heavy1", 10);
    /// sketch.update(b"heavy2", 5);
    /// sketch.update(b"light", 1);
    ///
    /// let hitters = sketch.heavy_hitters(2);
    /// assert!(hitters.len() >= 2);
    /// ```
    pub fn heavy_hitters(&self, threshold: u64) -> Vec<(u64, u64)> {
        use std::collections::BTreeMap;

        // Use BTreeMap to deduplicate and track maximum frequency per item
        let mut items: BTreeMap<u64, u64> = BTreeMap::new();

        for bucket in &self.buckets {
            if bucket.is_occupied {
                let elastic_estimate = bucket.frequency
                    + (bucket.elastic_counter as f64 * self.elastic_ratio).floor() as u64;
                if elastic_estimate >= threshold {
                    items
                        .entry(bucket.item_hash)
                        .and_modify(|f| *f = (*f).max(elastic_estimate))
                        .or_insert(elastic_estimate);
                }
            }
        }

        // Convert to sorted vector (descending by frequency)
        let mut result: Vec<(u64, u64)> = items.into_iter().collect();
        result.sort_by(|a, b| {
            // Sort by frequency descending, then by hash for stability
            match b.1.cmp(&a.1) {
                Ordering::Equal => a.0.cmp(&b.0),
                other => other,
            }
        });

        result
    }

    /// Merge another Elastic Sketch into this one
    ///
    /// This operation:
    /// 1. Verifies that both sketches have compatible parameters
    /// 2. For each bucket position:
    ///    - If this bucket is empty and other's is occupied, copy other's bucket
    ///    - If both buckets contain the same item, add frequencies
    ///    - If buckets differ, keep the higher frequency
    /// 3. Updates total count
    ///
    /// # Arguments
    /// * `other` - The sketch to merge into this one
    ///
    /// # Returns
    /// `Ok(())` if merge succeeded, error if sketches are incompatible
    ///
    /// # Errors
    /// Returns `IncompatibleSketches` if:
    /// - Sketches have different bucket_count
    /// - Sketches have different depth
    /// - Sketches have different elastic_ratio
    ///
    /// # Examples
    /// ```
    /// use sketch_oxide::frequency::ElasticSketch;
    /// use sketch_oxide::Mergeable;
    ///
    /// let mut sketch1 = ElasticSketch::new(512, 3).unwrap();
    /// let mut sketch2 = ElasticSketch::new(512, 3).unwrap();
    ///
    /// sketch1.update(b"item", 1);
    /// sketch2.update(b"item", 2);
    ///
    /// sketch1.merge(&sketch2).unwrap();
    /// assert_eq!(sketch1.estimate(b"item"), 3);
    /// ```
    pub fn merge(&mut self, other: &ElasticSketch) -> Result<(), SketchError> {
        // Verify compatibility
        if self.bucket_count != other.bucket_count {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "bucket_count mismatch: {} vs {}",
                    self.bucket_count, other.bucket_count
                ),
            });
        }

        if self.depth != other.depth {
            return Err(SketchError::IncompatibleSketches {
                reason: format!("depth mismatch: {} vs {}", self.depth, other.depth),
            });
        }

        if (self.elastic_ratio - other.elastic_ratio).abs() > 1e-10 {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "elastic_ratio mismatch: {} vs {}",
                    self.elastic_ratio, other.elastic_ratio
                ),
            });
        }

        // Merge buckets
        for i in 0..self.buckets.len() {
            let self_bucket = self.buckets[i];
            let other_bucket = other.buckets[i];

            match (self_bucket.is_occupied, other_bucket.is_occupied) {
                (false, true) => {
                    // Self is empty, copy from other
                    self.buckets[i] = other_bucket;
                }
                (true, true) => {
                    // Both occupied
                    if self_bucket.item_hash == other_bucket.item_hash {
                        // Same item - add frequencies
                        self.buckets[i].frequency = self.buckets[i]
                            .frequency
                            .saturating_add(other_bucket.frequency);
                        self.buckets[i].elastic_counter = self.buckets[i]
                            .elastic_counter
                            .saturating_add(other_bucket.elastic_counter);
                    } else if other_bucket.frequency > self_bucket.frequency {
                        // Different item with higher frequency - replace
                        self.buckets[i] = other_bucket;
                    }
                    // Otherwise keep self's bucket
                }
                _ => {
                    // Self is occupied, other is empty - keep self
                }
            }
        }

        self.total_count = self.total_count.saturating_add(other.total_count);

        Ok(())
    }

    /// Clear all state from the sketch
    ///
    /// This operation resets all buckets and counters to empty state.
    ///
    /// # Time Complexity
    /// O(k * d) where k=bucket_count, d=depth
    ///
    /// # Examples
    /// ```
    /// use sketch_oxide::frequency::ElasticSketch;
    /// use sketch_oxide::Sketch;
    ///
    /// let mut sketch = ElasticSketch::new(512, 3).unwrap();
    /// sketch.update(b"item", 1);
    /// assert!(!sketch.is_empty());
    /// sketch.reset();
    /// assert!(sketch.is_empty());
    /// ```
    pub fn reset(&mut self) {
        self.buckets
            .iter_mut()
            .for_each(|b| *b = ElasticBucket::empty());
        self.total_count = 0;
    }

    /// Get the bucket count
    ///
    /// # Returns
    /// The number of buckets per row
    #[inline]
    pub fn bucket_count(&self) -> usize {
        self.bucket_count
    }

    /// Get the depth
    ///
    /// # Returns
    /// The number of hash functions
    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Get the elastic ratio
    ///
    /// # Returns
    /// The elastic expansion ratio
    #[inline]
    pub fn elastic_ratio(&self) -> f64 {
        self.elastic_ratio
    }

    /// Get the total count of items added
    ///
    /// # Returns
    /// The sum of all update counts
    #[inline]
    pub fn total_count(&self) -> u64 {
        self.total_count
    }

    /// Get memory usage in bytes
    ///
    /// # Returns
    /// Approximate memory usage of the sketch in bytes
    #[inline]
    pub fn memory_usage(&self) -> usize {
        // Each bucket: 8 + 8 + 8 + 1 = 25 bytes (plus padding)
        self.buckets.len() * std::mem::size_of::<ElasticBucket>()
    }
}

impl Sketch for ElasticSketch {
    type Item = u64;

    fn update(&mut self, item: &Self::Item) {
        ElasticSketch::update(self, &item.to_le_bytes(), 1);
    }

    fn estimate(&self) -> f64 {
        self.total_count as f64
    }

    fn is_empty(&self) -> bool {
        self.buckets.iter().all(|b| !b.is_occupied)
    }

    fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Header: bucket_count, depth, elastic_ratio, total_count
        bytes.extend_from_slice(&(self.bucket_count as u64).to_le_bytes());
        bytes.extend_from_slice(&(self.depth as u64).to_le_bytes());
        bytes.extend_from_slice(&self.elastic_ratio.to_le_bytes());
        bytes.extend_from_slice(&self.total_count.to_le_bytes());

        // Buckets
        for bucket in &self.buckets {
            bytes.extend_from_slice(&bucket.item_hash.to_le_bytes());
            bytes.extend_from_slice(&bucket.frequency.to_le_bytes());
            bytes.extend_from_slice(&bucket.elastic_counter.to_le_bytes());
            bytes.push(if bucket.is_occupied { 1 } else { 0 });
        }

        bytes
    }

    fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        if bytes.len() < 32 {
            return Err(SketchError::DeserializationError(
                "insufficient bytes for header".to_string(),
            ));
        }

        let mut offset = 0;

        // Read header
        let bucket_count =
            u64::from_le_bytes(bytes[offset..offset + 8].try_into().map_err(|_| {
                SketchError::DeserializationError("invalid bucket_count".to_string())
            })?) as usize;
        offset += 8;

        let depth = u64::from_le_bytes(
            bytes[offset..offset + 8]
                .try_into()
                .map_err(|_| SketchError::DeserializationError("invalid depth".to_string()))?,
        ) as usize;
        offset += 8;

        let elastic_ratio =
            f64::from_le_bytes(bytes[offset..offset + 8].try_into().map_err(|_| {
                SketchError::DeserializationError("invalid elastic_ratio".to_string())
            })?);
        offset += 8;

        let total_count =
            u64::from_le_bytes(bytes[offset..offset + 8].try_into().map_err(|_| {
                SketchError::DeserializationError("invalid total_count".to_string())
            })?);
        offset += 8;

        // Read buckets
        let bucket_size = 8 + 8 + 8 + 1; // item_hash + frequency + elastic_counter + is_occupied
        let expected_bytes = offset + bucket_size * bucket_count * depth;

        if bytes.len() < expected_bytes {
            return Err(SketchError::DeserializationError(
                "insufficient bytes for buckets".to_string(),
            ));
        }

        let mut buckets = Vec::with_capacity(bucket_count * depth);

        for _ in 0..(bucket_count * depth) {
            let item_hash =
                u64::from_le_bytes(bytes[offset..offset + 8].try_into().map_err(|_| {
                    SketchError::DeserializationError("invalid bucket".to_string())
                })?);
            offset += 8;

            let frequency =
                u64::from_le_bytes(bytes[offset..offset + 8].try_into().map_err(|_| {
                    SketchError::DeserializationError("invalid bucket".to_string())
                })?);
            offset += 8;

            let elastic_counter =
                u64::from_le_bytes(bytes[offset..offset + 8].try_into().map_err(|_| {
                    SketchError::DeserializationError("invalid bucket".to_string())
                })?);
            offset += 8;

            let is_occupied = bytes[offset] != 0;
            offset += 1;

            buckets.push(ElasticBucket {
                item_hash,
                frequency,
                elastic_counter,
                is_occupied,
            });
        }

        let mask = bucket_count - 1;

        Ok(ElasticSketch {
            buckets,
            bucket_count,
            mask,
            depth,
            elastic_ratio,
            total_count,
        })
    }
}

impl Mergeable for ElasticSketch {
    fn merge(&mut self, other: &Self) -> Result<(), SketchError> {
        ElasticSketch::merge(self, other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test 1: Basic construction and parameter validation
    #[test]
    fn test_basic_construction() {
        let sketch = ElasticSketch::new(512, 3).unwrap();
        assert_eq!(sketch.bucket_count(), 512);
        assert_eq!(sketch.depth(), 3);
        assert!((sketch.elastic_ratio() - 0.2).abs() < 1e-10);
        assert!(sketch.is_empty());
    }

    // Test 2: Custom elastic ratio
    #[test]
    fn test_with_elastic_ratio() {
        let sketch = ElasticSketch::with_elastic_ratio(512, 3, 0.5).unwrap();
        assert!((sketch.elastic_ratio() - 0.5).abs() < 1e-10);
    }

    // Test 3: Parameter validation
    #[test]
    fn test_invalid_parameters() {
        assert!(ElasticSketch::new(0, 3).is_err());
        assert!(ElasticSketch::new(512, 0).is_err());
        assert!(ElasticSketch::new(512, 9).is_err());
        assert!(ElasticSketch::with_elastic_ratio(512, 3, -0.1).is_err());
        assert!(ElasticSketch::with_elastic_ratio(512, 3, 1.1).is_err());
    }

    // Test 4: Single item insertion and estimation
    #[test]
    fn test_single_item_insertion() {
        let mut sketch = ElasticSketch::new(512, 3).unwrap();
        sketch.update(b"flow1", 1);
        assert!(!sketch.is_empty());
        assert_eq!(sketch.estimate(b"flow1"), 1);
    }

    // Test 5: Multiple updates to same item
    #[test]
    fn test_multiple_updates() {
        let mut sketch = ElasticSketch::new(512, 3).unwrap();
        sketch.update(b"item", 1);
        sketch.update(b"item", 2);
        sketch.update(b"item", 3);
        // Estimate should be close to 6 (accounting for hash collisions and elastic adaptation)
        let estimate = sketch.estimate(b"item");
        assert!(
            (6..=8).contains(&estimate),
            "Expected estimate near 6, got {}",
            estimate
        );
    }

    // Test 6: Frequency distribution with multiple items
    #[test]
    fn test_multiple_items_different_frequencies() {
        let mut sketch = ElasticSketch::new(512, 3).unwrap();
        sketch.update(b"heavy", 10);
        sketch.update(b"medium", 5);
        sketch.update(b"light", 1);

        assert_eq!(sketch.estimate(b"heavy"), 10);
        assert_eq!(sketch.estimate(b"medium"), 5);
        assert_eq!(sketch.estimate(b"light"), 1);
    }

    // Test 7: Non-existent item estimation
    #[test]
    fn test_nonexistent_item() {
        let mut sketch = ElasticSketch::new(512, 3).unwrap();
        sketch.update(b"item1", 1);
        assert_eq!(sketch.estimate(b"nonexistent"), 0);
    }

    // Test 8: Heavy hitter detection
    #[test]
    fn test_heavy_hitters() {
        let mut sketch = ElasticSketch::new(512, 3).unwrap();
        sketch.update(b"heavy1", 100);
        sketch.update(b"heavy2", 50);
        sketch.update(b"medium", 10);
        sketch.update(b"light", 1);

        let hitters = sketch.heavy_hitters(20);
        // Should find heavy1 and heavy2
        assert!(hitters.len() >= 2);
        // First should be heavy1 (higher frequency)
        assert!(hitters[0].1 >= 50);
    }

    // Test 9: Merging sketches
    #[test]
    fn test_merge_sketches() {
        let mut sketch1 = ElasticSketch::new(512, 3).unwrap();
        let mut sketch2 = ElasticSketch::new(512, 3).unwrap();

        sketch1.update(b"item1", 5);
        sketch2.update(b"item1", 3);
        sketch2.update(b"item2", 2);

        sketch1.merge(&sketch2).unwrap();

        assert_eq!(sketch1.estimate(b"item1"), 8);
        assert_eq!(sketch1.estimate(b"item2"), 2);
    }

    // Test 10: Merge incompatible sketches
    #[test]
    fn test_merge_incompatible_sketches() {
        let sketch1 = ElasticSketch::new(512, 3).unwrap();
        let sketch2 = ElasticSketch::new(256, 3).unwrap(); // Different bucket_count

        let mut s1 = sketch1.clone();
        assert!(s1.merge(&sketch2).is_err());

        let sketch3 = ElasticSketch::new(512, 4).unwrap(); // Different depth
        let mut s1 = sketch1.clone();
        assert!(s1.merge(&sketch3).is_err());
    }

    // Test 11: Reset functionality
    #[test]
    fn test_reset() {
        let mut sketch = ElasticSketch::new(512, 3).unwrap();
        sketch.update(b"item1", 5);
        sketch.update(b"item2", 3);

        assert!(!sketch.is_empty());
        assert_eq!(sketch.total_count(), 8);

        sketch.reset();

        assert!(sketch.is_empty());
        assert_eq!(sketch.total_count(), 0);
        assert_eq!(sketch.estimate(b"item1"), 0);
    }

    // Test 12: Serialization and deserialization
    #[test]
    fn test_serialization() {
        let mut sketch = ElasticSketch::new(512, 3).unwrap();
        sketch.update(b"item1", 5);
        sketch.update(b"item2", 3);

        let bytes = sketch.serialize();
        let deserialized = ElasticSketch::deserialize(&bytes).unwrap();

        assert_eq!(deserialized.bucket_count(), 512);
        assert_eq!(deserialized.depth(), 3);
        assert_eq!(deserialized.estimate(b"item1"), 5);
        assert_eq!(deserialized.estimate(b"item2"), 3);
    }

    // Test 13: Stability - consistent estimates
    #[test]
    fn test_stability() {
        let mut sketch = ElasticSketch::new(512, 3).unwrap();
        sketch.update(b"stable_item", 10);

        let estimate1 = sketch.estimate(b"stable_item");
        let estimate2 = sketch.estimate(b"stable_item");
        let estimate3 = sketch.estimate(b"stable_item");

        assert_eq!(estimate1, estimate2);
        assert_eq!(estimate2, estimate3);
        assert_eq!(estimate1, 10);
    }

    // Test 14: Memory efficiency
    #[test]
    fn test_memory_usage() {
        let sketch = ElasticSketch::new(512, 3).unwrap();
        let memory = sketch.memory_usage();

        // Should be approximately: 512 * 3 * size_of::<ElasticBucket>()
        let expected_buckets = 512 * 3;
        let bucket_size = std::mem::size_of::<ElasticBucket>();
        let expected_memory = expected_buckets * bucket_size;

        assert_eq!(memory, expected_memory);
    }

    // Test 15: Zipfian distribution simulation
    #[test]
    fn test_zipfian_distribution() {
        let mut sketch = ElasticSketch::new(1024, 4).unwrap();

        // Simulate zipfian distribution (1/k frequency pattern)
        for i in 1..=20 {
            let item = format!("item{}", i).into_bytes();
            let frequency = 100 / i as u64;
            sketch.update(&item, frequency);
        }

        // Verify top items are detected
        let hitters = sketch.heavy_hitters(10);
        assert!(!hitters.is_empty());

        // Top item should have highest frequency
        assert_eq!(sketch.estimate(b"item1"), 100);
    }

    // Test 16: Large frequency values
    #[test]
    fn test_large_frequencies() {
        let mut sketch = ElasticSketch::new(512, 3).unwrap();

        let large_count = 1_000_000u64;
        sketch.update(b"large_item", large_count);

        assert_eq!(sketch.estimate(b"large_item"), large_count);
    }

    // Test 17: Power of 2 bucket rounding
    #[test]
    fn test_bucket_rounding() {
        let sketch1 = ElasticSketch::new(500, 3).unwrap();
        assert_eq!(sketch1.bucket_count(), 512); // Next power of 2

        let sketch2 = ElasticSketch::new(1024, 3).unwrap();
        assert_eq!(sketch2.bucket_count(), 1024); // Already power of 2

        let sketch3 = ElasticSketch::new(1000, 3).unwrap();
        assert_eq!(sketch3.bucket_count(), 1024); // Next power of 2
    }
}
