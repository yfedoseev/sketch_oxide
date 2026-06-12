//! Python bindings for the Scalable Bloom Filter.
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::membership::ScalableBloomFilter as RustScalableBloom;

/// Scalable Bloom Filter — grows automatically while keeping a target FPR (Almeida 2007).
#[pyclass(module = "sketch_oxide")]
pub struct ScalableBloomFilter {
    inner: RustScalableBloom,
}

#[pymethods]
impl ScalableBloomFilter {
    #[new]
    fn new(initial_capacity: usize, target_fpr: f64) -> PyResult<Self> {
        RustScalableBloom::new(initial_capacity, target_fpr)
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
        format!("ScalableBloomFilter(len={})", self.inner.len())
    }
}
