//! C-OPH — Circulant One Permutation Hashing (Li & Li, arXiv 2111.09544, 2021).
//!
//! One Permutation Hashing (OPH) draws a single permutation `σ` of the universe `[D]`, splits the
//! permuted coordinates into `K` equal bins, and takes the minimum non-zero index in each bin as that
//! bin's hash — far cheaper than classic MinHash's `K` permutations. Sparse data leaves some bins
//! **empty**, which *densification* fills from a non-empty bin. C-OPH carries the circulant idea of
//! [`CMinHash`](crate::similarity::CMinHash) into this framework: instead of an independent length-`D/K`
//! permutation per bin, it uses **one** small permutation `π` of length `D/K`, reused across bins by
//! **circulant shifts** (`π`, `π→1`, …). Its densification then *applies* the shifted `π` to the chosen
//! source bin rather than copying the value — the paper proves this attains the smallest Jaccard
//! estimation variance among all densified OPH schemes.
//!
//! For bin/hash slot `s` (`0..K`), the hash is `min over the bin's offsets o of π[(o − s) mod (D/K)]`,
//! lifted into the bin's disjoint value range by `+ source_bin · (D/K)` so values from different source
//! bins never collide accidentally. Two signatures built with the same `(D, K, seed)` estimate Jaccard
//! by the fraction of matching slots.
//!
//! # Implementation note
//!
//! Densification picks a non-empty *source* bin via a shared random bin permutation rotated per slot
//! (the standard rotation densification, shared across signatures so two sketches densify a slot from
//! the same probe order). Because the improved scheme applies `π_s` to the source bin, the per-bin
//! contents are retained and the signature is computed on demand.

use crate::common::{Result, SketchError};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::HashSet;

/// Sentinel for a slot that no bin could fill (only when the set is entirely empty).
const EMPTY: u64 = u64::MAX;

/// A C-OPH signature over a universe of size `D` with `K` bins (and `K` hash slots).
///
/// `D` must be a multiple of `K` (pad the universe with absent coordinates otherwise). For the
/// rigorous variance guarantee the paper assumes `K² ≤ D`; smaller `K` relative to `D` still works but
/// reuses circulant shifts.
///
/// # Example
/// ```
/// use sketch_oxide::similarity::COph;
///
/// let (d, k, seed) = (65536, 256, 7);
/// let mut a = COph::new(d, k, seed).unwrap();
/// let mut b = COph::new(d, k, seed).unwrap();
/// for i in 0..20_000u32 { a.add(i).unwrap(); }      // A = {0..20000}
/// for i in 10_000..30_000u32 { b.add(i).unwrap(); } // B = {10000..30000}
/// // True Jaccard = |10000..20000| / |0..30000| = 10000/30000 ≈ 0.333.
/// let j = a.jaccard(&b).unwrap();
/// assert!((j - 0.333).abs() < 0.07, "jaccard {j}");
/// ```
#[derive(Debug, Clone)]
pub struct COph {
    d: usize,
    k: usize,
    seed: u64,
    bin_size: usize,
    /// Initial permutation `σ` of `[D]`.
    sigma: Vec<u32>,
    /// Circulant small permutation `π` of `[D/K]`.
    pi: Vec<u32>,
    /// Shared random bin order for densification (a permutation of `[K]`).
    rho: Vec<u32>,
    /// Distinct elements added (universe indices).
    elements: HashSet<u32>,
}

