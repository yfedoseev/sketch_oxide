//! Python bindings for Unbiased Space-Saving (top-k, unbiased estimates).
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::frequency::UnbiasedSpaceSaving as RustUnbiasedSpaceSaving;

/// Unbiased Space-Saving — Space-Saving with unbiased frequency estimates (SIGMOD 2018).
#[pyclass(module = "sketch_oxide")]
pub struct UnbiasedSpaceSaving {
    inner: RustUnbiasedSpaceSaving<Vec<u8>>,
}

#[pymethods]
impl UnbiasedSpaceSaving {
    #[new]
    fn new(capacity: usize) -> PyResult<Self> {
        RustUnbiasedSpaceSaving::new(capacity)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.update(python_item_to_bytes(item)?);
        Ok(())
    }
    /// Estimated frequency, or None if not tracked.
    fn estimate(&self, item: &Bound<'_, PyAny>) -> PyResult<Option<u64>> {
        Ok(self.inner.estimate(&python_item_to_bytes(item)?))
    }
    fn top_k(&self, py: Python<'_>, k: usize) -> Vec<(PyObject, u64)> {
        self.inner
            .top_k(k)
            .into_iter()
            .map(|(key, c)| (PyBytes::new_bound(py, &key).into_any().unbind(), c))
            .collect()
    }
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    fn __repr__(&self) -> String {
        "UnbiasedSpaceSaving()".to_string()
    }
}
