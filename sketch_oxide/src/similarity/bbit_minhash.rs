//! b-bit minwise hashing — compact MinHash signatures.
//!
//! A MinHash signature stores `k` full 64-bit minima. b-bit minwise hashing (Li & König,
//! "b-Bit Minwise Hashing", WWW 2010) keeps only the lowest `b` bits of each minimum,
//! shrinking the signature by up to 64× (e.g. `b = 1`) at a small, quantified accuracy cost
//! — the regime that matters at trillion-token dedup scale, where signature size dominates.
//!
//! # Estimator
//!
//! Two independent `b`-bit minima collide with probability `2^-b` purely by chance, so for
//! the observed fraction `P` of matching positions the (large-set) Jaccard estimate is
//!
//! ```text
//! J ≈ (P − 2^-b) / (1 − 2^-b)
//! ```
//!
//! clamped to `[0, 1]`. Accuracy improves with `k` and (for similar sets) with `b`.

use crate::common::{Result, SketchError};

/// A bit-packed b-bit minwise signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BBitMinHash {
    bits: u8,
    len: usize,
    /// `len * bits` packed bits.
    packed: Vec<u8>,
}

impl BBitMinHash {
    /// Builds a b-bit signature from a full MinHash signature, keeping the low `bits` bits of
    /// each value.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `bits` is 0 or > 64, or `signature` is empty.
    pub fn from_signature(signature: &[u64], bits: u8) -> Result<Self> {
        if bits == 0 || bits > 64 {
            return Err(SketchError::InvalidParameter {
                param: "bits".to_string(),
                value: bits.to_string(),
                constraint: "must be in [1, 64]".to_string(),
            });
        }
        if signature.is_empty() {
            return Err(SketchError::InvalidParameter {
                param: "signature".to_string(),
                value: "empty".to_string(),
                constraint: "must be non-empty".to_string(),
            });
        }
        let total_bits = signature.len() * bits as usize;
        let mut packed = vec![0u8; total_bits.div_ceil(8)];
        let mask = if bits == 64 {
            u64::MAX
        } else {
            (1u64 << bits) - 1
        };
        for (i, &v) in signature.iter().enumerate() {
            set_bits(&mut packed, i * bits as usize, bits, v & mask);
        }
        Ok(Self {
            bits,
            len: signature.len(),
            packed,
        })
    }

    /// Number of positions (the original signature length).
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the signature is empty (always false for a constructed one).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Bits retained per position.
    #[inline]
    pub fn bits(&self) -> u8 {
        self.bits
    }

    /// Packed size in bytes.
    #[inline]
    pub fn byte_len(&self) -> usize {
        self.packed.len()
    }

    /// The b-bit value at position `i`.
    fn value_at(&self, i: usize) -> u64 {
        get_bits(&self.packed, i * self.bits as usize, self.bits)
    }

    /// Estimates the Jaccard similarity with another b-bit signature of the same `bits` and
    /// length, using the Li–König estimator.
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the two signatures differ in `bits` or length.
    pub fn jaccard(&self, other: &Self) -> Result<f64> {
        if self.bits != other.bits || self.len != other.len {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "shape mismatch: {}-bit×{} vs {}-bit×{}",
                    self.bits, self.len, other.bits, other.len
                ),
            });
        }
        let matches = (0..self.len)
            .filter(|&i| self.value_at(i) == other.value_at(i))
            .count();
        let p = matches as f64 / self.len as f64;
        let base = 2.0_f64.powi(-(self.bits as i32)); // 2^-b
        let j = (p - base) / (1.0 - base);
        Ok(j.clamp(0.0, 1.0))
    }
}

/// Writes the low `bits` of `value` into `buf` starting at `bit_offset`.
fn set_bits(buf: &mut [u8], bit_offset: usize, bits: u8, value: u64) {
    for j in 0..bits as usize {
        if (value >> j) & 1 == 1 {
            let pos = bit_offset + j;
            buf[pos / 8] |= 1 << (pos % 8);
        }
    }
}

/// Reads `bits` bits from `buf` starting at `bit_offset`.
fn get_bits(buf: &[u8], bit_offset: usize, bits: u8) -> u64 {
    let mut value = 0u64;
    for j in 0..bits as usize {
        let pos = bit_offset + j;
        if buf[pos / 8] & (1 << (pos % 8)) != 0 {
            value |= 1 << j;
        }
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(BBitMinHash::from_signature(&[1, 2, 3], 0).is_err());
        assert!(BBitMinHash::from_signature(&[1, 2, 3], 65).is_err());
        assert!(BBitMinHash::from_signature(&[], 4).is_err());
        assert!(BBitMinHash::from_signature(&[1, 2, 3], 4).is_ok());
    }

    #[test]
    fn round_trips_low_bits() {
        let sig: Vec<u64> = (0..100).collect();
        let bb = BBitMinHash::from_signature(&sig, 8).unwrap();
        for i in 0..100 {
            assert_eq!(bb.value_at(i), sig[i] & 0xFF);
        }
    }

    #[test]
    fn packed_size_is_compact() {
        let sig = vec![0xFFFF_FFFF_FFFF_FFFFu64; 1000];
        let bb = BBitMinHash::from_signature(&sig, 1).unwrap();
        // 1000 * 1 bit = 125 bytes vs 8000 bytes for the full signature => 64x smaller.
        assert_eq!(bb.byte_len(), 125);
    }

    #[test]
    fn identical_signatures_give_jaccard_one() {
        let sig: Vec<u64> = (0..200).collect();
        let a = BBitMinHash::from_signature(&sig, 8).unwrap();
        let b = a.clone();
        assert!((a.jaccard(&b).unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn estimates_partial_similarity() {
        // 70% of positions identical; the rest differ in their low bits.
        let n = 1000;
        let sig_a: Vec<u64> = (0..n).collect();
        let sig_b: Vec<u64> = (0..n)
            .map(|i| if i < 700 { i } else { i ^ 0xFFFF })
            .collect();
        let a = BBitMinHash::from_signature(&sig_a, 12).unwrap();
        let b = BBitMinHash::from_signature(&sig_b, 12).unwrap();
        let j = a.jaccard(&b).unwrap();
        assert!((j - 0.70).abs() < 0.05, "estimated Jaccard {j}");
    }

    #[test]
    fn disjoint_low_bits_give_near_zero() {
        let n = 2000;
        let sig_a: Vec<u64> = (0..n).map(|i| i * 2).collect(); // even low bits
        let sig_b: Vec<u64> = (0..n).map(|i| i * 2 + 1).collect(); // odd low bits
        let a = BBitMinHash::from_signature(&sig_a, 4).unwrap();
        let b = BBitMinHash::from_signature(&sig_b, 4).unwrap();
        // Low bits always differ in the LSB => never match => J ≈ 0.
        assert!(a.jaccard(&b).unwrap() < 0.05);
    }

    #[test]
    fn shape_mismatch_errors() {
        let a = BBitMinHash::from_signature(&[1, 2, 3], 4).unwrap();
        let b = BBitMinHash::from_signature(&[1, 2, 3], 8).unwrap();
        assert!(a.jaccard(&b).is_err());
        let c = BBitMinHash::from_signature(&[1, 2], 4).unwrap();
        assert!(a.jaccard(&c).is_err());
    }
}
