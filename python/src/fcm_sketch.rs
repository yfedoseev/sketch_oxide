//! Python bindings for the FCM Sketch frequency estimator.
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::frequency::FcmSketch as RustFcmSketch;

/// FCM-Sketch — a multi-level Count-Min for programmable data planes (CoNEXT 2020).
#[pyclass(module = "sketch_oxide")]
pub struct FcmSketch {
    inner: RustFcmSketch,
}

#[pymethods]
impl FcmSketch {
    #[new]
    fn new(depth: usize, leaf_width: usize, overflow_width: usize) -> PyResult<Self> {
        RustFcmSketch::new(depth, leaf_width, overflow_width)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.update(&python_item_to_bytes(item)?);
        Ok(())
    }
    fn estimate(&self, item: &Bound<'_, PyAny>) -> PyResult<u64> {
        Ok(self.inner.estimate(&python_item_to_bytes(item)?))
    }
    fn __repr__(&self) -> String {
        "FcmSketch()".to_string()
    }
}
