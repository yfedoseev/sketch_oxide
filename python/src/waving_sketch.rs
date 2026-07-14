//! Python bindings for the WavingSketch (unbiased top-k).
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::frequency::WavingSketch as RustWavingSketch;

/// WavingSketch — an unbiased, generic sketch for finding top-k items (VLDB 2020).
#[pyclass(module = "sketch_oxide")]
pub struct WavingSketch {
    inner: RustWavingSketch,
}

#[pymethods]
impl WavingSketch {
    #[new]
    fn new(num_buckets: usize, slots: usize) -> PyResult<Self> {
        RustWavingSketch::new(num_buckets, slots)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.insert(&python_item_to_bytes(item)?);
        Ok(())
    }
    fn estimate(&self, item: &Bound<'_, PyAny>) -> PyResult<i64> {
        Ok(self.inner.estimate(&python_item_to_bytes(item)?))
    }
    fn heavy_hitters(&self, py: Python<'_>, min_count: i64) -> Vec<(Py<PyAny>, i64)> {
        self.inner
            .heavy_hitters(min_count)
            .into_iter()
            .map(|(k, c)| (PyBytes::new(py, &k).into_any().unbind(), c))
            .collect()
    }
    fn __repr__(&self) -> String {
        "WavingSketch()".to_string()
    }
}
