//! Rosetta — range filtering via a hierarchy of prefix Bloom filters.
//!
//! Rosetta (Luo, Ding, Das, Dong, ... "Rosetta: A Robust Space-Time Optimized Range Filter for
//! Key-Value Stores", SIGMOD 2020) turns range queries into a handful of point queries. Every key
//! is inserted under *all* of its prefixes — its level-`L` prefix for `L = 0..=bits` — into a single
//! Bloom filter keyed by `(level, prefix)`. A range query `[low, high]` is **dyadically decomposed**
//! into the `O(bits)` canonical prefixes (segment-tree blocks) that exactly tile it, and each block
//! is checked against the Bloom: if any block's `(level, prefix)` is present the range may be
//! non-empty, otherwise it is definitely empty.
//!
//! Because a present key in `[low, high]` lies in exactly one decomposition block, and that block's
//! prefix is one of the key's inserted prefixes, the Bloom always reports it — so there are **no
//! false negatives**. False positives come only from Bloom collisions.

use crate::common::hash::xxhash;
use crate::common::{RangeFilter, Result, SketchError};

/// A Rosetta range filter over `bits`-bit unsigned keys.
///
/// # Example
/// ```
/// use sketch_oxide::range_filters::Rosetta;
///
/// let mut r = Rosetta::new(1000, 32, 0.01).unwrap();
/// assert!(!r.range_query(0, 1_000_000));  // empty filter: definitely nothing
///
/// for k in [100u64, 5_000, 900_000] { r.insert(k); }
/// assert!(r.range_query(4_000, 6_000));   // 5_000 is in range (never a false negative)
/// ```
///
/// Note: like any Bloom-based range filter, Rosetta's false-positive probability grows with the
/// query width, because a wide range decomposes into more prefix blocks (each an independent Bloom
/// check). Size `fpr` for the widest ranges you expect to query.
#[derive(Debug, Clone)]
pub struct Rosetta {
    bits: u32,
    /// Bloom bit array (as 64-bit words).
    words: Vec<u64>,
    m: u64,
    k: u32,
    n: usize,
}

