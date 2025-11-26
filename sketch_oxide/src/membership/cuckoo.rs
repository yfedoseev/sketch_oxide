//! Cuckoo Filter: Space-efficient membership with deletions (Fan 2014)
//!
//! Cuckoo filters support insertions, deletions, and membership queries
//! with better space efficiency than Counting Bloom filters.
//!
//! # Algorithm Overview
//!
//! - Each element is hashed to a fingerprint and two candidate buckets
//! - Insertions use cuckoo hashing: if both buckets full, relocate existing items
//! - Deletions remove matching fingerprints
//! - Queries check for fingerprint in either bucket
//!
//! # Comparison with Other Filters
//!
//! | Filter | Space | Deletions | FPR Control |
//! |--------|-------|-----------|-------------|
//! | Bloom | ~10 bits/item | No | Yes |
//! | Counting Bloom | ~40 bits/item | Yes | Yes |
//! | Cuckoo | ~12 bits/item | Yes | Limited |
//! | BinaryFuse | ~9 bits/item | No | Limited |
//!
//! # Time Complexity
//!
//! - Insert: O(1) amortized
//! - Delete: O(1)
//! - Query: O(1)
//!
//! # Space Complexity
//!
//! O(n) where n is capacity, approximately 12 bits per item
//!
//! # References
//!
//! - Fan et al. "Cuckoo Filter: Practically Better Than Bloom" (2014)
//!
//! # Examples
//!
//! ```
//! use sketch_oxide::membership::CuckooFilter;
//!
//! let mut filter = CuckooFilter::new(1000).unwrap();
//!
//! filter.insert(b"key1").unwrap();
//! filter.insert(b"key2").unwrap();
//!
//! assert!(filter.contains(b"key1"));
//! assert!(filter.contains(b"key2"));
//!
//! filter.remove(b"key1");
//! assert!(!filter.contains(b"key1"));
//! ```

use crate::common::SketchError;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use xxhash_rust::xxh64::xxh64;

/// Number of entries per bucket
const BUCKET_SIZE: usize = 4;

/// Maximum number of relocations before giving up
const MAX_KICKS: usize = 500;

/// A bucket containing fingerprints
#[derive(Clone, Debug)]
struct Bucket {
    fingerprints: [u16; BUCKET_SIZE],
}

impl Default for Bucket {
    fn default() -> Self {
        Bucket {
            fingerprints: [0; BUCKET_SIZE],
        }
    }
}

impl Bucket {
    /// Inserts a fingerprint if there's space
    fn insert(&mut self, fp: u16) -> bool {
        for slot in &mut self.fingerprints {
            if *slot == 0 {
                *slot = fp;
                return true;
            }
        }
        false
    }

    /// Removes a fingerprint if present
    fn remove(&mut self, fp: u16) -> bool {
        for slot in &mut self.fingerprints {
            if *slot == fp {
                *slot = 0;
                return true;
            }
        }
        false
    }

    /// Checks if fingerprint is present
    fn contains(&self, fp: u16) -> bool {
        self.fingerprints.contains(&fp)
    }

    /// Swaps a random fingerprint with the given one
    fn swap_random(&mut self, fp: u16, rng: &mut SmallRng) -> u16 {
        let idx = rng.random_range(0..BUCKET_SIZE);
        let old = self.fingerprints[idx];
        self.fingerprints[idx] = fp;
        old
    }
}

/// Cuckoo Filter for membership testing with deletions
///
/// Space-efficient alternative to Counting Bloom filters.
///
/// # Examples
///
/// ```
/// use sketch_oxide::membership::CuckooFilter;
///
/// let mut filter = CuckooFilter::new(100).unwrap();
/// filter.insert(b"hello").unwrap();
/// assert!(filter.contains(b"hello"));
///
/// filter.remove(b"hello");
/// assert!(!filter.contains(b"hello"));
/// ```
#[derive(Clone, Debug)]
pub struct CuckooFilter {
    /// Buckets containing fingerprints
    buckets: Vec<Bucket>,
    /// Number of buckets
    num_buckets: usize,
    /// Number of items stored
    count: usize,
    /// Random number generator for kicks
    rng: SmallRng,
}

impl CuckooFilter {
    /// Creates a new Cuckoo Filter
    ///
    /// # Arguments
    ///
    /// * `capacity` - Expected number of elements
    ///
    /// # Errors
    ///
    /// Returns error if capacity is 0
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::membership::CuckooFilter;
    ///
    /// let filter = CuckooFilter::new(1000).unwrap();
    /// assert!(filter.is_empty());
    /// ```
    pub fn new(capacity: usize) -> Result<Self, SketchError> {
        Self::with_seed(capacity, 0x12345678)
    }

