//! Extended RaBitQ — 1-bit vector quantization with an unbiased inner-product
//! estimator (Gao & Long, SIGMOD 2024/2025; [arXiv:2409.09913]).
//!
//! # Idea
//!
//! Each `D`-dimensional vector is normalized to the unit sphere, rotated by a
//! shared random orthonormal matrix `P`, and quantized to **one sign bit per
//! dimension**. Naively quantizing to signs is biased and axis-dependent; the
//! random rotation is what makes the sign code an (approximately) unbiased,
//! rotation-invariant estimator of direction — that is the RaBitQ insight.
//!
//! For a stored code `x̄ = sign(P·o)/√D` (unit-norm bi-valued vector) with the
//! per-vector scalar `norm_factor = ⟨x̄, P·o⟩ = ‖P·o‖₁/√D`, and a query rotated
//! to `w = P·q̂`, the estimator of the unit-vector inner product is
//!
//! ```text
//! ⟨o, q̂⟩ ≈ ⟨x̄, w⟩ / norm_factor
//! ```
//!
//! which recovers the true inner product because `⟨x̄, w⟩ ≈ cosθ·⟨o,q̂⟩` and
//! `norm_factor = cosθ` (θ = angle between the code and the rotated vector); the
//! random rotation drives the residual to mean zero with `O(1/√D)` deviation.
//! Full-precision magnitudes are restored by scaling with the stored norms.
//!
//! # Status
//!
//! This is a correct, statistically-validated reference implementation using a
//! dense `D×D` rotation (generated once and shared by all codes). The paper's
//! fast in-place transform (random sign flips + Fast Hadamard) is a follow-on
//! optimization that changes only the rotation cost, not the estimator.

use crate::common::SketchError;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

/// A RaBitQ quantizer: holds the shared random orthonormal rotation for a fixed
/// dimensionality. Encode vectors into [`RaBitQCode`]s, then estimate inner
/// products / L2 distances against full-precision queries.
#[derive(Clone, Debug)]
pub struct RaBitQ {
    dim: usize,
    /// Row-major `dim × dim` orthonormal rotation matrix `P`.
    rotation: Vec<f64>,
    inv_sqrt_d: f64,
}

/// The compact code for one vector: a sign bit per dimension plus the two
/// scalars needed to reconstruct magnitudes and debias the estimate.
#[derive(Clone, Debug, PartialEq)]
pub struct RaBitQCode {
    /// Packed sign bits of `P·o` (bit `i` set ⇒ component `i` ≥ 0).
    bits: Vec<u64>,
    /// `⟨x̄, P·o⟩ = ‖P·o‖₁ / √D` — the debiasing factor (0 for the zero vector).
    norm_factor: f64,
    /// Euclidean norm of the original (pre-normalization) vector.
    norm: f64,
}

impl RaBitQ {
    /// Creates a quantizer for `dim`-dimensional vectors with a random rotation
    /// derived deterministically from `seed`.
    ///
    /// # Errors
    /// Returns `InvalidParameter` if `dim` is 0.
    pub fn new(dim: usize, seed: u64) -> Result<Self, SketchError> {
        if dim == 0 {
            return Err(SketchError::InvalidParameter {
                param: "dim".to_string(),
                value: "0".to_string(),
                constraint: "must be greater than 0".to_string(),
            });
        }
        let mut rng = SmallRng::seed_from_u64(seed);
        let rotation = random_orthonormal(dim, &mut rng);
        Ok(Self {
            dim,
            rotation,
            inv_sqrt_d: 1.0 / (dim as f64).sqrt(),
        })
    }

    /// The dimensionality this quantizer operates on.
    #[must_use]
    pub fn dim(&self) -> usize {
        self.dim
    }

    /// Number of bytes a single code occupies (1 bit per dimension + 2 scalars).
    #[must_use]
    pub fn code_bytes(&self) -> usize {
        self.dim.div_ceil(64) * 8 + 16
    }

    /// Applies `P` to `v`, returning `P·v`.
    fn rotate(&self, v: &[f64]) -> Vec<f64> {
        let d = self.dim;
        let mut out = vec![0.0; d];
        for (i, o) in out.iter_mut().enumerate() {
            let row = &self.rotation[i * d..i * d + d];
            *o = row.iter().zip(v).map(|(&p, &x)| p * x).sum();
        }
        out
    }

