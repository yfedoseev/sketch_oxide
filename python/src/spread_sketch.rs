//! Python bindings for SpreadSketch (super-spreader detection).
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::frequency::SpreadSketch as RustSpreadSketch;

/// SpreadSketch — detects super-spreaders (sources contacting many distinct destinations).
#[pyclass(module = "sketch_oxide")]
pub struct SpreadSketch {
    inner: RustSpreadSketch,
}

#[pymethods]
impl SpreadSketch {
    #[new]
    fn new(depth: usize, width: usize, precision: u8) -> PyResult<Self> {
        RustSpreadSketch::new(depth, width, precision)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    /// Records that `src` contacted `dst` (both int/str/bytes/float).
    fn update(&mut self, src: &Bound<'_, PyAny>, dst: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner
            .update(&python_item_to_bytes(src)?, &python_item_to_bytes(dst)?);
        Ok(())
    }
    /// Estimated spread (number of distinct destinations) of `src`.
    fn spread(&self, src: &Bound<'_, PyAny>) -> PyResult<f64> {
        Ok(self.inner.spread(&python_item_to_bytes(src)?))
    }
    /// Whether `src`'s spread exceeds `threshold`.
    fn is_superspreader(&self, src: &Bound<'_, PyAny>, threshold: f64) -> PyResult<bool> {
        Ok(self
            .inner
            .is_superspreader(&python_item_to_bytes(src)?, threshold))
    }
    fn __repr__(&self) -> String {
        "SpreadSketch()".to_string()
    }
}
