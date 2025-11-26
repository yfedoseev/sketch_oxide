//! Rateless IBLT (Invertible Bloom Lookup Table) for Set Reconciliation
//!
//! This module implements a Rateless IBLT, a probabilistic data structure for
//! efficiently computing the symmetric difference between two sets in distributed
//! systems without knowing the difference size a priori.
//!
//! # Use Cases (2025)
//!
//! - **Ethereum Block Synchronization**: 5.6x faster than naive approaches
//! - **P2P Network Synchronization**: BitTorrent, IPFS, blockchain nodes
//! - **Distributed Cache Invalidation**: CDN cache management
//! - **Database Replication**: Efficient state synchronization
//! - **File Synchronization**: Dropbox-style sync protocols
//!
//! # Algorithm Overview
//!
//! The Rateless IBLT works by:
//! 1. Hashing each key-value pair to k positions (typically k=3)
//! 2. XORing data into cells at those positions
//! 3. Maintaining counts for each cell
//! 4. Supporting subtraction to compute symmetric differences
//! 5. Decoding via iterative peeling of singleton cells
//!
//! # Performance Characteristics
//!
//! - **Space**: O(c × d) where c ≈ 1.5-2.0, d = expected difference size
//! - **Insert/Delete**: O(k) where k = number of hash functions
//! - **Subtract**: O(n) where n = number of cells
//! - **Decode**: O(d × k) where d = actual difference size
//!
//! # Example
//!
//! ```
//! use sketch_oxide::reconciliation::RatelessIBLT;
//! use sketch_oxide::common::Reconcilable;
//!
//! // Create IBLTs for Alice and Bob
//! let mut alice = RatelessIBLT::new(100, 32).unwrap();
//! let mut bob = RatelessIBLT::new(100, 32).unwrap();
//!
//! // Both insert shared items
//! alice.insert(b"shared1", b"value1").unwrap();
//! alice.insert(b"shared2", b"value2").unwrap();
//! bob.insert(b"shared1", b"value1").unwrap();
//! bob.insert(b"shared2", b"value2").unwrap();
//!
//! // Alice has unique items
//! alice.insert(b"alice_only", b"alice_value").unwrap();
//!
//! // Bob has unique items
//! bob.insert(b"bob_only", b"bob_value").unwrap();
//!
//! // Compute difference: alice - bob
//! let mut diff = alice.clone();
//! diff.subtract(&bob).unwrap();
//!
//! // Decode to recover symmetric difference
//! let result = diff.decode().unwrap();
//!
//! // result.to_insert contains items in Alice but not Bob
//! // result.to_remove contains items in Bob but not Alice
//! println!("Items to insert: {}", result.to_insert.len());
//! println!("Items to remove: {}", result.to_remove.len());
//! ```
//!
//! # References
//!
//! - Goodrich, M. T., & Mitzenmacher, M. (2011). "Invertible bloom lookup tables"
//! - Eppstein, D., et al. (2011). "What's the difference? Efficient set reconciliation"
//! - Ozisik, A. P., et al. (2017). "Graphene: A new protocol for block propagation"

use crate::common::{hash::xxhash, Reconcilable, Result, SetDifference, SketchError};

/// Rateless IBLT for efficient set reconciliation
///
/// This structure uses k hash functions (typically k=3) to map each key-value
/// pair to k cells. Each cell maintains:
/// - `sum`: XOR of all values
/// - `count`: Number of items hashed to this cell
/// - `key_sum`: XOR of all keys
///
/// # Thread Safety
///
/// This structure is not thread-safe. Use external synchronization if needed.
#[derive(Clone)]
pub struct RatelessIBLT {
    /// Number of cells in the IBLT
    num_cells: usize,

    /// The cells storing XOR sums and counts
    cells: Vec<IBLTCell>,

    /// Number of hash functions (k parameter)
    hash_functions: usize,

    /// Maximum size for cell data
    cell_size: usize,
}

/// A single cell in the IBLT
///
/// Each cell stores XOR sums of keys and values, plus a count.
/// When count=1, the cell is a "singleton" and can be decoded.
#[derive(Clone, Debug)]
struct IBLTCell {
    /// XOR of all values in this cell
    sum: Vec<u8>,

    /// Count of items hashed to this cell (can be negative in i32)
    count: i32,

    /// XOR of all keys in this cell
    key_sum: Vec<u8>,
}

