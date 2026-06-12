//! Python bindings for SketchPolymer (per-item quantiles).
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::quantiles::SketchPolymer as RustSketchPolymer;

/// SketchPolymer — compact per-item value-distribution summaries for quantile queries.
#[pyclass(module = "sketch_oxide")]
pub struct SketchPolymer {
    inner: RustSketchPolymer,
}

#[pymethods]
impl SketchPolymer {
    #[new]
    fn new(depth: usize, width: usize, a: f64, threshold: u64) -> PyResult<Self> {
        RustSketchPolymer::new(depth, width, a, threshold)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    /// Records `value` for `item` (int/str/bytes/float key).
    fn update(&mut self, item: &Bound<'_, PyAny>, value: f64) -> PyResult<()> {
        self.inner.insert(&python_item_to_bytes(item)?, value);
        Ok(())
    }
    /// Approximate `w`-quantile of `item`'s values.
    fn quantile(&self, item: &Bound<'_, PyAny>, w: f64) -> PyResult<f64> {
        Ok(self.inner.quantile(&python_item_to_bytes(item)?, w))
    }
    fn __repr__(&self) -> String {
        "SketchPolymer()".to_string()
    }
}
