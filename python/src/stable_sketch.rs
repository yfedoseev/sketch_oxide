//! Python bindings for the Stable Sketch (heavy hitters / persistent items).
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::frequency::StableSketch as RustStableSketch;

/// Stable-Sketch — bucket-stability stochastic replacement for heavy hitters (WWW 2024).
#[pyclass(module = "sketch_oxide")]
pub struct StableSketch {
    inner: RustStableSketch,
}

#[pymethods]
impl StableSketch {
    #[new]
    fn new(rows: usize, cols: usize) -> PyResult<Self> {
        RustStableSketch::new(rows, cols)
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
    fn heavy_hitters(&self, py: Python<'_>, threshold: u64) -> Vec<(Py<PyAny>, u64)> {
        self.inner
            .heavy_hitters(threshold)
            .into_iter()
            .map(|(k, c)| (PyBytes::new(py, &k).into_any().unbind(), c))
            .collect()
    }
    fn __repr__(&self) -> String {
        "StableSketch()".to_string()
    }
}