    /// Creates a Cuckoo Filter with a specific seed
    pub fn with_seed(capacity: usize, seed: u64) -> Result<Self, SketchError> {
        if capacity == 0 {
            return Err(SketchError::InvalidParameter {
                param: "capacity".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }

        // Number of buckets = capacity / bucket_size * load_factor_safety
        // Using ~95% max load factor
        let num_buckets = (capacity as f64 / BUCKET_SIZE as f64 / 0.95).ceil() as usize;
        let num_buckets = num_buckets.next_power_of_two(); // Power of 2 for fast modulo

        Ok(CuckooFilter {
            buckets: vec![Bucket::default(); num_buckets],
            num_buckets,
            count: 0,
            rng: SmallRng::seed_from_u64(seed),
        })
    }

    /// Returns the number of items stored
    pub fn len(&self) -> usize {
        self.count
    }

    /// Returns true if the filter is empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Returns the capacity (maximum items)
    pub fn capacity(&self) -> usize {
        self.num_buckets * BUCKET_SIZE
    }

    /// Returns the load factor
    pub fn load_factor(&self) -> f64 {
        self.count as f64 / self.capacity() as f64
    }

    /// Inserts an element into the filter
    ///
    /// # Arguments
    ///
    /// * `key` - The key to insert
    ///
    /// # Errors
    ///
    /// Returns error if the filter is full and insertion fails after MAX_KICKS relocations
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::membership::CuckooFilter;
    ///
    /// let mut filter = CuckooFilter::new(100).unwrap();
    /// filter.insert(b"hello").unwrap();
    /// assert!(filter.contains(b"hello"));
    /// ```
    pub fn insert(&mut self, key: &[u8]) -> Result<(), SketchError> {
        let fp = self.fingerprint(key);
        let (i1, i2) = self.bucket_indices(key, fp);

        // Try inserting into either bucket
        if self.buckets[i1].insert(fp) {
            self.count += 1;
            return Ok(());
        }
        if self.buckets[i2].insert(fp) {
            self.count += 1;
            return Ok(());
        }

        // Both buckets full, need to relocate
        let mut current_fp = fp;
        let mut current_idx = if self.rng.random::<bool>() { i1 } else { i2 };

        for _ in 0..MAX_KICKS {
            current_fp = self.buckets[current_idx].swap_random(current_fp, &mut self.rng);
            current_idx = self.alt_index(current_idx, current_fp);

            if self.buckets[current_idx].insert(current_fp) {
                self.count += 1;
                return Ok(());
            }
        }

        Err(SketchError::InvalidParameter {
            param: "filter".to_string(),
            value: "full".to_string(),
            constraint: "filter is at capacity, cannot insert".to_string(),
        })
    }

    /// Checks if an element might be in the filter
    ///
    /// # Arguments
    ///
    /// * `key` - The key to check
    ///
    /// # Returns
    ///
    /// `true` if the element might be present, `false` if definitely not
    pub fn contains(&self, key: &[u8]) -> bool {
        let fp = self.fingerprint(key);
        let (i1, i2) = self.bucket_indices(key, fp);

        self.buckets[i1].contains(fp) || self.buckets[i2].contains(fp)
    }

    /// Removes an element from the filter
    ///
    /// # Arguments
    ///
    /// * `key` - The key to remove
    ///
    /// # Returns
    ///
    /// `true` if the element was found and removed, `false` otherwise
    ///
    /// # Note
    ///
    /// Removing an element that was never inserted can cause false negatives.
    pub fn remove(&mut self, key: &[u8]) -> bool {
        let fp = self.fingerprint(key);
        let (i1, i2) = self.bucket_indices(key, fp);

        if self.buckets[i1].remove(fp) {
            self.count -= 1;
            return true;
        }
        if self.buckets[i2].remove(fp) {
            self.count -= 1;
            return true;
        }
        false
    }

    /// Computes the fingerprint for a key
    #[inline]
    fn fingerprint(&self, key: &[u8]) -> u16 {
        // Use top 16 bits of hash, ensure non-zero
        let hash = xxh64(key, 0xDEADBEEF);
        ((hash >> 48) as u16) | 1
    }

    /// Computes the two bucket indices for a key
    #[inline]
    fn bucket_indices(&self, key: &[u8], fp: u16) -> (usize, usize) {
        let hash = xxh64(key, 0);
        let i1 = (hash as usize) % self.num_buckets;
        let i2 = self.alt_index(i1, fp);
        (i1, i2)
    }

    /// Computes the alternate bucket index
    #[inline]
    fn alt_index(&self, idx: usize, fp: u16) -> usize {
        // XOR with hash of fingerprint
        let fp_hash = xxh64(&fp.to_le_bytes(), 0) as usize;
        (idx ^ fp_hash) % self.num_buckets
    }

    /// Clears all items from the filter
    pub fn clear(&mut self) {
        for bucket in &mut self.buckets {
            bucket.fingerprints = [0; BUCKET_SIZE];
        }
        self.count = 0;
    }

    /// Serializes the filter to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&(self.num_buckets as u64).to_le_bytes());
        bytes.extend_from_slice(&(self.count as u64).to_le_bytes());

        for bucket in &self.buckets {
            for &fp in &bucket.fingerprints {
                bytes.extend_from_slice(&fp.to_le_bytes());
            }
        }

        bytes
    }

    /// Deserializes a filter from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SketchError> {
        if bytes.len() < 16 {
            return Err(SketchError::DeserializationError(
                "Insufficient data for CuckooFilter header".to_string(),
            ));
        }

        let num_buckets = u64::from_le_bytes(bytes[0..8].try_into().unwrap()) as usize;
        let count = u64::from_le_bytes(bytes[8..16].try_into().unwrap()) as usize;

        let expected_len = 16 + num_buckets * BUCKET_SIZE * 2;
        if bytes.len() < expected_len {
            return Err(SketchError::DeserializationError(format!(
                "Expected {} bytes, got {}",
                expected_len,
                bytes.len()
            )));
        }

        let mut buckets = Vec::with_capacity(num_buckets);
        let mut offset = 16;

        for _ in 0..num_buckets {
            let mut bucket = Bucket::default();
            for slot in &mut bucket.fingerprints {
                *slot = u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap());
                offset += 2;
            }
            buckets.push(bucket);
        }

