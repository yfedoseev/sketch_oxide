//! Python bindings for the KMV (k-minimum-values) cardinality sketch.

use crate::common::python_item_to_hash;
use pyo3::prelude::*;
use sketch_oxide::cardinality::KmvSketch as RustKmvSketch;

/// KMV (k-Minimum Values) — bottom-k distinct counting; exact for ≤ k distinct items.
///
/// Args:
///     k (int): number of smallest hash values retained.
#[pyclass(module = "sketch_oxide")]
pub struct KmvSketch {
    inner: RustKmvSketch,
}

#[pymethods]
impl KmvSketch {
    #[new]
    fn new(k: usize) -> PyResult<Self> {
        RustKmvSketch::new(k)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Adds an item (int, str, bytes, or float).
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        let h = python_item_to_hash(item)?;
        self.inner.add(&h.to_le_bytes());
        Ok(())
    }

    /// Estimated number of distinct items.
    fn estimate(&self) -> f64 {
        self.inner.estimate()
    }

    /// Whether no items have been added.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn __repr__(&self) -> String {
        format!("KmvSketch(estimate={:.0})", self.inner.estimate())
    }
}
