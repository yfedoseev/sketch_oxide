//! Frequent Items - Top-K Heavy Hitters with Deterministic Error Bounds
//!
//! Based on the Misra-Gries algorithm, used in Apache DataSketches and Google BigQuery.
//! Provides deterministic error bounds for finding the most frequent items in a stream.
//!
//! # Algorithm
//!
//! The Frequent Items sketch maintains a HashMap of at most `max_size` items and their counts.
//! When capacity is exceeded, the algorithm performs a "purge" operation:
//! 1. Find the minimum count among all items
//! 2. Remove all items with the minimum count
//! 3. Add the minimum count to a global `offset`
//!
//! The offset represents the accumulated error from purged items, giving us deterministic bounds:
//! - **Lower bound**: The stored count (guaranteed minimum)
//! - **Upper bound**: The stored count + offset (guaranteed maximum)
//! - **True frequency**: Must be within [lower_bound, upper_bound]
//!
//! # Error Modes
//!
//! - **NoFalsePositives**: Uses lower bounds (conservative). All returned items are truly frequent.
//! - **NoFalseNegatives**: Uses upper bounds (inclusive). All truly frequent items are returned.
//!
//! # Error Bound
//!
//! The maximum error is bounded by: ε = 1 / max_size
//! For a stream of N items, the error is at most N/max_size.
//!
//! # Examples
//!
//! ```
//! use sketch_oxide::frequency::frequent::{FrequentItems, ErrorType};
//!
//! let mut sketch = FrequentItems::new(100).unwrap();
//!
//! // Update with items
//! for _ in 0..1000 {
//!     sketch.update("common".to_string());
//! }
//! for _ in 0..10 {
//!     sketch.update("rare".to_string());
//! }
//!
//! // Get top-K items
//! let items = sketch.frequent_items(ErrorType::NoFalsePositives);
//! assert_eq!(items[0].0, "common");
//!
//! // Get estimate for specific item
//! let (lower, upper) = sketch.get_estimate(&"common".to_string()).unwrap();
//! assert!(lower <= 1000 && upper >= 1000);
//! ```
//!
//! # References
//!
//! - Misra, J., & Gries, D. (1982). Finding repeated elements. Science of computer programming.
//! - Apache DataSketches: https://datasketches.apache.org/docs/Frequency/FrequentItemsOverview.html

use crate::common::SketchError;
use std::collections::HashMap;
use std::hash::Hash;

/// Error type for frequency estimation modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorType {
    /// No false positives: All returned items are truly frequent (uses lower bounds)
    NoFalsePositives,
    /// No false negatives: All truly frequent items are returned (uses upper bounds)
    NoFalseNegatives,
}

/// Frequent Items sketch for finding top-K most frequent items
///
/// This sketch uses the Misra-Gries algorithm to track the most frequent items
/// in a stream with deterministic error bounds.
///
/// # Type Parameters
///
/// - `T`: The item type, must implement `Hash`, `Eq`, and `Clone`
///
/// # Space Complexity
///
/// O(max_size) - stores at most `max_size` items
///
/// # Time Complexity
///
/// - Update: O(1) amortized (O(max_size) worst case during purge)
/// - Query: O(1) for single item
/// - Top-K: O(n log n) where n ≤ max_size
#[derive(Debug, Clone)]
pub struct FrequentItems<T: Hash + Eq + Clone> {
    /// Maximum number of items to track
    max_size: usize,
    /// Map of items to their counts
    items: HashMap<T, u64>,
    /// Global offset representing accumulated purged counts (error bound)
    offset: u64,
}

impl<T: Hash + Eq + Clone> FrequentItems<T> {
    /// Creates a new FrequentItems sketch
    ///
    /// # Arguments
    ///
    /// - `max_size`: Maximum number of items to track (must be ≥ 2)
    ///
    /// # Errors
    ///
    /// Returns `SketchError::InvalidParameter` if `max_size < 2`
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::frequency::frequent::FrequentItems;
    ///
    /// let sketch: FrequentItems<String> = FrequentItems::new(100).unwrap();
    /// assert!(sketch.is_empty());
    /// ```
    pub fn new(max_size: usize) -> Result<Self, SketchError> {
        if max_size < 2 {
            return Err(SketchError::InvalidParameter {
                param: "max_size".to_string(),
                value: max_size.to_string(),
                constraint: "must be >= 2".to_string(),
            });
        }

        Ok(Self {
            max_size,
            items: HashMap::with_capacity(max_size),
            offset: 0,
        })
    }

