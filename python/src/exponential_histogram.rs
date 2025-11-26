//! Python bindings for ExponentialHistogram streaming window counter

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::streaming::ExponentialHistogram as RustExponentialHistogram;
use sketch_oxide::{Mergeable, Sketch};

/// ExponentialHistogram for time-bounded event counting with error bounds
///
/// Maintains approximate count over a sliding time window using O(log²N) space
/// (Datar et al. 2002). Perfect for rate limiting, anomaly detection, and windowed
/// statistics with guaranteed relative error bounds.
///
/// Args:
///     window_size (int): Size of the sliding window in time units
///     epsilon (float): Error bound (0.0 to 1.0), e.g., 0.1 for 10% error
///         - 0.01: 1% error, more buckets
///         - 0.1: 10% error (recommended)
///         - 0.5: 50% error, minimal buckets
///
/// Example:
///     >>> counter = ExponentialHistogram(3600, epsilon=0.05)  # 1 hour, 5% error
///     >>> counter.insert(1000, 1)  # Event at time 1000
///     >>> counter.insert(2000, 1)  # Event at time 2000
///     >>> est, lower, upper = counter.count(3000)
///     >>> print(f"Count: {est} (bounds: [{lower}, {upper}])")
///
/// Use Cases:
///     - Count requests in last N seconds (rate limiting)
///     - Detect traffic spikes (anomaly detection)
///     - Calculate moving averages
///     - Monitor streaming metrics
///
/// Notes:
///     - Error: (1 ± epsilon) * actual_count
///     - Space: O((1/epsilon) * log²(window_size))
///     - Call expire() periodically to free memory from old buckets
///     - Works with unsigned 64-bit timestamps and counts
#[pyclass(module = "sketch_oxide")]
pub struct ExponentialHistogram {
    inner: RustExponentialHistogram,
}

#[pymethods]
impl ExponentialHistogram {
    /// Create a new Exponential Histogram
    ///
    /// Args:
    ///     window_size: Size of the sliding window in time units
    ///     epsilon: Error bound (0.0 to 1.0), default 0.1 (10% error)
    ///
    /// Raises:
    ///     ValueError: If parameters are invalid
    ///         - window_size must be > 0
    ///         - epsilon must be in (0, 1)
    ///
    /// Example:
    ///     >>> # 60 second window, 5% error
    ///     >>> eh = ExponentialHistogram(60000, epsilon=0.05)
    #[new]
    #[pyo3(signature = (window_size, epsilon=0.1))]
    fn new(window_size: u64, epsilon: f64) -> PyResult<Self> {
        RustExponentialHistogram::new(window_size, epsilon)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Record event(s) at a specific timestamp
    ///
    /// Args:
    ///     timestamp: Time at which the event(s) occurred
    ///     count: Number of events to record (default: 1)
    ///
    /// Example:
    ///     >>> eh = ExponentialHistogram(1000, 0.1)
    ///     >>> eh.insert(100, 1)  # Single event at time 100
    ///     >>> eh.insert(200, 5)  # 5 events at time 200
    fn insert(&mut self, timestamp: u64, count: u64) {
        self.inner.insert(timestamp, count);
    }

    /// Get the approximate count within the sliding window
    ///
    /// Returns the count ending at current_time, covering [current_time - window_size, current_time].
    ///
    /// Args:
    ///     current_time: The current time (end of the window)
    ///
    /// Returns:
    ///     tuple: (estimate, lower_bound, upper_bound)
    ///         - estimate: Best estimate of count
    ///         - lower_bound: Conservative lower bound
    ///         - upper_bound: Optimistic upper bound
    ///
    /// Example:
    ///     >>> eh = ExponentialHistogram(100, 0.1)
    ///     >>> eh.insert(10, 1)
    ///     >>> eh.insert(20, 1)
    ///     >>> est, lower, upper = eh.count(100)
    ///     >>> print(f"Estimate: {est}, Range: [{lower}, {upper}]")
    fn count(&self, current_time: u64) -> (u64, u64, u64) {
        self.inner.count(current_time)
    }

    /// Expire old buckets outside the window
    ///
    /// Removes buckets that are entirely outside the window, keeping at most
    /// one straddling bucket for accuracy. Call this periodically to free memory.
    ///
    /// Args:
    ///     current_time: The current time
    ///
    /// Example:
    ///     >>> eh = ExponentialHistogram(1000, 0.1)
    ///     >>> # ... record events ...
    ///     >>> eh.expire(5000)  # Clean up old buckets
    fn expire(&mut self, current_time: u64) {
        self.inner.expire(current_time);
    }

    /// Merge another ExponentialHistogram into this one
    ///
    /// Combines two histograms that track the same window with same error bounds.
    ///
    /// Args:
    ///     other: Another ExponentialHistogram with same window_size and epsilon
    ///
    /// Raises:
    ///     ValueError: If histograms have incompatible parameters
    ///
    /// Example:
    ///     >>> eh1 = ExponentialHistogram(1000, 0.1)
    ///     >>> eh2 = ExponentialHistogram(1000, 0.1)
    ///     >>> # ... add data to both ...
    ///     >>> eh1.merge(eh2)
    fn merge(&mut self, other: &ExponentialHistogram) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Reset the histogram to empty state
    ///
    /// Example:
    ///     >>> eh = ExponentialHistogram(1000, 0.1)
    ///     >>> eh.insert(100, 5)
    ///     >>> eh.clear()
    ///     >>> assert eh.is_empty()
    fn clear(&mut self) {
        self.inner.clear();
    }

    /// Serialize the histogram to bytes
    ///
    /// Returns:
    ///     bytes: Serialized histogram data
    ///
    /// Example:
    ///     >>> eh = ExponentialHistogram(1000, 0.1)
    ///     >>> eh.insert(100, 5)
    ///     >>> data = eh.serialize()
    fn serialize<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new_bound(py, &self.inner.serialize())
    }

