//! Python bindings for the AdaSketch time-adaptive frequency sketch.

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::streaming::AdaSketch as RustAdaSketch;

/// AdaSketch — a time-adaptive Count-Min variant that decays older counts,
/// favouring recent activity.
///
/// Args:
///     depth (int): number of hash rows.
///     width (int): number of counters per row.
///     alpha (float): time-decay rate.
#[pyclass(module = "sketch_oxide")]
pub struct AdaSketch {
    inner: RustAdaSketch,
}

#[pymethods]
impl AdaSketch {
    #[new]
    fn new(depth: usize, width: usize, alpha: f64) -> PyResult<Self> {
        RustAdaSketch::new(depth, width, alpha)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Records one occurrence of an item (int, str, bytes, or float).
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.update(&python_item_to_bytes(item)?);
        Ok(())
    }

    /// Records `count` (possibly fractional) occurrences of an item.
    fn update_weighted(&mut self, item: &Bound<'_, PyAny>, count: f64) -> PyResult<()> {
        self.inner
            .update_weighted(&python_item_to_bytes(item)?, count);
        Ok(())
    }

    /// Time-decayed frequency estimate for an item.
    fn estimate(&self, item: &Bound<'_, PyAny>) -> PyResult<f64> {
        Ok(self.inner.estimate(&python_item_to_bytes(item)?))
    }

    /// Number of hash rows.
    fn depth(&self) -> usize {
        self.inner.depth()
    }

    /// Number of counters per row.
    fn width(&self) -> usize {
        self.inner.width()
    }

    /// Current internal logical time.
    fn time(&self) -> f64 {
        self.inner.time()
    }

    fn __repr__(&self) -> String {
        format!(
            "AdaSketch(depth={}, width={})",
            self.inner.depth(),
            self.inner.width()
        )
    }
}
