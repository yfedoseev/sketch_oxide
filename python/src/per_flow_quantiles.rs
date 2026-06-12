//! Python bindings for Per-Flow Quantiles.
use crate::common::python_item_to_hash;
use pyo3::prelude::*;
use sketch_oxide::quantiles::PerFlowQuantiles as RustPerFlow;

/// Per-Flow Quantiles — maintains an approximate quantile summary for each flow key in one sketch.
#[pyclass(module = "sketch_oxide")]
pub struct PerFlowQuantiles {
    inner: RustPerFlow,
}

#[pymethods]
impl PerFlowQuantiles {
    #[new]
    fn new(depth: usize, width: usize, relative_accuracy: f64) -> PyResult<Self> {
        RustPerFlow::new(depth, width, relative_accuracy)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    /// Records `value` for `flow` (int/str/bytes/float key).
    fn update(&mut self, flow: &Bound<'_, PyAny>, value: f64) -> PyResult<()> {
        self.inner.update(&python_item_to_hash(flow)?, value);
        Ok(())
    }
    /// Approximate φ-quantile of `flow`'s values.
    fn quantile(&self, flow: &Bound<'_, PyAny>, phi: f64) -> PyResult<Option<f64>> {
        Ok(self.inner.quantile(&python_item_to_hash(flow)?, phi))
    }
    fn __repr__(&self) -> String {
        "PerFlowQuantiles()".to_string()
    }
}
