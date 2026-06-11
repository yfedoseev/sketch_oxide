//! Python bindings for Recordinality cardinality estimation.

use crate::common::python_item_to_hash;
use pyo3::prelude::*;
use sketch_oxide::cardinality::Recordinality as RustRecordinality;

/// Recordinality — distinct counting from the number of record-breaking hash values.
///
/// Args:
///     k (int): size of the k-record buffer.
#[pyclass(module = "sketch_oxide")]
pub struct Recordinality {
    inner: RustRecordinality,
}

#[pymethods]
impl Recordinality {
    #[new]
    fn new(k: usize) -> PyResult<Self> {
        RustRecordinality::new(k)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Adds an item (int, str, bytes, or float).
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        let h = python_item_to_hash(item)?;
        self.inner.insert(&h.to_le_bytes());
        Ok(())
    }

    /// Estimated number of distinct items.
    fn estimate(&self) -> f64 {
        self.inner.estimate()
    }

    fn __repr__(&self) -> String {
        format!("Recordinality(estimate={:.0})", self.inner.estimate())
    }
}
