//! Python bindings for LSH Ensemble (set-containment search).
use pyo3::prelude::*;
use sketch_oxide::similarity::LshEnsemble as RustLshEnsemble;

/// LSH Ensemble — domain search by set *containment* across size partitions (Zhu, VLDB 2016).
#[pyclass(module = "sketch_oxide")]
pub struct LshEnsemble {
    inner: RustLshEnsemble<u64>,
}

#[pymethods]
impl LshEnsemble {
    #[new]
    fn new(num_perm: usize, num_partitions: usize) -> PyResult<Self> {
        RustLshEnsemble::new(num_perm, num_partitions)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    /// Indexes a set: its MinHash `signature`, distinct `size`, under integer `id`.
    fn add(&mut self, id: u64, signature: Vec<u64>, size: usize) -> PyResult<()> {
        self.inner
            .add(id, &signature, size)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    /// Ids of indexed sets whose estimated containment of the query clears `threshold`.
    fn query(&self, q_signature: Vec<u64>, q_size: usize, threshold: f64) -> PyResult<Vec<u64>> {
        self.inner
            .query(&q_signature, q_size, threshold)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn __repr__(&self) -> String {
        "LshEnsemble()".to_string()
    }
}
