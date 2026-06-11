//! bloomRF — range queries in a *single* Bloom filter via prefix hashing (Mößner, Riegger, Bernhardt
//! & Petrov, EDBT 2023).
//!
//! A plain Bloom filter answers only point queries. bloomRF extends one Bloom filter to **range**
//! queries with **prefix hashing**: each key is inserted at several **dyadic prefix levels** (the key
//! with its low bits masked off), so the filter records, for every encoded level, which prefixes are
//! occupied. A range query `[lo, hi]` is then resolved by a coarse-to-fine **dyadic descent**: probe
//! the coarse prefixes overlapping the range; an absent prefix prunes its whole subtree, while a
//! present one is refined to the next level, reporting a possible hit only if some finest-level prefix
//! overlapping the range is present. This needs no per-level filters (unlike a trie of Bloom filters),
//! giving near-optimal space and constant query cost.
//!
//! Like all range filters here it has **no false negatives**; false positives come from Bloom-filter
//! collisions and coarse prefix encoding.
//!
//! # Implementation note
//!
//! The paper's *piecewise-monotone hash functions* (which keep nearby prefixes in nearby bits to cut
//! cache misses) are a performance optimisation; this reference uses ordinary per-`(prefix, level)`
//! hashing over a single bit array, preserving the same membership semantics.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

const SEED: u64 = 0xB100_F2F0_0000_0001;
const KEY_BITS: u32 = 64;

/// A bloomRF point/range filter over `u64` keys.
///
/// # Example
/// ```
/// use sketch_oxide::range_filters::BloomRf;
///
/// // 1 Mbit filter, 4 hashes, encode dyadic levels from 16 bits up to 64 in steps of 8.
/// let mut f = BloomRf::new(1 << 20, 4, 16, 8).unwrap();
/// for i in (0..100_000u64).map(|i| i * 10) {
///     f.insert(i);
/// }
/// // Point query: an inserted key is present, no false negatives.
/// assert!(f.contains(50_000));
/// // Range query over a populated interval reports possible.
/// assert!(f.range_query(49_990, 50_010));
/// // A range far from any key is ruled out.
/// assert!(!f.range_query(5_000_000_000, 5_000_001_000));
/// ```
#[derive(Debug, Clone)]
pub struct BloomRf {
    bits: Vec<bool>,
    m: usize,
    k: u32,
    levels: Vec<u32>,
}

