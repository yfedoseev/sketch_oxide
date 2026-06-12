//! Python bindings for Per-Key Quantiles.
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::quantiles::PerKeyQuantiles as RustPerKey;

/// Per-Key Quantiles — an exact-key map of GK quantile summaries (one per tracked key).
#[pyclass(module = "sketch_oxide")]
pub struct PerKeyQuantiles {
    inner: RustPerKey<Vec<u8>>,
}

#[pymethods]
impl PerKeyQuantiles {
    #[new]
    fn new(capacity: usize, epsilon: f64) -> PyResult<Self> {
        RustPerKey::new(capacity, epsilon)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn update(&mut self, key: &Bound<'_, PyAny>, value: f64) -> PyResult<()> {
        self.inner.update(python_item_to_bytes(key)?, value);
        Ok(())
    }
    fn quantile(&self, key: &Bound<'_, PyAny>, phi: f64) -> PyResult<Option<f64>> {
        Ok(self.inner.quantile(&python_item_to_bytes(key)?, phi))
    }
    fn count(&self, key: &Bound<'_, PyAny>) -> PyResult<u64> {
        Ok(self.inner.count(&python_item_to_bytes(key)?))
    }
    fn __repr__(&self) -> String {
        "PerKeyQuantiles()".to_string()
    }
}
