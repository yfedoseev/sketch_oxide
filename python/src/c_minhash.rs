//! Python bindings for C-MinHash (circulant MinHash).
use pyo3::prelude::*;
use sketch_oxide::similarity::CMinHash as RustCMinHash;

/// C-MinHash — circulant MinHash reusing one permutation K times (Li & Li, ICML 2022).
///
/// Operates over a fixed universe [0, d); add element indices in that range.
#[pyclass(module = "sketch_oxide")]
pub struct CMinHash {
    inner: RustCMinHash,
}

#[pymethods]
impl CMinHash {
    #[new]
    fn new(d: usize, k: usize, seed: u64) -> PyResult<Self> {
        RustCMinHash::new(d, k, seed)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    /// Adds element index `i` (must be in [0, d)).
    fn update(&mut self, i: u32) -> PyResult<()> {
        self.inner
            .add(i)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn jaccard(&self, other: &CMinHash) -> PyResult<f64> {
        self.inner
            .jaccard(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn signature(&self) -> Vec<u32> {
        self.inner.signature().to_vec()
    }
    fn __repr__(&self) -> String {
        "CMinHash()".to_string()
    }
}