impl BloomRf {
    /// Creates a bloomRF with `num_bits` bits, `num_hashes` hash functions (`1..=16`), encoding dyadic
    /// prefixes from `min_level` bits up to 64 in steps of `level_step` (and always the full 64-bit
    /// key). Fewer/coarser levels save space at the cost of wider-range resolution.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if any constraint is violated.
    pub fn new(num_bits: usize, num_hashes: u32, min_level: u32, level_step: u32) -> Result<Self> {
        if num_bits == 0 {
            return Err(SketchError::InvalidParameter {
                param: "num_bits".to_string(),
                value: "0".to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        if !(1..=16).contains(&num_hashes) {
            return Err(SketchError::InvalidParameter {
                param: "num_hashes".to_string(),
                value: num_hashes.to_string(),
                constraint: "must be in 1..=16".to_string(),
            });
        }
        if !(1..=KEY_BITS).contains(&min_level) {
            return Err(SketchError::InvalidParameter {
                param: "min_level".to_string(),
                value: min_level.to_string(),
                constraint: "must be in 1..=64".to_string(),
            });
        }
        if level_step == 0 {
            return Err(SketchError::InvalidParameter {
                param: "level_step".to_string(),
                value: "0".to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        let mut levels: Vec<u32> = (min_level..=KEY_BITS)
            .step_by(level_step as usize)
            .collect();
        if *levels.last().unwrap() != KEY_BITS {
            levels.push(KEY_BITS);
        }
        Ok(Self {
            bits: vec![false; num_bits],
            m: num_bits,
            k: num_hashes,
            levels,
        })
    }

    /// `prefix` of `key` at level `l` (the top `l` bits).
    fn prefix(key: u64, l: u32) -> u64 {
        if l >= KEY_BITS {
            key
        } else {
            key >> (KEY_BITS - l)
        }
    }

    /// Bit positions for the token `(prefix, level)`.
    fn positions(&self, prefix: u64, level: u32, out: &mut [usize]) {
        let base = xxhash(&prefix.to_le_bytes(), SEED ^ (level as u64));
        for (i, slot) in out.iter_mut().enumerate().take(self.k as usize) {
            let h = base
                .wrapping_add((i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
                .wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
            *slot = (h % self.m as u64) as usize;
        }
    }

    /// Whether the `(prefix, level)` token is present.
    fn token_present(&self, prefix: u64, level: u32) -> bool {
        let mut pos = [0usize; 16];
        self.positions(prefix, level, &mut pos);
        pos[..self.k as usize].iter().all(|&p| self.bits[p])
    }

    /// Inserts `key`, recording all its encoded dyadic prefixes.
    pub fn insert(&mut self, key: u64) {
        let mut pos = [0usize; 16];
        for &l in &self.levels {
            self.positions(Self::prefix(key, l), l, &mut pos);
            for &p in &pos[..self.k as usize] {
                self.bits[p] = true;
            }
        }
    }

    /// Point membership of `key` (top encoded level). Never a false negative.
    pub fn contains(&self, key: u64) -> bool {
        self.token_present(key, KEY_BITS)
    }

    /// Coarse-to-fine dyadic descent over the encoded levels.
    fn probe(&self, level_idx: usize, lo: u64, hi: u64) -> bool {
        let l = self.levels[level_idx];
        let shift = KEY_BITS - l; // l ≤ 64 ⇒ shift ≤ 63 for l ≥ 1
        let p_lo = lo >> shift;
        let p_hi = hi >> shift;
        let last = level_idx + 1 == self.levels.len();
        for p in p_lo..=p_hi {
            if !self.token_present(p, l) {
                continue; // subtree empty
            }
            if last {
                return true; // finest-level prefix present and overlapping
            }
            let region_lo = p << shift;
            let region_hi = region_lo | ((1u64 << shift) - 1);
            if self.probe(level_idx + 1, lo.max(region_lo), hi.min(region_hi)) {
                return true;
            }
        }
        false
    }

    /// Returns `true` if a key may lie in `[lo, hi]` (no false negatives), `false` if certainly empty.
    pub fn range_query(&self, lo: u64, hi: u64) -> bool {
        if lo > hi {
            return false;
        }
        self.probe(0, lo, hi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(BloomRf::new(0, 4, 16, 8).is_err());
        assert!(BloomRf::new(1024, 0, 16, 8).is_err());
        assert!(BloomRf::new(1024, 17, 16, 8).is_err());
        assert!(BloomRf::new(1024, 4, 0, 8).is_err());
        assert!(BloomRf::new(1024, 4, 16, 0).is_err());
        assert!(BloomRf::new(1024, 4, 16, 8).is_ok());
    }

    #[test]
    fn no_false_negatives_point_and_range() {
        let mut f = BloomRf::new(1 << 21, 4, 12, 8).unwrap();
        let keys: Vec<u64> = (0..50_000u64)
            .map(|i| i.wrapping_mul(7919) % 1_000_000)
            .collect();
        for &k in &keys {
            f.insert(k);
        }
        for &k in &keys {
            assert!(f.contains(k), "point miss {k}");
            assert!(f.range_query(k, k), "range miss {k}");
            assert!(
                f.range_query(k.saturating_sub(5), k + 5),
                "range-around miss {k}"
            );
        }
    }

    #[test]
    fn rules_out_empty_ranges() {
        let mut f = BloomRf::new(1 << 21, 6, 12, 8).unwrap();
        // Keys live in [0, 1_000_000).
        for i in 0..50_000u64 {
            f.insert(i * 17 % 1_000_000);
        }
        // Ranges far above the key space are certainly empty.
        let positives = (0..1_000u64)
            .filter(|i| {
                let lo = 10_000_000_000u64 + i * 7919;
                f.range_query(lo, lo + 100)
            })
            .count();
        assert!(
            positives < 50,
            "too many far-range false positives: {positives}"
        );
    }

    #[test]
    fn point_false_positive_rate_is_bounded() {
        let mut f = BloomRf::new(1 << 22, 6, 12, 8).unwrap();
        let n = 100_000u64;
        for i in 0..n {
            f.insert(i);
        }
        let trials = 200_000u64;
        let fps = (10_000_000..10_000_000 + trials)
            .filter(|&i| f.contains(i))
            .count();
        let fpr = fps as f64 / trials as f64;
        assert!(fpr < 0.05, "point FPR {fpr} too high");
    }
}
