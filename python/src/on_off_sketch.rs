//! Python bindings for the On-Off Sketch (persistent items).
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::frequency::OnOffSketch as RustOnOffSketch;

/// On-Off Sketch — measures item *persistence* (presence across many time periods).
#[pyclass(module = "sketch_oxide")]
pub struct OnOffSketch {
    inner: RustOnOffSketch,
}

#[pymethods]
impl OnOffSketch {
    #[new]
    fn new(depth: usize, width: usize) -> PyResult<Self> {
        RustOnOffSketch::new(depth, width)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    /// Records `item` as present in the current period.
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.update(&python_item_to_bytes(item)?);
        Ok(())
    }
    /// Advances to a new time period.
    fn new_period(&mut self) {
        self.inner.new_period();
    }
    /// Estimated number of periods in which `item` appeared.
    fn persistence(&self, item: &Bound<'_, PyAny>) -> PyResult<u64> {
        Ok(self.inner.persistence(&python_item_to_bytes(item)?))
    }
    fn __repr__(&self) -> String {
        "OnOffSketch()".to_string()
    }
}
