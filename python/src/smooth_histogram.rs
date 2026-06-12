//! Python bindings for the smooth-histogram sliding-window sum.

use pyo3::prelude::*;
use sketch_oxide::streaming::SmoothHistogramSum as RustSmoothHistogramSum;

/// SmoothHistogramSum — sliding-window sum over a stream using the smooth
/// histogram framework (Braverman–Ostrovsky), with relative error `epsilon`.
///
/// Args:
///     epsilon (float): relative error bound.
///     window (int): sliding window length.
#[pyclass(module = "sketch_oxide")]
pub struct SmoothHistogramSum {
    inner: RustSmoothHistogramSum,
}

#[pymethods]
impl SmoothHistogramSum {
    #[new]
    fn new(epsilon: f64, window: usize) -> PyResult<Self> {
        RustSmoothHistogramSum::new(epsilon, window)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Records a (non-negative) value into the window.
    fn update(&mut self, value: f64) -> PyResult<()> {
        self.inner
            .update(value)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Estimated sum over the current window.
    fn query(&self) -> f64 {
        self.inner.query()
    }

    /// Number of checkpoints retained.
    fn num_checkpoints(&self) -> usize {
        self.inner.num_checkpoints()
    }

    /// Number of values processed in the window.
    fn count(&self) -> usize {
        self.inner.count()
    }

    fn __repr__(&self) -> String {
        format!(
            "SmoothHistogramSum(query={:.3}, checkpoints={})",
            self.inner.query(),
            self.inner.num_checkpoints()
        )
    }
}
