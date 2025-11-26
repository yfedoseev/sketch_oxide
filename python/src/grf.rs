//! Python bindings for GRF (Gorilla Range Filter)

use pyo3::prelude::*;
use sketch_oxide::range_filters::GRF as RustGRF;

/// GRF (Gorilla Range Filter): Shape-Based Range Filter for LSM-Trees (SIGMOD 2024)
///
/// Advanced range filter optimized for LSM-tree workloads. Uses shape encoding
/// to capture key distribution patterns, providing 30-50% better FPR than
/// traditional range filters for skewed data.
///
/// Args:
///     keys (list): List of integer keys to build the filter from (must not be empty)
///     bits_per_key (int): Number of bits per key (2-16, typically 4-8)
///         - 4: Compact, ~6% FPR
///         - 6: Balanced, ~1.5% FPR (recommended)
///         - 8: Accurate, ~0.4% FPR
///
/// Example:
///     >>> # Build from Zipf-distributed keys
///     >>> keys = [1] * 100 + [2] * 50 + [3] * 25 + list(range(4, 20))
///     >>> grf = GRF(keys, bits_per_key=6)
///     >>> assert grf.may_contain_range(1, 3)  # Contains heavy keys
///     >>> assert grf.may_contain(1)  # Point query
///
/// Key Advantages:
///     - 30-50% better FPR for skewed distributions (Zipf, power-law)
///     - Adaptive segmentation matches data patterns
///     - Optimized for LSM-tree compaction and merge
///     - Comparable space to Grafite
///     - Better cache performance
///
/// Production Use Cases:
///     - RocksDB/LevelDB SSTable filters
///     - Time-series databases (InfluxDB, TimescaleDB)
///     - Log aggregation (Elasticsearch, Loki)
///     - Financial time-series data
///     - Columnar databases (Parquet, ORC)
///
/// Performance:
///     - Build: O(n log n) for sorting + O(n) for segmentation
///     - Query: O(log n) binary search + O(k) segment checks
///     - Space: B bits per key
///
/// Notes:
///     - Keys are automatically sorted and deduplicated
///     - No false negatives (if returns False, range definitely empty)
///     - May have false positives based on bits_per_key
///     - Thread-safe for concurrent queries
#[pyclass(module = "sketch_oxide")]
pub struct GRF {
    inner: RustGRF,
}

