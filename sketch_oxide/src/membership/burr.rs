//! BuRR — Bumped Ribbon Retrieval, a near-optimal static membership filter.
//!
//! BuRR (Dillinger, Hübschle-Schneider, Sanders & Walzer, "Fast Succinct Retrieval and Approximate
//! Membership using Ribbon", SEA 2022) is a static approximate-membership filter that comes within
//! a few percent of the information-theoretic space bound (≈ `log2(1/fpr)` bits per key). It works
//! by solving a **banded linear system over GF(2)**: each key `x` contributes one equation
//!
//! ```text
//!     ⊕_{j : c(x)_j = 1}  Z[ s(x) + j ]  =  fingerprint(x)
//! ```
//!
//! where `s(x)` is a hashed band start, `c(x)` is a `w`-bit coefficient row (here `w = 64`, leading
//! bit forced to 1), and `Z` is the solution vector the filter stores. A query recomputes the same
//! row and checks whether the stored `Z` reproduces the key's fingerprint — a member always does
//! (no false negatives); a non-member matches only with probability `2^-r`.
//!
//! The "ribbon" structure makes the solve cheap: because every equation's support is a contiguous
//! `w`-wide band, Gaussian elimination is done **on the fly** with one pivot equation per row
//! position — no dense matrix. The **bumping** of BuRR is what reaches near-optimal density: rows
//! that cannot be placed (the band region is over-full) are *bumped* to a fallback layer rather
//! than forcing the whole construction to a lower load factor. Stacking a few ribbon layers this
//! way lets each layer run at high load with the small overflow handled by the next.
//!
//! # Design note
//!
//! This implements the genuine on-the-fly banded ribbon solve (insert → eliminate → back-substitute)
//! with bumping into recursively smaller ribbon layers, and an exact fingerprint set for the tiny
//! final residue — the clear, verifiable form of BuRR's contract. The published space-tuned refinements
//! (interleaved/​bit-packed storage of `Z` at exactly `r` bits per slot, and metadata-compressed
//! bumping thresholds) are layout optimizations over this same contract and are left as follow-ups;
//! they shrink bytes, not the set of keys accepted.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use std::collections::HashSet;

/// Ribbon width: each coefficient row spans 64 consecutive solution slots.
const W: usize = 64;
/// Target load factor per ribbon layer (slots ≈ keys / LOAD); the small overflow is bumped.
const LOAD: f64 = 0.90;
/// Number of stacked ribbon layers before the exact fallback catches the residue.
const MAX_LAYERS: usize = 4;

/// One solved ribbon layer.
#[derive(Debug, Clone)]
struct Layer {
    m: usize,
    seed: u64,
    /// Solution vector: `m` slots of `r`-bit fingerprints.
    z: Vec<u32>,
}

/// A Bumped Ribbon Retrieval approximate-membership filter (static; build once, query many).
///
/// # Example
/// ```
/// use sketch_oxide::membership::BurrFilter;
///
/// let keys: Vec<Vec<u8>> = (0..1000u32).map(|i| i.to_le_bytes().to_vec()).collect();
/// let filter = BurrFilter::build(&keys, 0.01).unwrap();
///
/// for k in &keys {
///     assert!(filter.contains(k)); // no false negatives
/// }
/// assert!(!filter.contains(b"definitely not a member"));
/// ```
#[derive(Debug, Clone)]
pub struct BurrFilter {
    r: u32,
    fp_mask: u32,
    layers: Vec<Layer>,
    /// Fingerprints of the residual keys that all ribbon layers bumped (exact, no false negatives).
    fallback: HashSet<u32>,
    fallback_seed: u64,
    len: usize,
}