impl IBLTCell {
    /// Create a new empty cell
    fn new(cell_size: usize) -> Self {
        Self {
            sum: vec![0u8; cell_size],
            count: 0,
            key_sum: vec![0u8; cell_size],
        }
    }

    /// Check if this cell is a singleton (count == ±1)
    fn is_singleton(&self) -> bool {
        self.count == 1 || self.count == -1
    }

    /// Check if cell is empty
    fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Add a key-value pair to this cell
    fn add(&mut self, key: &[u8], value: &[u8]) {
        Self::xor_data(&mut self.key_sum, key);
        Self::xor_data(&mut self.sum, value);
        self.count += 1;
    }

    /// Remove a key-value pair from this cell
    fn remove(&mut self, key: &[u8], value: &[u8]) {
        Self::xor_data(&mut self.key_sum, key);
        Self::xor_data(&mut self.sum, value);
        self.count -= 1;
    }

    /// XOR data into a buffer (growing buffer if needed)
    fn xor_data(buffer: &mut Vec<u8>, data: &[u8]) {
        // Extend buffer if needed
        if data.len() > buffer.len() {
            buffer.resize(data.len(), 0);
        }

        // XOR the data
        for (i, &byte) in data.iter().enumerate() {
            buffer[i] ^= byte;
        }
    }

    /// Extract key-value pair from singleton cell
    fn extract_pair(&self) -> (Vec<u8>, Vec<u8>) {
        // For singleton cells, key_sum and sum contain the actual key and value
        // We need to trim trailing zeros
        let key = Self::trim_zeros(&self.key_sum);
        let value = Self::trim_zeros(&self.sum);
        (key, value)
    }

    /// Trim trailing zeros from a byte vector
    fn trim_zeros(data: &[u8]) -> Vec<u8> {
        let mut end = data.len();
        while end > 0 && data[end - 1] == 0 {
            end -= 1;
        }
        data[..end].to_vec()
    }
}

/// Statistics about the IBLT structure
#[derive(Debug, Clone)]
pub struct RatelessIBLTStats {
    /// Number of cells in the IBLT
    pub num_cells: usize,

    /// Size of each cell in bytes
    pub cell_size: usize,
}