    /// Deserialize a histogram from bytes
    ///
    /// Args:
    ///     data: Serialized histogram data
    ///
    /// Returns:
    ///     ExponentialHistogram: Restored histogram
    ///
    /// Raises:
    ///     ValueError: If data is invalid
    ///
    /// Example:
    ///     >>> data = eh.serialize()
    ///     >>> restored = ExponentialHistogram.deserialize(data)
    #[staticmethod]
    fn deserialize(data: &[u8]) -> PyResult<Self> {
        RustExponentialHistogram::deserialize(data)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Get the window size parameter
    fn window_size(&self) -> u64 {
        self.inner.window_size()
    }

    /// Get the error bound (epsilon) parameter
    fn epsilon(&self) -> f64 {
        self.inner.epsilon()
    }

    /// Get the number of buckets (for diagnostics)
    ///
    /// Returns:
    ///     int: Current number of buckets
    fn num_buckets(&self) -> usize {
        self.inner.num_buckets()
    }

    /// Get the maximum buckets per level (k = ceil(1/epsilon))
    fn k(&self) -> usize {
        self.inner.k()
    }

    /// Get the error bound guarantee
    ///
    /// Returns:
    ///     float: The epsilon value
    fn error_bound(&self) -> f64 {
        self.inner.error_bound()
    }

    /// Get memory usage estimate in bytes
    fn memory_usage(&self) -> usize {
        self.inner.memory_usage()
    }

    /// Check if the histogram is empty
    ///
    /// Returns:
    ///     bool: True if no events have been recorded
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn __repr__(&self) -> String {
        format!(
            "ExponentialHistogram(window={}, epsilon={:.2}%, buckets={})",
            self.inner.window_size(),
            self.inner.epsilon() * 100.0,
            self.inner.num_buckets()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "ExponentialHistogram: window={}, epsilon={:.1}%, {} buckets (k={})",
            self.inner.window_size(),
            self.inner.epsilon() * 100.0,
            self.inner.num_buckets(),
            self.inner.k()
        )
    }
}
