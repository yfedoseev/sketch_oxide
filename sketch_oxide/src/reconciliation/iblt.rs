//! IBLT (Invertible Bloom Lookup Table) for Set Reconciliation
//!
//! This module implements a classic, fixed-size Invertible Bloom Lookup Table
//! (Goodrich & Mitzenmacher 2011; Eppstein et al. 2011): a probabilistic data
//! structure for computing the symmetric difference between two sets by exchanging
//! a compact summary whose size is chosen up front from an estimate of the
//! difference size `d`.
//!
//! # Naming note
//!
//! Earlier releases exported this type as `RatelessIBLT`. That name was a misnomer:
//! this is a *fixed-rate* IBLT (the cell count is fixed at construction from an
//! expected difference size), not the *rateless* construction of Yang, Gilad &
//! Alizadeh (SIGCOMM 2024), which streams an unbounded sequence of coded symbols and
//! needs no difference estimate. A true rateless IBLT is tracked separately on the
//! roadmap. The old names remain available as deprecated aliases ([`RatelessIBLT`],
//! [`RatelessIBLTStats`]) and will be removed in a future release.
//!
//! To size a fixed-rate IBLT without guessing `d`, pair it with a Strata Estimator
//! (planned) which estimates the difference size first.
//!
//! # Use Cases
//!
//! - **P2P / blockchain synchronization**: BitTorrent, IPFS, block propagation
//! - **Distributed cache invalidation**: CDN cache management
//! - **Database replication**: efficient state synchronization
//! - **File synchronization**: Dropbox-style sync protocols
//!
//! # Algorithm Overview
//!
//! The IBLT works by:
//! 1. Hashing each key-value pair to k positions (typically k=3)
//! 2. XORing key and value data into cells at those positions
//! 3. Maintaining a signed count and a key-check hash for each cell
//! 4. Supporting subtraction to compute symmetric differences
//! 5. Decoding via iterative peeling of *verified* singleton cells
//!
//! # Decoding correctness — the key-check hash
//!
//! A cell whose `count` is `±1` is only a candidate singleton. Collisions can
//! manufacture a false singleton: e.g. two distinct insertions plus one deletion of
//! a third key all land in one cell, leaving `count == 1` while `key_sum`/`sum` hold
//! XOR garbage. Extracting a pair from such a cell would emit a corrupt key/value.
//!
//! To prevent this, every cell also accumulates `check_sum`, the XOR of a check hash
//! of each key. A candidate is accepted as a genuine singleton only when the check
//! hash of the recovered key equals the stored `check_sum`. False singletons are
//! rejected, so decoding either returns correct pairs or reports the IBLT as
//! undecodable — it never emits garbage.
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
//! use sketch_oxide::reconciliation::Iblt;
//! use sketch_oxide::common::Reconcilable;
//!
//! // Create IBLTs for Alice and Bob
//! let mut alice = Iblt::new(100, 32).unwrap();
//! let mut bob = Iblt::new(100, 32).unwrap();
//!
//! // Both insert shared items
//! alice.insert(b"shared1", b"value1").unwrap();
//! alice.insert(b"shared2", b"value2").unwrap();
//! bob.insert(b"shared1", b"value1").unwrap();
//! bob.insert(b"shared2", b"value2").unwrap();
//!
//! // Alice and Bob each have unique items
//! alice.insert(b"alice_only", b"alice_value").unwrap();
//! bob.insert(b"bob_only", b"bob_value").unwrap();
//!
//! // Compute difference: alice - bob
//! let mut diff = alice.clone();
//! diff.subtract(&bob).unwrap();
//!
//! // Decode to recover symmetric difference
//! let result = diff.decode().unwrap();
//! // result.to_insert contains items in Alice but not Bob
//! // result.to_remove contains items in Bob but not Alice
//! assert_eq!(result.to_insert.len(), 1);
//! assert_eq!(result.to_remove.len(), 1);
//! ```
//!
//! # References
//!
//! - Goodrich, M. T., & Mitzenmacher, M. (2011). "Invertible bloom lookup tables"
//! - Eppstein, D., et al. (2011). "What's the difference? Efficient set reconciliation
//!   without prior context" (introduces the key-check hash used here)
//! - Ozisik, A. P., et al. (2017). "Graphene: A new protocol for block propagation"