impl Rosetta {
    /// Creates a Rosetta filter sized for `expected_keys` keys of `bits` bits at false-positive
    /// rate `fpr`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `expected_keys` is 0, `bits` is outside `1..=64`, or
    /// `fpr` is not in `(0, 1)`.
    pub fn new(expected_keys: usize, bits: u32, fpr: f64) -> Result<Self> {
        if expected_keys == 0 {
            return Err(SketchError::InvalidParameter {
                param: "expected_keys".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        if !(1..=64).contains(&bits) {
            return Err(SketchError::InvalidParameter {
                param: "bits".to_string(),
                value: bits.to_string(),
                constraint: "must be in 1..=64".to_string(),
            });
        }
        if !(fpr > 0.0 && fpr < 1.0) {
            return Err(SketchError::InvalidParameter {
                param: "fpr".to_string(),
                value: fpr.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }
        // Each key inserts (bits + 1) prefixes.
        let entries = (expected_keys as f64) * (bits as f64 + 1.0);
        let m = (-(entries * fpr.ln()) / (std::f64::consts::LN_2 * std::f64::consts::LN_2))
            .ceil()
            .max(64.0) as u64;
        let k = ((m as f64 / entries) * std::f64::consts::LN_2)
            .round()
            .clamp(1.0, 30.0) as u32;
        Ok(Self {
            bits,
            words: vec![0u64; m.div_ceil(64) as usize],
            m,
            k,
            n: 0,
        })
    }

    /// The level-`level` prefix of `key`: its top `level` bits.
    #[inline]
    fn prefix(&self, key: u64, level: u32) -> u64 {
        key.checked_shr(self.bits - level).unwrap_or(0)
    }

    #[inline]
    fn bit_index(&self, level: u32, prefix: u64, r: u32) -> u64 {
        let mut buf = [0u8; 12];
        buf[..4].copy_from_slice(&level.to_le_bytes());
        buf[4..].copy_from_slice(&prefix.to_le_bytes());
        xxhash(&buf, r as u64) % self.m
    }

    fn set(&mut self, level: u32, prefix: u64) {
        for r in 0..self.k {
            let idx = self.bit_index(level, prefix, r);
            self.words[(idx / 64) as usize] |= 1u64 << (idx % 64);
        }
    }

    fn test(&self, level: u32, prefix: u64) -> bool {
        (0..self.k).all(|r| {
            let idx = self.bit_index(level, prefix, r);
            self.words[(idx / 64) as usize] & (1u64 << (idx % 64)) != 0
        })
    }

    /// Inserts a key under all of its prefixes.
    pub fn insert(&mut self, key: u64) {
        for level in 0..=self.bits {
            let p = self.prefix(key, level);
            self.set(level, p);
        }
        self.n += 1;
    }

    /// Decomposes `[lo, hi]` into the canonical `(level, prefix)` dyadic blocks that exactly tile
    /// it.
    fn decompose(&self, lo: u64, hi: u64) -> Vec<(u32, u64)> {
        let mut out = Vec::new();
        let mut lo = lo as u128;
        let hi = hi as u128;
        while lo <= hi {
            // Largest power-of-two block that is aligned at `lo` and fits within `[lo, hi]`.
            let align = if lo == 0 {
                self.bits
            } else {
                (lo.trailing_zeros()).min(self.bits)
            };
            let mut sz = align;
            while sz > 0 && lo + (1u128 << sz) - 1 > hi {
                sz -= 1;
            }
            let level = self.bits - sz;
            out.push((level, (lo >> sz) as u64));
            lo += 1u128 << sz;
        }
        out
    }

    /// Whether the range `[low, high]` might contain a key. No false negatives.
    pub fn range_query(&self, low: u64, high: u64) -> bool {
        if low > high || self.n == 0 {
            return false;
        }
        let mask = if self.bits == 64 {
            u64::MAX
        } else {
            (1u64 << self.bits) - 1
        };
        let (low, high) = (low & mask, high.min(mask));
        self.decompose(low, high)
            .into_iter()
            .any(|(level, prefix)| self.test(level, prefix))
    }

    /// Number of keys inserted.
    #[inline]
    pub fn len(&self) -> usize {
        self.n
    }

    /// Whether no keys have been inserted.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Number of hash functions.
    #[inline]
    pub fn num_hashes(&self) -> u32 {
        self.k
    }
}

impl RangeFilter for Rosetta {
    fn may_contain_range(&self, low: u64, high: u64) -> bool {
        self.range_query(low, high)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(Rosetta::new(0, 32, 0.01).is_err());
        assert!(Rosetta::new(100, 0, 0.01).is_err());
        assert!(Rosetta::new(100, 65, 0.01).is_err());
        assert!(Rosetta::new(100, 32, 0.0).is_err());
        assert!(Rosetta::new(100, 32, 0.01).is_ok());
    }

    #[test]
    fn point_ranges_find_inserted_keys() {
        let mut r = Rosetta::new(1000, 32, 0.01).unwrap();
        let keys = [1u64, 42, 1000, 65_535, 1_000_000, 4_000_000_000];
        for &k in &keys {
            r.insert(k);
        }
        for &k in &keys {
            assert!(r.range_query(k, k), "false negative for {k}");
        }
    }

    #[test]
    fn no_false_negatives_over_many_keys_and_ranges() {
        let mut r = Rosetta::new(5000, 32, 0.01).unwrap();
        let keys: Vec<u64> = (0..5000u64)
            .map(|i| i.wrapping_mul(2_654_435_761) & 0xFFFF_FFFF)
            .collect();
        for &k in &keys {
            r.insert(k);
        }
        for &k in &keys {
            // Every inserted key must be found by a tight range around it.
            assert!(r.range_query(k.saturating_sub(1), k + 1), "FN near {k}");
        }
    }

    #[test]
    fn empty_ranges_rejected() {
        // Wide-range false positives compound across decomposition blocks, so use a low fpr to make
        // this deterministic-in-practice (negligible compounded FP).
        let mut r = Rosetta::new(1000, 32, 1e-6).unwrap();
        for k in [1_000_000u64, 2_000_000, 3_000_000] {
            r.insert(k);
        }
        // Gaps far from any key (and its prefixes) should be rejected.
        assert!(!r.range_query(10, 1000));
        assert!(!r.range_query(5_000_000, 6_000_000));
    }

    #[test]
    fn span_range_contains_key() {
        let mut r = Rosetta::new(1000, 32, 0.01).unwrap();
        r.insert(123_456);
        assert!(r.range_query(100_000, 200_000));
        assert!(r.range_query(0, u64::MAX)); // full range
    }

    #[test]
    fn false_positive_rate_is_bounded() {
        let mut r = Rosetta::new(2000, 24, 0.01).unwrap();
        for i in 0..2000u64 {
            r.insert(i * 3); // keys 0,3,6,... up to 6000, within 24 bits
        }
        // Query disjoint single-point ranges far above the inserted set.
        let mut fp = 0;
        for i in 0..2000u64 {
            let q = 8_000_000 + i; // well outside [0, 6000]
            if r.range_query(q, q) {
                fp += 1;
            }
        }
        let rate = fp as f64 / 2000.0;
        assert!(rate < 0.1, "false-positive rate {rate} too high");
    }

    #[test]
    fn empty_filter_rejects() {
        let r = Rosetta::new(100, 32, 0.01).unwrap();
        assert!(r.is_empty());
        assert!(!r.range_query(0, u64::MAX));
    }
}
