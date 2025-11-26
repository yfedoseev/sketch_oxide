//! Space-Saving Sketch for Heavy Hitters Detection
//!
//! The Space-Saving algorithm (Metwally et al., 2005) is a deterministic streaming algorithm
//! for finding the most frequent items (heavy hitters) in a data stream. It provides:
//! - Guaranteed no false negatives: All items with frequency > epsilon*N will be detected
//! - Bounded error: Each count estimate has an associated error bound
//! - Space efficient: Uses O(1/epsilon) counters
//!
//! # Algorithm
//!
//! The algorithm maintains at most k = ceil(1/epsilon) item-count pairs. When a new item arrives:
//! 1. If the item is already tracked, increment its count
//! 2. If there's room for a new item, add it with count 1
//! 3. Otherwise, replace the item with minimum count:
//!    - Set the new item's count to min_count + 1
//!    - Set the error to min_count (the count of the replaced item)
//!
//! The error represents the maximum overestimation: true_count in [count - error, count]
//!
//! # Key Properties
//!
//! - **No false negatives**: Any item with true frequency > N/k will be in the sketch
//! - **Bounded error**: For any tracked item, error <= N/k where N is stream length
//! - **Deterministic**: Unlike Count-Min Sketch, Space-Saving is deterministic
//!
//! # References
//!
//! - Metwally, A., Agrawal, D., & El Abbadi, A. (2005). "Efficient computation of
//!   frequent and top-k elements in data streams"
//! - Cormode, G., & Hadjieleftheriou, M. (2008). "Finding frequent items in data streams"
//!
//! # Examples
//!
//! ```
//! use sketch_oxide::frequency::SpaceSaving;
//!
//! // Create a sketch that guarantees detection of items with > 10% frequency
//! let mut sketch = SpaceSaving::new(0.1).unwrap();
//!
//! // Add items
//! for _ in 0..100 {
//!     sketch.update("frequent".to_string());
//! }
//! for _ in 0..5 {
//!     sketch.update("rare".to_string());
//! }
//!
//! // Query estimates - returns (lower_bound, upper_bound)
//! if let Some((lower, upper)) = sketch.estimate(&"frequent".to_string()) {
//!     assert!(lower <= 100 && upper >= 100);
//! }
//!
//! // Get heavy hitters (items appearing more than threshold fraction)
//! let heavy = sketch.heavy_hitters(0.05);
//! assert!(!heavy.is_empty());
//! ```

use crate::common::{Mergeable, Sketch, SketchError};
use std::collections::HashMap;
use std::hash::Hash;

/// Space-Saving Sketch for finding heavy hitters in a data stream
///
/// This sketch tracks the most frequent items while providing error bounds
/// for each estimate. It guarantees that any item with true frequency greater
/// than epsilon * stream_length will be tracked.
///
/// # Type Parameters
///
/// - `T`: The item type, must implement `Hash`, `Eq`, and `Clone`
///
/// # Space Complexity
///
/// O(1/epsilon) - stores at most ceil(1/epsilon) items
///
/// # Time Complexity
///
/// - Update: O(k) worst case to find minimum (can be optimized with heap)
/// - Query: O(1) for hash lookup
/// - Heavy hitters: O(k log k) for sorting
#[derive(Debug, Clone)]
pub struct SpaceSaving<T: Hash + Eq + Clone> {
    /// Maximum number of items to track: k = ceil(1/epsilon)
    capacity: usize,
    /// Map of items to (count, error) pairs
    /// - count: estimated frequency (may overestimate)
    /// - error: maximum overestimation amount
    items: HashMap<T, (u64, u64)>,
    /// Total number of items seen in the stream
    stream_length: u64,
    /// Epsilon parameter for error bound
    epsilon: f64,
}