use crate::common::{hash::xxhash, Reconcilable, Result, SetDifference, SketchError};

/// Seed for the per-cell key-check hash. Distinct from the position-hash seeds
/// (`0..k`) so the check is independent of cell placement.
const KEY_CHECK_SEED: u64 = 0x5165_4945_4c42_4954; // nothing-up-my-sleeve constant

/// Check hash of a key, accumulated (XOR) into a cell's `check_sum`.
///
/// Used to verify that a candidate singleton (count == ±1) genuinely holds a single
/// key rather than a collision of items that happens to sum to count ±1.
fn key_check(key: &[u8]) -> u64 {
    xxhash(key, KEY_CHECK_SEED)
}

/// Invertible Bloom Lookup Table for efficient set reconciliation
///
/// This structure uses k hash functions (typically k=3) to map each key-value
/// pair to k cells. Each cell maintains:
/// - `sum`: XOR of all values
/// - `key_sum`: XOR of all keys
/// - `count`: signed number of items hashed to this cell
/// - `check_sum`: XOR of a key-check hash, used to validate singletons
///
/// # Thread Safety
///
/// This structure is not thread-safe. Use external synchronization if needed.
#[derive(Clone)]
pub struct Iblt {
    /// Number of cells in the IBLT
    num_cells: usize,

    /// The cells storing XOR sums and counts
    cells: Vec<IbltCell>,

    /// Number of hash functions (k parameter)
    hash_functions: usize,

    /// Maximum size for cell data
    cell_size: usize,
}

/// A single cell in the IBLT
///
/// Each cell stores XOR sums of keys and values, a signed count, and a key-check
/// hash. When `count == ±1` *and* the key-check matches the recovered key, the cell
/// is a verified "singleton" and can be decoded.
#[derive(Clone, Debug)]
struct IbltCell {
    /// XOR of all values in this cell
    sum: Vec<u8>,

    /// Count of items hashed to this cell (can be negative in i32)
    count: i32,

    /// XOR of all keys in this cell
    key_sum: Vec<u8>,

    /// XOR of the key-check hash of every key in this cell
    check_sum: u64,
}

impl IbltCell {
    /// Create a new empty cell
    fn new(cell_size: usize) -> Self {
        Self {
            sum: vec![0u8; cell_size],
            count: 0,
            key_sum: vec![0u8; cell_size],
            check_sum: 0,
        }
    }

    /// Check if this cell is a *verified* singleton.
    ///
    /// A cell is a genuine singleton when its count is `±1` and the key-check hash of
    /// the recovered key matches the accumulated `check_sum`. The second condition
    /// rejects false singletons produced by collisions (see module docs).
    fn is_singleton(&self) -> bool {
        if self.count != 1 && self.count != -1 {
            return false;
        }
        let key = Self::trim_zeros(&self.key_sum);
        key_check(&key) == self.check_sum
    }

    /// Check if cell is empty
    fn is_empty(&self) -> bool {
        self.count == 0 && self.check_sum == 0 && self.key_sum.iter().all(|&b| b == 0)
    }

    /// Add a key-value pair to this cell
    fn add(&mut self, key: &[u8], value: &[u8]) {
        Self::xor_data(&mut self.key_sum, key);
        Self::xor_data(&mut self.sum, value);
        self.check_sum ^= key_check(key);
        self.count += 1;
    }

