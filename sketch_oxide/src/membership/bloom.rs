//! Standard Bloom Filter implementation
//!
//! A space-efficient probabilistic data structure for set membership queries.
//! Optimized for LSM-tree SSTable filtering.
//!
//! # Optimizations
//! - **Kirsch-Mitzenmacher double hashing**: Derive k hash functions from just 2 base hashes
//!   using h_i(x) = h1(x) + i * h2(x). This reduces k hash computations to just 2.
//! - **Lemire's fast range**: Use multiplication instead of modulo for range reduction
//! - **Unsafe unchecked access**: Skip bounds checks in hot paths
//!
//! # Features
//! - Configurable false positive rate
//! - Serialization/deserialization support
//! - Zero false negatives guaranteed
//!
//! # Example
//! ```
//! use sketch_oxide::membership::BloomFilter;
//!
//! // Create filter for 1000 elements with 1% false positive rate
//! let mut filter = BloomFilter::new(1000, 0.01);
//! filter.insert(b"key1");
//! filter.insert(b"key2");
//!
//! assert!(filter.contains(b"key1"));
//! assert!(!filter.contains(b"key3")); // Probably false
//! ```

use xxhash_rust::xxh64::xxh64;

/// Standard Bloom filter for membership testing
#[derive(Clone)]
pub struct BloomFilter {
    /// Bit array
    bits: Vec<u64>,
    /// Number of hash functions
    k: usize,
    /// Number of bits
    m: usize,
    /// Expected number of elements
    n: usize,
}

impl BloomFilter {
    /// Creates a new Bloom filter
    ///
    /// # Arguments
    /// * `n` - Expected number of elements
    /// * `fpr` - Desired false positive rate (e.g., 0.01 for 1%)
    ///
    /// # Panics
    /// Panics if `n` is 0 or `fpr` is not in range (0, 1)
    pub fn new(n: usize, fpr: f64) -> Self {
        Self::try_new(n, fpr).expect("invalid Bloom filter parameters")
    }

