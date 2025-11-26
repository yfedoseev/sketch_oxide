//! Counting Bloom Filter: Bloom filter with deletion support
//!
//! A Counting Bloom Filter uses counters instead of bits, enabling deletions.
//! Each position stores a counter (typically 4 bits) instead of a single bit.
//!
//! # Trade-offs vs Standard Bloom Filter
//!
//! | Aspect | Standard Bloom | Counting Bloom |
//! |--------|---------------|----------------|
//! | Memory | 1 bit/position | 4 bits/position |
//! | Deletions | No | Yes |
//! | False negatives | Never | Possible (overflow) |
//! | Use case | Static sets | Dynamic sets |
//!
//! # Algorithm Overview
//!
//! - Insert: Increment counters at k hash positions
//! - Delete: Decrement counters at k hash positions
//! - Query: Check if all k counters are non-zero
//!
//! # Time Complexity
//!
//! - Insert: O(k)
//! - Delete: O(k)
//! - Query: O(k)
//!
//! # Space Complexity
//!
//! O(m) where m is the number of counters (4 bits each)
//!
//! # References
//!
//! - Fan et al. "Summary Cache: A Scalable Wide-Area Web Cache Sharing Protocol" (2000)
//!
//! # Examples
//!
//! ```
//! use sketch_oxide::membership::CountingBloomFilter;
//!
//! let mut filter = CountingBloomFilter::new(1000, 0.01);
//!
//! filter.insert(b"key1");
//! filter.insert(b"key2");
//! assert!(filter.contains(b"key1"));
//!
//! filter.remove(b"key1");
//! assert!(!filter.contains(b"key1")); // Successfully removed
//! assert!(filter.contains(b"key2"));  // Still present
//! ```

use crate::common::SketchError;
use xxhash_rust::xxh64::xxh64;

/// Counting Bloom Filter with 4-bit counters
///
/// Supports insertions, deletions, and membership queries.
/// Uses 4x more memory than standard Bloom filter but enables deletions.
///
/// # Examples
///
/// ```
/// use sketch_oxide::membership::CountingBloomFilter;
///
/// let mut filter = CountingBloomFilter::new(100, 0.01);
/// filter.insert(b"hello");
/// assert!(filter.contains(b"hello"));
///
/// filter.remove(b"hello");
/// assert!(!filter.contains(b"hello"));
/// ```
#[derive(Clone, Debug)]
pub struct CountingBloomFilter {
    /// 4-bit counters packed into bytes (2 counters per byte)
    counters: Vec<u8>,
    /// Number of hash functions
    k: usize,
    /// Number of counters
    m: usize,
    /// Expected number of elements
    n: usize,
    /// Number of items currently in the filter (approximate)
    count: usize,
    /// Whether any counter has overflowed
    has_overflow: bool,
}

impl CountingBloomFilter {
    /// Maximum counter value (4 bits = 15)
    const MAX_COUNT: u8 = 15;

    /// Creates a new Counting Bloom Filter
    ///
    /// # Arguments
    ///
    /// * `n` - Expected number of elements
    /// * `fpr` - Desired false positive rate (e.g., 0.01 for 1%)
    ///
    /// # Panics
    ///
    /// Panics if `n` is 0 or `fpr` is not in range (0, 1)
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::membership::CountingBloomFilter;
    ///
    /// let filter = CountingBloomFilter::new(1000, 0.01);
    /// assert!(filter.is_empty());
    /// ```
    pub fn new(n: usize, fpr: f64) -> Self {
        assert!(n > 0, "Expected number of elements must be > 0");
        assert!(
            fpr > 0.0 && fpr < 1.0,
            "False positive rate must be in (0, 1)"
        );

        // Optimal bit count: m = -n * ln(fpr) / (ln(2)^2)
        let m = (-(n as f64) * fpr.ln() / (std::f64::consts::LN_2.powi(2))).ceil() as usize;

        // Optimal hash function count: k = (m/n) * ln(2)
        let k = ((m as f64 / n as f64) * std::f64::consts::LN_2).ceil() as usize;
        let k = k.max(1);

        Self::with_params(n, m, k)
    }

