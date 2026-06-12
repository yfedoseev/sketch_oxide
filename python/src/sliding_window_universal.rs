//! Python bindings for sliding-window universal (L2 / frequency-moment) sketching.

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::streaming::SlidingWindowUniversal as RustSlidingWindowUniversal;

/// SlidingWindowUniversal — universal sketch over the most recent `window` items,
/// supporting frequency-moment / L2-norm estimation that expires by block age.
///
/// Args:
///     window (int): number of recent items to retain.
///     block (int): items per block (expiry granularity).
///     epsilon (float): relative error target.
///     delta (float): failure probability.
#[pyclass(module = "sketch_oxide")]
pub struct SlidingWindowUniversal {
    inner: RustSlidingWindowUniversal,
}

#[pymethods]
impl SlidingWindowUniversal {
    #[new]
    fn new(window: usize, block: usize, epsilon: f64, delta: f64) -> PyResult<Self> {
        RustSlidingWindowUniversal::new(window, block, epsilon, delta)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Records one occurrence of an item (int, str, bytes, or float).
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner
            .update(&python_item_to_bytes(item)?)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Estimated L2 norm (second frequency moment, square-rooted) over the window.
    fn estimate_l2(&self) -> f64 {
        self.inner.estimate_l2()
    }

    /// Number of blocks currently retained.
    fn retained_blocks(&self) -> usize {
        self.inner.retained_blocks()
    }

    /// Configured block size.
    fn block_size(&self) -> usize {
        self.inner.block_size()
    }

    fn __repr__(&self) -> String {
        format!(
            "SlidingWindowUniversal(retained_blocks={}, l2={:.3})",
            self.inner.retained_blocks(),
            self.inner.estimate_l2()
        )
    }
}
