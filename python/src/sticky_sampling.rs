//! Python bindings for Sticky Sampling (frequent items).
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::frequency::StickySampling as RustStickySampling;

/// Sticky Sampling — randomized frequent-item counting with probabilistic guarantees (2002).
#[pyclass(module = "sketch_oxide")]
pub struct StickySampling {
    inner: RustStickySampling<Vec<u8>>,
}

#[pymethods]
impl StickySampling {
    #[new]
    fn new(s: f64, epsilon: f64, delta: f64) -> PyResult<Self> {
        RustStickySampling::new(s, epsilon, delta)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.insert(python_item_to_bytes(item)?);
        Ok(())
    }
    fn estimate(&self, item: &Bound<'_, PyAny>) -> PyResult<u64> {
        Ok(self.inner.estimate(&python_item_to_bytes(item)?))
    }
    fn query(&self, py: Python<'_>, s: f64) -> Vec<(Py<PyAny>, u64)> {
        self.inner
            .query(s)
            .into_iter()
            .map(|(k, c)| (PyBytes::new(py, &k).into_any().unbind(), c))
            .collect()
    }
    fn count(&self) -> u64 {
        self.inner.count()
    }
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    fn __repr__(&self) -> String {
        format!("StickySampling(count={})", self.inner.count())
    }
}
