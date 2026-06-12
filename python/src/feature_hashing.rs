//! Python bindings for the feature-hashing (hashing-trick) vectorizer.

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::learned::FeatureHasher as RustFeatureHasher;

/// FeatureHasher — the hashing trick: maps arbitrary string/byte features into a
/// fixed `dim`-dimensional vector by hashing, accumulating signed values, for
/// memory-bounded feature vectorization in ML pipelines.
///
/// Args:
///     dim (int): output vector dimensionality.
#[pyclass(module = "sketch_oxide")]
pub struct FeatureHasher {
    inner: RustFeatureHasher,
}

#[pymethods]
impl FeatureHasher {
    #[new]
    fn new(dim: usize) -> PyResult<Self> {
        RustFeatureHasher::new(dim)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Accumulates `value` into the bucket for `feature` (int, str, bytes, or float).
    fn add(&mut self, feature: &Bound<'_, PyAny>, value: f64) -> PyResult<()> {
        self.inner.add(&python_item_to_bytes(feature)?, value);
        Ok(())
    }

    /// The current `dim`-dimensional feature vector.
    fn vector(&self) -> Vec<f64> {
        self.inner.vector().to_vec()
    }

    /// Resets the vector to zero.
    fn reset(&mut self) {
        self.inner.reset();
    }

    /// Output dimensionality.
    fn dim(&self) -> usize {
        self.inner.dim()
    }

    fn __repr__(&self) -> String {
        format!("FeatureHasher(dim={})", self.inner.dim())
    }
}
