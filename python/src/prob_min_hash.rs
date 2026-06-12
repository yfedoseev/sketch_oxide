//! Python bindings for ProbMinHash (weighted Jaccard).
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::similarity::ProbMinHash as RustProbMinHash;

/// ProbMinHash — probability MinHash for weighted Jaccard similarity (Ertl 2020).
#[pyclass(module = "sketch_oxide")]
pub struct ProbMinHash {
    inner: RustProbMinHash<Vec<u8>>,
}

#[pymethods]
impl ProbMinHash {
    #[new]
    fn new(m: usize) -> PyResult<Self> {
        RustProbMinHash::new(m)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    /// Adds `item` with the given positive `weight`.
    fn update(&mut self, item: &Bound<'_, PyAny>, weight: f64) -> PyResult<()> {
        self.inner.add(python_item_to_bytes(item)?, weight);
        Ok(())
    }
    fn jaccard(&self, other: &ProbMinHash) -> PyResult<f64> {
        self.inner
            .jaccard(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn merge(&mut self, other: &ProbMinHash) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn __repr__(&self) -> String {
        "ProbMinHash()".to_string()
    }
}
