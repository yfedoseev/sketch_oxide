//! Stable Bloom Filter: Bounded false positives for unbounded streams (Deng 2006)
//!
//! Stable Bloom filters maintain bounded false positive rates for infinite streams
//! by continuously evicting stale information. Unlike traditional Bloom filters
//! that saturate over time, Stable Bloom filters achieve a stable state.
//!
//! # Algorithm Overview
//!
//! - Uses d-bit counters (typically 2-3 bits) instead of single bits
//! - On each insert, randomly decrement P counters before setting
//! - This "aging" process continuously evicts old information
//! - Achieves stable false positive rate independent of stream length
//!
//! # Use Cases
//!
//! - Deduplication in unbounded data streams
//! - Rate limiting with natural decay
//! - Network packet filtering
//! - Duplicate URL detection in web crawlers
//!
//! # Comparison with Other Filters
//!
//! | Filter | Bounded FPR | Deletions | Infinite Stream |
//! |--------|-------------|-----------|-----------------|
//! | Bloom | No (saturates) | No | No |
//! | Counting | No | Yes | No |
//! | Cuckoo | No | Yes | No |
//! | Stable | Yes | Implicit | Yes |
//!
//! # Time Complexity
//!
//! - Insert: O(k + P) where k = hash functions, P = decrement count
//! - Query: O(k)
//!
//! # Space Complexity
//!
//! O(m * d) bits where m = number of cells, d = bits per counter
//!
//! # References
//!
//! - Deng & Rafiei "Approximately Detecting Duplicates for Streaming Data
//!   using Stable Bloom Filters" (SIGMOD 2006)
//!
//! # Examples
//!
//! ```
//! use sketch_oxide::membership::StableBloomFilter;
//!
//! let mut filter = StableBloomFilter::new(10000, 0.01).unwrap();
//!
//! filter.insert(b"item1");
//! filter.insert(b"item2");
//!
//! // Recently inserted items are likely present
//! assert!(filter.contains(b"item1"));
//! assert!(filter.contains(b"item2"));
//! ```

use crate::common::SketchError;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use xxhash_rust::xxh64::xxh64;

/// Default number of bits per counter (max value = 2^d - 1)
const DEFAULT_COUNTER_BITS: u8 = 3;

/// Stable Bloom Filter for duplicate detection in unbounded streams
///
/// Maintains bounded false positive rate by continuously evicting stale data
/// through random counter decrements.
///
/// # Examples
///
/// ```
/// use sketch_oxide::membership::StableBloomFilter;
///
/// let mut filter = StableBloomFilter::new(10000, 0.01).unwrap();
/// filter.insert(b"hello");
/// assert!(filter.contains(b"hello"));
/// ```
#[derive(Clone, Debug)]
pub struct StableBloomFilter {
    /// Counter storage (packed based on counter_bits)
    counters: Vec<u8>,
    /// Number of counters
    m: usize,
    /// Number of hash functions
    k: usize,
    /// Bits per counter
    counter_bits: u8,
    /// Maximum counter value
    max_counter: u8,
    /// Number of counters to decrement per insert
    p: usize,
    /// Random number generator for decrements
    rng: SmallRng,
}

impl StableBloomFilter {
    /// Creates a new Stable Bloom Filter
    ///
    /// # Arguments
    ///
    /// * `expected_items` - Expected number of recent items to track
    /// * `fpr` - Target false positive rate
    ///
    /// # Errors
    ///
    /// Returns error if parameters are invalid
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::membership::StableBloomFilter;
    ///
    /// let filter = StableBloomFilter::new(10000, 0.01).unwrap();
    /// ```
    pub fn new(expected_items: usize, fpr: f64) -> Result<Self, SketchError> {
        Self::with_params(expected_items, fpr, DEFAULT_COUNTER_BITS, 0x12345678)
    }

