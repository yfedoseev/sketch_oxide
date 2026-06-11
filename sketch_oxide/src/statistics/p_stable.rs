//! Indyk p-stable sketch — Lp-norm estimation by stable random projections.
//!
//! To estimate the `Lp` norm of a high-dimensional vector `x` that arrives as a stream of
//! coordinate updates, Indyk ("Stable Distributions, Pseudorandom Generators, Embeddings, and Data
//! Stream Computation", JACM 2006) projects `x` onto `d` random vectors whose entries are i.i.d.
//! draws from a **p-stable distribution**. The defining property of a p-stable distribution is
//! that `Σ_i a_i S_i` is distributed as `‖a‖_p · S` for i.i.d. stable `S_i, S` — so each projection
//! `c_j = Σ_i x_i S_{ij}` is exactly `‖x‖_p` times a standard stable variate. Taking the **median**
//! of `|c_j|` and dividing by the median of `|S|` recovers `‖x‖_p`, robustly.
//!
//! The two stable distributions with simple density are the only `p` supported here:
//! - **`p = 1`** → the standard **Cauchy** distribution (median of `|Cauchy|` = 1);
//! - **`p = 2`** → the standard **Gaussian** distribution (median of `|N(0,1)|` = Φ⁻¹(0.75)).
//!
//! General `p ∈ (0, 2]` via Chambers–Mallows–Stuck variates with a calibrated scale is a documented
//! follow-up. The sketch is **linear**, so it supports increments and decrements of any coordinate
//! and merges by adding projections.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

/// Median of `|N(0, 1)|` = Φ⁻¹(0.75); divides the L2 estimator.
const HALF_NORMAL_MEDIAN: f64 = 0.674_489_750_196_081_7;

/// An Indyk p-stable Lp-norm sketch over `d` stable projections.
///
/// # Example
/// ```
/// use sketch_oxide::statistics::PStableLpSketch;
///
/// // Estimate the L2 norm of a sparse vector given as coordinate updates.
/// let mut sk = PStableLpSketch::new(2.0, 512).unwrap();
/// for i in 0..100u64 { sk.update(i, 3.0); } // 100 coords each = 3 → ‖x‖₂ = sqrt(100*9) = 30
/// let est = sk.estimate();
/// assert!((est - 30.0).abs() < 0.2 * 30.0, "L2 estimate {est}");
/// ```
#[derive(Debug, Clone)]
pub struct PStableLpSketch {
    p: f64,
    seed: u64,
    /// Projections `c_j = Σ_i x_i · S_{ij}`.
    projections: Vec<f64>,
}

impl PStableLpSketch {
    /// Creates a sketch estimating the `Lp` norm with `d` projections. `p` must be `1.0` (L1, via
    /// Cauchy projections) or `2.0` (L2, via Gaussian projections).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `p` is not 1.0 or 2.0, or `d` is 0.
    pub fn new(p: f64, d: usize) -> Result<Self> {
        Self::with_seed(p, d, 0x1057_AB1E_5EED)
    }

