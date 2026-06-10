//! Hash functions for data sketches
//!
//! Provides high-quality, non-cryptographic hash functions optimized for
//! probabilistic data structures.

use std::hash::{Hash, Hasher};
use twox_hash::XxHash64;

use siphasher::sip::SipHasher13;

/// Domain-separation constants for deriving a 128-bit [`Salt`] from key material.
const SALT_DERIVE_K0: u64 = 0x736b_6574_6368_5f6f; // "sketch_o"
const SALT_DERIVE_K1: u64 = 0x7869_6465_5f73_616c; // "xide_sal"

/// A 128-bit secret key for keyed, adversarially-robust hashing.
///
/// Plain seeded hashing (e.g. [`xxhash`]) is deterministic and public: an adversary who
/// knows the seed can craft inputs that collide in a sketch's cells, degrading its
/// accuracy, or probe a cardinality sketch to extract its parameters. [`keyed_hash`] uses
/// a *secret* `Salt` with SipHash-1-3 — the same keyed primitive Rust's `HashMap` uses for
/// HashDoS resistance — so an attacker who cannot see the salt cannot predict which inputs
/// collide.
///
/// # Threat model and limits
///
/// This targets **HashDoS-class robustness**: a remote attacker who supplies inputs but
/// does not know the key cannot engineer collisions. It is *not* a message-authentication
/// code and makes no cryptographic-integrity guarantee; for that use a real MAC
/// (BLAKE3-keyed, HMAC). The key is 128 bits so it is not the brute-force bottleneck.
///
/// # Policy
///
/// - **Opt-in.** Sketches hash without a salt by default so results stay reproducible
///   across runs and across language bindings. Use a salt only when you need robustness
///   against adversarial inputs or per-tenant isolation.
/// - **Stable for a sketch's lifetime.** The same salt must be used for every operation on
///   a sketch, and any sketches merged together must share the same salt — otherwise their
///   cells are not comparable.
/// - **Keep it secret.** Prefer [`Salt::random`] (CSPRNG). A salt the attacker can guess
///   provides no protection. The secret is never printed: `Debug` is redacted, and the raw
///   key is only reachable via the explicit [`expose_keys`](Salt::expose_keys).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Salt {
    k0: u64,
    k1: u64,
}

impl core::fmt::Debug for Salt {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Never leak the secret via logs/format output.
        f.write_str("Salt(<redacted>)")
    }
}

impl Salt {
    /// Creates a salt from a caller-supplied 128-bit key (two 64-bit words).
    ///
    /// Both words together should carry ~128 bits of entropy; prefer [`Salt::random`]
    /// unless you are restoring a previously stored key (see [`expose_keys`](Self::expose_keys)).
    #[inline]
    pub const fn from_keys(k0: u64, k1: u64) -> Self {
        Self { k0, k1 }
    }

    /// Generates a fresh random salt from the thread-local CSPRNG. The recommended way to
    /// obtain a salt for adversarial robustness.
    pub fn random() -> Self {
        use rand::Rng;
        let mut rng = rand::rng();
        Self {
            k0: rng.random(),
            k1: rng.random(),
        }
    }

    /// Derives a 128-bit salt from **high-entropy** key material (e.g. a 32-byte random
    /// token or a key from a KMS).
    ///
    /// The two key words are produced by two domain-separated SipHash-1-3 passes over the
    /// input. This is *not* a password KDF: low-entropy inputs (passphrases) remain
    /// brute-forceable. For passwords, run Argon2id/scrypt first and pass the derived key
    /// here, or use [`Salt::random`].
    pub fn from_bytes(high_entropy_key: &[u8]) -> Self {
        let mut h0 = SipHasher13::new_with_keys(SALT_DERIVE_K0, SALT_DERIVE_K1);
        h0.write(high_entropy_key);
        let mut h1 = SipHasher13::new_with_keys(SALT_DERIVE_K1, SALT_DERIVE_K0);
        h1.write(high_entropy_key);
        Self {
            k0: h0.finish(),
            k1: h1.finish(),
        }
    }

    /// Returns the raw 128-bit key as two words, for serialization or crossing the FFI
    /// boundary. Named to make exposure of the secret explicit at call sites; avoid logging
    /// the result.
    #[inline]
    pub const fn expose_keys(self) -> (u64, u64) {
        (self.k0, self.k1)
    }
}

/// Keyed 64-bit hash: a function of `data`, a `seed` (for independent hash functions), and
/// a secret [`Salt`], computed with SipHash-1-3.
///
/// Use in place of [`xxhash`] when adversarial robustness is required (see [`Salt`] for the
/// threat model). The construction is endianness-stable (the seed is mixed in as
/// little-endian bytes), so it is reproducible across platforms and language bindings given
/// the same salt.
///
/// # Examples
/// ```
/// use sketch_oxide::common::hash::{keyed_hash, Salt};
///
/// let salt = Salt::random();
/// let h0 = keyed_hash(b"user-42", 0, salt);
/// let h1 = keyed_hash(b"user-42", 1, salt); // independent hash function
/// assert_ne!(h0, h1);
/// // Same salt + inputs always agree, so a sketch's insert and query match.
/// assert_eq!(h0, keyed_hash(b"user-42", 0, salt));
/// ```
pub fn keyed_hash(data: &[u8], seed: u64, salt: Salt) -> u64 {
    let mut hasher = SipHasher13::new_with_keys(salt.k0, salt.k1);
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
        let salt = Salt::from_keys(0xDEAD_BEEF, 0xFEED_FACE);
        assert_eq!(keyed_hash(b"hello", 3, salt), keyed_hash(b"hello", 3, salt));
    }

    #[test]
    fn keyed_hash_varies_with_salt_seed_and_data() {
        let a = Salt::from_keys(1, 11);
        let b = Salt::from_keys(2, 22);
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
        // SipHash-keyed construction, not bare xxhash.
        assert_ne!(
            keyed_hash(b"abc", 0, Salt::from_keys(0, 0)),
            xxhash(b"abc", 0)
        );
    }

    #[test]
    fn salt_from_bytes_is_stable_and_distinct() {
        assert_eq!(Salt::from_bytes(b"secret"), Salt::from_bytes(b"secret"));
        assert_ne!(Salt::from_bytes(b"secret"), Salt::from_bytes(b"other"));
        // The two derived key words are domain-separated, so they differ.
        let (k0, k1) = Salt::from_bytes(b"secret").expose_keys();
        assert_ne!(k0, k1);
    }

    #[test]
    fn salt_random_is_unpredictable() {
        // Two random salts essentially never collide.
        assert_ne!(Salt::random(), Salt::random());
    }

    #[test]
    fn salt_debug_is_redacted() {
        let salt = Salt::from_keys(0x1234_5678, 0x9abc_def0);
        let shown = format!("{salt:?}");
        assert_eq!(shown, "Salt(<redacted>)");
        assert!(!shown.contains("1234"), "secret must not leak via Debug");
    }
}