impl RatelessIBLT {
    /// Create a new Rateless IBLT
    ///
    /// # Arguments
    ///
    /// * `expected_diff` - Expected size of set difference (d parameter)
    /// * `cell_size` - Maximum size for cell data in bytes
    ///
    /// # Returns
    ///
    /// A new IBLT configured for the expected difference size
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if:
    /// - `expected_diff` is 0
    /// - `cell_size` is too small (< 8 bytes)
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::reconciliation::RatelessIBLT;
    ///
    /// // Create IBLT expecting up to 100 differences
    /// let iblt = RatelessIBLT::new(100, 32).unwrap();
    /// ```
    pub fn new(expected_diff: usize, cell_size: usize) -> Result<Self> {
        if expected_diff == 0 {
            return Err(SketchError::InvalidParameter {
                param: "expected_diff".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }

        if cell_size < 8 {
            return Err(SketchError::InvalidParameter {
                param: "cell_size".to_string(),
                value: cell_size.to_string(),
                constraint: "must be >= 8".to_string(),
            });
        }

        // Use c ≈ 2.0 for reliable decode probability
        // Research shows c=1.5 gives ~95% success, c=2.0 gives ~99%+ success
        let c_factor = 2.0;
        let num_cells = ((expected_diff as f64 * c_factor).ceil() as usize).max(8);

        let cells = (0..num_cells).map(|_| IBLTCell::new(cell_size)).collect();

        Ok(Self {
            num_cells,
            cells,
            hash_functions: 3, // k=3 is optimal for most use cases
            cell_size,
        })
    }

    /// Insert a key-value pair into the IBLT
    ///
    /// The pair is hashed to k positions and added to all corresponding cells.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to insert
    /// * `value` - The value associated with the key
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::reconciliation::RatelessIBLT;
    ///
    /// let mut iblt = RatelessIBLT::new(100, 32).unwrap();
    /// iblt.insert(b"my_key", b"my_value").unwrap();
    /// ```
    pub fn insert(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        let positions = self.hash_key(key);

        for pos in positions {
            self.cells[pos].add(key, value);
        }

        Ok(())
    }

    /// Delete a key-value pair from the IBLT
    ///
    /// This is equivalent to inserting with negative count.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to delete
    /// * `value` - The value associated with the key
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::reconciliation::RatelessIBLT;
    ///
    /// let mut iblt = RatelessIBLT::new(100, 32).unwrap();
    /// iblt.insert(b"key", b"value").unwrap();
    /// iblt.delete(b"key", b"value").unwrap();
    /// ```
    pub fn delete(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        let positions = self.hash_key(key);

        for pos in positions {
            self.cells[pos].remove(key, value);
        }

        Ok(())
    }

    /// Get statistics about this IBLT
    ///
    /// # Returns
    ///
    /// Statistics including number of cells and cell size
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::reconciliation::RatelessIBLT;
    ///
    /// let iblt = RatelessIBLT::new(100, 32).unwrap();
    /// let stats = iblt.stats();
    /// println!("Cells: {}, Size: {}", stats.num_cells, stats.cell_size);
    /// ```
    pub fn stats(&self) -> RatelessIBLTStats {
        RatelessIBLTStats {
            num_cells: self.num_cells,
            cell_size: self.cell_size,
        }
    }

    /// Hash a key to k cell positions
    ///
    /// Uses k independent hash functions to map the key to k cell indices.
    fn hash_key(&self, key: &[u8]) -> Vec<usize> {
        let mut positions = Vec::with_capacity(self.hash_functions);

        for i in 0..self.hash_functions {
            let hash = xxhash(key, i as u64);
            let pos = (hash as usize) % self.num_cells;
            positions.push(pos);
        }

        positions
    }
}

impl Reconcilable for RatelessIBLT {
    /// Subtract another IBLT from this one
    ///
    /// This computes the element-wise XOR difference between the two IBLTs,
    /// leaving cells that contain the symmetric difference.
    ///
    /// # Arguments
    ///
    /// * `other` - The IBLT to subtract from this one
    ///
    /// # Errors
    ///
    /// Returns `IncompatibleSketches` if the IBLTs have different configurations
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::reconciliation::RatelessIBLT;
    /// use sketch_oxide::common::Reconcilable;
    ///
    /// let mut alice = RatelessIBLT::new(100, 32).unwrap();
    /// let mut bob = RatelessIBLT::new(100, 32).unwrap();
    ///
    /// alice.insert(b"a", b"1").unwrap();
    /// bob.insert(b"b", b"2").unwrap();
    ///
    /// alice.subtract(&bob).unwrap();
    /// ```
    fn subtract(&mut self, other: &Self) -> Result<()> {
        if self.num_cells != other.num_cells {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "Different number of cells: {} vs {}",
                    self.num_cells, other.num_cells
                ),
            });
        }

