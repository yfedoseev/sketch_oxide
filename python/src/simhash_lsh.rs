//! Python bindings for SimHash LSH (Hamming near-neighbour index).
use pyo3::prelude::*;
use sketch_oxide::similarity::SimHashLsh as RustSimHashLsh;

/// SimHash LSH — a blocked index over 64-bit SimHash signatures for Hamming near-neighbours.
#[pyclass(module = "sketch_oxide")]
pub struct SimHashLsh {
    inner: RustSimHashLsh<u64>,
}

#[pymethods]
impl SimHashLsh {
    #[new]
    fn new(num_blocks: usize) -> PyResult<Self> {
        RustSimHashLsh::new(num_blocks)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    /// Indexes a 64-bit SimHash `sig` under integer `id`.
    fn insert(&mut self, id: u64, sig: u64) {
        self.inner.insert(id, sig);
    }
    /// Candidate ids sharing a block with `sig`.
    fn query(&self, sig: u64) -> Vec<u64> {
        self.inner.query(sig)
    }
    fn __repr__(&self) -> String {
        "SimHashLsh()".to_string()
    }
}
