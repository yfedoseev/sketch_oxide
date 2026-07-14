//! Python bindings for the HeavyLocker (heavy hitters).
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::frequency::HeavyLocker as RustHeavyLocker;

/// HeavyLocker — heavy-hitter detection with lock-based protection of hot keys.
#[pyclass(module = "sketch_oxide")]
pub struct HeavyLocker {
    inner: RustHeavyLocker,
}

#[pymethods]
impl HeavyLocker {
    #[new]
    fn new(w: usize, d: usize, theta: f64, lock_tuning: f64, num_hashes: usize) -> PyResult<Self> {
        RustHeavyLocker::new(w, d, theta, lock_tuning, num_hashes)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.insert(&python_item_to_bytes(item)?);
        Ok(())
    }
    /// Estimated frequency of an item.
    fn estimate(&self, item: &Bound<'_, PyAny>) -> PyResult<u64> {
        Ok(self.inner.query(&python_item_to_bytes(item)?))
    }
    /// Heavy hitters above frequency fraction `phi`, as (bytes, count).
    fn heavy_hitters(&self, py: Python<'_>, phi: f64) -> Vec<(Py<PyAny>, u64)> {
        self.inner
            .heavy_hitters(phi)
            .into_iter()
            .map(|(k, c)| (PyBytes::new(py, &k).into_any().unbind(), c))
            .collect()
    }
    fn __repr__(&self) -> String {
        "HeavyLocker()".to_string()
    }
}
