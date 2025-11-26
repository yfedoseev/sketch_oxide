//! Python bindings for Sliding HyperLogLog time-windowed cardinality estimation

use pyo3::prelude::*;
use sketch_oxide::streaming::SlidingHyperLogLog as RustSlidingHyperLogLog;
use sketch_oxide::{Mergeable, Sketch};

/// SlidingHyperLogLog: Time-windowed Cardinality Estimation (Chabchoub et al., 2010)
///
/// Extends classic HyperLogLog with temporal awareness for cardinality estimation
/// over sliding time windows. Essential for real-time analytics, DDoS detection,
/// and streaming applications.
///
/// Args:
///     precision (int): Precision parameter (4-16), higher = more accurate but more memory
///         - precision 4: 16 registers, ~144 bytes, ~26% error
///         - precision 8: 256 registers, ~2.3 KB, ~6.5% error
///         - precision 10: 1024 registers, ~9.2 KB, ~3.25% error
///         - precision 12: 4096 registers, ~36 KB, ~1.6% error (recommended)
///         - precision 14: 16384 registers, ~147 KB, ~0.8% error
///         - precision 16: 65536 registers, ~590 KB, ~0.4% error
///     max_window_seconds (int): Maximum window size in seconds
///
/// Example:
///     >>> # Create sketch with precision 12, 1-hour max window
///     >>> hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
///     >>>
///     >>> # Add items with timestamps
///     >>> hll.update("user_123", timestamp=1000)
///     >>> hll.update("user_456", timestamp=1030)
///     >>> hll.update("user_789", timestamp=1060)
///     >>>
///     >>> # Estimate cardinality in last 60 seconds
///     >>> estimate = hll.estimate_window(current_time=1060, window_seconds=60)
///     >>> print(f"Unique items in window: {estimate:.0f}")
///     >>>
///     >>> # Decay old entries
///     >>> hll.decay(current_time=2000, window_seconds=600)
///
/// Production Use Cases (2025):
///     - Real-time dashboards: Unique users in last N minutes
///     - DDoS detection: Unique source IPs in sliding window
///     - Network telemetry: Unique flows over time
///     - CDN analytics: Geographic distribution over time
///     - Streaming aggregation: Time-windowed distinct counts
///
/// Performance:
///     - Update: O(1)
///     - Window Query: O(m) where m = 2^precision
///     - Decay: O(m)
///     - Merge: O(m)
///     - Space: ~9 bytes per register (e.g., 36KB for precision 12)
#[pyclass(module = "sketch_oxide")]
pub struct SlidingHyperLogLog {
    inner: RustSlidingHyperLogLog,
}

#[pymethods]
impl SlidingHyperLogLog {
    /// Create a new Sliding HyperLogLog sketch
    ///
    /// Args:
    ///     precision: Precision parameter (4-16)
    ///     max_window_seconds: Maximum window size in seconds
    ///
    /// Raises:
    ///     ValueError: If precision < 4 or > 16
    ///
    /// Example:
    ///     >>> # 1-hour window with precision 12
    ///     >>> hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
    #[new]
    fn new(precision: u8, max_window_seconds: u64) -> PyResult<Self> {
        RustSlidingHyperLogLog::new(precision, max_window_seconds)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Update the sketch with an item and timestamp
    ///
    /// Args:
    ///     item: Any hashable item (int, str, bytes, float)
    ///     timestamp: Unix timestamp in seconds (int)
    ///
    /// Example:
    ///     >>> hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
    ///     >>> hll.update("user_id", timestamp=1000)
    ///     >>> hll.update(42, timestamp=1030)
    #[pyo3(signature = (item, timestamp))]
    fn update(&mut self, item: &Bound<'_, PyAny>, timestamp: u64) -> PyResult<()> {
        use std::hash::{Hash, Hasher};
        use twox_hash::XxHash64;

        // Hash the item manually since hash_item is private
        let hash = if let Ok(val) = item.extract::<i64>() {
            let mut hasher = XxHash64::with_seed(0);
            val.hash(&mut hasher);
            hasher.finish()
        } else if let Ok(val) = item.extract::<u64>() {
            let mut hasher = XxHash64::with_seed(0);
            val.hash(&mut hasher);
            hasher.finish()
        } else if let Ok(val) = item.extract::<String>() {
            let mut hasher = XxHash64::with_seed(0);
            val.hash(&mut hasher);
            hasher.finish()
        } else if let Ok(val) = item.extract::<f64>() {
            let mut hasher = XxHash64::with_seed(0);
            val.to_bits().hash(&mut hasher);
            hasher.finish()
        } else if let Ok(bytes) = item.downcast::<pyo3::types::PyBytes>() {
            let mut hasher = XxHash64::with_seed(0);
            bytes.as_bytes().hash(&mut hasher);
            hasher.finish()
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Item must be int, str, bytes, or float",
            ));
        };

