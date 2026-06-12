//! Python bindings for sliding-window uniform sampling.

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::sampling::SlidingWindowSample as RustSlidingWindowSample;

/// SlidingWindowSample — maintains a single uniform sample drawn from the most
/// recent `window` items of a stream.
///
/// Args:
///     window (int): number of most-recent items eligible for sampling.
///     seed (int, optional): RNG seed for reproducibility.
#[pyclass(module = "sketch_oxide")]
pub struct SlidingWindowSample {
    inner: RustSlidingWindowSample<Vec<u8>>,
}

#[pymethods]
impl SlidingWindowSample {
    #[new]
    #[pyo3(signature = (window, seed=None))]
    fn new(window: u64, seed: Option<u64>) -> PyResult<Self> {
        let res = match seed {
            Some(s) => RustSlidingWindowSample::with_seed(window, s),
            None => RustSlidingWindowSample::new(window),
        };
        res.map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Pushes an item (int, str, bytes, or float) onto the stream.
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.push(python_item_to_bytes(item)?);
        Ok(())
    }

    /// Returns the current sampled item as `bytes`, or None if the window is empty.
    fn sample(&self, py: Python<'_>) -> Option<Py<PyBytes>> {
        self.inner
            .sample()
            .map(|v| PyBytes::new_bound(py, v).unbind())
    }

    /// Total number of items pushed.
    fn count(&self) -> u64 {
        self.inner.count()
    }

    /// Configured window length.
    fn window(&self) -> u64 {
        self.inner.window()
    }

    /// Number of items currently retained in the window.
    fn retained(&self) -> usize {
        self.inner.retained()
    }

    fn __repr__(&self) -> String {
        format!(
            "SlidingWindowSample(window={}, count={})",
            self.inner.window(),
            self.inner.count()
        )
    }
}
