//! Python bindings for the Stacked Filter.
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::membership::StackedFilter as RustStacked;

/// Stacked Filter — uses known negatives to shrink the filter and cut false positives (VLDB 2021).
#[pyclass(module = "sketch_oxide")]
pub struct StackedFilter {
    inner: RustStacked,
}

#[pymethods]
impl StackedFilter {
    /// Builds from a positive key set and a set of known (frequent) negatives.
    #[new]
    fn new(
        positives: Vec<Bound<'_, PyAny>>,
        known_negatives: Vec<Bound<'_, PyAny>>,
        fp: f64,
    ) -> PyResult<Self> {
        let pos: Vec<Vec<u8>> = positives
            .iter()
            .map(python_item_to_bytes)
            .collect::<PyResult<_>>()?;
        let neg: Vec<Vec<u8>> = known_negatives
            .iter()
            .map(python_item_to_bytes)
            .collect::<PyResult<_>>()?;
        let pos_refs: Vec<&[u8]> = pos.iter().map(|v| v.as_slice()).collect();
        let neg_refs: Vec<&[u8]> = neg.iter().map(|v| v.as_slice()).collect();
        RustStacked::build(&pos_refs, &neg_refs, fp)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn contains(&self, item: &Bound<'_, PyAny>) -> PyResult<bool> {
        Ok(self.inner.contains(&python_item_to_bytes(item)?))
    }
    fn __repr__(&self) -> String {
        "StackedFilter()".to_string()
    }
}