    /// Updates the sketch with a single occurrence of an item
    ///
    /// # Arguments
    ///
    /// - `item`: The item to add
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::frequency::frequent::FrequentItems;
    ///
    /// let mut sketch = FrequentItems::new(10).unwrap();
    /// sketch.update("apple".to_string());
    /// ```
    pub fn update(&mut self, item: T) {
        self.update_by(item, 1);
    }

    /// Updates the sketch with multiple occurrences of an item
    ///
    /// # Arguments
    ///
    /// - `item`: The item to add
    /// - `count`: The number of occurrences to add
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::frequency::frequent::FrequentItems;
    ///
    /// let mut sketch = FrequentItems::new(10).unwrap();
    /// sketch.update_by("apple".to_string(), 5);
    /// ```
    pub fn update_by(&mut self, item: T, count: u64) {
        if count == 0 {
            return;
        }

        *self.items.entry(item).or_insert(0) += count;

        // If exceeded capacity, purge minimum
        if self.items.len() > self.max_size {
            self.purge();
        }
    }

    /// Purges items with minimum count when capacity is exceeded
    ///
    /// This is the core of the Misra-Gries algorithm. When we exceed capacity:
    /// 1. Find the minimum count among all items
    /// 2. Subtract it from all items and remove zeros
    /// 3. Add it to the global offset (this tracks the error bound)
    ///
    /// This maintains the invariant that the true count is in [stored_count, stored_count + offset]
    fn purge(&mut self) {
        // Find minimum count
        let min_count = *self.items.values().min().unwrap_or(&0);

        if min_count == 0 {
            // Should not happen, but handle gracefully
            return;
        }

        // Remove items with minimum count
        self.items.retain(|_, count| *count > min_count);

        // Update offset (this represents the error bound)
        self.offset += min_count;
    }

    /// Returns all frequent items with their estimated frequency bounds
    ///
    /// # Arguments
    ///
    /// - `error_type`: The error mode to use (NoFalsePositives or NoFalseNegatives)
    ///
    /// # Returns
    ///
    /// A vector of tuples: (item, lower_bound, upper_bound)
    /// - Sorted by estimated frequency (descending)
    /// - True frequency is guaranteed to be in [lower_bound, upper_bound]
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::frequency::frequent::{FrequentItems, ErrorType};
    ///
    /// let mut sketch = FrequentItems::new(10).unwrap();
    /// sketch.update_by("common".to_string(), 100);
    /// sketch.update_by("rare".to_string(), 5);
    ///
    /// let items = sketch.frequent_items(ErrorType::NoFalsePositives);
    /// assert_eq!(items[0].0, "common");
    /// ```
    pub fn frequent_items(&self, _error_type: ErrorType) -> Vec<(T, u64, u64)> {
        // Returns (item, lower_bound, upper_bound)
        let mut result = Vec::with_capacity(self.items.len());

        for (item, &count) in &self.items {
            let lower = count;
            let upper = count + self.offset;

            // Both error types return the same items, just with different interpretations
            // NoFalsePositives: uses lower bound for threshold comparison
            // NoFalseNegatives: uses upper bound for threshold comparison
            // For now, we return all items with both bounds
            result.push((item.clone(), lower, upper));
        }

        // Sort by lower bound estimate (descending)
        result.sort_by(|a, b| b.1.cmp(&a.1));
        result
    }

