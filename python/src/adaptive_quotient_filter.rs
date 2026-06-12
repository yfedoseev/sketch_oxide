//! Python bindings for the Adaptive Quotient Filter.
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::membership::AdaptiveQuotientFilter as RustAqf;

/// Adaptive Quotient Filter — a quotient filter that adapts to fix repeated false positives.
#[pyclass(module = "sketch_oxide")]
pub struct AdaptiveQuotientFilter {
    inner: RustAqf,
}

#[pymethods]
impl AdaptiveQuotientFilter {
    #[new]
    fn new(q_bits: u32, r_bits: u32) -> PyResult<Self> {
        RustAqf::new(q_bits, r_bits)
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
    /// Adapts the filter after a confirmed false positive on `item`.
    fn adapt(&mut self, item: &Bound<'_, PyAny>) -> PyResult<usize> {
        Ok(self.inner.adapt(&python_item_to_bytes(item)?))
    }
    fn __len__(&self) -> usize {
        self.inner.len()
    }
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    fn __repr__(&self) -> String {
        format!("AdaptiveQuotientFilter(len={})", self.inner.len())
    }
}
