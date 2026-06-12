//! Python bindings for the Counting Quotient Filter.
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::membership::CountingQuotientFilter as RustCqf;

/// Counting Quotient Filter — a cache-friendly AMQ supporting counts and deletions (Pandey 2017).
#[pyclass(module = "sketch_oxide")]
pub struct CountingQuotientFilter {
    inner: RustCqf,
}

#[pymethods]
impl CountingQuotientFilter {
    #[new]
    fn new(q_bits: u32, r_bits: u32) -> PyResult<Self> {
        RustCqf::new(q_bits, r_bits)
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
    /// Removes one occurrence; returns the remaining count.
    fn remove(&mut self, item: &Bound<'_, PyAny>) -> PyResult<u64> {
        Ok(self.inner.remove(&python_item_to_bytes(item)?))
    }
    fn __len__(&self) -> usize {
        self.inner.len()
    }
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    fn __repr__(&self) -> String {
        format!("CountingQuotientFilter(len={})", self.inner.len())
    }
}
