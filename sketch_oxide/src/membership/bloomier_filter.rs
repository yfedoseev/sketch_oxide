//! Bloomier filter — a compact static map from keys to small values (Chazelle, Kilian, Rubinfeld &
//! Tal, 2004).
//!
//! Where an [`XorFilter`](crate::membership::XorFilter) answers *is this key present?*, a Bloomier
//! filter answers *what value is associated with this key?* for a fixed key→value map, in space close
//! to the entropy of the values. It uses the same 3-wise XOR-peeling construction as the XOR filter,
//! but each slot stores `fingerprint‖value` bits: a query XORs a key's three slots and checks the
//! fingerprint. A key from the build set always returns its exact value; a key that was never inserted
//! is rejected (returns `None`) with probability `1 − 2^{−fingerprint_bits}`.
//!
//! This makes it a *retrieval data structure* — the building block behind compressed static
//! dictionaries and monotone minimal perfect hashing — with no false negatives and a tunable
//! false-positive (wrong-value) rate on absent keys.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use std::collections::HashMap;

const MAX_BUILD_ATTEMPTS: u64 = 100;

/// A static key→value map with `value_bits`-wide values and `fingerprint_bits` of absence checking.
///
/// Keys are `u64`; values are `u64` that must fit in `value_bits`. Build with
/// [`BloomierFilter::from_pairs`].
///
/// # Example
/// ```
/// use sketch_oxide::membership::BloomierFilter;
///
/// // Map 5000 keys to values (here key → key % 1000).
/// let pairs: Vec<(u64, u64)> = (0..5000u64).map(|k| (k, k % 1000)).collect();
/// let bf = BloomierFilter::from_pairs(&pairs, 10, 16).unwrap(); // 10-bit values, 16-bit fingerprint
///
/// assert_eq!(bf.get(1234), Some(234)); // present key → exact value
/// assert_eq!(bf.get(7_000_000), None); // absent key → rejected (w.h.p.)
/// ```
#[derive(Debug, Clone)]
pub struct BloomierFilter {
    seed: u64,
    block_length: usize,
    /// Each slot holds `fingerprint‖value` (`fingerprint_bits + value_bits` bits).
    slots: Vec<u64>,
    value_bits: u8,
    fingerprint_bits: u8,
    size: usize,
}

impl BloomierFilter {
    /// Builds a Bloomier filter from `pairs` (later duplicates of a key win).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `value_bits + fingerprint_bits > 64`, either is 0, or any
    /// value does not fit in `value_bits`. [`SketchError::SerializationError`] if construction fails
    /// to find a peelable seed (astronomically unlikely).
    pub fn from_pairs(pairs: &[(u64, u64)], value_bits: u8, fingerprint_bits: u8) -> Result<Self> {
        if value_bits == 0 || fingerprint_bits == 0 || value_bits + fingerprint_bits > 64 {
            return Err(SketchError::InvalidParameter {
                param: "value_bits+fingerprint_bits".to_string(),
                value: format!("{value_bits}+{fingerprint_bits}"),
                constraint: "each > 0 and sum <= 64".to_string(),
            });
        }
        let value_mask = if value_bits == 64 {
            u64::MAX
        } else {
            (1u64 << value_bits) - 1
        };
        // De-duplicate keys (last value wins) and validate value width.
        let mut map: HashMap<u64, u64> = HashMap::with_capacity(pairs.len());
        for &(k, v) in pairs {
            if v > value_mask {
                return Err(SketchError::InvalidParameter {
                    param: "value".to_string(),
                    value: v.to_string(),
                    constraint: format!("must fit in {value_bits} bits"),
                });
            }
            map.insert(k, v);
        }
        let keys: Vec<u64> = map.keys().copied().collect();
        let n = keys.len();
        if n == 0 {
            return Ok(Self {
                seed: 0,
                block_length: 0,
                slots: Vec::new(),
                value_bits,
                fingerprint_bits,
                size: 0,
            });
        }
        let capacity = (1.23 * n as f64) as usize + 32;
        let block_length = capacity / 3 + 1;
        let m = block_length * 3;
        let fp_mask = (1u64 << fingerprint_bits) - 1;

        for attempt in 0..MAX_BUILD_ATTEMPTS {
            let seed = 0x9E37_79B9_7F4A_7C15u64.wrapping_mul(attempt + 1) ^ 0xB100;
            if let Some(slots) =
                Self::try_build(&keys, &map, seed, block_length, m, value_bits, fp_mask)
            {
                return Ok(Self {
                    seed,
                    block_length,
                    slots,
                    value_bits,
                    fingerprint_bits,
                    size: n,
                });
            }
        }
        Err(SketchError::SerializationError(
            "Bloomier filter construction failed to find a peelable seed".to_string(),
        ))
    }

    #[inline]
    fn reduce(x: u32, n: usize) -> usize {
        ((x as u64 * n as u64) >> 32) as usize
    }

    /// The three slot indices of `key_hash`.
    #[inline]
    fn slots_of(key_hash: u64, block_length: usize) -> [usize; 3] {
        let r0 = key_hash as u32;
        let r1 = key_hash.rotate_right(21) as u32;
        let r2 = key_hash.rotate_right(42) as u32;
        [
            Self::reduce(r0, block_length),
            block_length + Self::reduce(r1, block_length),
            2 * block_length + Self::reduce(r2, block_length),
        ]
    }

    /// Fingerprint of `key_hash`.
    #[inline]
    fn fingerprint(key_hash: u64, fp_mask: u64) -> u64 {
        ((key_hash >> 40) ^ key_hash) & fp_mask
    }