    /// Creates a Bloom filter, returning an error on invalid parameters instead
    /// of panicking — the library-wide constructor convention (fable5 doc 01 F4).
    ///
    /// # Errors
    /// Returns `InvalidParameter` if `n == 0` or `fpr` is not in `(0, 1)`.
    pub fn try_new(n: usize, fpr: f64) -> Result<Self, crate::common::SketchError> {
        use crate::common::SketchError;
        if n == 0 {
            return Err(SketchError::InvalidParameter {
                param: "n".to_string(),
                value: n.to_string(),
                constraint: "must be greater than 0".to_string(),
            });
        }
        if !(fpr > 0.0 && fpr < 1.0) {
            return Err(SketchError::InvalidParameter {
                param: "fpr".to_string(),
                value: fpr.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }

        // Optimal bit count: m = -n * ln(fpr) / (ln(2)^2)
        let m = (-(n as f64) * fpr.ln() / (std::f64::consts::LN_2.powi(2))).ceil() as usize;

        // Optimal hash function count: k = (m/n) * ln(2)
        let k = ((m as f64 / n as f64) * std::f64::consts::LN_2).ceil() as usize;
        let k = k.max(1); // At least one hash function

        let num_words = m.div_ceil(64); // Round up to nearest 64 bits

        Ok(Self {
            bits: vec![0u64; num_words],
            k,
            m,
            n,
        })
    }

    /// Creates a Bloom filter with specific parameters
    ///
    /// # Arguments
    /// * `n` - Expected number of elements
    /// * `m` - Number of bits
    /// * `k` - Number of hash functions
    pub fn with_params(n: usize, m: usize, k: usize) -> Self {
        assert!(n > 0, "Expected number of elements must be > 0");
        assert!(m > 0, "Number of bits must be > 0");
        assert!(k > 0, "Number of hash functions must be > 0");

        let num_words = m.div_ceil(64);

        Self {
            bits: vec![0u64; num_words],
            k,
            m,
            n,
        }
    }

    /// Compute two base hashes using Kirsch-Mitzenmacher technique
    /// Returns (h1, h2) where h1 and h2 are independent 64-bit hashes
    #[inline(always)]
    fn base_hashes(&self, key: &[u8]) -> (u64, u64) {
        // Use xxh64 with different seeds to get two independent hashes
        let h1 = xxh64(key, 0);
        let h2 = xxh64(key, 1);
        (h1, h2)
    }

    /// Lemire's fast range reduction: map hash to [0, range) without division
    /// This is equivalent to (hash % range) but faster
    #[inline(always)]
    fn fast_range(hash: u64, range: usize) -> usize {
        // fastrange64: ((__uint128_t)hash * range) >> 64
        // We use u128 to avoid overflow
        (((hash as u128) * (range as u128)) >> 64) as usize
    }

    /// Inserts an element into the filter
    ///
    /// Uses Kirsch-Mitzenmacher double hashing: compute only 2 hashes,
    /// derive k positions using h_i(x) = h1(x) + i * h2(x)
    #[inline]
    pub fn insert(&mut self, key: &[u8]) {
        let (h1, h2) = self.base_hashes(key);
        let m = self.m;

        for i in 0..self.k {
            // Kirsch-Mitzenmacher: h_i(x) = h1 + i * h2
            let combined = h1.wrapping_add((i as u64).wrapping_mul(h2));
            let bit_index = Self::fast_range(combined, m);
            let word_index = bit_index / 64;
            let bit_offset = bit_index % 64;

            // SAFETY: bit_index is always < m, and word_index < bits.len() by construction
            unsafe {
                *self.bits.get_unchecked_mut(word_index) |= 1u64 << bit_offset;
            }
        }
    }

    /// Checks if an element might be in the set
    ///
    /// Returns `true` if the element might be in the set (may be false positive)
    /// Returns `false` if the element is definitely not in the set (no false negatives)
    ///
    /// Uses Kirsch-Mitzenmacher double hashing for fast lookups.
    #[inline]
    pub fn contains(&self, key: &[u8]) -> bool {
        let (h1, h2) = self.base_hashes(key);
        let m = self.m;

        for i in 0..self.k {
            // Kirsch-Mitzenmacher: h_i(x) = h1 + i * h2
            let combined = h1.wrapping_add((i as u64).wrapping_mul(h2));
            let bit_index = Self::fast_range(combined, m);
            let word_index = bit_index / 64;
            let bit_offset = bit_index % 64;

            // SAFETY: bit_index is always < m, and word_index < bits.len() by construction
            let word = unsafe { *self.bits.get_unchecked(word_index) };
            if word & (1u64 << bit_offset) == 0 {
                return false;
            }
        }
        true
    }

    /// Clears all bits in the filter
    pub fn clear(&mut self) {
        self.bits.fill(0);
    }

    /// Returns the number of bits set to 1
    pub fn count_bits(&self) -> usize {
        self.bits
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum()
    }

    /// Returns the theoretical false positive rate
    pub fn false_positive_rate(&self) -> f64 {
        let bits_set = self.count_bits() as f64 / self.m as f64;
        bits_set.powi(self.k as i32)
    }

    /// Returns the memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        self.bits.len() * 8 // 8 bytes per u64
    }

    /// Serializes the filter to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Header: [n: 8 bytes][m: 8 bytes][k: 8 bytes]
        bytes.extend_from_slice(&self.n.to_le_bytes());
        bytes.extend_from_slice(&self.m.to_le_bytes());
        bytes.extend_from_slice(&self.k.to_le_bytes());

        // Bit array
        for word in &self.bits {
            bytes.extend_from_slice(&word.to_le_bytes());
        }

        bytes
    }

