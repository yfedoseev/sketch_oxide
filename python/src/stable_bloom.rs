//! Python bindings for StableBloomFilter for unbounded stream deduplication

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::membership::StableBloomFilter as RustStableBloomFilter;

/// Stable Bloom Filter for duplicate detection in unbounded streams
///
/// Maintains bounded false positive rates for infinite streams by continuously
/// evicting stale information through random counter decrements.
///
/// Args:
///     expected_items: Expected number of recent items to track
///     fpr: Target false positive rate (0.0 to 1.0), default 0.01 (1%)
///
/// Example:
///     >>> filter = StableBloomFilter(10000, fpr=0.01)
///     >>> filter.insert(b"item1")
///     >>> filter.insert(b"item2")
///     >>> # Recently inserted items are likely present
///     >>> assert filter.contains(b"item1")
///     >>> # Older items gradually decay and may become false negatives
///
/// Use Cases:
///     - Deduplication in unbounded data streams
///     - Rate limiting with natural decay
///     - Network packet filtering
///     - Duplicate URL detection in web crawlers
///
/// Notes:
///     - Unlike standard Bloom filters, does NOT saturate over time
///     - Achieves stable state where FPR remains bounded forever
///     - Old items naturally "decay" and may report false negatives
///     - No explicit delete operation - decay is automatic
#[pyclass(module = "sketch_oxide")]
pub struct StableBloomFilter {
    inner: RustStableBloomFilter,
}

#[pymethods]
impl StableBloomFilter {
    /// Create a new Stable Bloom Filter
    ///
    /// Args:
    ///     expected_items: Expected number of recent items to track
    ///     fpr: Target false positive rate (0.0 to 1.0), default 0.01 (1%)
    ///
    /// Raises:
    ///     ValueError: If parameters are invalid
    #[new]
    #[pyo3(signature = (expected_items, fpr=0.01))]
    fn new(expected_items: usize, fpr: f64) -> PyResult<Self> {
        RustStableBloomFilter::new(expected_items, fpr)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Insert an item into the filter
    ///
    /// This also randomly decrements some counters to maintain stability.
    ///
    /// Args:
    ///     key: Bytes to insert
    fn insert(&mut self, key: &[u8]) {
        self.inner.insert(key);
    }

    /// Check if an item might be present
    ///
    /// Args:
    ///     key: Bytes to check
    ///
    /// Returns:
    ///     bool: True if item might be present (recently inserted),
    ///           False if definitely not present or has decayed
    fn contains(&self, key: &[u8]) -> bool {
        self.inner.contains(key)
    }

    /// Get the count value for an item (higher = more recent)
    ///
    /// Args:
    ///     key: Bytes to check
    ///
    /// Returns:
    ///     int: Minimum counter value, higher suggests more recent insertion
    fn get_count(&self, key: &[u8]) -> u8 {
        self.inner.get_count(key)
    }

    /// Get the number of counters
    fn num_counters(&self) -> usize {
        self.inner.num_counters()
    }

    /// Get the number of hash functions
    fn num_hashes(&self) -> usize {
        self.inner.num_hashes()
    }

    /// Get the number of counters decremented per insert
    fn decrement_count(&self) -> usize {
        self.inner.decrement_count()
    }

    /// Get the fill ratio (fraction of non-zero counters)
    ///
    /// Returns:
    ///     float: Ratio of non-zero counters (0.0 to 1.0)
    fn fill_ratio(&self) -> f64 {
        self.inner.fill_ratio()
    }

    /// Clear all counters
    fn clear(&mut self) {
        self.inner.clear();
    }

    /// Get memory usage in bytes
    fn memory_usage(&self) -> usize {
        self.inner.memory_usage()
    }

    /// Serialize the filter to bytes
    fn to_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new_bound(py, &self.inner.to_bytes())
    }

    /// Deserialize a filter from bytes
    #[staticmethod]
    fn from_bytes(data: &[u8]) -> PyResult<Self> {
        RustStableBloomFilter::from_bytes(data)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "StableBloomFilter(counters={}, hashes={}, fill={:.1}%)",
            self.inner.num_counters(),
            self.inner.num_hashes(),
            self.inner.fill_ratio() * 100.0
        )
    }

    fn __contains__(&self, key: &[u8]) -> bool {
        self.inner.contains(key)
    }
}