    /// Creates a Counting Bloom Filter with specific parameters
    ///
    /// # Arguments
    ///
    /// * `n` - Expected number of elements
    /// * `m` - Number of counters
    /// * `k` - Number of hash functions
    pub fn with_params(n: usize, m: usize, k: usize) -> Self {
        assert!(n > 0, "Expected number of elements must be > 0");
        assert!(m > 0, "Number of counters must be > 0");
        assert!(k > 0, "Number of hash functions must be > 0");

        // Each byte holds 2 counters (4 bits each)
        let num_bytes = m.div_ceil(2);

        CountingBloomFilter {
            counters: vec![0u8; num_bytes],
            k,
            m,
            n,
            count: 0,
            has_overflow: false,
        }
    }

    /// Returns the number of counters
    pub fn num_counters(&self) -> usize {
        self.m
    }

    /// Returns the number of hash functions
    pub fn num_hash_functions(&self) -> usize {
        self.k
    }

    /// Returns the expected capacity
    pub fn capacity(&self) -> usize {
        self.n
    }

    /// Returns the approximate number of items inserted
    pub fn len(&self) -> usize {
        self.count
    }

    /// Returns true if no items have been inserted
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Returns true if any counter has overflowed
    ///
    /// Overflow means a counter reached 15 and more inserts were attempted.
    /// This can cause false negatives after deletions.
    pub fn has_overflow(&self) -> bool {
        self.has_overflow
    }

    /// Inserts an element into the filter
    ///
    /// # Arguments
    ///
    /// * `key` - The key to insert
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::membership::CountingBloomFilter;
    ///
    /// let mut filter = CountingBloomFilter::new(100, 0.01);
    /// filter.insert(b"hello");
    /// assert!(filter.contains(b"hello"));
    /// ```
    pub fn insert(&mut self, key: &[u8]) {
        for i in 0..self.k {
            let idx = self.hash(key, i);
            self.increment_counter(idx);
        }
        self.count += 1;
    }

    /// Removes an element from the filter
    ///
    /// # Arguments
    ///
    /// * `key` - The key to remove
    ///
    /// # Returns
    ///
    /// `true` if the element was present (all counters were non-zero),
    /// `false` otherwise (element was not present)
    ///
    /// # Note
    ///
    /// Removing an element that was never inserted can cause false negatives
    /// for other elements. Only remove elements you know were inserted.
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::membership::CountingBloomFilter;
    ///
    /// let mut filter = CountingBloomFilter::new(100, 0.01);
    /// filter.insert(b"hello");
    /// assert!(filter.remove(b"hello"));
    /// assert!(!filter.contains(b"hello"));
    /// ```
    pub fn remove(&mut self, key: &[u8]) -> bool {
        // First check if the element might be present
        if !self.contains(key) {
            return false;
        }

        // Decrement all counters
        for i in 0..self.k {
            let idx = self.hash(key, i);
            self.decrement_counter(idx);
        }

        if self.count > 0 {
            self.count -= 1;
        }

        true
    }

    /// Checks if an element might be in the filter
    ///
    /// # Arguments
    ///
    /// * `key` - The key to check
    ///
    /// # Returns
    ///
    /// `true` if the element might be present, `false` if definitely not present
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::membership::CountingBloomFilter;
    ///
    /// let mut filter = CountingBloomFilter::new(100, 0.01);
    /// filter.insert(b"hello");
    /// assert!(filter.contains(b"hello"));
    /// assert!(!filter.contains(b"world")); // Probably false
    /// ```
    pub fn contains(&self, key: &[u8]) -> bool {
        for i in 0..self.k {
            let idx = self.hash(key, i);
            if self.get_counter(idx) == 0 {
                return false;
            }
        }
        true
    }

