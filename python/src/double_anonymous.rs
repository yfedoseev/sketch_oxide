//! Python bindings for the Double-Anonymous Sketch (fair global top-k).
use crate::common::python_item_to_hash;
use pyo3::prelude::*;
use sketch_oxide::frequency::DoubleAnonymousSketch as RustDoubleAnonymous;

/// Double-Anonymous Sketch — top-K-fair global top-K across disjoint streams (SIGMOD 2023).
#[pyclass(module = "sketch_oxide")]
pub struct DoubleAnonymousSketch {
    inner: RustDoubleAnonymous,
}

#[pymethods]
impl DoubleAnonymousSketch {
    #[new]
    fn new(width: usize, depth: usize, k: usize, seed: u64) -> PyResult<Self> {
        RustDoubleAnonymous::new(width, depth, k, seed)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.insert(python_item_to_hash(item)?);
        Ok(())
    }
    /// Estimated frequency of an item.
    fn estimate(&self, item: &Bound<'_, PyAny>) -> PyResult<f64> {
        Ok(self.inner.estimate(python_item_to_hash(item)?))
    }
    /// Top-k items as (hashed_key, frequency).
    fn top_k(&self, k: usize) -> Vec<(u64, f64)> {
        self.inner.top_k(k)
    }
    /// Merges another site's sketch (sums count parts, unions candidates).
    fn merge(&mut self, other: &DoubleAnonymousSketch) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn __repr__(&self) -> String {
        "DoubleAnonymousSketch()".to_string()
    }
}
