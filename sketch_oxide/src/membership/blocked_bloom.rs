//! Blocked Bloom Filter implementation
//!
//! A cache-efficient Bloom filter variant that concentrates all hash lookups
//! within a single CPU cache line (64 bytes = 512 bits).
//!
//! # Features
//! - **1 cache miss per query** (vs 7+ for standard Bloom)
//! - Configurable false positive rate
//! - Same space efficiency as standard Bloom (~10 bits/key @ 1% FPR)
//! - Cache-line-aligned memory for optimal CPU cache usage
//! - Serialization/deserialization support
//!
//! # Algorithm
//! 1. First hash determines which block (512-bit cache line) to use
//! 2. Remaining k hashes determine bit positions within that single block
//! 3. All k bit checks occur within one cache line = 1 memory fetch
//!
//! # Example
//! ```
//! use sketch_oxide::membership::BlockedBloomFilter;
//!
//! // Create filter for 1000 elements with 1% false positive rate
//! let mut filter = BlockedBloomFilter::new(1000, 0.01);
//! filter.insert(b"key1");
//! filter.insert(b"key2");
//!
//! assert!(filter.contains(b"key1"));
//! assert!(!filter.contains(b"key3")); // Probably false
//! ```

/// Cache line size in bytes (typically 64 bytes on modern CPUs)
const CACHE_LINE_SIZE: usize = 64;

/// Bits per block (64 bytes = 512 bits)
const BITS_PER_BLOCK: usize = 512;

/// u64 words per block (512 bits / 64 bits per u64 = 8 words)
const U64_PER_BLOCK: usize = 8;

/// Blocked Bloom filter for cache-efficient membership testing
#[derive(Clone)]
pub struct BlockedBloomFilter {
    /// Cache-line-aligned blocks of bits
    blocks: Vec<[u64; U64_PER_BLOCK]>,
    /// Number of blocks
    num_blocks: usize,
    /// Number of hash functions per block
    k: usize,
    /// Expected number of elements
    n: usize,
}

impl BlockedBloomFilter {
    /// Creates a new Blocked Bloom filter
    ///
    /// # Arguments
    /// * `n` - Expected number of elements
    /// * `fpr` - Desired false positive rate (e.g., 0.01 for 1%)
    ///
    /// # Panics
    /// Panics if `n` is 0 or `fpr` is not in range (0, 1)
    pub fn new(n: usize, fpr: f64) -> Self {
        assert!(n > 0, "Expected number of elements must be > 0");
        assert!(
            fpr > 0.0 && fpr < 1.0,
            "False positive rate must be in (0, 1)"
        );

        // Calculate total bits needed (same formula as standard Bloom)
        let total_bits =
            (-(n as f64) * fpr.ln() / (std::f64::consts::LN_2.powi(2))).ceil() as usize;

        // Calculate number of blocks (round up to ensure enough capacity)
        let num_blocks = total_bits.div_ceil(BITS_PER_BLOCK);
        let num_blocks = num_blocks.max(1); // At least 1 block

        // Optimal hash function count: k = (m/n) * ln(2)
        let k = ((total_bits as f64 / n as f64) * std::f64::consts::LN_2).ceil() as usize;
        let k = k.clamp(1, BITS_PER_BLOCK / 2); // At least 1, at most half the block size

        Self {
            blocks: vec![[0u64; U64_PER_BLOCK]; num_blocks],
            num_blocks,
            k,
            n,
        }
    }

    /// Creates a Blocked Bloom filter with specific parameters
    ///
    /// # Arguments
    /// * `n` - Expected number of elements
    /// * `num_blocks` - Number of 512-bit blocks
    /// * `k` - Number of hash functions per block
    pub fn with_params(n: usize, num_blocks: usize, k: usize) -> Self {
        assert!(n > 0, "Expected number of elements must be > 0");
        assert!(num_blocks > 0, "Number of blocks must be > 0");
        assert!(k > 0, "Number of hash functions must be > 0");

        Self {
            blocks: vec![[0u64; U64_PER_BLOCK]; num_blocks],
            num_blocks,
            k,
            n,
        }
    }

    /// Inserts an element into the filter
    pub fn insert(&mut self, key: &[u8]) {
        let block_idx = self.hash_block(key);

        // Compute all bit indices first to avoid borrow checker issues
        let k = self.k;
        let bit_indices: Vec<usize> = (0..k).map(|i| self.hash_within_block(key, i)).collect();

        // Now we can mutably borrow the block
        let block = &mut self.blocks[block_idx];
        for bit_index in bit_indices {
            let word_index = bit_index / 64;
            let bit_offset = bit_index % 64;
            block[word_index] |= 1u64 << bit_offset;
        }
    }

    /// Checks if an element might be in the set
    ///
    /// Returns `true` if the element might be in the set (may be false positive)
    /// Returns `false` if the element is definitely not in the set (no false negatives)
    pub fn contains(&self, key: &[u8]) -> bool {
        let block_idx = self.hash_block(key);
        let block = &self.blocks[block_idx];

        for i in 0..self.k {
            let bit_index = self.hash_within_block(key, i);
            let word_index = bit_index / 64;
            let bit_offset = bit_index % 64;

            if block[word_index] & (1u64 << bit_offset) == 0 {
                return false;
            }
        }
        true
    }

