//! Python bindings for Filtered Space-Saving (top-k).
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::frequency::FilteredSpaceSaving as RustFilteredSpaceSaving;

/// Filtered Space-Saving — Space-Saving with a pre-filter that rejects cold items (Cormode 2008).
#[pyclass(module = "sketch_oxide")]
pub struct FilteredSpaceSaving {
    inner: RustFilteredSpaceSaving<Vec<u8>>,
}

#[pymethods]
impl FilteredSpaceSaving {
    #[new]
    fn new(capacity: usize, filter_size: usize) -> PyResult<Self> {
        RustFilteredSpaceSaving::new(capacity, filter_size)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.update(python_item_to_bytes(item)?);
        Ok(())
    }
    /// Estimated (lower, upper) frequency bounds, or None if not tracked.
    fn estimate(&self, item: &Bound<'_, PyAny>) -> PyResult<Option<(u64, u64)>> {
        Ok(self.inner.estimate(&python_item_to_bytes(item)?))
    }
    /// Top-k items as (bytes, count).
    fn top_k(&self, py: Python<'_>, k: usize) -> Vec<(Py<PyAny>, u64)> {
        self.inner
            .top_k(k)
            .into_iter()
            .map(|(key, c)| (PyBytes::new(py, &key).into_any().unbind(), c))
            .collect()
    }
    fn count(&self) -> u64 {
        self.inner.count()
    }
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    fn __repr__(&self) -> String {
        "FilteredSpaceSaving()".to_string()
    }
}