        if self.cell_size != other.cell_size {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "Different cell sizes: {} vs {}",
                    self.cell_size, other.cell_size
                ),
            });
        }

        // Element-wise subtraction (XOR for data, subtract for counts)
        for (i, other_cell) in other.cells.iter().enumerate() {
            let cell = &mut self.cells[i];

            // XOR the sums
            for j in 0..cell.sum.len().min(other_cell.sum.len()) {
                cell.sum[j] ^= other_cell.sum[j];
            }

            for j in 0..cell.key_sum.len().min(other_cell.key_sum.len()) {
                cell.key_sum[j] ^= other_cell.key_sum[j];
            }

            // Subtract counts
            cell.count -= other_cell.count;
        }

        Ok(())
    }

    /// Decode the IBLT to recover set differences
    ///
    /// This uses the "peeling" algorithm:
    /// 1. Find singleton cells (count == ±1)
    /// 2. Extract the key-value pair
    /// 3. Remove it from all k positions
    /// 4. Repeat until no more singletons or error
    ///
    /// # Returns
    ///
    /// A `SetDifference` containing:
    /// - `to_insert`: Items with positive count (in this IBLT but not other)
    /// - `to_remove`: Items with negative count (in other IBLT but not this)
    ///
    /// # Errors
    ///
    /// Returns `ReconciliationError` if:
    /// - Decoding fails (too many items, corruption, etc.)
    /// - IBLT is undecodable (no singletons but cells remain)
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::reconciliation::RatelessIBLT;
    /// use sketch_oxide::common::Reconcilable;
    ///
    /// let mut iblt = RatelessIBLT::new(100, 32).unwrap();
    /// iblt.insert(b"key1", b"value1").unwrap();
    /// iblt.insert(b"key2", b"value2").unwrap();
    ///
    /// let diff = iblt.decode().unwrap();
    /// assert_eq!(diff.to_insert.len(), 2);
    /// ```
    fn decode(&self) -> Result<SetDifference> {
        // Create working copy for peeling
        let mut working = self.clone();

        let mut to_insert = Vec::new();
        let mut to_remove = Vec::new();

        // Peeling algorithm with iteration limit
        // Allow generous iterations for large sets
        let max_iterations = self.num_cells * 100;
        let mut iterations = 0;
        let mut prev_singleton_count = usize::MAX;
        let mut stuck_count = 0;

        loop {
            iterations += 1;
            if iterations > max_iterations {
                return Err(SketchError::ReconciliationError {
                    reason: "Decode iteration limit exceeded".to_string(),
                });
            }

            // Find all singleton cells for this iteration
            let singletons: Vec<usize> = working
                .cells
                .iter()
                .enumerate()
                .filter(|(_, cell)| cell.is_singleton())
                .map(|(idx, _)| idx)
                .collect();

            let singleton_count = singletons.len();

            // Check if we're making progress
            if singleton_count == 0 {
                // No singletons - check if we're done
                let all_empty = working.cells.iter().all(|cell| cell.is_empty());

                if all_empty {
                    // Successfully decoded
                    break;
                } else {
                    // Undecodable - too many collisions or capacity exceeded
                    let non_empty = working.cells.iter().filter(|c| !c.is_empty()).count();
                    return Err(SketchError::ReconciliationError {
                        reason: format!(
                            "Unable to decode: {} non-empty cells remain without singletons",
                            non_empty
                        ),
                    });
                }
            }

            // Detect if we're stuck (no progress)
            if singleton_count == prev_singleton_count {
                stuck_count += 1;
                if stuck_count > 10 {
                    return Err(SketchError::ReconciliationError {
                        reason: "Decode stalled: no progress after multiple iterations".to_string(),
                    });
                }
            } else {
                stuck_count = 0;
            }
            prev_singleton_count = singleton_count;

            // Process first singleton (just process one per iteration for correctness)
            if let Some(&idx) = singletons.first() {
                let cell = &working.cells[idx];
                let count = cell.count;
                let (key, value) = cell.extract_pair();

                // Store the pair based on count sign
                if count > 0 {
                    to_insert.push((key.clone(), value.clone()));
                } else {
                    to_remove.push((key.clone(), value.clone()));
                }

                // Remove this item from all k positions
                let positions = working.hash_key(&key);
                for pos in positions {
                    if count > 0 {
                        working.cells[pos].remove(&key, &value);
                    } else {
                        working.cells[pos].add(&key, &value);
                    }
                }
            }
        }

        Ok(SetDifference {
            to_insert,
            to_remove,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_construction() {
        let iblt = RatelessIBLT::new(100, 32);
        assert!(iblt.is_ok());
    }

    #[test]
    fn test_basic_insert() {
        let mut iblt = RatelessIBLT::new(10, 32).unwrap();
        let result = iblt.insert(b"key", b"value");
        assert!(result.is_ok());
    }

    #[test]
    fn test_basic_decode() {
        let mut iblt = RatelessIBLT::new(10, 32).unwrap();
        iblt.insert(b"key", b"value").unwrap();

        let diff = iblt.decode().unwrap();
        assert_eq!(diff.to_insert.len(), 1);
    }

    #[test]
    fn test_basic_subtraction() {
        let mut iblt1 = RatelessIBLT::new(10, 32).unwrap();
        let iblt2 = RatelessIBLT::new(10, 32).unwrap();

        iblt1.insert(b"key", b"value").unwrap();
        iblt1.subtract(&iblt2).unwrap();

        let diff = iblt1.decode().unwrap();
        assert_eq!(diff.to_insert.len(), 1);
    }

    #[test]
    fn test_cell_is_singleton() {
        let mut cell = IBLTCell::new(32);
        assert!(!cell.is_singleton());

        cell.count = 1;
        assert!(cell.is_singleton());

        cell.count = -1;
        assert!(cell.is_singleton());

        cell.count = 2;
        assert!(!cell.is_singleton());
    }

    #[test]
    fn test_cell_xor_operations() {
        let mut cell = IBLTCell::new(32);

        cell.add(b"key1", b"value1");
        assert_eq!(cell.count, 1);

        cell.add(b"key1", b"value1");
        assert_eq!(cell.count, 2);

        cell.remove(b"key1", b"value1");
        assert_eq!(cell.count, 1);
    }
}
