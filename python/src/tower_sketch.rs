//! Python bindings for the Tower Sketch frequency estimator.
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::frequency::TowerSketch as RustTowerSketch;

/// Tower Sketch — a multi-resolution counter layout for frequency estimation.
#[pyclass(module = "sketch_oxide")]
pub struct TowerSketch {
    inner: RustTowerSketch,
}

#[pymethods]
impl TowerSketch {
    #[new]
    fn new(base_width: usize) -> PyResult<Self> {
        RustTowerSketch::new(base_width)
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
        "TowerSketch()".to_string()
    }
}