    /// Returns the approximate count for an element
    ///
    /// This is the minimum counter value among all hash positions.
    /// It's an upper bound on the actual count.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to count
    ///
    /// # Returns
    ///
    /// Minimum counter value (0-15)
    pub fn count_estimate(&self, key: &[u8]) -> u8 {
        let mut min_count = Self::MAX_COUNT;
        for i in 0..self.k {
            let idx = self.hash(key, i);
            let count = self.get_counter(idx);
            min_count = min_count.min(count);
        }
        min_count
    }

    /// Gets the counter value at a given index
    #[inline]
    fn get_counter(&self, idx: usize) -> u8 {
        let byte_idx = idx / 2;
        if idx.is_multiple_of(2) {
            self.counters[byte_idx] & 0x0F
        } else {
            (self.counters[byte_idx] >> 4) & 0x0F
        }
    }

    /// Increments the counter at a given index
    #[inline]
    fn increment_counter(&mut self, idx: usize) {
        let byte_idx = idx / 2;
        let current = self.get_counter(idx);

        if current < Self::MAX_COUNT {
            if idx.is_multiple_of(2) {
                self.counters[byte_idx] = (self.counters[byte_idx] & 0xF0) | (current + 1);
            } else {
                self.counters[byte_idx] = (self.counters[byte_idx] & 0x0F) | ((current + 1) << 4);
            }
        } else {
            self.has_overflow = true;
        }
    }

    /// Decrements the counter at a given index
    #[inline]
    fn decrement_counter(&mut self, idx: usize) {
        let byte_idx = idx / 2;
        let current = self.get_counter(idx);

        if current > 0 {
            if idx.is_multiple_of(2) {
                self.counters[byte_idx] = (self.counters[byte_idx] & 0xF0) | (current - 1);
            } else {
                self.counters[byte_idx] = (self.counters[byte_idx] & 0x0F) | ((current - 1) << 4);
            }
        }
    }

    /// Computes hash for a key and hash function index
    #[inline]
    fn hash(&self, key: &[u8], i: usize) -> usize {
        // Double hashing: h(k, i) = h1(k) + i * h2(k)
        let h1 = xxh64(key, 0) as usize;
        let h2 = xxh64(key, 0x9E3779B9) as usize;
        (h1.wrapping_add(i.wrapping_mul(h2))) % self.m
    }

    /// Clears all counters
    pub fn clear(&mut self) {
        self.counters.fill(0);
        self.count = 0;
        self.has_overflow = false;
    }

    /// Merges another Counting Bloom Filter into this one
    ///
    /// Counters are added together (with saturation at MAX_COUNT).
    ///
    /// # Errors
    ///
    /// Returns error if filters have different parameters
    pub fn merge(&mut self, other: &Self) -> Result<(), SketchError> {
        if self.m != other.m || self.k != other.k {
            return Err(SketchError::IncompatibleSketches {
                reason: "Counting Bloom filters have different parameters".to_string(),
            });
        }

        for i in 0..self.m {
            let self_count = self.get_counter(i);
            let other_count = other.get_counter(i);
            let new_count =
                (self_count as u16 + other_count as u16).min(Self::MAX_COUNT as u16) as u8;

            let byte_idx = i / 2;
            if i % 2 == 0 {
                self.counters[byte_idx] = (self.counters[byte_idx] & 0xF0) | new_count;
            } else {
                self.counters[byte_idx] = (self.counters[byte_idx] & 0x0F) | (new_count << 4);
            }
        }

        self.count += other.count;
        self.has_overflow = self.has_overflow || other.has_overflow;

        Ok(())
    }

    /// Serializes the filter to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(24 + self.counters.len());

        bytes.extend_from_slice(&(self.m as u64).to_le_bytes());
        bytes.extend_from_slice(&(self.k as u64).to_le_bytes());
        bytes.extend_from_slice(&(self.n as u64).to_le_bytes());
        bytes.extend_from_slice(&self.counters);

