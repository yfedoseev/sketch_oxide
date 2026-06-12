//! Python bindings for MinHash LSH (banded near-neighbour index).
use pyo3::prelude::*;
use sketch_oxide::similarity::MinHashLsh as RustMinHashLsh;

/// MinHash LSH — a banded locality-sensitive index over MinHash signatures (int ids).
#[pyclass(module = "sketch_oxide")]
pub struct MinHashLsh {
    inner: RustMinHashLsh<u64>,
}

#[pymethods]
impl MinHashLsh {
    #[new]
    fn new(num_bands: usize, rows_per_band: usize) -> PyResult<Self> {
        RustMinHashLsh::new(num_bands, rows_per_band)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    /// Indexes `signature` (a list of ints) under integer `id`.
    fn insert(&mut self, id: u64, signature: Vec<u64>) -> PyResult<()> {
        self.inner
            .insert(id, &signature)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    /// Candidate ids sharing a band with `signature`.
    fn query(&self, signature: Vec<u64>) -> PyResult<Vec<u64>> {
        self.inner
            .query(&signature)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn __repr__(&self) -> String {
        "MinHashLsh()".to_string()
    }
}