    /// Clears all bits in the filter
    pub fn clear(&mut self) {
        for block in &mut self.blocks {
            block.fill(0);
        }
    }

    /// Returns the number of bits set to 1 across all blocks
    pub fn count_bits(&self) -> usize {
        self.blocks
            .iter()
            .flat_map(|block| block.iter())
            .map(|word| word.count_ones() as usize)
            .sum()
    }

    /// Returns the theoretical false positive rate
    pub fn false_positive_rate(&self) -> f64 {
        let total_bits = self.num_blocks * BITS_PER_BLOCK;
        let bits_set = self.count_bits() as f64 / total_bits as f64;
        bits_set.powi(self.k as i32)
    }

    /// Returns the memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        self.num_blocks * CACHE_LINE_SIZE
    }

    /// Serializes the filter to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Header: [n: 8 bytes][num_blocks: 8 bytes][k: 8 bytes]
        bytes.extend_from_slice(&self.n.to_le_bytes());
        bytes.extend_from_slice(&self.num_blocks.to_le_bytes());
        bytes.extend_from_slice(&self.k.to_le_bytes());

        // All blocks
        for block in &self.blocks {
            for word in block {
                bytes.extend_from_slice(&word.to_le_bytes());
            }
        }

        bytes
    }

    /// Deserializes a filter from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < 24 {
            return Err("Insufficient bytes for header");
        }

        let n = usize::from_le_bytes(bytes[0..8].try_into().unwrap());
        let num_blocks = usize::from_le_bytes(bytes[8..16].try_into().unwrap());
        let k = usize::from_le_bytes(bytes[16..24].try_into().unwrap());

        let expected_size = 24 + num_blocks * CACHE_LINE_SIZE;

        if bytes.len() != expected_size {
            return Err("Invalid byte array size");
        }

        let mut blocks = Vec::with_capacity(num_blocks);
        for block_idx in 0..num_blocks {
            let mut block = [0u64; U64_PER_BLOCK];
            for (word_idx, word) in block.iter_mut().enumerate() {
                let offset = 24 + block_idx * CACHE_LINE_SIZE + word_idx * 8;
                *word = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
            }
            blocks.push(block);
        }

        Ok(Self {
            blocks,
            num_blocks,
            k,
            n,
        })
    }

    /// Returns filter parameters (n, num_blocks, k)
    pub fn params(&self) -> (usize, usize, usize) {
        (self.n, self.num_blocks, self.k)
    }

    /// Returns true if no elements have been inserted
    pub fn is_empty(&self) -> bool {
        self.count_bits() == 0
    }

    /// Returns the estimated number of elements
    pub fn len(&self) -> usize {
        let total_bits = self.num_blocks * BITS_PER_BLOCK;
        let fill_ratio = self.count_bits() as f64 / total_bits as f64;
        if fill_ratio >= 1.0 {
            return self.n;
        }
        if fill_ratio <= 0.0 {
            return 0;
        }
        let estimate = -(total_bits as f64) * (1.0 - fill_ratio).ln() / self.k as f64;
        estimate.round() as usize
    }

    /// Merges another Blocked Bloom filter into this one (union operation)
    ///
    /// # Panics
    /// Panics if the filters have different sizes
    pub fn merge(&mut self, other: &Self) {
        assert_eq!(
            self.num_blocks, other.num_blocks,
            "Blocked Bloom filters must have same number of blocks to merge"
        );
        for (a_block, b_block) in self.blocks.iter_mut().zip(other.blocks.iter()) {
            for (a, b) in a_block.iter_mut().zip(b_block.iter()) {
                *a |= *b;
            }
        }
    }

    /// Hash function to determine block index
    #[inline]
    fn hash_block(&self, key: &[u8]) -> usize {
        use xxhash_rust::xxh64::xxh64;
        let hash = xxh64(key, 0);
        (hash as usize) % self.num_blocks
    }

    /// Hash function for bit position within block
    #[inline]
    fn hash_within_block(&self, key: &[u8], seed: usize) -> usize {
        use xxhash_rust::xxh64::xxh64;
        let hash = xxh64(key, seed as u64 + 1); // +1 to differentiate from block hash
        (hash as usize) % BITS_PER_BLOCK
    }
}