    /// Deserializes a filter from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < 24 {
            return Err("Insufficient bytes for header");
        }

        let n = usize::from_le_bytes(bytes[0..8].try_into().unwrap());
        let m = usize::from_le_bytes(bytes[8..16].try_into().unwrap());
        let k = usize::from_le_bytes(bytes[16..24].try_into().unwrap());

        // Bound `m` (the bit count) read from untrusted input BEFORE it drives
        // any allocation or size arithmetic. A crafted `m` near usize::MAX would
        // overflow `num_words * 8` — under overflow-checks=true that aborts the
        // process (DoS). Cap at 2^32 bits (<=512 MiB of backing storage).
        const MAX_M_BITS: usize = 1 << 32;
        if m > MAX_M_BITS {
            return Err("Invalid byte array size");
        }

        let num_words = m.div_ceil(64);
        // Checked arithmetic so overflow returns an Err instead of aborting.
        let expected_size = num_words
            .checked_mul(8)
            .and_then(|words_bytes| words_bytes.checked_add(24))
            .ok_or("Invalid byte array size")?;

        if bytes.len() != expected_size {
            return Err("Invalid byte array size");
        }

        let mut bits = Vec::with_capacity(num_words);
        for i in 0..num_words {
            let offset = 24 + i * 8;
            let word = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
            bits.push(word);
        }

        Ok(Self { bits, k, m, n })
    }

    /// Returns filter parameters (n, m, k)
    pub fn params(&self) -> (usize, usize, usize) {
        (self.n, self.m, self.k)
    }

    /// Returns true if no elements have been inserted
    pub fn is_empty(&self) -> bool {
        self.count_bits() == 0
    }

    /// Returns the expected number of elements (capacity)
    pub fn len(&self) -> usize {
        // Approximate count based on fill ratio
        // This is an estimate, not exact count
        let fill_ratio = self.count_bits() as f64 / self.m as f64;
        if fill_ratio >= 1.0 {
            return self.n;
        }
        if fill_ratio <= 0.0 {
            return 0;
        }
        // Estimate: n ≈ -m * ln(1 - fill_ratio) / k
        let estimate = -(self.m as f64) * (1.0 - fill_ratio).ln() / self.k as f64;
        estimate.round() as usize
    }

    /// Merges another Bloom filter into this one (union operation).
    ///
    /// # Panics
    /// Panics if the filters have different sizes. Prefer [`try_merge`](Self::try_merge),
    /// which returns a `Result` (the library-wide merge convention — fable5 doc 01 F4);
    /// this method is retained for backward compatibility and delegates to it.
    pub fn merge(&mut self, other: &Self) {
        self.try_merge(other)
            .expect("Bloom filters must have same size to merge");
    }

    /// Merges another Bloom filter into this one (union), returning an error
    /// instead of panicking on a size mismatch — matching the `Mergeable::merge`
    /// convention used across the library (fable5 doc 01 F4).
    ///
    /// # Errors
    /// Returns `IncompatibleSketches` if the two filters have different bit-array
    /// sizes (they must be built with the same parameters to be mergeable).
    pub fn try_merge(&mut self, other: &Self) -> Result<(), crate::common::SketchError> {
        if self.bits.len() != other.bits.len() {
            return Err(crate::common::SketchError::IncompatibleSketches {
                reason: format!(
                    "Bloom filter size mismatch: {} vs {} words",
                    self.bits.len(),
                    other.bits.len()
                ),
            });
        }
        for (a, b) in self.bits.iter_mut().zip(other.bits.iter()) {
            *a |= *b;
        }
        Ok(())
    }
}

