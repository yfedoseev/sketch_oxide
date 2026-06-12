//! Python bindings for sliding-window quantile estimation.

use pyo3::prelude::*;
use sketch_oxide::streaming::SlidingWindowQuantiles as RustSlidingWindowQuantiles;

/// SlidingWindowQuantiles — approximate quantiles over the most recent `window`
/// values, maintained as a sequence of KLL-style blocks that expire by age.
///
/// Args:
///     window (int): number of recent values to retain.
///     block (int): values per block (expiry granularity).
///     k (int): per-block KLL accuracy parameter.
#[pyclass(module = "sketch_oxide")]
pub struct SlidingWindowQuantiles {
    inner: RustSlidingWindowQuantiles,
}

#[pymethods]
impl SlidingWindowQuantiles {
    #[new]
    fn new(window: usize, block: usize, k: u16) -> PyResult<Self> {
        RustSlidingWindowQuantiles::new(window, block, k)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Records a value into the window.
    fn update(&mut self, value: f64) {
        self.inner.update(value);
    }

    /// Estimated value at quantile `phi` in [0, 1], or None if empty.
    fn quantile(&self, phi: f64) -> Option<f64> {
        self.inner.quantile(phi)
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
            "SlidingWindowQuantiles(retained_blocks={})",
            self.inner.retained_blocks()
        )
    }
}
