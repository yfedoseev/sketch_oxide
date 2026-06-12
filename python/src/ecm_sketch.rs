//! Python bindings for the Exponential Count-Min (ECM) sliding-window sketch.

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::streaming::EcmSketch as RustEcmSketch;

/// EcmSketch — Exponential Count-Min: per-counter exponential histograms give
/// sliding-window frequency estimates over the last `window` time units.
///
/// Args:
///     depth (int): number of hash rows.
///     width (int): number of counters per row.
///     window (int): sliding window length (time units).
///     eh_epsilon (float): per-counter exponential-histogram error.
#[pyclass(module = "sketch_oxide")]
pub struct EcmSketch {
    inner: RustEcmSketch,
}

#[pymethods]
impl EcmSketch {
    #[new]
    fn new(depth: usize, width: usize, window: u64, eh_epsilon: f64) -> PyResult<Self> {
        RustEcmSketch::new(depth, width, window, eh_epsilon)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Records an item (int, str, bytes, or float) at the given timestamp.
    fn update(&mut self, item: &Bound<'_, PyAny>, timestamp: u64) -> PyResult<()> {
        self.inner.update(&python_item_to_bytes(item)?, timestamp);
        Ok(())
    }

    /// Estimated frequency of an item within the window ending at `now`.
    fn estimate(&self, item: &Bound<'_, PyAny>, now: u64) -> PyResult<u64> {
        Ok(self.inner.estimate(&python_item_to_bytes(item)?, now))
    }

    /// Drops buckets that have fallen out of the window at time `now`.
    fn expire(&mut self, now: u64) {
        self.inner.expire(now);
    }

    /// Number of hash rows.
    fn depth(&self) -> usize {
        self.inner.depth()
    }

    /// Number of counters per row.
    fn width(&self) -> usize {
        self.inner.width()
    }

    fn __repr__(&self) -> String {
        format!(
            "EcmSketch(depth={}, width={})",
            self.inner.depth(),
            self.inner.width()
        )
    }
}