        Ok(CuckooFilter {
            buckets,
            num_buckets,
            count,
            rng: SmallRng::from_os_rng(),
        })
    }

    /// Returns the memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        self.num_buckets * BUCKET_SIZE * 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let filter = CuckooFilter::new(1000).unwrap();
        assert!(filter.is_empty());
    }

    #[test]
    fn test_insert_contains() {
        let mut filter = CuckooFilter::new(100).unwrap();
        filter.insert(b"hello").unwrap();
        assert!(filter.contains(b"hello"));
        assert!(!filter.is_empty());
    }

    #[test]
    fn test_remove() {
        let mut filter = CuckooFilter::new(100).unwrap();
        filter.insert(b"hello").unwrap();
        assert!(filter.contains(b"hello"));

        assert!(filter.remove(b"hello"));
        assert!(!filter.contains(b"hello"));
    }

    #[test]
    fn test_multiple_inserts() {
        let mut filter = CuckooFilter::new(100).unwrap();
        filter.insert(b"key1").unwrap();
        filter.insert(b"key2").unwrap();
        filter.insert(b"key3").unwrap();

        assert!(filter.contains(b"key1"));
        assert!(filter.contains(b"key2"));
        assert!(filter.contains(b"key3"));
    }

    #[test]
    fn test_remove_maintains_others() {
        let mut filter = CuckooFilter::new(100).unwrap();
        filter.insert(b"key1").unwrap();
        filter.insert(b"key2").unwrap();

        filter.remove(b"key1");
        assert!(!filter.contains(b"key1"));
        assert!(filter.contains(b"key2"));
    }

    #[test]
    fn test_serialization() {
        let mut filter = CuckooFilter::new(100).unwrap();
        filter.insert(b"key1").unwrap();
        filter.insert(b"key2").unwrap();

        let bytes = filter.to_bytes();
        let restored = CuckooFilter::from_bytes(&bytes).unwrap();

        assert!(restored.contains(b"key1"));
        assert!(restored.contains(b"key2"));
        assert_eq!(filter.len(), restored.len());
    }

    #[test]
    fn test_clear() {
        let mut filter = CuckooFilter::new(100).unwrap();
        filter.insert(b"hello").unwrap();
        assert!(!filter.is_empty());

        filter.clear();
        assert!(filter.is_empty());
        assert!(!filter.contains(b"hello"));
    }

    #[test]
    fn test_load_factor() {
        let mut filter = CuckooFilter::new(100).unwrap();
        assert_eq!(filter.load_factor(), 0.0);

        for i in 0u32..50 {
            filter.insert(&i.to_le_bytes()).unwrap();
        }

        assert!(filter.load_factor() > 0.0);
    }
}
