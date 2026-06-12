//! Python bindings for SuperMinHash (low-variance Jaccard estimation).
use crate::common::python_item_to_hash;
use pyo3::prelude::*;
use sketch_oxide::similarity::SuperMinHash as RustSuperMinHash;

/// SuperMinHash — a lower-variance MinHash for Jaccard similarity (Ertl 2017).
#[pyclass(module = "sketch_oxide")]
pub struct SuperMinHash {
    inner: RustSuperMinHash,
}

#[pymethods]
impl SuperMinHash {
    #[new]
    fn new(m: usize) -> PyResult<Self> {
        RustSuperMinHash::new(m)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.add(&python_item_to_hash(item)?);
        Ok(())
    }
    fn jaccard(&self, other: &SuperMinHash) -> PyResult<f64> {
        self.inner
            .jaccard(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn merge(&mut self, other: &SuperMinHash) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn signature(&self) -> Vec<f64> {
        self.inner.signature().to_vec()
    }
    fn __repr__(&self) -> String {
        "SuperMinHash()".to_string()
    }
}
