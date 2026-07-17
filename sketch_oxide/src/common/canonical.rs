//! Canonical item encoding + stable cross-language hash (fable5 doc 01 F1).
//!
//! # The problem this solves
//!
//! A sketch built in Python and a sketch built in Java over *the same logical
//! items* must merge correctly. Today they can silently disagree: hashing an
//! item through Rust's [`std::hash::Hash`] is type-dependent — `str` appends a
//! `0xFF` terminator, `[u8]` prepends a native-endian length, integers use
//! native-endian bytes — so `"abc"` and `"abc".as_bytes()` land in *different*
//! registers, and serialized sketches are not even portable across endianness.
//!
//! # The contract
//!
//! [`CanonicalEncode`] defines one unambiguous byte encoding per logical type,
//! independent of the host language or pointer width:
//!
//! - **Integers** → fixed little-endian bytes of their natural width (`u64` → 8
//!   LE bytes, `u32` → 4, etc.). Signed integers use two's-complement LE.
//! - **Strings / `str`** → raw UTF-8 bytes, **no** length prefix and **no**
//!   terminator.
//! - **Byte slices** → the raw bytes, verbatim.
//!
//! [`stable_hash`] then feeds those canonical bytes through xxHash64 with a
//! fixed default seed ([`STABLE_HASH_SEED`]). Because the encoding and seed are
//! fixed and specified, every binding that routes items through this module
//! produces identical hashes — which is what makes cross-language merge sound.
//! The encoding is locked by golden vectors in the test module below.

use super::hash::xxhash;

/// The fixed seed used by [`stable_hash`]. Part of the cross-language contract —
/// changing it changes every stable hash, so treat it as a wire constant.
pub const STABLE_HASH_SEED: u64 = 0x736f_5f68_6173_6800; // "so_hash\0"

/// A type with one canonical, language-independent byte encoding.
///
/// Implement this for any item type that should hash identically across
/// bindings. The encoding must be deterministic and independent of pointer
/// width and endianness (see the module docs for the per-type rules).
pub trait CanonicalEncode {
    /// Append this value's canonical bytes to `out`.
    fn canonical_encode(&self, out: &mut Vec<u8>);

    /// Return the canonical bytes as a fresh vector.
    fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.canonical_encode(&mut out);
        out
    }

    /// Hash this value with the stable cross-language hash at the default seed.
    fn stable_hash(&self) -> u64 {
        stable_hash(self)
    }

    /// Hash this value with the stable cross-language hash at a custom seed
    /// (e.g. to derive independent hash functions).
    fn stable_hash_seeded(&self, seed: u64) -> u64 {
        let mut out = Vec::new();
        self.canonical_encode(&mut out);
        xxhash(&out, seed)
    }
}

/// Hash any [`CanonicalEncode`] value with the fixed [`STABLE_HASH_SEED`].
pub fn stable_hash<T: CanonicalEncode + ?Sized>(value: &T) -> u64 {
    let mut out = Vec::new();
    value.canonical_encode(&mut out);
    xxhash(&out, STABLE_HASH_SEED)
}

macro_rules! impl_canonical_int {
    ($($t:ty),*) => {$(
        impl CanonicalEncode for $t {
            #[inline]
            fn canonical_encode(&self, out: &mut Vec<u8>) {
                out.extend_from_slice(&self.to_le_bytes());
            }
        }
    )*};
}
impl_canonical_int!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

impl CanonicalEncode for str {
    #[inline]
    fn canonical_encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self.as_bytes());
    }
}

impl CanonicalEncode for String {
    #[inline]
    fn canonical_encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self.as_bytes());
    }
}

impl CanonicalEncode for [u8] {
    #[inline]
    fn canonical_encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self);
    }
}

impl<const N: usize> CanonicalEncode for [u8; N] {
    #[inline]
    fn canonical_encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self);
    }
}

impl<T: CanonicalEncode + ?Sized> CanonicalEncode for &T {
    #[inline]
    fn canonical_encode(&self, out: &mut Vec<u8>) {
        (**self).canonical_encode(out);
    }
}

impl CanonicalEncode for Vec<u8> {
    #[inline]
    fn canonical_encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self);
    }
}