    /// Creates a Stable Bloom Filter with custom parameters
    ///
    /// # Arguments
    ///
    /// * `expected_items` - Expected number of recent items to track
    /// * `fpr` - Target false positive rate
    /// * `counter_bits` - Bits per counter (1-8)
    /// * `seed` - Random seed for decrement selection
    pub fn with_params(
        expected_items: usize,
        fpr: f64,
        counter_bits: u8,
        seed: u64,
    ) -> Result<Self, SketchError> {
        if expected_items == 0 {
            return Err(SketchError::InvalidParameter {
                param: "expected_items".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }

        if fpr <= 0.0 || fpr >= 1.0 {
            return Err(SketchError::InvalidParameter {
                param: "fpr".to_string(),
                value: fpr.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }

        if counter_bits == 0 || counter_bits > 8 {
            return Err(SketchError::InvalidParameter {
                param: "counter_bits".to_string(),
                value: counter_bits.to_string(),
                constraint: "must be in [1, 8]".to_string(),
            });
        }

        // Calculate optimal parameters
        // m = -n * ln(fpr) / (ln(2)^2) but scaled up for stable behavior
        let m =
            ((-(expected_items as f64) * fpr.ln()) / (2.0_f64.ln().powi(2)) * 2.0).ceil() as usize;
        let m = m.max(64); // Minimum size

        // k = (m/n) * ln(2)
        let k = ((m as f64 / expected_items as f64) * 2.0_f64.ln()).ceil() as usize;
        let k = k.clamp(1, 30);

        // P (cells to decrement) = m * k / (n * (2^d - 1))
        // This achieves stable state where expected decrements = expected increments
        let max_counter = (1u16 << counter_bits) - 1;
        let p = ((m * k) as f64 / (expected_items as f64 * max_counter as f64)).ceil() as usize;
        let p = p.clamp(1, m);

        // Pack counters efficiently
        let counters_per_byte = 8 / counter_bits as usize;
        let byte_count = m.div_ceil(counters_per_byte);

        Ok(StableBloomFilter {
            counters: vec![0; byte_count],
            m,
            k,
            counter_bits,
            max_counter: max_counter as u8,
            p,
            rng: SmallRng::seed_from_u64(seed),
        })
    }

    /// Returns the number of counters
    pub fn num_counters(&self) -> usize {
        self.m
    }

    /// Returns the number of hash functions
    pub fn num_hashes(&self) -> usize {
        self.k
    }

    /// Returns the number of counters decremented per insert
    pub fn decrement_count(&self) -> usize {
        self.p
    }

    /// Inserts an element into the filter
    ///
    /// This also randomly decrements P counters to maintain stability.
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::membership::StableBloomFilter;
    ///
    /// let mut filter = StableBloomFilter::new(1000, 0.01).unwrap();
    /// filter.insert(b"hello");
    /// assert!(filter.contains(b"hello"));
    /// ```
    pub fn insert(&mut self, key: &[u8]) {
        // First, decrement P random counters
        for _ in 0..self.p {
            let idx = self.rng.random_range(0..self.m);
            self.decrement_counter(idx);
        }

        // Then set all k hash positions to max
        let indices = self.hash_indices(key);
        for idx in indices {
            self.set_counter(idx, self.max_counter);
        }
    }

    /// Checks if an element might be in the filter
    ///
    /// # Returns
    ///
    /// `true` if all counters for the element are non-zero (might be present),
    /// `false` if any counter is zero (definitely not recently present)
    pub fn contains(&self, key: &[u8]) -> bool {
        let indices = self.hash_indices(key);
        indices.into_iter().all(|idx| self.get_counter(idx) > 0)
    }

    /// Gets the minimum counter value for an element
    ///
    /// Higher values suggest more recent insertion.
    pub fn get_count(&self, key: &[u8]) -> u8 {
        let indices = self.hash_indices(key);
        indices
            .into_iter()
            .map(|idx| self.get_counter(idx))
            .min()
            .unwrap_or(0)
    }

    /// Computes hash indices for a key
    #[inline]
    fn hash_indices(&self, key: &[u8]) -> Vec<usize> {
        let h1 = xxh64(key, 0);
        let h2 = xxh64(key, h1);

        (0..self.k)
            .map(|i| {
                let combined = h1.wrapping_add((i as u64).wrapping_mul(h2));
                (combined as usize) % self.m
            })
            .collect()
    }

    /// Gets a counter value
    #[inline]
    fn get_counter(&self, idx: usize) -> u8 {
        let counters_per_byte = 8 / self.counter_bits as usize;
        let byte_idx = idx / counters_per_byte;
        let offset = (idx % counters_per_byte) * self.counter_bits as usize;
        let mask = self.max_counter;

        (self.counters[byte_idx] >> offset) & mask
    }

    /// Sets a counter value
    #[inline]
    fn set_counter(&mut self, idx: usize, value: u8) {
        let counters_per_byte = 8 / self.counter_bits as usize;
        let byte_idx = idx / counters_per_byte;
        let offset = (idx % counters_per_byte) * self.counter_bits as usize;
        let mask = self.max_counter;

        // Clear the counter bits
        self.counters[byte_idx] &= !(mask << offset);
        // Set new value
        self.counters[byte_idx] |= (value & mask) << offset;
    }

    /// Decrements a counter by 1 (saturating at 0)
    #[inline]
    fn decrement_counter(&mut self, idx: usize) {
        let current = self.get_counter(idx);
        if current > 0 {
            self.set_counter(idx, current - 1);
        }
    }

    /// Returns the estimated fill ratio (fraction of non-zero counters)
    pub fn fill_ratio(&self) -> f64 {
        let non_zero = (0..self.m).filter(|&i| self.get_counter(i) > 0).count();
        non_zero as f64 / self.m as f64
    }

    /// Clears all counters
    pub fn clear(&mut self) {
        self.counters.fill(0);
    }

    /// Returns memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        self.counters.len()
    }

    /// Serializes the filter to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&(self.m as u64).to_le_bytes());
        bytes.extend_from_slice(&(self.k as u64).to_le_bytes());
        bytes.push(self.counter_bits);
        bytes.push(self.max_counter);
        bytes.extend_from_slice(&(self.p as u64).to_le_bytes());
        bytes.extend_from_slice(&(self.counters.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&self.counters);

        bytes
    }

    /// Deserializes a filter from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SketchError> {
        if bytes.len() < 34 {
            return Err(SketchError::DeserializationError(
                "Insufficient data for StableBloomFilter header".to_string(),
            ));
        }

        let m = u64::from_le_bytes(bytes[0..8].try_into().unwrap()) as usize;
        let k = u64::from_le_bytes(bytes[8..16].try_into().unwrap()) as usize;
        let counter_bits = bytes[16];
        let max_counter = bytes[17];
        let p = u64::from_le_bytes(bytes[18..26].try_into().unwrap()) as usize;
        let counter_len = u64::from_le_bytes(bytes[26..34].try_into().unwrap()) as usize;

        if bytes.len() < 34 + counter_len {
            return Err(SketchError::DeserializationError(format!(
                "Expected {} bytes, got {}",
                34 + counter_len,
                bytes.len()
            )));
        }

        let counters = bytes[34..34 + counter_len].to_vec();

        Ok(StableBloomFilter {
            counters,
            m,
            k,
            counter_bits,
            max_counter,
            p,
            rng: SmallRng::from_os_rng(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let filter = StableBloomFilter::new(1000, 0.01).unwrap();
        assert!(filter.num_counters() > 0);
        assert!(filter.num_hashes() > 0);
    }

    #[test]
    fn test_invalid_params() {
        assert!(StableBloomFilter::new(0, 0.01).is_err());
        assert!(StableBloomFilter::new(1000, 0.0).is_err());
        assert!(StableBloomFilter::new(1000, 1.0).is_err());
        assert!(StableBloomFilter::with_params(1000, 0.01, 0, 0).is_err());
        assert!(StableBloomFilter::with_params(1000, 0.01, 9, 0).is_err());
    }

    #[test]
    fn test_insert_contains() {
        let mut filter = StableBloomFilter::new(1000, 0.01).unwrap();
        filter.insert(b"hello");
        assert!(filter.contains(b"hello"));
    }

    #[test]
    fn test_multiple_inserts() {
        let mut filter = StableBloomFilter::new(1000, 0.01).unwrap();
        filter.insert(b"key1");
        filter.insert(b"key2");
        filter.insert(b"key3");

        // Recently inserted should be present
        assert!(filter.contains(b"key1"));
        assert!(filter.contains(b"key2"));
        assert!(filter.contains(b"key3"));
    }

    #[test]
    fn test_not_contains() {
        let filter = StableBloomFilter::new(1000, 0.01).unwrap();
        // Empty filter should not contain anything
        assert!(!filter.contains(b"missing"));
    }

    #[test]
    fn test_stability() {
        // Test that filter doesn't saturate with many inserts
        let mut filter = StableBloomFilter::with_params(100, 0.1, 2, 42).unwrap();

        // Insert many more items than capacity
        for i in 0..10000u64 {
            filter.insert(&i.to_le_bytes());
        }

        // Filter should not be completely full
        let fill = filter.fill_ratio();
        assert!(fill < 1.0, "Filter should not saturate: fill={}", fill);
        assert!(
            fill > 0.5,
            "Filter should maintain some data: fill={}",
            fill
        );
    }

    #[test]
    fn test_get_count() {
        let mut filter = StableBloomFilter::new(1000, 0.01).unwrap();
        filter.insert(b"hello");

        let count = filter.get_count(b"hello");
        assert!(count > 0);

        let count_missing = filter.get_count(b"missing");
        assert_eq!(count_missing, 0);
    }

    #[test]
    fn test_clear() {
        let mut filter = StableBloomFilter::new(1000, 0.01).unwrap();
        filter.insert(b"hello");
        assert!(filter.contains(b"hello"));

        filter.clear();
        assert!(!filter.contains(b"hello"));
        assert_eq!(filter.fill_ratio(), 0.0);
    }

    #[test]
    fn test_serialization() {
        let mut filter = StableBloomFilter::new(1000, 0.01).unwrap();
        filter.insert(b"key1");
        filter.insert(b"key2");

        let bytes = filter.to_bytes();
        let restored = StableBloomFilter::from_bytes(&bytes).unwrap();

        assert!(restored.contains(b"key1"));
        assert!(restored.contains(b"key2"));
        assert_eq!(filter.num_counters(), restored.num_counters());
        assert_eq!(filter.num_hashes(), restored.num_hashes());
    }

    #[test]
    fn test_different_counter_bits() {
        for bits in 1..=8 {
            let filter = StableBloomFilter::with_params(100, 0.1, bits, 0).unwrap();
            let expected_max = if bits == 8 { 255u8 } else { (1u8 << bits) - 1 };
            assert_eq!(filter.max_counter, expected_max);
        }
    }

    #[test]
    fn test_counter_packing() {
        // Test with 2-bit counters (4 per byte)
        let mut filter = StableBloomFilter::with_params(100, 0.1, 2, 42).unwrap();

        // Insert and verify counters are set properly
        filter.insert(b"test");
        assert!(filter.contains(b"test"));
    }

    #[test]
    fn test_memory_usage() {
        let filter = StableBloomFilter::new(1000, 0.01).unwrap();
        let memory = filter.memory_usage();
        assert!(memory > 0);
    }
}