    /// Like [`new`](Self::new) with an explicit projection seed.
    ///
    /// # Errors
    /// As [`new`](Self::new).
    pub fn with_seed(p: f64, d: usize, seed: u64) -> Result<Self> {
        if p != 1.0 && p != 2.0 {
            return Err(SketchError::InvalidParameter {
                param: "p".to_string(),
                value: p.to_string(),
                constraint: "must be 1.0 (L1/Cauchy) or 2.0 (L2/Gaussian)".to_string(),
            });
        }
        if d == 0 {
            return Err(SketchError::InvalidParameter {
                param: "d".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            p,
            seed,
            projections: vec![0.0; d],
        })
    }

    /// A uniform `(0, 1)` value deterministically derived from a coordinate, projection, and salt.
    #[inline]
    fn uniform(&self, coord: u64, j: usize, salt: u64) -> f64 {
        let h = xxhash(
            &coord.to_le_bytes(),
            self.seed ^ (j as u64).wrapping_mul(0x9E37_79B9) ^ salt,
        );
        // Map to (0, 1): use 53 mantissa bits, then nudge off the endpoints.
        let u = (h >> 11) as f64 / (1u64 << 53) as f64;
        u.clamp(f64::MIN_POSITIVE, 1.0 - f64::EPSILON)
    }

    /// The standard p-stable variate `S_{coord, j}` for this sketch's `p` (deterministic in the
    /// coordinate, so repeated updates to a coordinate use the same projection entry).
    #[inline]
    fn stable_variate(&self, coord: u64, j: usize) -> f64 {
        if self.p == 1.0 {
            // Standard Cauchy: tan(π (u − 1/2)).
            let u = self.uniform(coord, j, 1);
            (std::f64::consts::PI * (u - 0.5)).tan()
        } else {
            // Standard Gaussian via Box–Muller.
            let u1 = self.uniform(coord, j, 1);
            let u2 = self.uniform(coord, j, 2);
            (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
        }
    }

    /// Adds `delta` to coordinate `coord` (deletions allowed via negative `delta`).
    pub fn update(&mut self, coord: u64, delta: f64) {
        for j in 0..self.projections.len() {
            self.projections[j] += delta * self.stable_variate(coord, j);
        }
    }

    /// Estimates `‖x‖_p` as the median of `|c_j|` divided by the median of `|S|`.
    pub fn estimate(&self) -> f64 {
        let mut abs: Vec<f64> = self.projections.iter().map(|c| c.abs()).collect();
        abs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = {
            let n = abs.len();
            if n % 2 == 1 {
                abs[n / 2]
            } else {
                0.5 * (abs[n / 2 - 1] + abs[n / 2])
            }
        };
        // Divide by the median of |standard stable|: 1 for Cauchy, Φ⁻¹(0.75) for half-normal.
        let scale = if self.p == 1.0 {
            1.0
        } else {
            HALF_NORMAL_MEDIAN
        };
        median / scale
    }

    /// The norm order `p`.
    #[inline]
    pub fn p(&self) -> f64 {
        self.p
    }

    /// Number of projections.
    #[inline]
    pub fn width(&self) -> usize {
        self.projections.len()
    }

    /// Merges another sketch (adds projections). Both must share `p`, width, and seed.
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the configurations differ.
    pub fn merge(&mut self, other: &Self) -> Result<()> {
        if self.p != other.p
            || self.seed != other.seed
            || self.projections.len() != other.projections.len()
        {
            return Err(SketchError::IncompatibleSketches {
                reason: "p, seed, or width mismatch".to_string(),
            });
        }
        for (a, b) in self.projections.iter_mut().zip(&other.projections) {
            *a += *b;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(PStableLpSketch::new(0.5, 100).is_err());
        assert!(PStableLpSketch::new(2.0, 0).is_err());
        assert!(PStableLpSketch::new(1.0, 100).is_ok());
        assert!(PStableLpSketch::new(2.0, 100).is_ok());
    }

    #[test]
    fn l2_norm_of_uniform_vector() {
        let mut sk = PStableLpSketch::with_seed(2.0, 1024, 7).unwrap();
        for i in 0..400u64 {
            sk.update(i, 2.0); // 400 coords of 2 → ‖x‖₂ = sqrt(400*4) = 40
        }
        let est = sk.estimate();
        assert!((est - 40.0).abs() < 0.2 * 40.0, "L2 estimate {est}");
    }

    #[test]
    fn l1_norm_of_uniform_vector() {
        let mut sk = PStableLpSketch::with_seed(1.0, 2048, 11).unwrap();
        for i in 0..300u64 {
            sk.update(i, 5.0); // ‖x‖₁ = 300 * 5 = 1500
        }
        let est = sk.estimate();
        assert!((est - 1500.0).abs() < 0.25 * 1500.0, "L1 estimate {est}");
    }

    #[test]
    fn updates_are_linear_and_reversible() {
        let mut sk = PStableLpSketch::with_seed(2.0, 512, 3).unwrap();
        for i in 0..50u64 {
            sk.update(i, 1.0);
        }
        // Remove half the mass; estimate should drop accordingly.
        for i in 0..50u64 {
            sk.update(i, -1.0);
        }
        assert!(
            sk.estimate() < 1.0,
            "after full deletion estimate {}",
            sk.estimate()
        );
    }

    #[test]
    fn merge_adds_disjoint_supports() {
        // Two sketches over disjoint coordinate sets; merged L2 norm = sqrt of summed squares.
        let mut a = PStableLpSketch::with_seed(2.0, 1024, 42).unwrap();
        let mut b = PStableLpSketch::with_seed(2.0, 1024, 42).unwrap();
        for i in 0..100u64 {
            a.update(i, 3.0); // ‖·‖₂² = 900
        }
        for i in 100..200u64 {
            b.update(i, 4.0); // ‖·‖₂² = 1600
        }
        a.merge(&b).unwrap();
        let expected = (900.0f64 + 1600.0).sqrt(); // = 50
        let est = a.estimate();
        assert!(
            (est - expected).abs() < 0.2 * expected,
            "merged L2 {est} vs {expected}"
        );
    }

    #[test]
    fn merge_mismatch_errors() {
        let mut a = PStableLpSketch::with_seed(2.0, 256, 1).unwrap();
        let b = PStableLpSketch::with_seed(1.0, 256, 1).unwrap();
        assert!(a.merge(&b).is_err());
    }

    #[test]
    fn empty_estimate_is_zero() {
        let sk = PStableLpSketch::new(2.0, 128).unwrap();
        assert_eq!(sk.estimate(), 0.0);
    }
}