impl std::fmt::Debug for BloomFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BloomFilter")
            .field("n", &self.n)
            .field("m", &self.m)
            .field("k", &self.k)
            .field("bits_set", &self.count_bits())
            .field(
                "fpr",
                &format!("{:.4}%", self.false_positive_rate() * 100.0),
            )
            .field("memory_bytes", &self.memory_usage())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_new_validates_parameters_instead_of_panicking() {
        assert!(BloomFilter::try_new(0, 0.01).is_err());
        assert!(BloomFilter::try_new(1000, 0.0).is_err());
        assert!(BloomFilter::try_new(1000, 1.0).is_err());
        assert!(BloomFilter::try_new(1000, 0.01).is_ok());
    }

    #[test]
    fn try_merge_errors_on_size_mismatch_and_unions_on_match() {
        let mut a = BloomFilter::new(1000, 0.01);
        let mut b = BloomFilter::new(1000, 0.01);
        a.insert(b"x");
        b.insert(b"y");
        assert!(a.try_merge(&b).is_ok());
        assert!(a.contains(b"x") && a.contains(b"y"));

        // Different size -> Err, not panic.
        let differently_sized = BloomFilter::new(5000, 0.01);
        assert!(a.try_merge(&differently_sized).is_err());
    }

    #[test]
    fn test_new() {
        let filter = BloomFilter::new(1000, 0.01);
        let (n, m, k) = filter.params();

        assert_eq!(n, 1000);
        assert!(m > 0, "Number of bits should be > 0");
        assert!(k > 0, "Number of hash functions should be > 0");
    }

    #[test]
    fn test_insert_and_contains() {
        let mut filter = BloomFilter::new(100, 0.01);

        filter.insert(b"key1");
        filter.insert(b"key2");
        filter.insert(b"key3");

        assert!(filter.contains(b"key1"));
        assert!(filter.contains(b"key2"));
        assert!(filter.contains(b"key3"));
    }

    #[test]
    fn test_no_false_negatives() {
        let mut filter = BloomFilter::new(1000, 0.01);
        let keys: Vec<Vec<u8>> = (0..1000)
            .map(|i| format!("key{}", i).into_bytes())
            .collect();

        for key in &keys {
            filter.insert(key);
        }

        // No false negatives
        for key in &keys {
            assert!(
                filter.contains(key),
                "False negative for {:?}",
                String::from_utf8_lossy(key)
            );
        }
    }

    #[test]
    fn test_false_positive_rate() {
        let mut filter = BloomFilter::new(1000, 0.01);
        let keys: Vec<Vec<u8>> = (0..1000)
            .map(|i| format!("key{}", i).into_bytes())
            .collect();

        for key in &keys {
            filter.insert(key);
        }

        // Test with non-inserted keys
        let test_keys: Vec<Vec<u8>> = (10000..20000)
            .map(|i| format!("test{}", i).into_bytes())
            .collect();

        let false_positives = test_keys.iter().filter(|key| filter.contains(key)).count();

        let actual_fpr = false_positives as f64 / test_keys.len() as f64;

        // Actual FPR should be close to target (within 3x)
        assert!(actual_fpr < 0.03, "FPR too high: {:.4}", actual_fpr);
    }

    #[test]
    fn test_empty_filter() {
        let filter = BloomFilter::new(100, 0.01);

        // Empty filter should return false for all keys
        assert!(!filter.contains(b"key1"));
        assert!(!filter.contains(b"key2"));
        assert!(!filter.contains(b"any_key"));
    }

    #[test]
    fn test_clear() {
        let mut filter = BloomFilter::new(100, 0.01);

        filter.insert(b"key1");
        filter.insert(b"key2");
        assert!(filter.contains(b"key1"));

        filter.clear();

        assert!(!filter.contains(b"key1"));
        assert!(!filter.contains(b"key2"));
        assert_eq!(filter.count_bits(), 0);
    }

    #[test]
    fn test_serialization() {
        let mut filter = BloomFilter::new(100, 0.01);
        filter.insert(b"key1");
        filter.insert(b"key2");
        filter.insert(b"key3");

        let bytes = filter.to_bytes();
        let deserialized = BloomFilter::from_bytes(&bytes).unwrap();

        assert_eq!(filter.params(), deserialized.params());
        assert!(deserialized.contains(b"key1"));
        assert!(deserialized.contains(b"key2"));
        assert!(deserialized.contains(b"key3"));
        assert!(!deserialized.contains(b"key4"));
    }

    #[test]
    fn test_serialization_empty() {
        let filter = BloomFilter::new(100, 0.01);
        let bytes = filter.to_bytes();
        let deserialized = BloomFilter::from_bytes(&bytes).unwrap();

        assert_eq!(filter.params(), deserialized.params());
        assert!(!deserialized.contains(b"any_key"));
    }

    #[test]
    fn test_with_params() {
        let filter = BloomFilter::with_params(1000, 10000, 7);
        let (n, m, k) = filter.params();

        assert_eq!(n, 1000);
        assert_eq!(m, 10000);
        assert_eq!(k, 7);
    }

    #[test]
    fn test_binary_keys() {
        let mut filter = BloomFilter::new(100, 0.01);
        let binary_keys = vec![vec![0u8, 1, 2, 3], vec![255, 254, 253], vec![0, 0, 0, 0]];

        for key in &binary_keys {
            filter.insert(key);
        }

        for key in &binary_keys {
            assert!(filter.contains(key));
        }
    }

    #[test]
    fn test_large_keys() {
        let mut filter = BloomFilter::new(100, 0.01);
        let large_key = vec![42u8; 10000];

        filter.insert(&large_key);
        assert!(filter.contains(&large_key));
    }

    #[test]
    fn test_memory_usage() {
        let filter = BloomFilter::new(1000, 0.01);
        let memory = filter.memory_usage();

        assert!(memory > 0);
        assert_eq!(memory, filter.bits.len() * 8);
    }

    #[test]
    fn test_count_bits() {
        let mut filter = BloomFilter::new(100, 0.01);
        assert_eq!(filter.count_bits(), 0);

        filter.insert(b"key1");
        let bits_after_one = filter.count_bits();
        assert!(bits_after_one > 0);

        filter.insert(b"key2");
        let bits_after_two = filter.count_bits();
        assert!(bits_after_two >= bits_after_one);
    }

    #[test]
    #[should_panic(expected = "param: \"n\"")]
    fn test_new_panics_on_zero_n() {
        BloomFilter::new(0, 0.01);
    }

    #[test]
    #[should_panic(expected = "param: \"fpr\"")]
    fn test_new_panics_on_invalid_fpr() {
        BloomFilter::new(100, 1.5);
    }

    #[test]
    fn test_debug_format() {
        let mut filter = BloomFilter::new(1000, 0.01);
        filter.insert(b"test");

        let debug_str = format!("{:?}", filter);
        assert!(debug_str.contains("BloomFilter"));
        assert!(debug_str.contains("n"));
        assert!(debug_str.contains("m"));
        assert!(debug_str.contains("k"));
    }

    #[test]
    fn test_from_bytes_rejects_oversized_m_without_panic() {
        // Craft a header claiming an enormous bit count `m` with a short tail.
        // Must return Err, never panic, overflow-abort, or attempt a huge alloc.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0u64.to_le_bytes()); // n
        bytes.extend_from_slice(&u64::MAX.to_le_bytes()); // m (oversized)
        bytes.extend_from_slice(&1u64.to_le_bytes()); // k
        // no bit-array body follows
        assert!(BloomFilter::from_bytes(&bytes).is_err());
    }

    #[test]
    fn test_from_bytes_rejects_truncated_body() {
        // `m` within the cap but the declared body is missing.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1000u64.to_le_bytes()); // n
        bytes.extend_from_slice(&100_000u64.to_le_bytes()); // m
        bytes.extend_from_slice(&7u64.to_le_bytes()); // k
        // header only, no words
        assert!(BloomFilter::from_bytes(&bytes).is_err());
    }
}
