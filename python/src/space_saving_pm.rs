//! Python bindings for Space-Saving± (bounded-deletion heavy hitters).
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::frequency::SpaceSavingPlusMinus as RustSpaceSavingPm;

/// Space-Saving± — heavy hitters in the bounded-deletion model (insertions and deletions).
#[pyclass(module = "sketch_oxide")]
pub struct SpaceSavingPlusMinus {
    inner: RustSpaceSavingPm<Vec<u8>>,
}

#[pymethods]
impl SpaceSavingPlusMinus {
    #[new]
    fn new(epsilon: f64) -> PyResult<Self> {
        RustSpaceSavingPm::new(epsilon)
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
    /// Heavy hitters with net frequency at least `min_net`, as (bytes, count).
    fn heavy_hitters(&self, py: Python<'_>, min_net: u64) -> Vec<(PyObject, u64)> {
        self.inner
            .heavy_hitters(min_net)
            .into_iter()
            .map(|(k, c)| (PyBytes::new_bound(py, &k).into_any().unbind(), c))
            .collect()
    }
    fn __repr__(&self) -> String {
        "SpaceSavingPlusMinus()".to_string()
    }
}