#[pymethods]
impl GRF {
    /// Build a GRF filter from a list of keys
    ///
    /// Args:
    ///     keys: List of integer keys (will be sorted and deduplicated)
    ///     bits_per_key: Number of bits per key (2-16, typically 4-8)
    ///
    /// Raises:
    ///     ValueError: If keys is empty or bits_per_key is invalid
    ///
    /// Example:
    ///     >>> keys = [10, 20, 30, 40, 50]
    ///     >>> grf = GRF(keys, bits_per_key=6)
    #[new]
    fn new(keys: Vec<u64>, bits_per_key: usize) -> PyResult<Self> {
        RustGRF::build(&keys, bits_per_key)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Check if a range of values might be in the filter
    ///
    /// Args:
    ///     low: Lower bound of range (inclusive)
    ///     high: Upper bound of range (inclusive)
    ///
    /// Returns:
    ///     bool: True if range might contain keys, False if definitely does not
    ///
    /// Example:
    ///     >>> keys = [10, 20, 30, 40, 50]
    ///     >>> grf = GRF(keys, bits_per_key=6)
    ///     >>> assert grf.may_contain_range(15, 25)  # Contains 20
    ///     >>> assert grf.may_contain_range(10, 50)  # Full range
    fn may_contain_range(&self, low: u64, high: u64) -> bool {
        use sketch_oxide::common::RangeFilter;
        self.inner.may_contain_range(low, high)
    }

    /// Check if a single key may be in the filter (point query)
    ///
    /// Args:
    ///     key: Key to check
    ///
    /// Returns:
    ///     bool: True if key might be present, False if definitely not present
    ///
    /// Example:
    ///     >>> keys = [10, 20, 30, 40, 50]
    ///     >>> grf = GRF(keys, bits_per_key=6)
    ///     >>> assert grf.may_contain(20)
    ///     >>> assert grf.may_contain(10)
    fn may_contain(&self, key: u64) -> bool {
        self.inner.may_contain(key)
    }

    /// Calculate expected FPR for a given range width
    ///
    /// GRF's FPR adapts to distribution. For skewed data, typically
    /// better than the theoretical Grafite bound.
    ///
    /// Args:
    ///     range_width: Width of the query range
    ///
    /// Returns:
    ///     float: Expected false positive rate (0.0 to 1.0)
    ///
    /// Example:
    ///     >>> keys = [10, 20, 30, 40, 50]
    ///     >>> grf = GRF(keys, bits_per_key=6)
    ///     >>> fpr = grf.expected_fpr(10)
    ///     >>> assert 0.0 <= fpr <= 1.0
    fn expected_fpr(&self, range_width: u64) -> f64 {
        self.inner.expected_fpr(range_width)
    }

    /// Get the number of keys in the filter
    ///
    /// Returns:
    ///     int: Number of unique keys
    ///
    /// Example:
    ///     >>> keys = [10, 20, 20, 30]  # Duplicate
    ///     >>> grf = GRF(keys, bits_per_key=6)
    ///     >>> assert grf.key_count() == 3  # Deduplicated
    fn key_count(&self) -> usize {
        self.inner.key_count()
    }

    /// Get the bits per key configuration
    ///
    /// Returns:
    ///     int: Bits per key
    fn bits_per_key(&self) -> usize {
        self.inner.bits_per_key()
    }

    /// Get the number of segments
    ///
    /// Returns:
    ///     int: Number of shape-based segments created
    ///
    /// Example:
    ///     >>> keys = list(range(100))
    ///     >>> grf = GRF(keys, bits_per_key=6)
    ///     >>> segs = grf.segment_count()
    ///     >>> print(f"Created {segs} segments")
    fn segment_count(&self) -> usize {
        self.inner.segment_count()
    }

    /// Get filter statistics
    ///
    /// Returns:
    ///     dict: Dictionary with statistics
    ///         - key_count: Number of unique keys
    ///         - segment_count: Number of segments
    ///         - avg_keys_per_segment: Average keys per segment
    ///         - bits_per_key: Bits per key configuration
    ///         - total_bits: Total bits used
    ///         - memory_bytes: Memory overhead in bytes
    ///
    /// Example:
    ///     >>> keys = list(range(1000))
    ///     >>> grf = GRF(keys, bits_per_key=6)
    ///     >>> stats = grf.stats()
    ///     >>> print(f"Keys: {stats['key_count']}, Segments: {stats['segment_count']}")
    ///     >>> print(f"Memory: {stats['memory_bytes']} bytes")
    fn stats(&self) -> PyResult<Py<PyAny>> {
        let stats = self.inner.stats();
        Python::with_gil(|py| {
            let dict = pyo3::types::PyDict::new_bound(py);
            dict.set_item("key_count", stats.key_count)?;
            dict.set_item("segment_count", stats.segment_count)?;
            dict.set_item("avg_keys_per_segment", stats.avg_keys_per_segment)?;
            dict.set_item("bits_per_key", stats.bits_per_key)?;
            dict.set_item("total_bits", stats.total_bits)?;
            dict.set_item("memory_bytes", stats.memory_bytes)?;
            Ok(dict.into())
        })
    }

    fn __repr__(&self) -> String {
        let stats = self.inner.stats();
        format!(
            "GRF(keys={}, segments={}, bits_per_key={})",
            stats.key_count, stats.segment_count, stats.bits_per_key
        )
    }

    fn __str__(&self) -> String {
        let stats = self.inner.stats();
        format!(
            "GRF({} keys in {} segments, {} bits/key)",
            stats.key_count, stats.segment_count, stats.bits_per_key
        )
    }
}
