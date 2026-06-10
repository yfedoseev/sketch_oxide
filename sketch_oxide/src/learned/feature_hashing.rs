//! Feature hashing (the "hashing trick") — fixed-size feature vectorization.
//!
//! Feature hashing (Weinberger et al., "Feature Hashing for Large Scale Multitask Learning",
//! ICML 2009) maps an unbounded, string-keyed feature space into a fixed-dimension vector
//! without storing a vocabulary: each feature hashes to a coordinate and a `±1` sign, and its
//! value is added there. The signed hash makes collisions cancel in expectation, so inner
//! products are preserved in expectation — the standard input layer for online learning
//! (Vowpal Wabbit) and the scikit-learn `FeatureHasher`.
//!
//! It is `CountSketch` arithmetic applied to vectorization rather than frequency estimation.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

/// A feature hasher producing `dim`-dimensional vectors.
///
/// # Example
/// ```
/// use sketch_oxide::learned::FeatureHasher;
///
/// let mut h = FeatureHasher::new(1024).unwrap();
/// h.add(b"color=blue", 1.0);
/// h.add(b"size=large", 2.0);
/// h.add(b"color=blue", 1.0); // same feature accumulates
/// let v = h.vector();
/// assert_eq!(v.len(), 1024);
/// ```
#[derive(Debug, Clone)]
pub struct FeatureHasher {
    dim: usize,
    vector: Vec<f64>,
}

impl FeatureHasher {
    /// Creates a hasher producing vectors of dimension `dim`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `dim` is 0.
    pub fn new(dim: usize) -> Result<Self> {
        if dim == 0 {
            return Err(SketchError::InvalidParameter {
                param: "dim".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            dim,
            vector: vec![0.0; dim],
        })
    }

    /// Coordinate and `±1` sign of `feature`.
    #[inline]
    fn coord_sign(&self, feature: &[u8]) -> (usize, f64) {
        let h = xxhash(feature, 0);
        let coord = (h % self.dim as u64) as usize;
        let sign = if xxhash(feature, 1) & 1 == 0 {
            1.0
        } else {
            -1.0
        };
        (coord, sign)
    }

    /// Adds `value` for `feature` into the vector (signed, accumulating).
    pub fn add(&mut self, feature: &[u8], value: f64) {
        let (coord, sign) = self.coord_sign(feature);
        self.vector[coord] += sign * value;
    }

    /// The current feature vector.
    #[inline]
    pub fn vector(&self) -> &[f64] {
        &self.vector
    }

    /// Consumes the hasher and returns the vector.
    pub fn into_vector(self) -> Vec<f64> {
        self.vector
    }

    /// Resets all coordinates to zero (keeps `dim`).
    pub fn reset(&mut self) {
        self.vector.iter_mut().for_each(|x| *x = 0.0);
    }

    /// Output dimension.
    #[inline]
    pub fn dim(&self) -> usize {
        self.dim
    }

    /// One-shot transform of `(feature, value)` pairs into a `dim`-vector.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `dim` is 0.
    pub fn transform<'a, I>(dim: usize, features: I) -> Result<Vec<f64>>
    where
        I: IntoIterator<Item = (&'a [u8], f64)>,
    {
        let mut h = Self::new(dim)?;
        for (f, v) in features {
            h.add(f, v);
        }
        Ok(h.into_vector())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dot(a: &[f64], b: &[f64]) -> f64 {
        a.iter().zip(b).map(|(x, y)| x * y).sum()
    }

    #[test]
    fn rejects_zero_dim() {
        assert!(FeatureHasher::new(0).is_err());
        assert!(FeatureHasher::new(64).is_ok());
    }

    #[test]
    fn same_feature_accumulates() {
        let mut h = FeatureHasher::new(1024).unwrap();
        h.add(b"x", 1.0);
        h.add(b"x", 1.0);
        h.add(b"x", 1.0);
        // The coordinate for "x" holds ±3.
        let total: f64 = h.vector().iter().map(|v| v.abs()).sum();
        assert!((total - 3.0).abs() < 1e-9, "total magnitude {total}");
    }

    #[test]
    fn deterministic_coordinates() {
        let v1 = FeatureHasher::transform(256, [(b"a".as_slice(), 1.0), (b"b".as_slice(), 2.0)])
            .unwrap();
        let v2 = FeatureHasher::transform(256, [(b"a".as_slice(), 1.0), (b"b".as_slice(), 2.0)])
            .unwrap();
        assert_eq!(v1, v2);
    }

    #[test]
    fn preserves_inner_product_in_expectation() {
        // Two documents sharing some features: hashed inner product ≈ true inner product.
        let dim = 8192;
        let doc_a: Vec<(&[u8], f64)> =
            vec![(b"the", 3.0), (b"cat", 1.0), (b"sat", 1.0), (b"mat", 1.0)];
        let doc_b: Vec<(&[u8], f64)> = vec![(b"the", 2.0), (b"cat", 1.0), (b"ran", 1.0)];
        // True inner product: the:3*2 + cat:1*1 = 7.
        let va = FeatureHasher::transform(dim, doc_a).unwrap();
        let vb = FeatureHasher::transform(dim, doc_b).unwrap();
        let hashed = dot(&va, &vb);
        assert!(
            (hashed - 7.0).abs() < 1.0,
            "hashed inner product {hashed} vs true 7"
        );
    }

    #[test]
    fn reset_clears() {
        let mut h = FeatureHasher::new(64).unwrap();
        h.add(b"x", 5.0);
        h.reset();
        assert!(h.vector().iter().all(|&v| v == 0.0));
    }

    #[test]
    fn dimension_is_fixed() {
        let h = FeatureHasher::new(512).unwrap();
        assert_eq!(h.dim(), 512);
        assert_eq!(h.vector().len(), 512);
    }
}