impl<T: Hash + Eq + Clone> SpaceSaving<T> {
    /// Creates a new Space-Saving sketch with the specified error bound
    ///
    /// # Arguments
    ///
    /// * `epsilon` - Error bound in (0, 1). Items with frequency > epsilon*N are guaranteed
    ///   to be tracked. Smaller epsilon means more accuracy but more space.
    ///
    /// # Returns
    ///
    /// A new `SpaceSaving` sketch or an error if parameters are invalid.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if epsilon is not in (0, 1).
    ///
    /// # Space
    ///
    /// The sketch uses ceil(1/epsilon) counters. For example:
    /// - epsilon = 0.1 -> 10 counters
    /// - epsilon = 0.01 -> 100 counters
    /// - epsilon = 0.001 -> 1000 counters
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::frequency::SpaceSaving;
    ///
    /// // Track items with > 1% frequency
    /// let sketch: SpaceSaving<String> = SpaceSaving::new(0.01).unwrap();
    /// ```
    pub fn new(epsilon: f64) -> Result<Self, SketchError> {
        if epsilon <= 0.0 || epsilon >= 1.0 {
            return Err(SketchError::InvalidParameter {
                param: "epsilon".to_string(),
                value: epsilon.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }

        let capacity = (1.0 / epsilon).ceil() as usize;
        // Ensure minimum capacity of 2
        let capacity = capacity.max(2);

        Ok(Self {
            capacity,
            items: HashMap::with_capacity(capacity),
            stream_length: 0,
            epsilon,
        })
    }

    /// Creates a new Space-Saving sketch with explicit capacity
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of items to track (must be >= 2)
    ///
    /// # Returns
    ///
    /// A new `SpaceSaving` sketch or an error if parameters are invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::frequency::SpaceSaving;
    ///
    /// let sketch: SpaceSaving<u64> = SpaceSaving::with_capacity(100).unwrap();
    /// ```
    pub fn with_capacity(capacity: usize) -> Result<Self, SketchError> {
        if capacity < 2 {
            return Err(SketchError::InvalidParameter {
                param: "capacity".to_string(),
                value: capacity.to_string(),
                constraint: "must be >= 2".to_string(),
            });
        }

        let epsilon = 1.0 / capacity as f64;

        Ok(Self {
            capacity,
            items: HashMap::with_capacity(capacity),
            stream_length: 0,
            epsilon,
        })
    }

    /// Updates the sketch with a single occurrence of an item
    ///
    /// # Arguments
    ///
    /// * `item` - The item to add to the stream
    ///
    /// # Algorithm
    ///
    /// 1. If item is already tracked: increment its count
    /// 2. Else if capacity not reached: add item with count=1, error=0
    /// 3. Else: replace minimum-count item with new item, setting error=min_count
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::frequency::SpaceSaving;
    ///
    /// let mut sketch = SpaceSaving::new(0.1).unwrap();
    /// sketch.update("hello".to_string());
    /// sketch.update("world".to_string());
    /// sketch.update("hello".to_string());
    /// ```
    #[inline]
    pub fn update(&mut self, item: T) {
        self.stream_length += 1;

        // Case 1: Item already tracked - increment count
        if let Some((count, _error)) = self.items.get_mut(&item) {
            *count += 1;
            return;
        }

        // Case 2: Space available - add new item with count=1, error=0
        if self.items.len() < self.capacity {
            self.items.insert(item, (1, 0));
            return;
        }

        // Case 3: At capacity - replace minimum count item
        // Find item with minimum count
        let min_count = self
            .items
            .values()
            .map(|(count, _)| *count)
            .min()
            .unwrap_or(0);

        // Find and remove an item with minimum count
        let min_item = self
            .items
            .iter()
            .find(|(_, (count, _))| *count == min_count)
            .map(|(k, _)| k.clone());

        if let Some(old_item) = min_item {
            self.items.remove(&old_item);
            // Insert new item with count = min_count + 1, error = min_count
            self.items.insert(item, (min_count + 1, min_count));
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
    /// - `Some((lower_bound, upper_bound))` if the item is tracked
    /// - `None` if the item is not currently tracked
    ///
    /// The true frequency is guaranteed to be in [lower_bound, upper_bound]:
    /// - lower_bound = count - error (never negative)
    /// - upper_bound = count
    ///
    /// If an item is not tracked, its true frequency is at most N/k where
    /// N is stream_length and k is capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::frequency::SpaceSaving;
    ///
    /// let mut sketch = SpaceSaving::new(0.1).unwrap();
    /// for _ in 0..42 {
    ///     sketch.update("test".to_string());
    /// }
    ///
    /// if let Some((lower, upper)) = sketch.estimate(&"test".to_string()) {
    ///     assert!(lower <= 42 && upper >= 42);
    /// }
    /// ```
    #[inline]
    pub fn estimate(&self, item: &T) -> Option<(u64, u64)> {
        self.items.get(item).map(|&(count, error)| {
            let lower = count.saturating_sub(error);
            let upper = count;
            (lower, upper)
        })
    }

    /// Returns all items that may be heavy hitters
    ///
    /// A heavy hitter is an item whose true frequency exceeds `threshold * stream_length`.
    ///
    /// # Arguments
    ///
    /// * `threshold` - Frequency threshold in (0, 1)
    ///
    /// # Returns
    ///
    /// A vector of tuples: (item, lower_bound, upper_bound)
    /// - Sorted by upper bound (estimated count) in descending order
    /// - All items whose upper bound exceeds the threshold are included
    ///
    /// # Guarantees
    ///
    /// - **No false negatives**: Any item with true frequency > threshold*N will be included
    ///   (if threshold >= epsilon)
    /// - **Possible false positives**: Some returned items may have true frequency below threshold
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::frequency::SpaceSaving;
    ///
    /// let mut sketch = SpaceSaving::new(0.01).unwrap();
    ///
    /// // Add items with different frequencies
    /// for _ in 0..100 { sketch.update("common".to_string()); }
    /// for _ in 0..10 { sketch.update("rare".to_string()); }
    ///
    /// // Get items appearing more than 5% of the time
    /// let heavy = sketch.heavy_hitters(0.05);
    /// ```
    pub fn heavy_hitters(&self, threshold: f64) -> Vec<(T, u64, u64)> {
        let min_count = (threshold * self.stream_length as f64).ceil() as u64;

        let mut result: Vec<_> = self
            .items
            .iter()
            .filter_map(|(item, &(count, error))| {
                // Include if upper bound (count) exceeds threshold
                // This ensures no false negatives
                if count >= min_count {
                    let lower = count.saturating_sub(error);
                    Some((item.clone(), lower, count))
                } else {
                    None
                }
            })
            .collect();

        // Sort by upper bound (count) descending
        result.sort_by(|a, b| b.2.cmp(&a.2));
        result
    }

    /// Returns the top-k most frequent items
    ///
    /// # Arguments
    ///
    /// * `k` - Number of items to return
    ///
    /// # Returns
    ///
    /// A vector of at most k tuples: (item, lower_bound, upper_bound)
    /// sorted by upper bound descending.
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::frequency::SpaceSaving;
    ///
    /// let mut sketch = SpaceSaving::new(0.01).unwrap();
    /// for i in 0..100u64 {
    ///     for _ in 0..i { sketch.update(i); }
    /// }
    ///
    /// let top10 = sketch.top_k(10);
    /// assert!(top10.len() <= 10);
    /// ```
    pub fn top_k(&self, k: usize) -> Vec<(T, u64, u64)> {
        let mut result: Vec<_> = self
            .items
            .iter()
            .map(|(item, &(count, error))| {
                let lower = count.saturating_sub(error);
                (item.clone(), lower, count)
            })
            .collect();

        // Sort by upper bound (count) descending
        result.sort_by(|a, b| b.2.cmp(&a.2));
        result.truncate(k);
        result
    }

    /// Returns the capacity (maximum number of items tracked)
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the total number of items seen in the stream
    #[inline]
    pub fn stream_length(&self) -> u64 {
        self.stream_length
    }

    /// Returns the epsilon parameter
    #[inline]
    pub fn epsilon(&self) -> f64 {
        self.epsilon
    }

    /// Returns the number of items currently tracked
    #[inline]
    pub fn num_items(&self) -> usize {
        self.items.len()
    }

    /// Returns whether the sketch is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the maximum possible error for any item
    ///
    /// For tracked items, their individual error may be lower.
    /// For untracked items, their true count is at most this value.
    #[inline]
    pub fn max_error(&self) -> u64 {
        (self.stream_length as f64 * self.epsilon).ceil() as u64
    }
}

impl<T: Hash + Eq + Clone> SpaceSaving<T> {
    /// Merges another Space-Saving sketch into this one
    ///
    /// The merge operation combines two sketches while maintaining error bounds.
    /// After merging, error bounds are conservatively adjusted.
    ///
    /// # Arguments
    ///
    /// * `other` - The sketch to merge
    ///
    /// # Errors
    ///
    /// Returns `IncompatibleSketches` if the sketches have different capacities.
    ///
    /// # Algorithm
    ///
    /// 1. Add counts for items in both sketches
    /// 2. For items only in one sketch, add the other's max_error to their error
    /// 3. Reduce to capacity by removing items with smallest counts
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::frequency::SpaceSaving;
    ///
    /// let mut sketch1: SpaceSaving<String> = SpaceSaving::new(0.1).unwrap();
    /// let mut sketch2: SpaceSaving<String> = SpaceSaving::new(0.1).unwrap();
    ///
    /// sketch1.update("a".to_string());
    /// sketch2.update("b".to_string());
    ///
    /// sketch1.merge(&sketch2).unwrap();
    /// ```
    pub fn merge(&mut self, other: &Self) -> Result<(), SketchError> {
        if self.capacity != other.capacity {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "capacity mismatch: {} vs {} (different epsilon parameters)",
                    self.capacity, other.capacity
                ),
            });
        }

        // Calculate max errors for conservative bounds
        let other_max_error = other.max_error();
        let self_max_error = self.max_error();

        // Merge items from other into self
        for (item, &(other_count, other_error)) in &other.items {
            if let Some((count, error)) = self.items.get_mut(item) {
                // Item in both: sum counts, sum errors
                *count += other_count;
                *error += other_error;
            } else {
                // Item only in other: add self's max_error to account for possible missed counts
                let new_error = other_error + self_max_error;
                self.items.insert(item.clone(), (other_count, new_error));
            }
        }

        // For items only in self, add other's max_error
        for (item, (count, error)) in self.items.iter_mut() {
            if !other.items.contains_key(item) {
                *error += other_max_error;
                // Ensure error doesn't exceed count
                *error = (*error).min(*count);
            }
        }

        // Update stream length
        self.stream_length += other.stream_length;

        // Reduce to capacity if needed
        while self.items.len() > self.capacity {
            // Find and remove item with minimum count
            let min_item = self
                .items
                .iter()
                .min_by_key(|(_, (count, _))| *count)
                .map(|(k, _)| k.clone());

            if let Some(item) = min_item {
                self.items.remove(&item);
            } else {
                break;
            }
        }

        Ok(())
    }
}

// Implement the Sketch trait
impl<T: Hash + Eq + Clone + 'static> Sketch for SpaceSaving<T> {
    type Item = T;