impl COph {
    /// Creates a C-OPH sketch over universe `[d]` with `k` bins, deriving `σ`, `π` and the
    /// densification order from `seed`. Signatures compared with [`jaccard`](Self::jaccard) must share
    /// the same `(d, k, seed)`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `d == 0`, `k` is not in `1..=d`, or `d` is not a multiple
    /// of `k`.
    pub fn new(d: usize, k: usize, seed: u64) -> Result<Self> {
        if d == 0 {
            return Err(SketchError::InvalidParameter {
                param: "d".to_string(),
                value: "0".to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        if k == 0 || k > d {
            return Err(SketchError::InvalidParameter {
                param: "k".to_string(),
                value: k.to_string(),
                constraint: "must be in 1..=d".to_string(),
            });
        }
        if !d.is_multiple_of(k) {
            return Err(SketchError::InvalidParameter {
                param: "d".to_string(),
                value: d.to_string(),
                constraint: format!("must be a multiple of k = {k}"),
            });
        }
        let mut rng = SmallRng::seed_from_u64(seed);
        let bin_size = d / k;
        let sigma = random_permutation(d, &mut rng);
        let pi = random_permutation(bin_size, &mut rng);
        let rho = random_permutation(k, &mut rng);
        Ok(Self {
            d,
            k,
            seed,
            bin_size,
            sigma,
            pi,
            rho,
            elements: HashSet::new(),
        })
    }

    /// Adds element `i` (must be in `[0, d)`).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `i >= d`.
    pub fn add(&mut self, i: u32) -> Result<()> {
        if i as usize >= self.d {
            return Err(SketchError::InvalidParameter {
                param: "i".to_string(),
                value: i.to_string(),
                constraint: format!("must be < d = {}", self.d),
            });
        }
        self.elements.insert(i);
        Ok(())
    }

    /// `π` circularly shifted right by `s`, evaluated at within-bin offset `o`: `π[(o − s) mod bin]`.
    fn pi_shift(&self, o: usize, s: usize) -> u32 {
        let shift = s % self.bin_size;
        self.pi[(o + self.bin_size - shift) % self.bin_size]
    }

    /// Minimum shifted-`π` value over a bin's offsets, lifted into the bin's disjoint range.
    fn bin_hash(&self, offsets: &[u32], source_bin: usize, slot: usize) -> u64 {
        let m = offsets
            .iter()
            .map(|&o| self.pi_shift(o as usize, slot))
            .min()
            .unwrap_or(0);
        m as u64 + (source_bin as u64) * (self.bin_size as u64)
    }

    /// Computes the `K`-slot densified C-OPH signature.
    pub fn signature(&self) -> Vec<u64> {
        // Group elements into bins by their permuted within-bin offset.
        let mut bins: Vec<Vec<u32>> = vec![Vec::new(); self.k];
        for &i in &self.elements {
            let p = self.sigma[i as usize] as usize;
            bins[p / self.bin_size].push((p % self.bin_size) as u32);
        }

        let mut sig = vec![EMPTY; self.k];
        // First scan: slot s hashes its own bin s with shift s.
        for s in 0..self.k {
            if !bins[s].is_empty() {
                sig[s] = self.bin_hash(&bins[s], s, s);
            }
        }
        // Densification: fill each empty slot from a non-empty source bin (shared rotated order),
        // applying the slot's own circulant shift to that bin.
        for (s, slot) in sig.iter_mut().enumerate() {
            if *slot == EMPTY {
                for t in 0..self.k {
                    let b = self.rho[(s + t) % self.k] as usize;
                    if !bins[b].is_empty() {
                        *slot = self.bin_hash(&bins[b], b, s);
                        break;
                    }
                }
            }
        }
        sig
    }

    /// Estimates the Jaccard similarity with `other` (the fraction of matching slots).
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the two sketches differ in `(d, k, seed)`.
    pub fn jaccard(&self, other: &Self) -> Result<f64> {
        if self.d != other.d || self.k != other.k || self.seed != other.seed {
            return Err(SketchError::IncompatibleSketches {
                reason: "COph sketches must share the same (d, k, seed)".to_string(),
            });
        }
        let a = self.signature();
        let b = other.signature();
        let matches = a
            .iter()
            .zip(&b)
            .filter(|(x, y)| x == y && **x != EMPTY)
            .count();
        Ok(matches as f64 / self.k as f64)
    }

    /// Number of bins / hash slots (the signature length).
    #[inline]
    pub fn num_bins(&self) -> usize {
        self.k
    }
}

/// A uniformly random permutation of `[n]` via Fisher–Yates.
fn random_permutation(n: usize, rng: &mut SmallRng) -> Vec<u32> {
    let mut p: Vec<u32> = (0..n as u32).collect();
    for i in (1..n).rev() {
        let j = rng.random_range(0..=i);
        p.swap(i, j);
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(COph::new(0, 4, 1).is_err());
        assert!(COph::new(100, 0, 1).is_err());
        assert!(COph::new(100, 101, 1).is_err());
        assert!(COph::new(100, 7, 1).is_err()); // 100 not a multiple of 7
        assert!(COph::new(100, 10, 1).is_ok());
    }

    #[test]
    fn add_validates_range() {
        let mut h = COph::new(64, 8, 1).unwrap();
        assert!(h.add(63).is_ok());
        assert!(h.add(64).is_err());
    }

    fn jaccard_of(d: usize, k: usize, a: std::ops::Range<u32>, b: std::ops::Range<u32>) -> f64 {
        let mut ha = COph::new(d, k, 42).unwrap();
        let mut hb = COph::new(d, k, 42).unwrap();
        for i in a {
            ha.add(i).unwrap();
        }
        for i in b {
            hb.add(i).unwrap();
        }
        ha.jaccard(&hb).unwrap()
    }

    #[test]
    fn estimates_jaccard() {
        // True Jaccard = |10000..20000| / |0..30000| = 10000/30000 ≈ 0.333.
        let j = jaccard_of(65536, 256, 0..20_000, 10_000..30_000);
        assert!((j - 0.333).abs() < 0.07, "jaccard {j}");
    }

    #[test]
    fn identical_sets_are_one() {
        let j = jaccard_of(8192, 128, 0..4000, 0..4000);
        assert_eq!(j, 1.0);
    }

    #[test]
    fn disjoint_sets_are_near_zero() {
        let j = jaccard_of(65536, 256, 0..10_000, 30_000..40_000);
        assert!(j < 0.05, "disjoint jaccard {j}");
    }

    #[test]
    fn densification_fills_sparse_sets() {
        // Far fewer elements than bins => many empty bins exercised by densification. Identical sparse
        // sets must still estimate Jaccard 1.0, and the signature must be fully filled (no sentinels).
        let mut a = COph::new(4096, 64, 9).unwrap();
        let mut b = COph::new(4096, 64, 9).unwrap();
        for i in (0..50u32).map(|i| i * 79 % 4096) {
            a.add(i).unwrap();
            b.add(i).unwrap();
        }
        assert!(a.signature().iter().all(|&v| v != EMPTY));
        assert_eq!(a.jaccard(&b).unwrap(), 1.0);
    }

    #[test]
    fn rejects_mismatched_signatures() {
        let a = COph::new(64, 8, 1).unwrap();
        let b = COph::new(64, 8, 2).unwrap();
        assert!(a.jaccard(&b).is_err());
    }
}