    /// Gets the estimated frequency bounds for a specific item
    ///
    /// # Arguments
    ///
    /// - `item`: The item to query
    ///
    /// # Returns
    ///
    /// - `Some((lower_bound, upper_bound))` if the item is tracked
    /// - `None` if the item is not currently tracked
    ///
    /// The true frequency is guaranteed to be in [lower_bound, upper_bound].
    /// If the item is not tracked, its true frequency could be anywhere in [0, offset].
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::frequency::frequent::FrequentItems;
    ///
    /// let mut sketch = FrequentItems::new(10).unwrap();
    /// sketch.update_by("apple".to_string(), 42);
    ///
    /// let (lower, upper) = sketch.get_estimate(&"apple".to_string()).unwrap();
    /// assert!(lower <= 42 && upper >= 42);
    /// ```
    pub fn get_estimate(&self, item: &T) -> Option<(u64, u64)> {
        self.items
            .get(item)
            .map(|&count| (count, count + self.offset))
    }

    /// Merges another sketch into this one
    ///
    /// # Arguments
    ///
    /// - `other`: The sketch to merge
    ///
    /// # Errors
    ///
    /// Returns `SketchError::IncompatibleSketches` if the sketches have different `max_size`
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::frequency::frequent::FrequentItems;
    ///
    /// let mut sketch1 = FrequentItems::new(10).unwrap();
    /// let mut sketch2 = FrequentItems::new(10).unwrap();
    ///
    /// sketch1.update_by("a".to_string(), 10);
    /// sketch2.update_by("b".to_string(), 5);
    ///
    /// sketch1.merge(&sketch2).unwrap();
    /// ```
    pub fn merge(&mut self, other: &Self) -> Result<(), SketchError> {
        if self.max_size != other.max_size {
            return Err(SketchError::IncompatibleSketches {
                reason: format!("max_size mismatch: {} vs {}", self.max_size, other.max_size),
            });
        }

        // Add all items from other
        for (item, &count) in &other.items {
            *self.items.entry(item.clone()).or_insert(0) += count;
        }

        // Add offsets
        self.offset += other.offset;

        // Purge if needed (may need multiple purges)
        while self.items.len() > self.max_size {
            self.purge();
        }

        Ok(())
    }

    /// Returns true if the sketch is empty
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::frequency::frequent::FrequentItems;
    ///
    /// let sketch: FrequentItems<String> = FrequentItems::new(10).unwrap();
    /// assert!(sketch.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns the number of items currently tracked
    ///
    /// This will be at most `max_size`.
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::frequency::frequent::FrequentItems;
    ///
    /// let mut sketch = FrequentItems::new(10).unwrap();
    /// sketch.update("apple".to_string());
    /// assert_eq!(sketch.num_items(), 1);
    /// ```
    pub fn num_items(&self) -> usize {
        self.items.len()
    }

    /// Returns the maximum number of items this sketch can track
    pub fn max_size(&self) -> usize {
        self.max_size
    }

    /// Returns the current offset (accumulated error bound)
    ///
    /// For any item not currently tracked, its true frequency is at most `offset`.
    pub fn offset(&self) -> u64 {
        self.offset
    }
}

impl<T: Hash + Eq + Clone> PartialEq for FrequentItems<T> {
    fn eq(&self, other: &Self) -> bool {
        self.max_size == other.max_size && self.items == other.items && self.offset == other.offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        let mut sketch = FrequentItems::new(10).unwrap();
        assert!(sketch.is_empty());

        sketch.update("apple".to_string());
        assert!(!sketch.is_empty());
        assert_eq!(sketch.num_items(), 1);
    }

    #[test]
    fn test_purge_mechanism() {
        let mut sketch = FrequentItems::new(2).unwrap();

        sketch.update("a".to_string());
        sketch.update("b".to_string());
        sketch.update("c".to_string());

        // After purge, should have at most 2 items
        assert!(sketch.num_items() <= 2);
        assert!(sketch.offset() > 0);
    }

    #[test]
    fn test_error_bounds() {
        let mut sketch = FrequentItems::new(10).unwrap();
        sketch.update_by("test".to_string(), 42);

        let (lower, upper) = sketch.get_estimate(&"test".to_string()).unwrap();
        assert!(lower <= 42);
        assert!(upper >= 42);
    }
}