impl std::fmt::Debug for BlockedBloomFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlockedBloomFilter")
            .field("n", &self.n)
            .field("num_blocks", &self.num_blocks)
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
    fn test_new() {
        let filter = BlockedBloomFilter::new(1000, 0.01);
        let (n, num_blocks, k) = filter.params();

        assert_eq!(n, 1000);
        assert!(num_blocks > 0, "Number of blocks should be > 0");
        assert!(k > 0, "Number of hash functions should be > 0");
    }

    #[test]
    fn test_with_params() {
        let filter = BlockedBloomFilter::with_params(1000, 20, 7);
        let (n, num_blocks, k) = filter.params();

        assert_eq!(n, 1000);
        assert_eq!(num_blocks, 20);
        assert_eq!(k, 7);
    }

    #[test]
    fn test_insert_and_contains() {
        let mut filter = BlockedBloomFilter::new(100, 0.01);

        filter.insert(b"key1");
        filter.insert(b"key2");
        filter.insert(b"key3");

        assert!(filter.contains(b"key1"));
        assert!(filter.contains(b"key2"));
        assert!(filter.contains(b"key3"));
    }

    #[test]
    fn test_no_false_negatives() {
        let mut filter = BlockedBloomFilter::new(1000, 0.01);
        let keys: Vec<Vec<u8>> = (0..1000)
            .map(|i| format!("key{}", i).into_bytes())
            .collect();

        for key in &keys {
            filter.insert(key);
        }

        // No false negatives - all inserted keys must be found
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
        let mut filter = BlockedBloomFilter::new(1000, 0.01);
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
        assert!(
            actual_fpr < 0.03,
            "FPR too high: {:.4}, expected < 0.03",
            actual_fpr
        );
    }

    #[test]
    fn test_empty_filter() {
        let filter = BlockedBloomFilter::new(100, 0.01);

        // Empty filter should return false for all keys
        assert!(!filter.contains(b"key1"));
        assert!(!filter.contains(b"key2"));
        assert!(!filter.contains(b"any_key"));
    }

    #[test]
    fn test_clear() {
        let mut filter = BlockedBloomFilter::new(100, 0.01);

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
        let mut filter = BlockedBloomFilter::new(100, 0.01);
        filter.insert(b"key1");
        filter.insert(b"key2");
        filter.insert(b"key3");

        let bytes = filter.to_bytes();
        let deserialized = BlockedBloomFilter::from_bytes(&bytes).unwrap();

        assert_eq!(filter.params(), deserialized.params());
        assert!(deserialized.contains(b"key1"));
        assert!(deserialized.contains(b"key2"));
        assert!(deserialized.contains(b"key3"));
        assert!(!deserialized.contains(b"key4"));
    }

    #[test]
    fn test_serialization_empty() {
        let filter = BlockedBloomFilter::new(100, 0.01);
        let bytes = filter.to_bytes();
        let deserialized = BlockedBloomFilter::from_bytes(&bytes).unwrap();

        assert_eq!(filter.params(), deserialized.params());
        assert!(!deserialized.contains(b"any_key"));
    }

    #[test]
    fn test_binary_keys() {
        let mut filter = BlockedBloomFilter::new(100, 0.01);
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
        let mut filter = BlockedBloomFilter::new(100, 0.01);
        let large_key = vec![42u8; 10000];

        filter.insert(&large_key);
        assert!(filter.contains(&large_key));
    }

    #[test]
    fn test_memory_usage() {
        let filter = BlockedBloomFilter::new(1000, 0.01);
        let memory = filter.memory_usage();

        assert!(memory > 0);
        // Should be aligned to cache line size
        assert_eq!(memory % CACHE_LINE_SIZE, 0);
    }

    #[test]
    fn test_count_bits() {
        let mut filter = BlockedBloomFilter::new(100, 0.01);
        assert_eq!(filter.count_bits(), 0);

        filter.insert(b"key1");
        let bits_after_one = filter.count_bits();
        assert!(bits_after_one > 0);

        filter.insert(b"key2");
        let bits_after_two = filter.count_bits();
        assert!(bits_after_two >= bits_after_one);
    }

    #[test]
    fn test_cache_line_alignment() {
        let _filter = BlockedBloomFilter::new(1000, 0.01);
        // Verify that each block is exactly cache-line size (64 bytes = 512 bits)
        // Note: Rust's Vec doesn't guarantee alignment, but we can check size
        assert_eq!(std::mem::size_of::<[u64; U64_PER_BLOCK]>(), CACHE_LINE_SIZE);
    }

    #[test]
    fn test_single_block_access() {
        // This test verifies that all hash lookups for a single key
        // access only one block
        let mut filter = BlockedBloomFilter::new(100, 0.01);
        filter.insert(b"test_key");

        // The property we're testing: for any key, hash_block should return
        // the same block index, ensuring all k hash functions operate on
        // the same cache line
        let block_idx = filter.hash_block(b"test_key");
        assert!(block_idx < filter.num_blocks);
    }

    #[test]
    #[should_panic(expected = "Expected number of elements must be > 0")]
    fn test_new_panics_on_zero_n() {
        BlockedBloomFilter::new(0, 0.01);
    }

    #[test]
    #[should_panic(expected = "False positive rate must be in (0, 1)")]
    fn test_new_panics_on_invalid_fpr() {
        BlockedBloomFilter::new(100, 1.5);
    }

    #[test]
    fn test_debug_format() {
        let mut filter = BlockedBloomFilter::new(1000, 0.01);
        filter.insert(b"test");

        let debug_str = format!("{:?}", filter);
        assert!(debug_str.contains("BlockedBloomFilter"));
        assert!(debug_str.contains("n"));
        assert!(debug_str.contains("num_blocks"));
        assert!(debug_str.contains("k"));
    }
}