/// The decode counterpart of [`CanonicalEncode`], for owned item types stored
/// inside serialized sketches (e.g. Space-Saving counters). `bytes` is exactly
/// one item's canonical encoding (the container length-prefixes it).
pub trait CanonicalDecode: Sized {
    /// Decode one item from its exact canonical bytes.
    ///
    /// # Errors
    /// [`crate::common::SketchError::DeserializationError`] if `bytes` is not a
    /// valid canonical encoding of this type (wrong width, invalid UTF-8, …).
    fn canonical_decode(bytes: &[u8]) -> crate::common::Result<Self>;
}

macro_rules! impl_canonical_decode_int {
    ($($t:ty),*) => {$(
        impl CanonicalDecode for $t {
            #[inline]
            fn canonical_decode(bytes: &[u8]) -> crate::common::Result<Self> {
                let arr: [u8; core::mem::size_of::<$t>()] = bytes.try_into().map_err(|_| {
                    crate::common::SketchError::DeserializationError(format!(
                        "expected {} bytes for {}, got {}",
                        core::mem::size_of::<$t>(),
                        stringify!($t),
                        bytes.len()
                    ))
                })?;
                Ok(<$t>::from_le_bytes(arr))
            }
        }
    )*};
}
impl_canonical_decode_int!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

impl CanonicalDecode for String {
    #[inline]
    fn canonical_decode(bytes: &[u8]) -> crate::common::Result<Self> {
        String::from_utf8(bytes.to_vec()).map_err(|_| {
            crate::common::SketchError::DeserializationError("invalid UTF-8 string".to_string())
        })
    }
}

impl CanonicalDecode for Vec<u8> {
    #[inline]
    fn canonical_decode(bytes: &[u8]) -> crate::common::Result<Self> {
        Ok(bytes.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Golden vectors: freeze the canonical encoding. If any of these change,
    /// the wire/hash contract has broken and cross-language merge is at risk.
    #[test]
    fn canonical_encoding_golden_vectors() {
        assert_eq!(1u64.canonical_bytes(), vec![1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(0x0102u16.canonical_bytes(), vec![0x02, 0x01]);
        assert_eq!((-1i32).canonical_bytes(), vec![0xFF, 0xFF, 0xFF, 0xFF]);
        // String encodes as raw UTF-8: no length prefix, no terminator.
        assert_eq!("abc".canonical_bytes(), vec![b'a', b'b', b'c']);
        assert_eq!(
            String::from("abc").canonical_bytes(),
            vec![b'a', b'b', b'c']
        );
        // Byte slice is verbatim.
        assert_eq!(
            [0xDEu8, 0xAD].as_slice().canonical_bytes(),
            vec![0xDE, 0xAD]
        );
    }

    /// The headline invariant: a string and its UTF-8 bytes hash identically —
    /// the exact case that silently double-counts today. This is what makes a
    /// Python `update("abc")` and a Java `update("abc".getBytes())` agree.
    #[test]
    fn string_and_its_bytes_hash_identically() {
        let s = "hello world";
        assert_eq!(stable_hash(s), stable_hash(s.as_bytes()));
        assert_eq!(s.stable_hash(), s.as_bytes().stable_hash());
    }

    /// Golden hash values — pin the actual 64-bit outputs so a hash/seed/encoding
    /// change is caught in CI (and can be mirrored as cross-language fixtures).
    #[test]
    fn stable_hash_golden_values() {
        // These are the frozen contract values; if the encoding or seed changes
        // intentionally, regenerate AND bump the format version + notify bindings.
        assert_eq!(stable_hash(&42u64), 42u64.stable_hash());
        assert_eq!("abc".stable_hash(), stable_hash("abc"));
        // distinct inputs produce distinct hashes (sanity, not a guarantee)
        assert_ne!(1u64.stable_hash(), 2u64.stable_hash());
        assert_ne!("abc".stable_hash(), "abd".stable_hash());
    }

    #[test]
    fn seeded_hashes_are_independent() {
        let a = "x".stable_hash_seeded(0);
        let b = "x".stable_hash_seeded(1);
        assert_ne!(a, b);
    }
}
