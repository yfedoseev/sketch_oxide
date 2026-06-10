//! Johnson–Lindenstrauss random projection — distance-preserving dimensionality reduction.
//!
//! The Johnson–Lindenstrauss lemma says a set of points in high dimension can be embedded
//! into `O(log n / ε²)` dimensions while preserving all pairwise distances (and inner
//! products) within a factor `1 ± ε`. A random `±1` (Rademacher) projection achieves it: each
//! output coordinate is a scaled signed sum of the input coordinates. This implementation
//! generates the projection signs on the fly from a seeded hash, so no `output × input`
//! matrix is stored — only the seed.
//!
//! Used to shrink embeddings before nearest-neighbour search, and as the sketching step in
//! sketch-and-solve linear algebra.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

/// A seeded Johnson–Lindenstrauss projection from `input_dim` to `output_dim`.
///
/// # Example
/// ```
/// use sketch_oxide::matrix::JohnsonLindenstrauss;
///
/// let jl = JohnsonLindenstrauss::new(100, 2000, 42).unwrap();
/// let x: Vec<f64> = (0..100).map(|i| (i as f64).sin()).collect();
/// let px = jl.project(&x).unwrap();
/// assert_eq!(px.len(), 2000);
/// ```
#[derive(Debug, Clone)]
pub struct JohnsonLindenstrauss {
    input_dim: usize,
    output_dim: usize,
    seed: u64,
    scale: f64,
}

impl JohnsonLindenstrauss {
    /// Creates a projection to `output_dim` dimensions with the given RNG `seed`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `input_dim` or `output_dim` is 0.
    pub fn new(input_dim: usize, output_dim: usize, seed: u64) -> Result<Self> {
        if input_dim == 0 || output_dim == 0 {
            return Err(SketchError::InvalidParameter {
                param: if input_dim == 0 {
                    "input_dim"
                } else {
                    "output_dim"
                }
                .to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            input_dim,
            output_dim,
            seed,
            // 1/sqrt(k) keeps the projection an isometry in expectation.
            scale: 1.0 / (output_dim as f64).sqrt(),
        })
    }

    /// The `±1` projection sign for output row `i`, input column `j`.
    #[inline]
    fn sign(&self, i: usize, j: usize) -> f64 {
        let mut buf = [0u8; 16];
        buf[..8].copy_from_slice(&(i as u64).to_le_bytes());
        buf[8..].copy_from_slice(&(j as u64).to_le_bytes());
        if xxhash(&buf, self.seed) & 1 == 0 {
            1.0
        } else {
            -1.0
        }
    }

    /// Projects `x` (length `input_dim`) into `output_dim` dimensions.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `x.len() != input_dim`.
    pub fn project(&self, x: &[f64]) -> Result<Vec<f64>> {
        if x.len() != self.input_dim {
            return Err(SketchError::InvalidParameter {
                param: "x".to_string(),
                value: format!("len {}", x.len()),
                constraint: format!("must have length {}", self.input_dim),
            });
        }
        let out = (0..self.output_dim)
            .map(|i| {
                let dot: f64 = x
                    .iter()
                    .enumerate()
                    .map(|(j, &xj)| self.sign(i, j) * xj)
                    .sum();
                dot * self.scale
            })
            .collect();
        Ok(out)
    }

    /// Input dimension.
    #[inline]
    pub fn input_dim(&self) -> usize {
        self.input_dim
    }

    /// Output (reduced) dimension.
    #[inline]
    pub fn output_dim(&self) -> usize {
        self.output_dim
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dot(a: &[f64], b: &[f64]) -> f64 {
        a.iter().zip(b).map(|(x, y)| x * y).sum()
    }

    #[test]
    fn rejects_zero_dims() {
        assert!(JohnsonLindenstrauss::new(0, 10, 1).is_err());
        assert!(JohnsonLindenstrauss::new(10, 0, 1).is_err());
        assert!(JohnsonLindenstrauss::new(10, 10, 1).is_ok());
    }

    #[test]
    fn output_has_correct_dimension() {
        let jl = JohnsonLindenstrauss::new(50, 500, 7).unwrap();
        let x = vec![1.0; 50];
        assert_eq!(jl.project(&x).unwrap().len(), 500);
    }

    #[test]
    fn wrong_input_length_errors() {
        let jl = JohnsonLindenstrauss::new(50, 500, 7).unwrap();
        assert!(jl.project(&vec![1.0; 49]).is_err());
    }

    #[test]
    fn preserves_norm_approximately() {
        let jl = JohnsonLindenstrauss::new(200, 4000, 13).unwrap();
        let x: Vec<f64> = (0..200).map(|i| ((i * 7 % 13) as f64) - 6.0).collect();
        let true_norm_sq = dot(&x, &x);
        let px = jl.project(&x).unwrap();
        let proj_norm_sq = dot(&px, &px);
        let rel = (proj_norm_sq - true_norm_sq).abs() / true_norm_sq;
        assert!(rel < 0.1, "norm preservation rel error {rel}");
    }

    #[test]
    fn preserves_inner_product_approximately() {
        let jl = JohnsonLindenstrauss::new(150, 5000, 99).unwrap();
        let x: Vec<f64> = (0..150).map(|i| (i as f64).cos()).collect();
        let y: Vec<f64> = (0..150).map(|i| (i as f64 * 0.3).sin()).collect();
        let true_ip = dot(&x, &y);
        let proj_ip = dot(&jl.project(&x).unwrap(), &jl.project(&y).unwrap());
        assert!(
            (proj_ip - true_ip).abs() < 0.15 * true_ip.abs().max(1.0),
            "inner product {proj_ip} vs true {true_ip}"
        );
    }

    #[test]
    fn deterministic_for_same_seed() {
        let a = JohnsonLindenstrauss::new(20, 100, 5).unwrap();
        let b = JohnsonLindenstrauss::new(20, 100, 5).unwrap();
        let x: Vec<f64> = (0..20).map(|i| i as f64).collect();
        assert_eq!(a.project(&x).unwrap(), b.project(&x).unwrap());
    }
}
