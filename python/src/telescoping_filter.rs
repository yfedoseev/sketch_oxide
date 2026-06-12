//! Python bindings for the Telescoping (adaptive) Filter.
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::membership::TelescopingFilter as RustTelescoping;

/// Telescoping Filter — a practical adaptive filter with variable-length fingerprints (ESA 2021).
#[pyclass(module = "sketch_oxide")]
pub struct TelescopingFilter {
    inner: RustTelescoping,
}

#[pymethods]
impl TelescopingFilter {
    #[new]
    fn new(q: u32, r: u32) -> PyResult<Self> {
        RustTelescoping::new(q, r)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn insert(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.insert(&python_item_to_bytes(item)?);
        Ok(())
    }
    fn contains(&self, item: &Bound<'_, PyAny>) -> PyResult<bool> {
        Ok(self.inner.contains(&python_item_to_bytes(item)?))
    }
    /// Adapts after a confirmed false positive on `item` so it never recurs.
    fn adapt(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.adapt(&python_item_to_bytes(item)?);
        Ok(())
    }
    fn __len__(&self) -> usize {
        self.inner.len()
    }
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    fn __repr__(&self) -> String {
        format!("TelescopingFilter(len={})", self.inner.len())
    }
}
