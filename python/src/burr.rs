//! Python bindings for the BuRR (Bumped Ribbon Retrieval) filter.
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::membership::BurrFilter as RustBurr;

/// BuRR — a near-space-optimal static AMQ built from a fixed key set (Dillinger 2022).
#[pyclass(module = "sketch_oxide")]
pub struct BurrFilter {
    inner: RustBurr,
}

#[pymethods]
impl BurrFilter {
    /// Builds the filter from a list of keys (int/str/bytes/float) at the target false-positive rate.
    #[new]
    fn new(keys: Vec<Bound<'_, PyAny>>, fpr: f64) -> PyResult<Self> {
        let byte_keys: Vec<Vec<u8>> = keys
            .iter()
            .map(python_item_to_bytes)
            .collect::<PyResult<_>>()?;
        RustBurr::build(&byte_keys, fpr)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
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
        format!("BurrFilter(len={})", self.inner.len())
    }
}
