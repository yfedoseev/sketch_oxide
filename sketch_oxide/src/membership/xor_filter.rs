//! XOR filter — fast, compact static membership testing (Graf & Lemire, JEA 2020).
//!
//! An XOR filter is an immutable approximate-membership structure built once from a fixed key set. It
//! stores a fingerprint array of `≈1.23·n` slots; each key maps to **three** slots (one per third of
//! the array) and the filter maintains the invariant that, for every key, the XOR of its three slots
//! equals the key's fingerprint. A query recomputes the three slots and the fingerprint and checks the
//! XOR — exactly three memory accesses, no false negatives, and a false-positive rate of `2^{−bits}`
//! (≈0.39% for `Xor8`, ≈0.0015% for `Xor16`).
//!
//! Construction "peels" the 3-hypergraph: it repeatedly removes a slot touched by a single remaining
//! key, recording the order, then back-substitutes fingerprints in reverse so each key's slot can be
//! set without disturbing already-placed keys. If a hash seed yields an unpeelable graph (rare), the
//! build retries with a new seed.
//!
//! This is the predecessor of [`BinaryFuseFilter`](crate::membership::BinaryFuseFilter); XOR filters
//! are simpler and widely referenced by name (`Xor8`/`Xor16`), while Binary Fuse is slightly smaller.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use std::collections::HashSet;

const MAX_BUILD_ATTEMPTS: u64 = 100;

/// An immutable XOR filter with `bits`-wide fingerprints (8 or 16).
///
/// Keys are `u64` (hash arbitrary data into a `u64` first). Build with [`XorFilter::from_keys`].
///
/// # Example
/// ```
/// use sketch_oxide::membership::XorFilter;
///
/// let keys: Vec<u64> = (0..10_000).collect();
/// let filter = XorFilter::from_keys(&keys, 8).unwrap();
///
/// // No false negatives.
/// assert!(keys.iter().all(|k| filter.contains(*k)));
/// // Non-keys are almost always rejected.
/// assert!(!filter.contains(123_456_789));
/// ```
#[derive(Debug, Clone)]
pub struct XorFilter {
    seed: u64,
    /// Slots per block; the array has `3 · block_length` slots.
    block_length: usize,
    /// Fingerprint slots (each holds a `bits`-wide value).
    fingerprints: Vec<u16>,
    /// Fingerprint width mask `(1 << bits) - 1`.
    mask: u16,
    bits: u8,
    size: usize,
}

impl XorFilter {
    /// Builds a filter from `keys` with `bits`-wide fingerprints (`8` or `16`).
    ///
    /// Duplicate keys are de-duplicated. An empty key set yields an empty filter that rejects
    /// everything.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `bits` is not 8 or 16; [`SketchError::SerializationError`]
    /// if construction fails to find a working seed after many attempts (astronomically unlikely).
    pub fn from_keys(keys: &[u64], bits: u8) -> Result<Self> {
        if bits != 8 && bits != 16 {
            return Err(SketchError::InvalidParameter {
                param: "bits".to_string(),
                value: bits.to_string(),
                constraint: "must be 8 or 16".to_string(),
            });
        }
        let mask: u16 = if bits == 16 { 0xFFFF } else { 0x00FF };
        let distinct: Vec<u64> = {
            let set: HashSet<u64> = keys.iter().copied().collect();
            set.into_iter().collect()
        };
        let n = distinct.len();
        if n == 0 {
            return Ok(Self {
                seed: 0,
                block_length: 0,
                fingerprints: Vec::new(),
                mask,
                bits,
                size: 0,
            });
        }
        let capacity = (1.23 * n as f64) as usize + 32;
        let block_length = capacity / 3 + 1;
        let m = block_length * 3;

        for attempt in 0..MAX_BUILD_ATTEMPTS {
            let seed = 0x9E37_79B9_7F4A_7C15u64.wrapping_mul(attempt + 1) ^ 0xF00D;
            if let Some(fingerprints) = Self::try_build(&distinct, seed, block_length, m, mask) {
                return Ok(Self {
                    seed,
                    block_length,
                    fingerprints,
                    mask,
                    bits,
                    size: n,
                });
            }
        }
        Err(SketchError::SerializationError(
            "XOR filter construction failed to find a peelable seed".to_string(),
        ))
    }

    /// `((x · n) >> 32)` — Lemire's fast reduction of a 32-bit value into `0..n`.
    #[inline]
    fn reduce(x: u32, n: usize) -> usize {
        ((x as u64 * n as u64) >> 32) as usize
    }

    /// The three slot indices of `key_hash`.
    #[inline]
    fn slots(key_hash: u64, block_length: usize) -> [usize; 3] {
        // Three full-width 32-bit sub-hashes via rotation (a bare `>> 42` would leave only 22 bits
        // and cluster the third slot, breaking peeling).
        let r0 = key_hash as u32;
        let r1 = key_hash.rotate_right(21) as u32;
        let r2 = key_hash.rotate_right(42) as u32;
        [
            Self::reduce(r0, block_length),
            block_length + Self::reduce(r1, block_length),
            2 * block_length + Self::reduce(r2, block_length),
        ]
    }

    /// The fingerprint of `key_hash`.
    #[inline]
    fn fingerprint(key_hash: u64, mask: u16) -> u16 {
        ((key_hash ^ (key_hash >> 32)) as u16) & mask
    }

