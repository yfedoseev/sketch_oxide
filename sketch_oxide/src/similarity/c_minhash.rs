//! C-MinHash — circulant MinHash that reuses one permutation `K` times (Li & Li, ICML 2022).
//!
//! Classic MinHash needs `K` independent permutations (or hash functions) to produce a `K`-dimensional
//! signature for Jaccard estimation. **C-MinHash** rigorously reduces this to **two** permutations of
//! the universe `[D]`: an initial permutation `σ` shuffles the data to break any structure, and a
//! second permutation `π` is **re-used `K` times via circulant shifts** (`π`, `π→1`, `π→2`, …) to
//! produce the `K` hash values. Although the `K` hashes are correlated, the authors prove the Jaccard
//! estimator's variance is *strictly smaller* than classic MinHash's — better accuracy with far less
//! randomness.
//!
//! For a set `S ⊆ [D]`, the `k`-th hash is `min_{i∈S} π[(σ[i] − k) mod D]`. Two C-MinHash signatures
//! built with the **same `(D, K, seed)`** estimate Jaccard similarity by the fraction of matching
//! coordinates.

use crate::common::{Result, SketchError};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

/// A C-MinHash signature over a universe of size `D`.
///
/// # Example
/// ```
/// use sketch_oxide::similarity::CMinHash;
///
/// let (d, k, seed) = (4096, 400, 7);
/// let mut a = CMinHash::new(d, k, seed).unwrap();
/// let mut b = CMinHash::new(d, k, seed).unwrap();
/// for i in 0..2000u32 { a.add(i).unwrap(); }      // A = {0..2000}
/// for i in 1000..3000u32 { b.add(i).unwrap(); }   // B = {1000..3000}
/// // True Jaccard = |1000..2000| / |0..3000| = 1000/3000 ≈ 0.333.
/// let j = a.jaccard(&b).unwrap();
/// assert!((j - 0.333).abs() < 0.06, "jaccard {j}");
/// ```
#[derive(Debug, Clone)]
pub struct CMinHash {
    d: usize,
    k: usize,
    seed: u64,
    /// Initial permutation `σ` of `[D]`.
    sigma: Vec<u32>,
    /// Circulant permutation `π` of `[D]`.
    pi: Vec<u32>,
    /// Current per-coordinate minima (`d` = sentinel for "no element yet").
    mins: Vec<u32>,
}

impl CMinHash {
    /// Creates a signature over universe `[d]` with `k` hashes, deriving `σ` and `π` from `seed`.
    /// Signatures compared with [`jaccard`](Self::jaccard) must share the same `(d, k, seed)`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `d == 0`, `k == 0`, or `k > d`.
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
        let mut rng = SmallRng::seed_from_u64(seed);
        let sigma = random_permutation(d, &mut rng);
        let pi = random_permutation(d, &mut rng);
        Ok(Self {
            d,
            k,
            seed,
            sigma,
            pi,
            mins: vec![d as u32; k],
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
        let s = self.sigma[i as usize] as usize;
        for k in 0..self.k {
            // π circularly shifted right by k, evaluated at σ[i]: π[(s - k) mod D].
            let idx = (s + self.d - (k % self.d)) % self.d;
            let v = self.pi[idx];
            if v < self.mins[k] {
                self.mins[k] = v;
            }
        }
        Ok(())
    }

    /// Estimates the Jaccard similarity with `other` (the fraction of matching coordinates).
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the two signatures differ in `(d, k, seed)`.
    pub fn jaccard(&self, other: &Self) -> Result<f64> {
        if self.d != other.d || self.k != other.k || self.seed != other.seed {
            return Err(SketchError::IncompatibleSketches {
                reason: "CMinHash signatures must share the same (d, k, seed)".to_string(),
            });
        }
        let matches = self
            .mins
            .iter()
            .zip(&other.mins)
            .filter(|(a, b)| a == b)
            .count();
        Ok(matches as f64 / self.k as f64)
    }

    /// The signature (`k` minima).
    #[inline]
    pub fn signature(&self) -> &[u32] {
        &self.mins
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
        assert!(CMinHash::new(0, 10, 1).is_err());
        assert!(CMinHash::new(100, 0, 1).is_err());
        assert!(CMinHash::new(100, 101, 1).is_err());
        assert!(CMinHash::new(100, 50, 1).is_ok());
    }

    #[test]
    fn add_validates_range() {
        let mut h = CMinHash::new(64, 16, 1).unwrap();
        assert!(h.add(63).is_ok());
        assert!(h.add(64).is_err());
    }

    fn jaccard_of(d: usize, k: usize, a: std::ops::Range<u32>, b: std::ops::Range<u32>) -> f64 {
        let mut ha = CMinHash::new(d, k, 42).unwrap();
        let mut hb = CMinHash::new(d, k, 42).unwrap();
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
        // True Jaccard = |2000..4000| / |0..6000| = 2000/6000 ≈ 0.333.
        let j = jaccard_of(8192, 600, 0..4000, 2000..6000);
        assert!((j - 0.333).abs() < 0.06, "jaccard {j}");
    }

    #[test]
    fn identical_sets_are_one() {
        let j = jaccard_of(4096, 256, 0..2000, 0..2000);
        assert_eq!(j, 1.0);
    }

    #[test]
    fn disjoint_sets_are_near_zero() {
        let j = jaccard_of(8192, 400, 0..2000, 4000..6000);
        assert!(j < 0.03, "disjoint jaccard {j}");
    }

    #[test]
    fn rejects_mismatched_signatures() {
        let a = CMinHash::new(64, 16, 1).unwrap();
        let b = CMinHash::new(64, 16, 2).unwrap();
        assert!(a.jaccard(&b).is_err());
    }
}
