//! Python bindings for the MV-Sketch (heavy hitters).
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::frequency::MvSketch as RustMvSketch;

/// MV-Sketch — majority-vote based heavy-hitter detection in a compact sketch.
#[pyclass(module = "sketch_oxide")]
pub struct MvSketch {
    inner: RustMvSketch,
}

#[pymethods]
impl MvSketch {
    #[new]
    fn new(depth: usize, width: usize) -> PyResult<Self> {
        RustMvSketch::new(depth, width)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.update(&python_item_to_bytes(item)?);
        Ok(())
    }
    fn estimate(&self, item: &Bound<'_, PyAny>) -> PyResult<i64> {
        Ok(self.inner.estimate(&python_item_to_bytes(item)?))
    }
    /// Items whose estimated frequency exceeds `threshold`, as (bytes, count).
    fn heavy_hitters(&self, py: Python<'_>, threshold: i64) -> Vec<(PyObject, i64)> {
        self.inner
            .heavy_hitters(threshold)
            .into_iter()
            .map(|(k, c)| (PyBytes::new_bound(py, &k).into_any().unbind(), c))
            .collect()
    }
    fn __repr__(&self) -> String {
        "MvSketch()".to_string()
    }
}
