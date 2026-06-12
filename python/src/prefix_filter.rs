//! Python bindings for the Prefix Filter.
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::membership::PrefixFilter as RustPrefixFilter;

/// Prefix Filter — a fast, compact AMQ that pairs a small bin filter with a spare (Even 2022).
#[pyclass(module = "sketch_oxide")]
pub struct PrefixFilter {
    inner: RustPrefixFilter,
}

#[pymethods]
impl PrefixFilter {
    #[new]
    fn new(expected_keys: usize, fp_bits: u32, bin_capacity: usize) -> PyResult<Self> {
        RustPrefixFilter::new(expected_keys, fp_bits, bin_capacity)
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
        format!("PrefixFilter(len={})", self.inner.len())
    }
}