    fn update(&mut self, item: &Self::Item) {
        SpaceSaving::update(self, item.clone());
    }

    fn estimate(&self) -> f64 {
        // Return the count of the most frequent item, or 0 if empty
        self.items
            .values()
            .map(|(count, _)| *count)
            .max()
            .unwrap_or(0) as f64
    }

    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn serialize(&self) -> Vec<u8> {
        // This is a simplified serialization that works for basic types
        // For complex types T, serde feature should be used
        let mut bytes = Vec::new();

        // Header: capacity (8 bytes) + stream_length (8 bytes) + epsilon (8 bytes) + num_items (8 bytes)
        bytes.extend_from_slice(&self.capacity.to_le_bytes());
        bytes.extend_from_slice(&self.stream_length.to_le_bytes());
        bytes.extend_from_slice(&self.epsilon.to_le_bytes());
        bytes.extend_from_slice(&self.items.len().to_le_bytes());

        // Note: Full item serialization requires T to implement serialization
        // This basic implementation only works when items HashMap is empty
        // For production use with arbitrary T, use serde feature

        bytes
    }

    fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        if bytes.len() < 32 {
            return Err(SketchError::DeserializationError(
                "insufficient bytes for header".to_string(),
            ));
        }

        let mut offset = 0;

        let capacity = usize::from_le_bytes(
            bytes[offset..offset + 8]
                .try_into()
                .map_err(|_| SketchError::DeserializationError("invalid capacity".to_string()))?,
        );
        offset += 8;

