//! Python bindings for the Bubble Sketch (top-k frequent items).
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::frequency::BubbleSketch as RustBubbleSketch;

/// Bubble Sketch — frequency estimation with a built-in top-k of frequent items.
#[pyclass(module = "sketch_oxide")]
pub struct BubbleSketch {
    inner: RustBubbleSketch,
}

#[pymethods]
impl BubbleSketch {
    #[new]
    fn new(w: usize, b: usize, k: usize, alpha: f64) -> PyResult<Self> {
        RustBubbleSketch::new(w, b, k, alpha)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.insert(&python_item_to_bytes(item)?);
        Ok(())
    }
    fn estimate(&self, item: &Bound<'_, PyAny>) -> PyResult<u64> {
        Ok(self.inner.estimate(&python_item_to_bytes(item)?))
    }
    /// Returns the top-k items as a list of (bytes, count).
    fn top_k(&self, py: Python<'_>) -> Vec<(Py<PyAny>, u64)> {
        self.inner
            .top_k()
            .into_iter()
            .map(|(k, c)| (PyBytes::new(py, &k).into_any().unbind(), c))
            .collect()
    }
    fn __repr__(&self) -> String {
        "BubbleSketch()".to_string()
    }
}
