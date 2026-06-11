//! Python bindings for the Flajolet–Martin (PCSA) cardinality sketch.

use crate::common::python_item_to_hash;
use pyo3::prelude::*;
use sketch_oxide::cardinality::FmSketch as RustFmSketch;

/// Flajolet–Martin / PCSA — the original probabilistic distinct-counting sketch.
///
/// Args:
///     m (int): number of bitmaps (relative error ≈ 0.78/√m).
#[pyclass(module = "sketch_oxide")]
pub struct FmSketch {
    inner: RustFmSketch,
}

#[pymethods]
impl FmSketch {
    #[new]
    fn new(m: usize) -> PyResult<Self> {
        RustFmSketch::new(m)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Adds an item (int, str, bytes, or float).
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        let h = python_item_to_hash(item)?;
        self.inner.add(&h.to_le_bytes());
        Ok(())
    }

    /// Estimated number of distinct items.
    fn estimate(&self) -> f64 {
        self.inner.estimate()
    }

    /// Whether no items have been added.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Merges another sketch with the same number of bitmaps.
    fn merge(&mut self, other: &FmSketch) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("FmSketch(estimate={:.0})", self.inner.estimate())
    }
}
