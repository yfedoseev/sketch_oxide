//! Python bindings for SlidingWindowCounter

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::streaming::SlidingWindowCounter as RustSlidingWindowCounter;

/// Sliding Window Counter using Exponential Histogram
///
/// Maintains approximate count over a sliding time window using O(log²N) space.
/// Perfect for time-bounded counting, rate limiting, and anomaly detection.
///
/// Args:
///     window_size: Size of the sliding window in time units
///     epsilon: Error bound (0.0 to 1.0), e.g., 0.1 for 10% error
///
/// Example:
///     >>> counter = SlidingWindowCounter(3600, epsilon=0.05)  # 1 hour, 5% error
///     >>> counter.increment(1000)  # Event at time 1000
///     >>> counter.increment(2000)  # Event at time 2000
///     >>> count = counter.count(3000)  # Count events in window
///
/// Use Cases:
///     - Count events in last N seconds/minutes
///     - Rate limiting (requests per time window)
///     - Anomaly detection (sudden spikes)
///     - Moving averages
///
/// Notes:
///     - Error: (1 ± epsilon) * actual_count
///     - Space: O((1/epsilon) * log²(window_size))
///     - Call expire() periodically to free memory from old buckets
#[pyclass(module = "sketch_oxide")]
pub struct SlidingWindowCounter {
    inner: RustSlidingWindowCounter,
}

#[pymethods]
impl SlidingWindowCounter {
    /// Create a new Sliding Window Counter
    ///
    /// Args:
    ///     window_size: Size of the sliding window in time units
    ///     epsilon: Error bound (0.0 to 1.0), default 0.1 (10% error)
    ///
    /// Raises:
    ///     ValueError: If parameters are invalid
    #[new]
    #[pyo3(signature = (window_size, epsilon=0.1))]
    fn new(window_size: u64, epsilon: f64) -> PyResult<Self> {
        RustSlidingWindowCounter::new(window_size, epsilon)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Increment the counter at the given timestamp
    ///
    /// Args:
    ///     timestamp: The time at which the event occurred
    fn increment(&mut self, timestamp: u64) {
        self.inner.increment(timestamp);
    }

    /// Increment by a specific amount at the given timestamp
    ///
    /// Args:
    ///     timestamp: The time at which the events occurred
    ///     count: Number of events to record
    fn increment_by(&mut self, timestamp: u64, count: u64) {
        self.inner.increment_by(timestamp, count);
    }

    /// Get the approximate count within the window ending at the given time
    ///
    /// Args:
    ///     current_time: The current time (end of the window)
    ///
    /// Returns:
    ///     int: Approximate count of events in [current_time - window_size, current_time]
    fn count(&self, current_time: u64) -> u64 {
        self.inner.count(current_time)
    }

    /// Get the count for a specific time range
    ///
    /// Args:
    ///     start: Start of the range (inclusive)
    ///     end: End of the range (inclusive)
    ///
    /// Returns:
    ///     int: Approximate count in the range
    fn count_range(&self, start: u64, end: u64) -> u64 {
        self.inner.count_range(start, end)
    }

    /// Expire old buckets outside the window
    ///
    /// Call this periodically to free memory from old buckets.
    ///
    /// Args:
    ///     current_time: The current time
    fn expire(&mut self, current_time: u64) {
        self.inner.expire(current_time);
    }

    /// Get the window size
    fn window_size(&self) -> u64 {
        self.inner.window_size()
    }

    /// Get the error bound
    fn epsilon(&self) -> f64 {
        self.inner.epsilon()
    }

    /// Get the number of buckets (for diagnostics)
    fn num_buckets(&self) -> usize {
        self.inner.num_buckets()
    }

    /// Clear all buckets
    fn clear(&mut self) {
        self.inner.clear();
    }

    /// Get memory usage in bytes
    fn memory_usage(&self) -> usize {
        self.inner.memory_usage()
    }

    /// Serialize the counter to bytes
    fn to_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new_bound(py, &self.inner.to_bytes())
    }

    /// Deserialize a counter from bytes
    #[staticmethod]
    fn from_bytes(data: &[u8]) -> PyResult<Self> {
        RustSlidingWindowCounter::from_bytes(data)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "SlidingWindowCounter(window={}, epsilon={:.2}%, buckets={})",
            self.inner.window_size(),
            self.inner.epsilon() * 100.0,
            self.inner.num_buckets()
        )
    }
}