impl BurrFilter {
    /// Builds a BuRR filter over `keys` targeting false-positive rate `fpr`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `fpr` is not in `(0, 1)`.
    pub fn build(keys: &[Vec<u8>], fpr: f64) -> Result<Self> {
        if !(fpr > 0.0 && fpr < 1.0) {
            return Err(SketchError::InvalidParameter {
                param: "fpr".to_string(),
                value: fpr.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }
        // Fingerprint width r ≈ log2(1/fpr), clamped to a u32 slot.
        let r = (-fpr.log2()).ceil().clamp(1.0, 32.0) as u32;
        let fp_mask = if r >= 32 { u32::MAX } else { (1u32 << r) - 1 };

        let len = keys.len();
        let mut layers = Vec::new();
        let mut remaining: Vec<&[u8]> = keys.iter().map(|k| k.as_slice()).collect();

        for layer_idx in 0..MAX_LAYERS {
            if remaining.is_empty() {
                break;
            }
            let seed = 0x51_7c_c1_b7 ^ (layer_idx as u64).wrapping_mul(0x9E37_79B9);
            let m = ((remaining.len() as f64 / LOAD).ceil() as usize).max(W);
            let (z, bumped) = Self::solve_layer(&remaining, m, seed, fp_mask);
            layers.push(Layer { m, seed, z });
            remaining = bumped;
        }

        // Whatever a few layers could not place goes into an exact fingerprint set.
        let fallback_seed = 0xB1_6B_00_B5;
        let fallback: HashSet<u32> = remaining
            .iter()
            .map(|k| Self::fingerprint(k, fallback_seed, fp_mask))
            .collect();

        Ok(Self {
            r,
            fp_mask,
            layers,
            fallback,
            fallback_seed,
            len,
        })
    }

    /// Tests membership. Returns `true` for every inserted key (no false negatives) and for a
    /// non-member only with probability ≈ `2^-r`.
    pub fn contains(&self, key: &[u8]) -> bool {
        for layer in &self.layers {
            let (s, coeff, fp) = Self::row(key, layer.seed, layer.m, self.fp_mask);
            if Self::eval_row(&layer.z, s, coeff) == fp {
                return true;
            }
        }
        self.fallback
            .contains(&Self::fingerprint(key, self.fallback_seed, self.fp_mask))
    }

    /// Number of keys the filter was built from.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the filter holds no keys.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Fingerprint width in bits (`r`); the false-positive rate is ≈ `2^-r`.
    #[inline]
    pub fn fingerprint_bits(&self) -> u32 {
        self.r
    }

    /// Number of stacked ribbon layers.
    #[inline]
    pub fn num_layers(&self) -> usize {
        self.layers.len()
    }

    /// Number of keys that landed in the exact fallback set (bumped from every ribbon layer).
    #[inline]
    pub fn fallback_len(&self) -> usize {
        self.fallback.len()
    }

    /// Approximate stored size in bits per key (the ribbon `Z` vectors plus the fallback).
    pub fn bits_per_key(&self) -> f64 {
        if self.len == 0 {
            return 0.0;
        }
        let z_bits: usize = self.layers.iter().map(|l| l.m * self.r as usize).sum();
        let fallback_bits = self.fallback.len() * 32;
        (z_bits + fallback_bits) as f64 / self.len as f64
    }

    // --- ribbon mechanics --------------------------------------------------------------------

    /// The `(start, coefficient, fingerprint)` row for a key in a layer of `m` slots.
    #[inline]
    fn row(key: &[u8], seed: u64, m: usize, fp_mask: u32) -> (usize, u64, u32) {
        let h1 = xxhash(key, seed.wrapping_mul(4).wrapping_add(1));
        let h2 = xxhash(key, seed.wrapping_mul(4).wrapping_add(2));
        let fp = Self::fingerprint(key, seed, fp_mask);
        // Band start in [0, m - W]; m >= W is guaranteed at construction.
        let start = (h1 % (m - W + 1) as u64) as usize;
        // Coefficient row: 64 random bits with the leading (lowest) bit forced to 1.
        let coeff = h2 | 1;
        (start, coeff, fp)
    }

    #[inline]
    fn fingerprint(key: &[u8], seed: u64, fp_mask: u32) -> u32 {
        (xxhash(key, seed.wrapping_mul(4).wrapping_add(3)) as u32) & fp_mask
    }

    /// XOR of the solution slots selected by `coeff`, anchored at `start`.
    #[inline]
    fn eval_row(z: &[u32], start: usize, coeff: u64) -> u32 {
        let mut acc = 0u32;
        let mut c = coeff;
        while c != 0 {
            let j = c.trailing_zeros() as usize;
            acc ^= z[start + j]; // start + j < m by the band invariant (start <= m - W, j < W)
            c &= c - 1;
        }
        acc
    }

    /// Solves one ribbon layer on the fly, returning the solution vector and the keys it bumped.
    fn solve_layer<'a>(
        keys: &[&'a [u8]],
        m: usize,
        seed: u64,
        fp_mask: u32,
    ) -> (Vec<u32>, Vec<&'a [u8]>) {
        // One pivot equation per row position: its coefficient (leading bit at this position) and rhs.
        let mut pivot_coeff = vec![0u64; m];
        let mut pivot_rhs = vec![0u32; m];
        let mut bumped = Vec::new();

        for &key in keys {
            let (start, coeff_full, fp) = Self::row(key, seed, m, fp_mask);
            if !Self::insert_row(start, coeff_full, fp, m, &mut pivot_coeff, &mut pivot_rhs) {
                bumped.push(key);
            }
        }

        // Back-substitution from high positions down; positions without a pivot are free (zero).
        let mut z = vec![0u32; m];
        for pos in (0..m).rev() {
            if pivot_coeff[pos] == 0 {
                continue;
            }
            let mut val = pivot_rhs[pos];
            let mut c = pivot_coeff[pos] & !1u64; // all but the leading bit
            while c != 0 {
                let j = c.trailing_zeros() as usize;
                val ^= z[pos + j];
                c &= c - 1;
            }
            z[pos] = val;
        }
        (z, bumped)
    }

    /// Inserts one equation by on-the-fly Gaussian elimination. Returns `false` if it cannot be
    /// placed (inconsistent → must be bumped).
    fn insert_row(
        start: usize,
        coeff_full: u64,
        fp: u32,
        m: usize,
        pivot_coeff: &mut [u64],
        pivot_rhs: &mut [u32],
    ) -> bool {
        let mut pos = start;
        let mut coeff = coeff_full;
        let mut rhs = fp;
        loop {
            if coeff == 0 {
                // Fully eliminated: consistent (0 = 0) → placed, else contradictory → bump.
                return rhs == 0;
            }
            let lead = coeff.trailing_zeros() as usize;
            pos += lead;
            coeff >>= lead; // leading 1 now at bit 0, i.e. at position `pos`
            if pos >= m {
                // Leading variable is off the end (a zero variable); same consistency test.
                return rhs == 0;
            }
            // Drop coefficient bits that reference slots past the end (those variables are zero).
            let span = m - pos;
            if span < 64 {
                coeff &= (1u64 << span) - 1;
            }
            if pivot_coeff[pos] == 0 {
                pivot_coeff[pos] = coeff;
                pivot_rhs[pos] = rhs;
                return true;
            }
            // Eliminate against the existing pivot at this position and continue.
            coeff ^= pivot_coeff[pos];
            rhs ^= pivot_rhs[pos];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keyset(n: u32) -> Vec<Vec<u8>> {
        (0..n).map(|i| i.to_le_bytes().to_vec()).collect()
    }

    #[test]
    fn rejects_bad_fpr() {
        assert!(BurrFilter::build(&keyset(10), 0.0).is_err());
        assert!(BurrFilter::build(&keyset(10), 1.0).is_err());
        assert!(BurrFilter::build(&keyset(10), 0.01).is_ok());
    }

    #[test]
    fn empty_filter() {
        let f = BurrFilter::build(&[], 0.01).unwrap();
        assert!(f.is_empty());
        assert!(!f.contains(b"anything"));
    }

    #[test]
    fn no_false_negatives_small() {
        let keys = keyset(100);
        let f = BurrFilter::build(&keys, 0.01).unwrap();
        for k in &keys {
            assert!(f.contains(k), "false negative for {k:?}");
        }
    }

    #[test]
    fn no_false_negatives_large() {
        // The real test of the ribbon solve + bumping: every one of many keys must be found.
        let keys = keyset(5000);
        let f = BurrFilter::build(&keys, 0.01).unwrap();
        for k in &keys {
            assert!(f.contains(k), "false negative for {k:?}");
        }
    }

    #[test]
    fn no_false_negatives_string_keys() {
        let keys: Vec<Vec<u8>> = (0..2000)
            .map(|i| format!("user::{i}").into_bytes())
            .collect();
        let f = BurrFilter::build(&keys, 0.001).unwrap();
        for k in &keys {
            assert!(f.contains(k));
        }
    }

    #[test]
    fn false_positive_rate_is_bounded() {
        let keys = keyset(4000);
        let f = BurrFilter::build(&keys, 0.01).unwrap(); // r ≈ 7 → fpr ≈ 0.8%
        let mut fp = 0;
        for i in 1_000_000u32..1_010_000 {
            if f.contains(&i.to_le_bytes()) {
                fp += 1;
            }
        }
        let rate = fp as f64 / 10_000.0;
        // Generous bound (target ~0.01); confirms the fingerprint check actually filters.
        assert!(rate < 0.05, "false-positive rate too high: {rate}");
    }

    #[test]
    fn duplicate_keys_are_consistent() {
        let mut keys = keyset(50);
        keys.extend(keyset(50)); // every key twice
        let f = BurrFilter::build(&keys, 0.01).unwrap();
        for k in &keyset(50) {
            assert!(f.contains(k));
        }
    }

    #[test]
    fn bumping_keeps_density_high() {
        // At 90% load most keys are placed in ribbon layers; only a small residue reaches the
        // exact fallback. This checks the bumping cascade actually converges.
        let keys = keyset(5000);
        let f = BurrFilter::build(&keys, 0.01).unwrap();
        assert!(f.num_layers() >= 1);
        assert!(
            f.fallback_len() < keys.len() / 10,
            "fallback held too many keys: {}",
            f.fallback_len()
        );
    }
}

// ---------------------------------------------------------------------------
// Capability-trait adoption (fable5 doc 01 F3): this is a build-once/immutable
// filter (no inherent `insert`), so it implements `Filter` but deliberately
// NOT `Update` — immutability is enforced by the type system.
// ---------------------------------------------------------------------------
use crate::common::capabilities::*;

impl Filter<[u8]> for BurrFilter {
    fn contains(&self, item: &[u8]) -> bool {
        BurrFilter::contains(self, item)
    }
}
