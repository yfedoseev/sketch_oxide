//! Python bindings for the Count-Min-Log frequency sketch.
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::frequency::CountMinLog as RustCountMinLog;

/// Count-Min-Log — Count-Min with approximate logarithmic counters (Pitel & Fouquier, 2015).
#[pyclass(module = "sketch_oxide")]
pub struct CountMinLog {
    inner: RustCountMinLog,
}

#[pymethods]
impl CountMinLog {
    #[new]
    fn new(width: usize, depth: usize, seed: u64) -> PyResult<Self> {
        RustCountMinLog::new(width, depth, seed)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    /// Records one occurrence of an item (int, str, bytes, or float).
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.add(&python_item_to_bytes(item)?);
        Ok(())
    }
    /// Estimated frequency of an item.
    fn estimate(&self, item: &Bound<'_, PyAny>) -> PyResult<f64> {
        Ok(self.inner.estimate(&python_item_to_bytes(item)?))
    }
    fn __repr__(&self) -> String {
        "CountMinLog()".to_string()
    }
}
