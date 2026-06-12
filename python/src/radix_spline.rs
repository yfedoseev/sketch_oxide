//! Python bindings for the RadixSpline learned index.

use pyo3::prelude::*;
use sketch_oxide::range_filters::RadixSpline as RustRadixSpline;

/// RadixSpline — a single-pass learned index that fits a spline to a sorted key
/// array with a radix table over spline points, giving position estimates within
/// `max_error`.
///
/// Args:
///     keys (list[int]): sorted integer keys.
///     max_error (int): maximum position error of the spline.
///     radix_bits (int): bits used for the radix lookup table.
#[pyclass(module = "sketch_oxide")]
pub struct RadixSpline {
    inner: RustRadixSpline,
}

#[pymethods]
impl RadixSpline {
    #[new]
    fn new(keys: Vec<u64>, max_error: usize, radix_bits: u32) -> PyResult<Self> {
        RustRadixSpline::build(&keys, max_error, radix_bits)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Estimated array position of `key`.
    fn estimate_position(&self, key: u64) -> f64 {
        self.inner.estimate_position(key)
    }

    /// `(lo, hi)` search bound bracketing the true position of `key`.
    fn search_bound(&self, key: u64) -> (usize, usize) {
        self.inner.search_bound(key)
    }

    /// Number of spline points.
    fn num_spline_points(&self) -> usize {
        self.inner.num_spline_points()
    }

    /// Configured maximum position error.
    fn max_error(&self) -> usize {
        self.inner.max_error()
    }

    /// Number of indexed keys.
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the index is empty.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn __repr__(&self) -> String {
        format!(
            "RadixSpline(len={}, spline_points={})",
            self.inner.len(),
            self.inner.num_spline_points()
        )
    }
}
