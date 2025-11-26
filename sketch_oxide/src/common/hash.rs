//! Hash functions for data sketches
//!
//! Provides high-quality, non-cryptographic hash functions optimized for
//! probabilistic data structures.

use std::hash::{Hash, Hasher};
use twox_hash::XxHash64;

/// MurmurHash3 32-bit implementation
///
/// MurmurHash3 is a non-cryptographic hash function designed by Austin Appleby.
/// It provides excellent distribution and speed for hash table and sketch applications.
///
/// # Arguments
/// * `data` - The data to hash
/// * `seed` - The hash seed for independent hash functions
///
/// # Returns
/// A 32-bit hash value
///
/// # Examples
/// ```
/// use sketch_oxide::common::hash::murmur3_hash;
///
/// let hash = murmur3_hash(b"hello world", 0);
/// println!("Hash: {}", hash);
/// ```
pub fn murmur3_hash(data: &[u8], seed: u32) -> u32 {
    let mut hash = seed;
    let len = data.len();

    // Process 4-byte blocks
    let chunks = len / 4;
    for i in 0..chunks {
        let k = u32::from_le_bytes([
            data[i * 4],
            data[i * 4 + 1],
            data[i * 4 + 2],
            data[i * 4 + 3],
        ]);

        let k = k.wrapping_mul(0xcc9e2d51);
        let k = k.rotate_left(15);
        let k = k.wrapping_mul(0x1b873593);

        hash ^= k;
        hash = hash.rotate_left(13);
        hash = hash.wrapping_mul(5).wrapping_add(0xe6546b64);
    }

    // Process remaining bytes
    let remainder = len % 4;
    if remainder > 0 {
        let offset = chunks * 4;
        let mut k: u32 = 0;

        if remainder >= 3 {
            k ^= (data[offset + 2] as u32) << 16;
        }
        if remainder >= 2 {
            k ^= (data[offset + 1] as u32) << 8;
        }
        k ^= data[offset] as u32;

        k = k.wrapping_mul(0xcc9e2d51);
        k = k.rotate_left(15);
        k = k.wrapping_mul(0x1b873593);
        hash ^= k;
    }

    // Finalization
    hash ^= len as u32;
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x85ebca6b);
    hash ^= hash >> 13;
    hash = hash.wrapping_mul(0xc2b2ae35);
    hash ^= hash >> 16;

    hash
}

/// XXHash 64-bit implementation
///
/// XXHash is an extremely fast non-cryptographic hash function designed by Yann Collet.
/// It offers excellent speed and distribution properties.
///
/// # Arguments
/// * `data` - The data to hash
/// * `seed` - The hash seed for independent hash functions
///
/// # Returns
/// A 64-bit hash value
///
/// # Examples
/// ```
/// use sketch_oxide::common::hash::xxhash;
///
/// let hash = xxhash(b"hello world", 0);
/// println!("Hash: {}", hash);
/// ```
pub fn xxhash(data: &[u8], seed: u64) -> u64 {
    let mut hasher = XxHash64::with_seed(seed);
    hasher.write(data);
    hasher.finish()
}

/// MurmurHash3 64-bit implementation
///
/// Extended version of MurmurHash3 that produces 64-bit hashes.
///
/// # Arguments
/// * `data` - The data to hash
/// * `seed` - The hash seed for independent hash functions
///
/// # Returns
/// A 64-bit hash value
pub fn murmur3_hash64(data: &[u8], seed: u64) -> u64 {
    // Use xxhash for 64-bit hashing (it's faster and better distributed)
    xxhash(data, seed)
}

/// Generic 64-bit hash function
///
/// Convenience alias for murmur3_hash64
///
/// # Arguments
/// * `data` - The data to hash
/// * `seed` - The hash seed
///
/// # Returns
/// A 64-bit hash value
pub fn hash_64(data: &[u8], seed: u64) -> u64 {
    xxhash(data, seed)
}

/// Hash any value that implements Hash trait using MurmurHash3
///
/// This is a convenience function for hashing Rust types.
///
/// # Arguments
/// * `value` - The value to hash
/// * `seed` - The hash seed
///
/// # Returns
/// A 32-bit hash value
pub fn hash_value<T: Hash>(value: &T, seed: u32) -> u32 {
    use std::hash::Hasher as StdHasher;

    // Use a simple hasher to convert T to bytes
    struct ByteHasher {
        bytes: Vec<u8>,
    }

    impl StdHasher for ByteHasher {
        fn finish(&self) -> u64 {
            0 // Not used
        }

        fn write(&mut self, bytes: &[u8]) {
            self.bytes.extend_from_slice(bytes);
        }
    }

    let mut hasher = ByteHasher { bytes: Vec::new() };
    value.hash(&mut hasher);
    murmur3_hash(&hasher.bytes, seed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_murmur3_basic() {
        let hash = murmur3_hash(b"test", 0);
        assert!(hash > 0);
    }

    #[test]
    fn test_xxhash_basic() {
        let hash = xxhash(b"test", 0);
        assert!(hash > 0);
    }

    #[test]
    fn test_hash_value_basic() {
        let hash = hash_value(&42u64, 0);
        assert!(hash > 0);
    }
}
