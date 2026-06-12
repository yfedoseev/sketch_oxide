//! Python bindings for the SlidingSketch sliding-window frequency sketch.

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::streaming::SlidingSketch as RustSlidingSketch;

/// SlidingSketch — a generic sliding-window frequency sketch that partitions the
/// window into zones, each a Count-Min sketch, to age out old counts smoothly.
///
/// Args:
///     window (int): sliding window length (time units).
///     zones (int): number of time zones the window is split into.
///     depth (int): Count-Min hash rows per zone.
///     width (int): Count-Min counters per row.
#[pyclass(module = "sketch_oxide")]
pub struct SlidingSketch {
    inner: RustSlidingSketch,
}

#[pymethods]
impl SlidingSketch {
    #[new]
    fn new(window: u64, zones: usize, depth: usize, width: usize) -> PyResult<Self> {
        RustSlidingSketch::new(window, zones, depth, width)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Records one occurrence of an item (int, str, bytes, or float).
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.update(&python_item_to_bytes(item)?);
        Ok(())
    }

    /// Estimated frequency of an item within the current window.
    fn estimate(&self, item: &Bound<'_, PyAny>) -> PyResult<u64> {
        Ok(self.inner.estimate(&python_item_to_bytes(item)?))
    }

    /// Number of time zones.
    fn num_zones(&self) -> usize {
        self.inner.num_zones()
    }

    fn __repr__(&self) -> String {
        format!("SlidingSketch(num_zones={})", self.inner.num_zones())
    }
}
