//! Python bindings for SplineSketch quantile estimation

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::quantiles::SplineSketch as RustSplineSketch;
use sketch_oxide::{Mergeable, Sketch};

/// SplineSketch for high-accuracy quantile estimation with monotone cubic spline interpolation
///
/// SplineSketch provides 2-20x better accuracy than T-Digest on non-skewed data using
/// piecewise monotone cubic spline interpolation instead of linear interpolation.
/// Ideal for latency monitoring and precise percentile estimation.
///
/// Args:
///     max_samples (int): Maximum number of samples to retain (default: 200)
///         - Higher values = better accuracy but more memory
///         - 200: Recommended balance
///         - 500+: High-precision quantiles
///
/// Example:
///     >>> spline = SplineSketch(max_samples=200)
///     >>> for latency in latencies:
///     ...     spline.update(latency, 1.0)
///     >>> p50 = spline.query(0.5)
///     >>> p99 = spline.query(0.99)
///
/// Notes:
///     - Works with unsigned 64-bit integers
///     - Supports weighted values
///     - Fully mergeable with other SplineSketches
///     - Space: O(max_samples)
///     - Query time: O(1)
///     - Update time: O(log n)
#[pyclass(module = "sketch_oxide")]
pub struct SplineSketch {
    inner: RustSplineSketch,
}

#[pymethods]
impl SplineSketch {
    /// Create a new SplineSketch with specified maximum sample count
    ///
    /// Args:
    ///     max_samples: Maximum samples to retain (default: 200)
    ///
    /// Example:
    ///     >>> spline = SplineSketch(max_samples=200)
    #[new]
    #[pyo3(signature = (max_samples=200))]
    fn new(max_samples: usize) -> Self {
        Self {
            inner: RustSplineSketch::new(max_samples),
        }
    }

    /// Update the sketch with a weighted value
    ///
    /// Args:
    ///     value: Integer value to add (u64)
    ///     weight: Weight of this value (default: 1.0)
    ///
    /// Example:
    ///     >>> spline = SplineSketch(200)
    ///     >>> spline.update(100, 1.0)
    ///     >>> spline.update(200, 2.0)  # Value 200 has twice the weight
    fn update(&mut self, value: u64, weight: f64) {
        self.inner.update(value, weight);
    }

    /// Estimate a quantile value
    ///
    /// Args:
    ///     quantile: Quantile to estimate (0.0 to 1.0)
    ///         - 0.5: median
    ///         - 0.95: 95th percentile
    ///         - 0.99: 99th percentile
    ///
    /// Returns:
    ///     int: Estimated quantile value
    ///
    /// Raises:
    ///     RuntimeError: If sketch is empty
    ///
    /// Example:
    ///     >>> spline = SplineSketch(200)
    ///     >>> for i in range(1, 1001):
    ///     ...     spline.update(i, 1.0)
    ///     >>> median = spline.query(0.5)
    ///     >>> p99 = spline.query(0.99)
    fn query(&self, quantile: f64) -> PyResult<u64> {
        if self.inner.is_empty() {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Cannot query empty sketch",
            ));
        }
        Ok(self.inner.query(quantile))
    }

    /// Merge another SplineSketch into this one
    ///
    /// Args:
    ///     other: Another SplineSketch with the same max_samples
    ///
    /// Raises:
    ///     ValueError: If sketches have different max_samples
    ///
    /// Example:
    ///     >>> spline1 = SplineSketch(200)
    ///     >>> spline2 = SplineSketch(200)
    ///     >>> # ... add data to both ...
    ///     >>> spline1.merge(spline2)  # Combine sketches
    fn merge(&mut self, other: &SplineSketch) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Reset the sketch to empty state
    ///
    /// Example:
    ///     >>> spline = SplineSketch(200)
    ///     >>> spline.update(100, 1.0)
    ///     >>> spline.reset()
    ///     >>> assert spline.is_empty()
    fn reset(&mut self) {
        self.inner.reset();
    }

    /// Serialize the sketch to bytes
    ///
    /// Returns:
    ///     bytes: Serialized sketch data
    ///
    /// Example:
    ///     >>> spline = SplineSketch(200)
    ///     >>> # ... add data ...
    ///     >>> data = spline.serialize()
    fn serialize<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new_bound(py, &self.inner.serialize())
    }

    /// Deserialize a sketch from bytes
    ///
    /// Args:
    ///     data: Serialized sketch data
    ///
    /// Returns:
    ///     SplineSketch: Restored sketch
    ///
    /// Raises:
    ///     ValueError: If data is invalid
    ///
    /// Example:
    ///     >>> data = spline.serialize()
    ///     >>> restored = SplineSketch.deserialize(data)
    #[staticmethod]
    fn deserialize(data: &[u8]) -> PyResult<Self> {
        RustSplineSketch::deserialize(data)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Get the number of samples currently held
    fn sample_count(&self) -> usize {
        self.inner.sample_count()
    }

    /// Get the total weight (sum of all weighted values)
    fn total_weight(&self) -> f64 {
        self.inner.total_weight()
    }

    /// Get the minimum value observed
    ///
    /// Returns:
    ///     int or None: Minimum value, or None if empty
    fn min(&self) -> Option<u64> {
        self.inner.min()
    }

    /// Get the maximum value observed
    ///
    /// Returns:
    ///     int or None: Maximum value, or None if empty
    fn max(&self) -> Option<u64> {
        self.inner.max()
    }

    /// Get the maximum number of samples this sketch can retain
    fn max_samples(&self) -> usize {
        self.inner.max_samples()
    }

    /// Check if the sketch is empty
    ///
    /// Returns:
    ///     bool: True if no values have been added
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn __repr__(&self) -> String {
        format!(
            "SplineSketch(max_samples={}, samples={}, weight={:.1})",
            self.inner.max_samples(),
            self.inner.sample_count(),
            self.inner.total_weight()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "SplineSketch: {} samples (max {}), weight {:.1}",
            self.inner.sample_count(),
            self.inner.max_samples(),
            self.inner.total_weight()
        )
    }

    fn __len__(&self) -> usize {
        self.inner.sample_count()
    }
}