    /// Attempts one peeling construction; returns the fingerprint array on success.
    fn try_build(
        keys: &[u64],
        seed: u64,
        block_length: usize,
        m: usize,
        mask: u16,
    ) -> Option<Vec<u16>> {
        // Per-slot incidence: how many keys touch it, and the XOR of their hashes.
        let mut count = vec![0u32; m];
        let mut xor_acc = vec![0u64; m];
        let key_hashes: Vec<u64> = keys
            .iter()
            .map(|&k| xxhash(&k.to_le_bytes(), seed))
            .collect();
        for &kh in &key_hashes {
            for p in Self::slots(kh, block_length) {
                count[p] += 1;
                xor_acc[p] ^= kh;
            }
        }

        // Peel: repeatedly take a slot of degree 1.
        let mut queue: Vec<usize> = (0..m).filter(|&p| count[p] == 1).collect();
        let mut stack: Vec<(usize, u64)> = Vec::with_capacity(keys.len());
        while let Some(slot) = queue.pop() {
            if count[slot] != 1 {
                continue; // stale entry
            }
            let kh = xor_acc[slot];
            stack.push((slot, kh));
            for p in Self::slots(kh, block_length) {
                count[p] -= 1;
                xor_acc[p] ^= kh;
                if count[p] == 1 {
                    queue.push(p);
                }
            }
        }
        if stack.len() != keys.len() {
            return None; // graph was not fully peelable
        }

        // Back-substitute fingerprints in reverse peeling order.
        let mut fingerprints = vec![0u16; m];
        for &(slot, kh) in stack.iter().rev() {
            let mut fp = Self::fingerprint(kh, mask);
            for p in Self::slots(kh, block_length) {
                if p != slot {
                    fp ^= fingerprints[p];
                }
            }
            fingerprints[slot] = fp;
        }
        Some(fingerprints)
    }

    /// Tests whether `key` is (probably) in the set. No false negatives; false-positive rate
    /// `≈ 2^{−bits}`.
    pub fn contains(&self, key: u64) -> bool {
        if self.size == 0 {
            return false;
        }
        let kh = xxhash(&key.to_le_bytes(), self.seed);
        let fp = Self::fingerprint(kh, self.mask);
        let [a, b, c] = Self::slots(kh, self.block_length);
        fp == (self.fingerprints[a] ^ self.fingerprints[b] ^ self.fingerprints[c])
    }

    /// Number of distinct keys the filter was built from.
    #[inline]
    pub fn len(&self) -> usize {
        self.size
    }

    /// Whether the filter holds no keys.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Fingerprint width in bits (8 or 16).
    #[inline]
    pub fn bits_per_fingerprint(&self) -> u8 {
        self.bits
    }

    /// Total number of fingerprint slots.
    #[inline]
    pub fn slot_count(&self) -> usize {
        self.fingerprints.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_bits() {
        assert!(XorFilter::from_keys(&[1, 2, 3], 4).is_err());
        assert!(XorFilter::from_keys(&[1, 2, 3], 8).is_ok());
        assert!(XorFilter::from_keys(&[1, 2, 3], 16).is_ok());
    }

    #[test]
    fn no_false_negatives() {
        let keys: Vec<u64> = (0..20_000).map(|i| i * 2_654_435_761).collect();
        let filter = XorFilter::from_keys(&keys, 8).unwrap();
        for &k in &keys {
            assert!(filter.contains(k), "missing key {k}");
        }
        assert_eq!(filter.len(), 20_000);
    }

    #[test]
    fn xor8_false_positive_rate() {
        let keys: Vec<u64> = (0..50_000u64).collect();
        let filter = XorFilter::from_keys(&keys, 8).unwrap();
        let mut fp = 0;
        let trials = 100_000u64;
        for k in 1_000_000..1_000_000 + trials {
            if filter.contains(k) {
                fp += 1;
            }
        }
        let rate = fp as f64 / trials as f64;
        // Expected ≈ 1/256 ≈ 0.0039; allow generous headroom.
        assert!(rate < 0.012, "Xor8 FPR {rate}");
    }

    #[test]
    fn xor16_lower_false_positive_rate() {
        let keys: Vec<u64> = (0..50_000u64).collect();
        let filter = XorFilter::from_keys(&keys, 16).unwrap();
        let mut fp = 0;
        let trials = 200_000u64;
        for k in 5_000_000..5_000_000 + trials {
            if filter.contains(k) {
                fp += 1;
            }
        }
        let rate = fp as f64 / trials as f64;
        // Expected ≈ 1/65536 ≈ 0.0000153; comfortably under this loose bound.
        assert!(rate < 0.001, "Xor16 FPR {rate}");
    }

    #[test]
    fn handles_duplicates() {
        let keys = [7u64, 7, 7, 42, 42, 100];
        let filter = XorFilter::from_keys(&keys, 8).unwrap();
        assert_eq!(filter.len(), 3);
        assert!(filter.contains(7) && filter.contains(42) && filter.contains(100));
    }

    #[test]
    fn empty_filter_rejects() {
        let filter = XorFilter::from_keys(&[], 8).unwrap();
        assert!(filter.is_empty());
        assert!(!filter.contains(1));
    }
}
