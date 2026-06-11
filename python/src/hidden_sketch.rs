//! Python bindings for the Hidden Sketch (reversible frequent-item key recovery).
use crate::common::python_item_to_hash;
use pyo3::prelude::*;
use sketch_oxide::frequency::HiddenSketch as RustHiddenSketch;

/// Hidden Sketch — a space-efficient reversible sketch for frequent-item key recovery.
#[pyclass(module = "sketch_oxide")]
pub struct HiddenSketch {
    inner: RustHiddenSketch,
}

#[pymethods]
impl HiddenSketch {
    #[new]
    fn new(num_cells: usize, depth: usize) -> PyResult<Self> {
        RustHiddenSketch::new(num_cells, depth)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.insert(python_item_to_hash(item)?);
        Ok(())
    }
    fn estimate(&self, item: &Bound<'_, PyAny>) -> PyResult<u64> {
        Ok(self.inner.estimate(python_item_to_hash(item)?))
    }
    fn __repr__(&self) -> String {
        "HiddenSketch()".to_string()
    }
}