        bytes
    }

    /// Deserializes a filter from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SketchError> {
        if bytes.len() < 24 {
            return Err(SketchError::DeserializationError(
                "Insufficient data for CountingBloomFilter header".to_string(),
            ));
        }

        let m = u64::from_le_bytes(bytes[0..8].try_into().unwrap()) as usize;
        let k = u64::from_le_bytes(bytes[8..16].try_into().unwrap()) as usize;
        let n = u64::from_le_bytes(bytes[16..24].try_into().unwrap()) as usize;

        let expected_len = 24 + m.div_ceil(2);
        if bytes.len() < expected_len {
            return Err(SketchError::DeserializationError(format!(
                "Expected {} bytes, got {}",
                expected_len,
                bytes.len()
            )));
        }

        let counters = bytes[24..expected_len].to_vec();

        // Count non-zero positions to estimate count
        let mut count = 0;
        for i in 0..m {
            let byte_idx = i / 2;
            let val = if i % 2 == 0 {
                counters[byte_idx] & 0x0F
            } else {
                (counters[byte_idx] >> 4) & 0x0F
            };
            if val > 0 {
                count += 1;
            }
        }
        count /= k.max(1);

        Ok(CountingBloomFilter {
            counters,
            k,
            m,
            n,
            count,
            has_overflow: false,
        })
    }

    /// Returns the theoretical false positive rate
    pub fn false_positive_rate(&self) -> f64 {
        let fill_ratio = 1.0 - (-((self.count * self.k) as f64) / self.m as f64).exp();
        fill_ratio.powi(self.k as i32)
    }

    /// Returns the memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        self.counters.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let filter = CountingBloomFilter::new(1000, 0.01);
        assert!(filter.is_empty());
        assert!(!filter.has_overflow());
    }

    #[test]
    fn test_insert_contains() {
        let mut filter = CountingBloomFilter::new(100, 0.01);
        filter.insert(b"hello");
        assert!(filter.contains(b"hello"));
        assert!(!filter.is_empty());
    }

    #[test]
    fn test_remove() {
        let mut filter = CountingBloomFilter::new(100, 0.01);
        filter.insert(b"hello");
        assert!(filter.contains(b"hello"));

        assert!(filter.remove(b"hello"));
        assert!(!filter.contains(b"hello"));
    }

    #[test]
    fn test_multiple_inserts() {
        let mut filter = CountingBloomFilter::new(100, 0.01);
        filter.insert(b"key1");
        filter.insert(b"key2");
        filter.insert(b"key3");

        assert!(filter.contains(b"key1"));
        assert!(filter.contains(b"key2"));
        assert!(filter.contains(b"key3"));
        assert!(!filter.contains(b"key4"));
    }

    #[test]
    fn test_remove_maintains_others() {
        let mut filter = CountingBloomFilter::new(100, 0.01);
        filter.insert(b"key1");
        filter.insert(b"key2");

        filter.remove(b"key1");
        assert!(!filter.contains(b"key1"));
        assert!(filter.contains(b"key2"));
    }

    #[test]
    fn test_count_estimate() {
        let mut filter = CountingBloomFilter::new(100, 0.01);
        filter.insert(b"hello");
        filter.insert(b"hello");
        filter.insert(b"hello");

        let count = filter.count_estimate(b"hello");
        assert!(count >= 1, "Count should be at least 1");
    }

    #[test]
    fn test_serialization() {
        let mut filter = CountingBloomFilter::new(100, 0.01);
        filter.insert(b"key1");
        filter.insert(b"key2");

        let bytes = filter.to_bytes();
        let restored = CountingBloomFilter::from_bytes(&bytes).unwrap();

        assert!(restored.contains(b"key1"));
        assert!(restored.contains(b"key2"));
        assert!(!restored.contains(b"key3"));
    }

    #[test]
    fn test_clear() {
        let mut filter = CountingBloomFilter::new(100, 0.01);
        filter.insert(b"hello");
        assert!(!filter.is_empty());

        filter.clear();
        assert!(filter.is_empty());
        assert!(!filter.contains(b"hello"));
    }
}
