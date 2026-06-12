//! Python bindings for One Permutation Hashing.
use crate::common::python_item_to_hash;
use pyo3::prelude::*;
use sketch_oxide::similarity::OnePermutationHash as RustOph;

/// One Permutation Hashing — fast MinHash using a single permutation with densification.
#[pyclass(module = "sketch_oxide")]
pub struct OnePermutationHash {
    inner: RustOph,
}

#[pymethods]
impl OnePermutationHash {
    #[new]
    fn new(k: usize) -> PyResult<Self> {
        RustOph::new(k)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.update(&python_item_to_hash(item)?);
        Ok(())
    }
    fn jaccard(&self, other: &OnePermutationHash) -> PyResult<f64> {
        self.inner
            .jaccard(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    /// The densified signature (a list of ints).
    fn signature(&self) -> Vec<u64> {
        self.inner.signature()
    }
    fn __repr__(&self) -> String {
        "OnePermutationHash()".to_string()
    }
}
