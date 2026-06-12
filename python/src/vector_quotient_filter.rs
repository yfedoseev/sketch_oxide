//! Python bindings for the Vector Quotient Filter.
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::membership::VectorQuotientFilter as RustVqf;

/// Vector Quotient Filter — a SIMD-friendly AMQ with deletions (Pandey 2021).
#[pyclass(module = "sketch_oxide")]
pub struct VectorQuotientFilter {
    inner: RustVqf,
}

#[pymethods]
impl VectorQuotientFilter {
    #[new]
    fn new(num_blocks: usize, block_capacity: usize) -> PyResult<Self> {
        RustVqf::new(num_blocks, block_capacity)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    /// Inserts a key; returns False if the filter is full.
    fn insert(&mut self, item: &Bound<'_, PyAny>) -> PyResult<bool> {
        Ok(self.inner.insert(&python_item_to_bytes(item)?))
    }
    fn contains(&self, item: &Bound<'_, PyAny>) -> PyResult<bool> {
        Ok(self.inner.contains(&python_item_to_bytes(item)?))
    }
    fn remove(&mut self, item: &Bound<'_, PyAny>) -> PyResult<bool> {
        Ok(self.inner.remove(&python_item_to_bytes(item)?))
    }
    fn __len__(&self) -> usize {
        self.inner.len()
    }
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    fn __repr__(&self) -> String {
        format!("VectorQuotientFilter(len={})", self.inner.len())
    }
}