    /// The cell content for a key: `fingerprint‖value`.
    #[inline]
    fn content(key_hash: u64, value: u64, value_bits: u8, fp_mask: u64) -> u64 {
        (Self::fingerprint(key_hash, fp_mask) << value_bits) | value
    }

    #[allow(clippy::too_many_arguments)]
    fn try_build(
        keys: &[u64],
        map: &HashMap<u64, u64>,
        seed: u64,
        block_length: usize,
        m: usize,
        value_bits: u8,
        fp_mask: u64,
    ) -> Option<Vec<u64>> {
        let mut count = vec![0u32; m];
        let mut xor_acc = vec![0u64; m];
        let key_hashes: Vec<u64> = keys
            .iter()
            .map(|&k| xxhash(&k.to_le_bytes(), seed))
            .collect();
        for &kh in &key_hashes {
            for p in Self::slots_of(kh, block_length) {
                count[p] += 1;
                xor_acc[p] ^= kh;
            }
        }

        let mut queue: Vec<usize> = (0..m).filter(|&p| count[p] == 1).collect();
        let mut stack: Vec<(usize, u64)> = Vec::with_capacity(keys.len());
        while let Some(slot) = queue.pop() {
            if count[slot] != 1 {
                continue;
            }
            let kh = xor_acc[slot];
            stack.push((slot, kh));
            for p in Self::slots_of(kh, block_length) {
                count[p] -= 1;
                xor_acc[p] ^= kh;
                if count[p] == 1 {
                    queue.push(p);
                }
            }
        }
        if stack.len() != keys.len() {
            return None;
        }

        // Recover each key's hash → original key to look up its value. The peeling stored key hashes;
        // map them back via a hash→value table built from the same hashing.
        let mut hash_to_value: HashMap<u64, u64> = HashMap::with_capacity(keys.len());
        for (&k, &kh) in keys.iter().zip(&key_hashes) {
            hash_to_value.insert(kh, map[&k]);
        }

        let mut slots = vec![0u64; m];
        for &(slot, kh) in stack.iter().rev() {
            let value = hash_to_value[&kh];
            let mut content = Self::content(kh, value, value_bits, fp_mask);
            for p in Self::slots_of(kh, block_length) {
                if p != slot {
                    content ^= slots[p];
                }
            }
            slots[slot] = content;
        }
        Some(slots)
    }

    /// Returns the value associated with `key`, or `None` if `key` was not in the build set (the
    /// latter is wrong with probability `2^{−fingerprint_bits}`).
    pub fn get(&self, key: u64) -> Option<u64> {
        if self.size == 0 {
            return None;
        }
        let kh = xxhash(&key.to_le_bytes(), self.seed);
        let [a, b, c] = Self::slots_of(kh, self.block_length);
        let content = self.slots[a] ^ self.slots[b] ^ self.slots[c];
        let fp_mask = (1u64 << self.fingerprint_bits) - 1;
        let stored_fp = content >> self.value_bits;
        if stored_fp != Self::fingerprint(kh, fp_mask) {
            return None;
        }
        let value_mask = if self.value_bits == 64 {
            u64::MAX
        } else {
            (1u64 << self.value_bits) - 1
        };
        Some(content & value_mask)
    }

    /// Number of keys in the map.
    #[inline]
    pub fn len(&self) -> usize {
        self.size
    }

    /// Whether the map is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Bits per stored value.
    #[inline]
    pub fn value_bits(&self) -> u8 {
        self.value_bits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(BloomierFilter::from_pairs(&[(1, 0)], 0, 8).is_err());
        assert!(BloomierFilter::from_pairs(&[(1, 0)], 8, 0).is_err());
        assert!(BloomierFilter::from_pairs(&[(1, 0)], 40, 40).is_err()); // sum > 64
        assert!(BloomierFilter::from_pairs(&[(1, 1024)], 10, 8).is_err()); // value too large
        assert!(BloomierFilter::from_pairs(&[(1, 5)], 10, 8).is_ok());
    }

    #[test]
    fn retrieves_all_values_exactly() {
        let pairs: Vec<(u64, u64)> = (0..10_000u64)
            .map(|k| (k.wrapping_mul(2_654_435_761), k % 1000))
            .collect();
        let bf = BloomierFilter::from_pairs(&pairs, 10, 16).unwrap();
        for &(k, v) in &pairs {
            assert_eq!(bf.get(k), Some(v), "key {k}");
        }
        assert_eq!(bf.len(), 10_000);
    }

    #[test]
    fn rejects_absent_keys() {
        let pairs: Vec<(u64, u64)> = (0..50_000u64).map(|k| (k, k % 256)).collect();
        let bf = BloomierFilter::from_pairs(&pairs, 8, 16).unwrap();
        let mut false_hits = 0;
        let trials = 100_000u64;
        for k in 1_000_000..1_000_000 + trials {
            if bf.get(k).is_some() {
                false_hits += 1;
            }
        }
        let rate = false_hits as f64 / trials as f64;
        // Expected ≈ 2^-16 ≈ 0.0000153; comfortably under this loose bound.
        assert!(rate < 0.001, "absent-key acceptance rate {rate}");
    }

    #[test]
    fn last_value_wins_on_duplicate_key() {
        let bf = BloomierFilter::from_pairs(&[(7, 1), (7, 2), (7, 9)], 8, 12).unwrap();
        assert_eq!(bf.len(), 1);
        assert_eq!(bf.get(7), Some(9));
    }

    #[test]
    fn empty_map() {
        let bf = BloomierFilter::from_pairs(&[], 8, 8).unwrap();
        assert!(bf.is_empty());
        assert_eq!(bf.get(1), None);
    }
}