        let stream_length =
            u64::from_le_bytes(bytes[offset..offset + 8].try_into().map_err(|_| {
                SketchError::DeserializationError("invalid stream_length".to_string())
            })?);
        offset += 8;

        let epsilon = f64::from_le_bytes(
            bytes[offset..offset + 8]
                .try_into()
                .map_err(|_| SketchError::DeserializationError("invalid epsilon".to_string()))?,
        );
        offset += 8;

        let num_items = usize::from_le_bytes(
            bytes[offset..offset + 8]
                .try_into()
                .map_err(|_| SketchError::DeserializationError("invalid num_items".to_string()))?,
        );

        if num_items > 0 {
            return Err(SketchError::DeserializationError(
                "generic deserialization of items not supported; use serde feature".to_string(),
            ));
        }

        Ok(Self {
            capacity,
            items: HashMap::with_capacity(capacity),
            stream_length,
            epsilon,
        })
    }
}

// Implement the Mergeable trait
impl<T: Hash + Eq + Clone + 'static> Mergeable for SpaceSaving<T> {
    fn merge(&mut self, other: &Self) -> Result<(), SketchError> {
        SpaceSaving::merge(self, other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // ==========================================================================
    // TDD Test Cases - Written FIRST
    // ==========================================================================

    /// Test 1: Basic insertion within capacity
    /// Items added when there's space should have count=1, error=0
    #[test]
    fn test_basic_insertion() {
        let mut sketch: SpaceSaving<String> = SpaceSaving::new(0.1).unwrap(); // capacity = 10

        // Insert items within capacity
        sketch.update("apple".to_string());
        sketch.update("banana".to_string());
        sketch.update("cherry".to_string());

        // All items should be tracked
        assert_eq!(sketch.num_items(), 3);

        // Each item should have count=1, error=0
        let (lower, upper) = sketch.estimate(&"apple".to_string()).unwrap();
        assert_eq!(lower, 1);
        assert_eq!(upper, 1);

        let (lower, upper) = sketch.estimate(&"banana".to_string()).unwrap();
        assert_eq!(lower, 1);
        assert_eq!(upper, 1);

        // Updating same item should increment count
        sketch.update("apple".to_string());
        let (lower, upper) = sketch.estimate(&"apple".to_string()).unwrap();
        assert_eq!(lower, 2);
        assert_eq!(upper, 2);
    }

    /// Test 2: Replacement at capacity
    /// When at capacity, new items should replace the minimum count item
    #[test]
    fn test_replacement_at_capacity() {
        // Small capacity for easy testing
        let mut sketch: SpaceSaving<u32> = SpaceSaving::with_capacity(3).unwrap();

        // Fill to capacity
        sketch.update(1);
        sketch.update(2);
        sketch.update(3);
        assert_eq!(sketch.num_items(), 3);

        // All have count=1, error=0
        for i in 1..=3 {
            let (lower, upper) = sketch.estimate(&i).unwrap();
            assert_eq!(lower, 1);
            assert_eq!(upper, 1);
        }

        // Add new item - should replace one of the existing items
        sketch.update(4);

        // Should still have 3 items
        assert_eq!(sketch.num_items(), 3);

        // Item 4 should have count=2 (min_count + 1 = 1 + 1), error=1
        let (lower, upper) = sketch.estimate(&4).unwrap();
        assert_eq!(upper, 2); // count
        assert_eq!(lower, 1); // count - error = 2 - 1

        // One of items 1,2,3 should be gone
        let tracked: HashSet<_> = (1..=4).filter(|i| sketch.estimate(i).is_some()).collect();
        assert_eq!(tracked.len(), 3);
        assert!(tracked.contains(&4));
    }

    /// Test 3: Error bounds correctness
    /// True count should always be within [lower, upper]
    #[test]
    fn test_error_bounds() {
        let mut sketch: SpaceSaving<u32> = SpaceSaving::with_capacity(5).unwrap();

        // Insert known frequencies
        for _ in 0..100 {
            sketch.update(1); // true count = 100
        }
        for _ in 0..50 {
            sketch.update(2); // true count = 50
        }
        for _ in 0..25 {
            sketch.update(3); // true count = 25
        }

        // Check bounds contain true counts
        let (lower1, upper1) = sketch.estimate(&1).unwrap();
        assert!(
            lower1 <= 100 && upper1 >= 100,
            "Item 1: {} <= 100 <= {}",
            lower1,
            upper1
        );

        let (lower2, upper2) = sketch.estimate(&2).unwrap();
        assert!(
            lower2 <= 50 && upper2 >= 50,
            "Item 2: {} <= 50 <= {}",
            lower2,
            upper2
        );

        let (lower3, upper3) = sketch.estimate(&3).unwrap();
        assert!(
            lower3 <= 25 && upper3 >= 25,
            "Item 3: {} <= 25 <= {}",
            lower3,
            upper3
        );
    }

    /// Test 4: No false negatives property
    /// Items with frequency > epsilon*N must be detected
    #[test]
    fn test_no_false_negatives() {
        let epsilon = 0.01; // 1% threshold
        let mut sketch: SpaceSaving<u32> = SpaceSaving::new(epsilon).unwrap();

        let n = 10000u64;

        // Create a heavy hitter with > 1% frequency (let's do 5%)
        let heavy_hitter_count = (n as f64 * 0.05) as u64; // 500 occurrences

        // Insert heavy hitter
        for _ in 0..heavy_hitter_count {
            sketch.update(42);
        }

        // Insert many other items to fill the stream
        for i in 0..(n - heavy_hitter_count) {
            sketch.update(i as u32 + 1000); // Different items
        }

        // Heavy hitter MUST be in the sketch (no false negatives)
        let estimate = sketch.estimate(&42);
        assert!(
            estimate.is_some(),
            "Heavy hitter (5% frequency) must be tracked in sketch with 1% epsilon"
        );

        // It should also appear in heavy_hitters with threshold < its frequency
        let heavy = sketch.heavy_hitters(0.02); // 2% threshold
        let heavy_items: HashSet<_> = heavy.iter().map(|(item, _, _)| *item).collect();
        assert!(
            heavy_items.contains(&42),
            "Heavy hitter must appear in heavy_hitters result"
        );
    }

    /// Test 5: Merge correctness
    /// Merged sketch should maintain error bounds
    #[test]
    fn test_merge_correctness() {
        let mut sketch1: SpaceSaving<u32> = SpaceSaving::with_capacity(10).unwrap();
        let mut sketch2: SpaceSaving<u32> = SpaceSaving::with_capacity(10).unwrap();

        // Insert items into sketch1
        for _ in 0..50 {
            sketch1.update(1);
        }
        for _ in 0..30 {
            sketch1.update(2);
        }

        // Insert items into sketch2
        for _ in 0..40 {
            sketch2.update(1);
        }
        for _ in 0..20 {
            sketch2.update(3);
        }

        // True counts after merge should be:
        // Item 1: 50 + 40 = 90
        // Item 2: 30 + 0 = 30
        // Item 3: 0 + 20 = 20

        sketch1.merge(&sketch2).unwrap();

        // Check merged stream length
        assert_eq!(
            sketch1.stream_length(),
            50 + 30 + 40 + 20,
            "Stream length should be sum of both"
        );

        // Check bounds contain true counts
        let (lower1, upper1) = sketch1.estimate(&1).unwrap();
        assert!(
            lower1 <= 90 && upper1 >= 90,
            "Merged item 1: {} <= 90 <= {}",
            lower1,
            upper1
        );

        // Items 2 and 3 should have bounds that include their true counts
        // Note: they may have additional error from merge
        if let Some((lower2, upper2)) = sketch1.estimate(&2) {
            assert!(
                lower2 <= 30 && upper2 >= 30,
                "Merged item 2: {} <= 30 <= {}",
                lower2,
                upper2
            );
        }

        if let Some((lower3, upper3)) = sketch1.estimate(&3) {
            assert!(
                lower3 <= 20 && upper3 >= 20,
                "Merged item 3: {} <= 20 <= {}",
                lower3,
                upper3
            );
        }
    }

    /// Test 5b: Merge with incompatible sketches should fail
    #[test]
    fn test_merge_incompatible() {
        let mut sketch1: SpaceSaving<u32> = SpaceSaving::with_capacity(10).unwrap();
        let sketch2: SpaceSaving<u32> = SpaceSaving::with_capacity(20).unwrap();

        let result = sketch1.merge(&sketch2);
        assert!(result.is_err());

        if let Err(SketchError::IncompatibleSketches { reason }) = result {
            assert!(reason.contains("capacity mismatch"));
        } else {
            panic!("Expected IncompatibleSketches error");
        }
    }

    /// Test 6: Zipf distribution (realistic heavy-tailed data)
    /// Tests with power-law distribution typical of real-world data
    #[test]
    fn test_zipf_distribution() {
        let mut sketch: SpaceSaving<u32> = SpaceSaving::new(0.01).unwrap();

        // Simulate Zipf distribution: item i has frequency proportional to 1/i
        // Total items = sum(1/i for i in 1..=100) * 1000 ~= 5187
        let mut true_counts = HashMap::new();
        let scale = 1000.0;

        for i in 1..=100u32 {
            let count = (scale / i as f64).ceil() as u64;
            true_counts.insert(i, count);
            for _ in 0..count {
                sketch.update(i);
            }
        }

        // Top items should definitely be tracked
        // Item 1 has ~1000 occurrences, item 2 has ~500, etc.
        for i in 1..=5u32 {
            let true_count = true_counts[&i];
            if let Some((lower, upper)) = sketch.estimate(&i) {
                assert!(
                    lower <= true_count && upper >= true_count,
                    "Zipf item {}: {} <= {} <= {}",
                    i,
                    lower,
                    true_count,
                    upper
                );
            } else {
                // Top items should not be evicted
                panic!("Top Zipf item {} should be tracked", i);
            }
        }

        // Heavy hitters should include top items
        let heavy = sketch.heavy_hitters(0.05);
        assert!(
            !heavy.is_empty(),
            "Should have heavy hitters in Zipf distribution"
        );

        // Item 1 should definitely be a heavy hitter (has ~20% frequency)
        let heavy_items: HashSet<_> = heavy.iter().map(|(item, _, _)| *item).collect();
        assert!(heavy_items.contains(&1), "Item 1 should be a heavy hitter");
    }

    /// Test 7: Serialization round-trip
    #[test]
    fn test_serialization() {
        // Test basic serialization of empty sketch
        let sketch: SpaceSaving<String> = SpaceSaving::new(0.1).unwrap();

        let bytes = sketch.serialize();
        assert!(!bytes.is_empty());

        // For empty sketch, deserialization should work
        let restored: SpaceSaving<String> = SpaceSaving::deserialize(&bytes).unwrap();
        assert_eq!(restored.capacity(), sketch.capacity());
        assert_eq!(restored.stream_length(), sketch.stream_length());
        assert!((restored.epsilon() - sketch.epsilon()).abs() < 1e-10);
    }

    // ==========================================================================
    // Additional tests for edge cases and API completeness
    // ==========================================================================

    #[test]
    fn test_parameter_validation() {
        // Epsilon too small
        assert!(SpaceSaving::<u32>::new(0.0).is_err());
        assert!(SpaceSaving::<u32>::new(-0.1).is_err());

        // Epsilon too large
        assert!(SpaceSaving::<u32>::new(1.0).is_err());
        assert!(SpaceSaving::<u32>::new(1.5).is_err());

        // Valid epsilon
        assert!(SpaceSaving::<u32>::new(0.5).is_ok());
        assert!(SpaceSaving::<u32>::new(0.001).is_ok());

        // Capacity too small
        assert!(SpaceSaving::<u32>::with_capacity(0).is_err());
        assert!(SpaceSaving::<u32>::with_capacity(1).is_err());

        // Valid capacity
        assert!(SpaceSaving::<u32>::with_capacity(2).is_ok());
        assert!(SpaceSaving::<u32>::with_capacity(100).is_ok());
    }

    #[test]
    fn test_empty_sketch() {
        let sketch: SpaceSaving<String> = SpaceSaving::new(0.1).unwrap();

        assert!(sketch.is_empty());
        assert_eq!(sketch.num_items(), 0);
        assert_eq!(sketch.stream_length(), 0);
        assert!(sketch.estimate(&"anything".to_string()).is_none());
        assert!(sketch.heavy_hitters(0.1).is_empty());
        assert!(sketch.top_k(10).is_empty());
    }

    #[test]
    fn test_top_k() {
        let mut sketch: SpaceSaving<u32> = SpaceSaving::with_capacity(20).unwrap();

        // Insert items with varying frequencies
        for i in 1..=10u32 {
            for _ in 0..(i * 10) {
                sketch.update(i);
            }
        }

        // Get top 3
        let top3 = sketch.top_k(3);
        assert_eq!(top3.len(), 3);

        // Should be items 10, 9, 8 (highest frequencies)
        assert_eq!(top3[0].0, 10);
        assert_eq!(top3[1].0, 9);
        assert_eq!(top3[2].0, 8);
    }

    #[test]
    fn test_sketch_trait() {
        let mut sketch: SpaceSaving<u32> = SpaceSaving::new(0.1).unwrap();

        // Use Sketch trait methods
        <SpaceSaving<u32> as Sketch>::update(&mut sketch, &42);
        <SpaceSaving<u32> as Sketch>::update(&mut sketch, &42);
        <SpaceSaving<u32> as Sketch>::update(&mut sketch, &42);

        assert!(!<SpaceSaving<u32> as Sketch>::is_empty(&sketch));
        assert_eq!(<SpaceSaving<u32> as Sketch>::estimate(&sketch), 3.0);
    }

    #[test]
    fn test_mergeable_trait() {
        let mut sketch1: SpaceSaving<u32> = SpaceSaving::new(0.1).unwrap();
        let mut sketch2: SpaceSaving<u32> = SpaceSaving::new(0.1).unwrap();

        sketch1.update(1);
        sketch2.update(1);

        <SpaceSaving<u32> as Mergeable>::merge(&mut sketch1, &sketch2).unwrap();

        assert_eq!(sketch1.stream_length(), 2);
    }

    #[test]
    fn test_max_error() {
        let mut sketch: SpaceSaving<u32> = SpaceSaving::new(0.1).unwrap(); // capacity = 10

        // Insert 1000 items
        for i in 0..1000 {
            sketch.update(i);
        }

        // Max error should be approximately stream_length * epsilon = 1000 * 0.1 = 100
        let max_err = sketch.max_error();
        assert!(
            max_err <= 100 + 1,
            "Max error should be bounded by N * epsilon"
        );
    }

    #[test]
    fn test_capacity_calculation() {
        // epsilon = 0.1 -> capacity = ceil(1/0.1) = 10
        let sketch1: SpaceSaving<u32> = SpaceSaving::new(0.1).unwrap();
        assert_eq!(sketch1.capacity(), 10);

        // epsilon = 0.01 -> capacity = ceil(1/0.01) = 100
        let sketch2: SpaceSaving<u32> = SpaceSaving::new(0.01).unwrap();
        assert_eq!(sketch2.capacity(), 100);

        // epsilon = 0.001 -> capacity = ceil(1/0.001) = 1000
        let sketch3: SpaceSaving<u32> = SpaceSaving::new(0.001).unwrap();
        assert_eq!(sketch3.capacity(), 1000);
    }

    #[test]
    fn test_clone() {
        let mut sketch: SpaceSaving<String> = SpaceSaving::new(0.1).unwrap();
        sketch.update("test".to_string());

        let cloned = sketch.clone();
        assert_eq!(cloned.capacity(), sketch.capacity());
        assert_eq!(cloned.stream_length(), sketch.stream_length());
        assert_eq!(
            cloned.estimate(&"test".to_string()),
            sketch.estimate(&"test".to_string())
        );
    }
}
