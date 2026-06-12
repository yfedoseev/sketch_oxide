//! Python bindings for the Aleph Filter.
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::membership::AlephFilter as RustAlephFilter;

/// Aleph Filter — a dynamic, resizable AMQ with stable false-positive rate.
#[pyclass(module = "sketch_oxide")]
pub struct AlephFilter {
    inner: RustAlephFilter,
}

#[pymethods]
impl AlephFilter {
    #[new]
    fn new(initial_log_buckets: u32, fingerprint_bits: u32) -> PyResult<Self> {
        RustAlephFilter::new(initial_log_buckets, fingerprint_bits)
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
    fn __len__(&self) -> usize {
        self.inner.len()
    }
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    fn __repr__(&self) -> String {
        format!("AlephFilter(len={})", self.inner.len())
    }
}