        self.inner
            .update(&hash, timestamp)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Estimate cardinality over a time window
    ///
    /// Returns the estimated number of unique items observed within the
    /// time window ending at current_time and spanning window_seconds.
    ///
    /// Args:
    ///     current_time: End of the time window (Unix timestamp)
    ///     window_seconds: Size of the window in seconds
    ///
    /// Returns:
    ///     float: Estimated cardinality for the window
    ///
    /// Example:
    ///     >>> hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
    ///     >>> for i in range(1000):
    ///     ...     hll.update(i, timestamp=1000 + i)
    ///     >>> # Estimate for last 100 seconds
    ///     >>> estimate = hll.estimate_window(current_time=1500, window_seconds=100)
    fn estimate_window(&self, current_time: u64, window_seconds: u64) -> f64 {
        self.inner.estimate_window(current_time, window_seconds)
    }

    /// Estimate total cardinality (all history)
    ///
    /// Returns the estimated number of unique items across all time,
    /// ignoring window constraints.
    ///
    /// Returns:
    ///     float: Estimated total cardinality
    ///
    /// Example:
    ///     >>> hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
    ///     >>> for i in range(1000):
    ///     ...     hll.update(i, timestamp=1000)
    ///     >>> total = hll.estimate_total()
    ///     >>> assert abs(total - 1000.0) < 50.0
    fn estimate_total(&self) -> f64 {
        self.inner.estimate_total()
    }

    /// Explicitly decay old entries
    ///
    /// Removes entries older than the specified window. Useful for
    /// memory management and maintaining accuracy.
    ///
    /// Args:
    ///     current_time: Current timestamp
    ///     window_seconds: Window size in seconds
    ///
    /// Example:
    ///     >>> hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
    ///     >>> hll.update("old_item", timestamp=1000)
    ///     >>> # Decay old entries
    ///     >>> hll.decay(current_time=5000, window_seconds=600)
    fn decay(&mut self, current_time: u64, window_seconds: u64) -> PyResult<()> {
        self.inner
            .decay(current_time, window_seconds)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Merge another Sliding HyperLogLog into this one
    ///
    /// Takes the maximum value from each register pair, preserving the timestamp
    /// of the maximum value. Both sketches must have same precision.
    ///
    /// Args:
    ///     other: Another SlidingHyperLogLog with same precision and window size
    ///
    /// Raises:
    ///     ValueError: If sketches have different precisions or window sizes
    ///
    /// Example:
    ///     >>> hll1 = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
    ///     >>> hll2 = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
    ///     >>> for i in range(100):
    ///     ...     hll1.update(i, timestamp=1000)
    ///     >>> for i in range(50, 150):
    ///     ...     hll2.update(i, timestamp=1000)
    ///     >>> hll1.merge(hll2)
    fn merge(&mut self, other: &SlidingHyperLogLog) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Check if the sketch is empty
    ///
    /// Returns:
    ///     bool: True if no items have been added
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get the precision parameter
    ///
    /// Returns:
    ///     int: Precision value (4-16)
    fn precision(&self) -> u8 {
        self.inner.precision()
    }

    /// Get the number of registers
    ///
    /// Returns:
    ///     int: Number of registers (m = 2^precision)
    fn num_registers(&self) -> usize {
        self.inner.num_registers()
    }

    /// Get the standard error of the estimate
    ///
    /// Returns:
    ///     float: Standard error (approximately 1.04 / sqrt(m))
    fn standard_error(&self) -> f64 {
        self.inner.standard_error()
    }

    /// Serialize the sketch to bytes
    ///
    /// Returns:
    ///     bytes: Serialized sketch data
    fn serialize<'py>(&self, py: Python<'py>) -> Bound<'py, pyo3::types::PyBytes> {
        pyo3::types::PyBytes::new_bound(py, &self.inner.serialize())
    }

    /// Deserialize a sketch from bytes
    ///
    /// Args:
    ///     data: Serialized sketch bytes
    ///
    /// Returns:
    ///     SlidingHyperLogLog: Deserialized sketch
    ///
    /// Raises:
    ///     ValueError: If deserialization fails
    #[staticmethod]
    fn deserialize(data: &[u8]) -> PyResult<Self> {
        RustSlidingHyperLogLog::deserialize(data)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Get statistics about the sketch
    ///
    /// Returns:
    ///     dict: Dictionary with statistics
    ///         - precision: Precision parameter
    ///         - max_window_seconds: Maximum window size
    ///         - total_updates: Total number of updates
    ///
    /// Example:
    ///     >>> hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
    ///     >>> stats = hll.stats()
    ///     >>> assert stats['precision'] == 12
    fn stats(&self) -> PyResult<Py<PyAny>> {
        let stats = self.inner.stats();
        Python::with_gil(|py| {
            let dict = pyo3::types::PyDict::new_bound(py);
            dict.set_item("precision", stats.precision)?;
            dict.set_item("max_window_seconds", stats.max_window_seconds)?;
            dict.set_item("total_updates", stats.total_updates)?;
            Ok(dict.into())
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "SlidingHyperLogLog(precision={}, estimate={:.0})",
            self.inner.precision(),
            self.inner.estimate_total()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "SlidingHyperLogLog(precision={}, estimate={:.0})",
            self.inner.precision(),
            self.inner.estimate_total()
        )
    }
}