    /// Remove a key-value pair from this cell
    fn remove(&mut self, key: &[u8], value: &[u8]) {
        Self::xor_data(&mut self.key_sum, key);
        Self::xor_data(&mut self.sum, value);
        self.check_sum ^= key_check(key);
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
pub struct IbltStats {
    /// Number of cells in the IBLT
    pub num_cells: usize,

    /// Size of each cell in bytes
    pub cell_size: usize,
}

impl Iblt {
    /// Create a new fixed-rate IBLT
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
    /// use sketch_oxide::reconciliation::Iblt;
    ///
    /// // Create IBLT expecting up to 100 differences
    /// let iblt = Iblt::new(100, 32).unwrap();
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

        let cells = (0..num_cells).map(|_| IbltCell::new(cell_size)).collect();

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
    /// use sketch_oxide::reconciliation::Iblt;
    ///
    /// let mut iblt = Iblt::new(100, 32).unwrap();
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
    /// use sketch_oxide::reconciliation::Iblt;
    ///
    /// let mut iblt = Iblt::new(100, 32).unwrap();
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
    /// use sketch_oxide::reconciliation::Iblt;
    ///
    /// let iblt = Iblt::new(100, 32).unwrap();
    /// let stats = iblt.stats();
    /// println!("Cells: {}, Size: {}", stats.num_cells, stats.cell_size);
    /// ```
    pub fn stats(&self) -> IbltStats {
        IbltStats {
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

impl Reconcilable for Iblt {
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
    /// use sketch_oxide::reconciliation::Iblt;
    /// use sketch_oxide::common::Reconcilable;
    ///
    /// let mut alice = Iblt::new(100, 32).unwrap();
    /// let mut bob = Iblt::new(100, 32).unwrap();
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

        // Element-wise subtraction (XOR for data and check, subtract for counts)
        for (i, other_cell) in other.cells.iter().enumerate() {
            let cell = &mut self.cells[i];

            // XOR the sums
            for j in 0..cell.sum.len().min(other_cell.sum.len()) {
                cell.sum[j] ^= other_cell.sum[j];
            }

            for j in 0..cell.key_sum.len().min(other_cell.key_sum.len()) {
                cell.key_sum[j] ^= other_cell.key_sum[j];
            }

            // XOR the key-check (XOR is its own inverse, so subtract == add here)
            cell.check_sum ^= other_cell.check_sum;

            // Subtract counts
            cell.count -= other_cell.count;
        }

        Ok(())
    }

    /// Decode the IBLT to recover set differences
    ///
    /// This uses the "peeling" algorithm:
    /// 1. Find a *verified* singleton cell (count == ±1 and key-check matches)
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
    /// - IBLT is undecodable (no verified singletons but cells remain)
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::reconciliation::Iblt;
    /// use sketch_oxide::common::Reconcilable;
    ///
    /// let mut iblt = Iblt::new(100, 32).unwrap();
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

        // Peeling algorithm with iteration limit.
        // Each successful iteration peels exactly one item, so the number of
        // iterations is bounded by the number of items in the difference; the cell
        // count gives a generous upper bound.
        let max_iterations = self.num_cells * 100;
        let mut iterations = 0;

        loop {
            iterations += 1;
            if iterations > max_iterations {
                return Err(SketchError::ReconciliationError {
                    reason: "Decode iteration limit exceeded".to_string(),
                });
            }

            // Find the first verified singleton cell for this iteration.
            let Some(idx) = working.cells.iter().position(IbltCell::is_singleton) else {
                // No verified singletons remain. Either fully peeled (success) or
                // genuinely undecodable.
                if working.cells.iter().all(IbltCell::is_empty) {
                    break;
                }
                let non_empty = working.cells.iter().filter(|c| !c.is_empty()).count();
                return Err(SketchError::ReconciliationError {
                    reason: format!(
                        "Unable to decode: {non_empty} non-empty cells remain without verified singletons"
                    ),
                });
            };

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

        Ok(SetDifference {
            to_insert,
            to_remove,
        })
    }
}

/// Deprecated alias for [`Iblt`].
///
/// The `RatelessIBLT` name was a misnomer — this is a classic fixed-rate IBLT, not
/// the SIGCOMM 2024 rateless construction. Use [`Iblt`].
#[deprecated(
    since = "0.2.0",
    note = "renamed to `Iblt`: this is a classic fixed-rate IBLT, not a rateless one. \
            A true rateless IBLT is planned separately."
)]
pub type RatelessIBLT = Iblt;

/// Deprecated alias for [`IbltStats`].
#[deprecated(since = "0.2.0", note = "renamed to `IbltStats`")]
pub type RatelessIBLTStats = IbltStats;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_construction() {
        let iblt = Iblt::new(100, 32);
        assert!(iblt.is_ok());
    }

    #[test]
    fn test_basic_insert() {
        let mut iblt = Iblt::new(10, 32).unwrap();
        let result = iblt.insert(b"key", b"value");
        assert!(result.is_ok());
    }

    #[test]
    fn test_basic_decode() {
        let mut iblt = Iblt::new(10, 32).unwrap();
        iblt.insert(b"key", b"value").unwrap();

        let diff = iblt.decode().unwrap();
        assert_eq!(diff.to_insert.len(), 1);
        assert_eq!(diff.to_insert[0].0, b"key");
        assert_eq!(diff.to_insert[0].1, b"value");
    }

    #[test]
    fn test_basic_subtraction() {
        let mut iblt1 = Iblt::new(10, 32).unwrap();
        let iblt2 = Iblt::new(10, 32).unwrap();

        iblt1.insert(b"key", b"value").unwrap();
        iblt1.subtract(&iblt2).unwrap();

        let diff = iblt1.decode().unwrap();
        assert_eq!(diff.to_insert.len(), 1);
    }

    #[test]
    fn test_symmetric_difference() {
        let mut alice = Iblt::new(100, 32).unwrap();
        let mut bob = Iblt::new(100, 32).unwrap();

        for i in 0..50 {
            let key = format!("shared{i}");
            alice.insert(key.as_bytes(), b"v").unwrap();
            bob.insert(key.as_bytes(), b"v").unwrap();
        }
        alice.insert(b"alice_only", b"av").unwrap();
        bob.insert(b"bob_only", b"bv").unwrap();

        let mut diff = alice.clone();
        diff.subtract(&bob).unwrap();
        let result = diff.decode().unwrap();

        assert_eq!(result.to_insert.len(), 1);
        assert_eq!(result.to_insert[0].0, b"alice_only");
        assert_eq!(result.to_remove.len(), 1);
        assert_eq!(result.to_remove[0].0, b"bob_only");
    }

    #[test]
    fn test_empty_iblt_decodes_to_nothing() {
        let iblt = Iblt::new(10, 32).unwrap();
        let diff = iblt.decode().unwrap();
        assert!(diff.to_insert.is_empty());
        assert!(diff.to_remove.is_empty());
    }

    #[test]
    fn test_cell_is_singleton_requires_key_check() {
        // A cell holding a single genuine key is a verified singleton.
        let mut cell = IbltCell::new(32);
        assert!(!cell.is_singleton());
        cell.add(b"key1", b"value1");
        assert!(cell.is_singleton());

        // Count alone is not enough: a collision of two inserts and one delete of a
        // *different* key leaves count == 1 but must NOT verify as a singleton.
        let mut collided = IbltCell::new(32);
        collided.add(b"alpha", b"1");
        collided.add(b"beta", b"2");
        collided.remove(b"gamma", b"3");
        assert_eq!(collided.count, 1);
        assert!(
            !collided.is_singleton(),
            "false singleton (count==1 from a collision) must be rejected by the key-check"
        );
    }

    #[test]
    fn test_cell_xor_operations() {
        let mut cell = IbltCell::new(32);

        cell.add(b"key1", b"value1");
        assert_eq!(cell.count, 1);

        cell.add(b"key1", b"value1");
        assert_eq!(cell.count, 2);

        cell.remove(b"key1", b"value1");
        assert_eq!(cell.count, 1);
    }

    #[test]
    #[allow(deprecated)]
    fn test_deprecated_alias_still_resolves() {
        // The old name must keep compiling (as a deprecated alias) for back-compat.
        let _iblt: RatelessIBLT = Iblt::new(10, 32).unwrap();
    }
}
