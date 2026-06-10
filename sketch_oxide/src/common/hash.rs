//! Hash functions for data sketches
//!
//! Provides high-quality, non-cryptographic hash functions optimized for
//! probabilistic data structures.

use std::hash::{Hash, Hasher};
use twox_hash::XxHash64;

/// Seed used to derive a [`Salt`] from arbitrary key material.
const SALT_DERIVATION_SEED: u64 = 0x5361_6c74_4465_7276; // "SaltDerv"

/// A secret salt for keyed, adversarially-robust hashing.
///
/// Plain seeded hashing (e.g. [`xxhash`]) is deterministic and public: an adversary who
/// knows the seed can craft inputs that collide in a sketch's cells, degrading its
/// accuracy, or probe a cardinality sketch to extract its parameters. Mixing a *secret*
/// salt into the hash closes that gap — outputs become unpredictable without the salt.
///
/// # Policy
///
/// - **Opt-in.** Sketches hash without a salt by default so results stay reproducible
///   across runs and across language bindings. Pass a salt only when you need robustness
///   against adversarial inputs or per-tenant isolation.
/// - **Stable for a sketch's lifetime.** The same salt must be used for every operation on
///   a sketch, and any sketches merged together must share the same salt — otherwise their
///   cells are not comparable.
/// - **Keep it secret.** For real robustness draw the secret from a CSPRNG (e.g. via the
///   privacy module's randomness) and never derive it from attacker-visible data. A salt
///   the attacker can guess provides no protection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Salt(u64);

impl Salt {
    /// Creates a salt from a 64-bit secret.
    #[inline]
    pub const fn new(secret: u64) -> Self {
        Self(secret)
    }

    /// Derives a salt from arbitrary secret key material (e.g. a passphrase or token).
    pub fn from_bytes(key_material: &[u8]) -> Self {
        Self(xxhash(key_material, SALT_DERIVATION_SEED))
    }

    /// The raw 64-bit secret.
    #[inline]
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Keyed 64-bit hash: a function of `data`, a `seed` (for independent hash functions),
/// and a secret [`Salt`].
///
/// Use in place of [`xxhash`] when adversarial robustness is required. The construction is
/// endianness-stable (the seed is mixed in as little-endian bytes), so it is reproducible
/// across platforms and language bindings given the same salt.
///
/// Note that `keyed_hash(data, seed, Salt::new(0))` is *not* equal to `xxhash(data, seed)`
/// — keyed hashing is a distinct, opt-in construction.
///
/// # Examples
/// ```
/// use sketch_oxide::common::hash::{keyed_hash, Salt};
///
/// let salt = Salt::from_bytes(b"per-tenant-secret");
/// let h0 = keyed_hash(b"user-42", 0, salt);
/// let h1 = keyed_hash(b"user-42", 1, salt); // independent hash function
/// assert_ne!(h0, h1);
/// ```
pub fn keyed_hash(data: &[u8], seed: u64, salt: Salt) -> u64 {
    let mut hasher = XxHash64::with_seed(salt.0);
    hasher.write(&seed.to_le_bytes());
    hasher.write(data);
    hasher.finish()
}

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

    #[test]
    fn keyed_hash_is_deterministic() {
        let salt = Salt::new(0xDEAD_BEEF);
        assert_eq!(keyed_hash(b"hello", 3, salt), keyed_hash(b"hello", 3, salt));
    }

    #[test]
    fn keyed_hash_varies_with_salt_seed_and_data() {
        let a = Salt::new(1);
        let b = Salt::new(2);
        assert_ne!(
            keyed_hash(b"x", 0, a),
            keyed_hash(b"x", 0, b),
            "salt matters"
        );
        assert_ne!(
            keyed_hash(b"x", 0, a),
            keyed_hash(b"x", 1, a),
            "seed matters"
        );
        assert_ne!(
            keyed_hash(b"x", 0, a),
            keyed_hash(b"y", 0, a),
            "data matters"
        );
    }

    #[test]
    fn keyed_hash_differs_from_plain_xxhash() {
        // Salt 0 is still the keyed construction, not bare xxhash.
        assert_ne!(keyed_hash(b"abc", 0, Salt::new(0)), xxhash(b"abc", 0));
    }

    #[test]
    fn salt_from_bytes_is_stable() {
        assert_eq!(Salt::from_bytes(b"secret"), Salt::from_bytes(b"secret"));
        assert_ne!(Salt::from_bytes(b"secret"), Salt::from_bytes(b"other"));
    }
}
