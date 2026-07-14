//! Python bindings for Lossy Counting (frequent items).
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::frequency::LossyCounting as RustLossyCounting;

/// Lossy Counting — deterministic frequent-item counting with bounded error (Manku & Motwani 2002).
#[pyclass(module = "sketch_oxide")]
pub struct LossyCounting {
    inner: RustLossyCounting<Vec<u8>>,
}

#[pymethods]
impl LossyCounting {
    #[new]
    fn new(epsilon: f64) -> PyResult<Self> {
        RustLossyCounting::new(epsilon)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.insert(python_item_to_bytes(item)?);
        Ok(())
    }
    fn estimate(&self, item: &Bound<'_, PyAny>) -> PyResult<u64> {
        Ok(self.inner.estimate(&python_item_to_bytes(item)?))
    }
    /// Items with frequency above fraction `s`, as (bytes, count).
    fn query(&self, py: Python<'_>, s: f64) -> Vec<(Py<PyAny>, u64)> {
        self.inner
            .query(s)
            .into_iter()
            .map(|(k, c)| (PyBytes::new(py, &k).into_any().unbind(), c))
            .collect()
    }
    /// Total number of items processed.
    fn count(&self) -> u64 {
        self.inner.count()
    }
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    fn __repr__(&self) -> String {
        format!("LossyCounting(count={})", self.inner.count())
    }
}