    /// Encodes a vector into its 1-bit RaBitQ code.
    ///
    /// # Errors
    /// Returns `InvalidParameter` if `v.len()` does not match [`dim`](Self::dim).
    pub fn encode(&self, v: &[f64]) -> Result<RaBitQCode, SketchError> {
        if v.len() != self.dim {
            return Err(SketchError::InvalidParameter {
                param: "vector length".to_string(),
                value: v.len().to_string(),
                constraint: format!("must equal dim {}", self.dim),
            });
        }
        let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();

        let mut bits = vec![0u64; self.dim.div_ceil(64)];
        if norm == 0.0 {
            // Zero vector: no direction; leave bits zero, norm_factor 0.
            return Ok(RaBitQCode {
                bits,
                norm_factor: 0.0,
                norm: 0.0,
            });
        }

        // Rotate the unit vector and take component signs.
        let mut abs_sum = 0.0;
        let inv_norm = 1.0 / norm;
        let rotated = self.rotate(v); // linear, so P(v/‖v‖) = (P v)/‖v‖
        for (i, &yi_scaled) in rotated.iter().enumerate() {
            let yi = yi_scaled * inv_norm; // component of P·o
            abs_sum += yi.abs();
            if yi >= 0.0 {
                bits[i / 64] |= 1u64 << (i % 64);
            }
        }
        let norm_factor = abs_sum * self.inv_sqrt_d;

        Ok(RaBitQCode {
            bits,
            norm_factor,
            norm,
        })
    }

    /// Estimates the inner product `⟨v, q⟩` between the vector behind `code` and
    /// a full-precision query `q`.
    ///
    /// # Errors
    /// Returns `InvalidParameter` if `q.len()` does not match [`dim`](Self::dim).
    pub fn estimate_inner_product(&self, code: &RaBitQCode, q: &[f64]) -> Result<f64, SketchError> {
        if q.len() != self.dim {
            return Err(SketchError::InvalidParameter {
                param: "query length".to_string(),
                value: q.len().to_string(),
                constraint: format!("must equal dim {}", self.dim),
            });
        }
        let qnorm = q.iter().map(|x| x * x).sum::<f64>().sqrt();
        if qnorm == 0.0 || code.norm == 0.0 || code.norm_factor == 0.0 {
            return Ok(0.0);
        }

        // w = P·q̂
        let inv_qnorm = 1.0 / qnorm;
        let w = self.rotate(q);

        // ⟨x̄, w⟩ = (1/√D) Σ x_i w_i, with x_i = ±1 from the sign bits.
        let mut acc = 0.0;
        for (i, &wi_scaled) in w.iter().enumerate() {
            let wi = wi_scaled * inv_qnorm; // component of P·q̂
            let bit = (code.bits[i / 64] >> (i % 64)) & 1;
            if bit == 1 {
                acc += wi;
            } else {
                acc -= wi;
            }
        }
        let x_dot_w = acc * self.inv_sqrt_d;

        // ⟨o, q̂⟩ ≈ ⟨x̄, w⟩ / norm_factor, then rescale by the two norms.
        let unit_ip = x_dot_w / code.norm_factor;
        Ok(unit_ip * code.norm * qnorm)
    }

    /// Estimates the squared Euclidean distance `‖v − q‖²`.
    ///
    /// # Errors
    /// Returns `InvalidParameter` if `q.len()` does not match [`dim`](Self::dim).
    pub fn estimate_l2_squared(&self, code: &RaBitQCode, q: &[f64]) -> Result<f64, SketchError> {
        let qnorm_sq = q.iter().map(|x| x * x).sum::<f64>();
        let ip = self.estimate_inner_product(code, q)?;
        Ok((code.norm * code.norm + qnorm_sq - 2.0 * ip).max(0.0))
    }
}

impl RaBitQCode {
    /// Euclidean norm of the encoded vector.
    #[must_use]
    pub fn norm(&self) -> f64 {
        self.norm
    }
}

/// Generates a `d × d` orthonormal matrix (row-major) by Gram–Schmidt
/// orthonormalization of a Gaussian random matrix.
fn random_orthonormal(d: usize, rng: &mut SmallRng) -> Vec<f64> {
    // Fill rows with independent standard-normal entries (Box–Muller).
    let mut m = vec![0.0f64; d * d];
    for slot in m.iter_mut() {
        *slot = standard_normal(rng);
    }
    // Modified Gram–Schmidt over rows.
    for i in 0..d {
        // Subtract projections onto previously-finalized rows.
        for j in 0..i {
            let dot = row_dot(&m, d, i, j);
            for k in 0..d {
                m[i * d + k] -= dot * m[j * d + k];
            }
        }
        // Normalize row i.
        let norm = (0..d).map(|k| m[i * d + k].powi(2)).sum::<f64>().sqrt();
        // A Gaussian matrix is full-rank with probability 1; guard anyway.
        let inv = if norm > 1e-12 { 1.0 / norm } else { 0.0 };
        for k in 0..d {
            m[i * d + k] *= inv;
        }
    }
    m
}

fn row_dot(m: &[f64], d: usize, i: usize, j: usize) -> f64 {
    (0..d).map(|k| m[i * d + k] * m[j * d + k]).sum()
}

