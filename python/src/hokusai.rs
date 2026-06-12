//! Python bindings for the Hokusai time-aggregated frequency sketch.

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::streaming::Hokusai as RustHokusai;

/// Hokusai — time-adaptive sketching: maintains Count-Min sketches at multiple
/// time resolutions so older intervals are stored at coarser granularity.
///
/// Args:
///     levels (int): number of time-resolution levels.
///     width (int): counters per row.
///     depth (int): number of hash rows.
#[pyclass(module = "sketch_oxide")]
pub struct Hokusai {
    inner: RustHokusai,
}

#[pymethods]
impl Hokusai {
    #[new]
    fn new(levels: u32, width: usize, depth: usize) -> PyResult<Self> {
        RustHokusai::new(levels, width, depth)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Records one occurrence of an item (int, str, bytes, or float).
    fn add(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.add(&python_item_to_bytes(item)?);
        Ok(())
    }

    /// Records `count` occurrences of an item.
    fn add_count(&mut self, item: &Bound<'_, PyAny>, count: u64) -> PyResult<()> {
        self.inner.add_count(&python_item_to_bytes(item)?, count);
        Ok(())
    }

    /// Advances to the next time interval, ageing older levels.
    fn tick(&mut self) {
        self.inner.tick();
    }

    /// Estimated count for an item at a given time-resolution level.
    fn estimate_window(&self, item: &Bound<'_, PyAny>, level: u32) -> PyResult<u64> {
        Ok(self
            .inner
            .estimate_window(&python_item_to_bytes(item)?, level))
    }

    /// Estimated count for an item in the current interval.
    fn estimate_current(&self, item: &Bound<'_, PyAny>) -> PyResult<u64> {
        Ok(self.inner.estimate_current(&python_item_to_bytes(item)?))
    }

    /// Number of intervals spanned by a given level.
    fn window_span(&self, level: u32) -> u64 {
        self.inner.window_span(level)
    }

    /// Current internal time (number of ticks).
    fn time(&self) -> u64 {
        self.inner.time()
    }

    /// Number of time-resolution levels.
    fn levels(&self) -> u32 {
        self.inner.levels()
    }

    fn __repr__(&self) -> String {
        format!(
            "Hokusai(levels={}, time={})",
            self.inner.levels(),
            self.inner.time()
        )
    }
}
