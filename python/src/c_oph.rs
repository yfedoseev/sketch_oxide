//! Python bindings for C-OPH (circulant one permutation hashing).
use pyo3::prelude::*;
use sketch_oxide::similarity::COph as RustCOph;

/// C-OPH — circulant One Permutation Hashing with improved densification (Li & Li, 2021).
///
/// Operates over a fixed universe [0, d); add element indices in that range.
#[pyclass(module = "sketch_oxide")]
pub struct COph {
    inner: RustCOph,
}

#[pymethods]
impl COph {
    #[new]
    fn new(d: usize, k: usize, seed: u64) -> PyResult<Self> {
        RustCOph::new(d, k, seed)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    /// Adds element index `i` (must be in [0, d)).
    fn update(&mut self, i: u32) -> PyResult<()> {
        self.inner
            .add(i)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn jaccard(&self, other: &COph) -> PyResult<f64> {
        self.inner
            .jaccard(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn signature(&self) -> Vec<u64> {
        self.inner.signature()
    }
    fn __repr__(&self) -> String {
        "COph()".to_string()
    }
}