/// One standard-normal sample via Box–Muller.
fn standard_normal(rng: &mut SmallRng) -> f64 {
    // u1 in (0,1] to keep ln finite.
    let u1: f64 = 1.0 - rng.random::<f64>();
    let u2: f64 = rng.random::<f64>();
    (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dot(a: &[f64], b: &[f64]) -> f64 {
        a.iter().zip(b).map(|(x, y)| x * y).sum()
    }

    fn rms(xs: &[f64]) -> f64 {
        (xs.iter().map(|x| x * x).sum::<f64>() / xs.len() as f64).sqrt()
    }

    #[test]
    fn rejects_bad_dims() {
        assert!(RaBitQ::new(0, 1).is_err());
        let q = RaBitQ::new(8, 1).unwrap();
        assert!(q.encode(&[0.0; 4]).is_err());
        let code = q.encode(&[1.0; 8]).unwrap();
        assert!(q.estimate_inner_product(&code, &[0.0; 4]).is_err());
    }

    #[test]
    fn zero_vectors_estimate_zero() {
        let q = RaBitQ::new(16, 7).unwrap();
        let code = q.encode(&[0.0; 16]).unwrap();
        assert_eq!(q.estimate_inner_product(&code, &[1.0; 16]).unwrap(), 0.0);
        let nonzero = q.encode(&[1.0; 16]).unwrap();
        assert_eq!(q.estimate_inner_product(&nonzero, &[0.0; 16]).unwrap(), 0.0);
    }

    #[test]
    fn rotation_is_orthonormal() {
        // Rows must be orthonormal: R·Rᵀ = I.
        let d = 32;
        let q = RaBitQ::new(d, 123).unwrap();
        for i in 0..d {
            for j in 0..d {
                let dot: f64 = (0..d)
                    .map(|k| q.rotation[i * d + k] * q.rotation[j * d + k])
                    .sum();
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((dot - expected).abs() < 1e-9, "R Rᵀ[{i}][{j}]={dot}");
            }
        }
    }

    #[test]
    fn inner_product_estimator_tracks_truth_within_error_bound() {
        // Over many random unit-vector pairs, the estimated inner product must
        // track the true one with O(1/√D) RMSE and near-zero bias — the RaBitQ
        // guarantee. A miscoded estimator would fail this decisively.
        let d = 256;
        let rq = RaBitQ::new(d, 42).unwrap();
        let mut rng = SmallRng::seed_from_u64(999);

        let mut errors = Vec::new();
        for _ in 0..400 {
            let o: Vec<f64> = (0..d).map(|_| standard_normal(&mut rng)).collect();
            let query: Vec<f64> = (0..d).map(|_| standard_normal(&mut rng)).collect();
            let code = rq.encode(&o).unwrap();
            let est = rq.estimate_inner_product(&code, &query).unwrap();
            let truth = dot(&o, &query);
            // Normalize the error by the vector magnitudes so the bound is scale-free.
            let scale = code.norm() * query.iter().map(|x| x * x).sum::<f64>().sqrt();
            errors.push((est - truth) / scale);
        }

        let rmse = rms(&errors);
        let bias = errors.iter().sum::<f64>() / errors.len() as f64;
        // Theory: normalized error ~ 1/√D ≈ 0.0625 for D=256. Allow generous slack.
        assert!(
            rmse < 0.12,
            "RaBitQ inner-product RMSE {rmse} exceeds bound"
        );
        assert!(
            bias.abs() < 0.03,
            "RaBitQ inner-product bias {bias} too large"
        );
    }

    #[test]
    fn l2_distance_estimator_is_accurate() {
        let d = 256;
        let rq = RaBitQ::new(d, 5).unwrap();
        let mut rng = SmallRng::seed_from_u64(2024);

        let mut rel_errors = Vec::new();
        for _ in 0..200 {
            let o: Vec<f64> = (0..d).map(|_| standard_normal(&mut rng)).collect();
            let query: Vec<f64> = (0..d).map(|_| standard_normal(&mut rng)).collect();
            let code = rq.encode(&o).unwrap();
            let est = rq.estimate_l2_squared(&code, &query).unwrap();
            let truth: f64 = o.iter().zip(&query).map(|(a, b)| (a - b).powi(2)).sum();
            rel_errors.push((est - truth) / truth);
        }
        let rmse = rms(&rel_errors);
        assert!(rmse < 0.12, "RaBitQ L2 relative RMSE {rmse} too large");
    }

    #[test]
    fn code_is_compact() {
        let rq = RaBitQ::new(1024, 1).unwrap();
        // 1024 bits = 128 bytes + 16 bytes of scalars, vs 1024*8 = 8192 bytes raw.
        assert_eq!(rq.code_bytes(), 128 + 16);
    }
}
