//! Python bindings for BlockedBloomFilter membership testing

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::membership::BlockedBloomFilter as RustBlockedBloomFilter;

/// Blocked Bloom Filter for cache-efficient membership testing
///
/// Cache-line-aligned Bloom filter variant that achieves 1 cache miss per query
/// (vs 7+ for standard Bloom). Same space efficiency as standard Bloom (~10 bits/key).
///
/// Args:
///     n: Expected number of elements
///     fpr: Desired false positive rate (0.0 to 1.0), default 0.01 (1%)
///
/// Example:
///     >>> filter = BlockedBloomFilter(10000, fpr=0.01)
///     >>> filter.insert(b"key1")
///     >>> filter.insert(b"key2")
///     >>> assert filter.contains(b"key1")
///     >>> assert filter.contains(b"key2")
///
/// Use cases:
///     - High-throughput query workloads
///     - Memory-bound systems where cache efficiency matters
///     - When query latency consistency is important
///
/// Notes:
///     - Each block is 512 bits (64 bytes = cache line)
///     - All hash lookups within single cache line
///     - Slightly higher FPR than standard Bloom due to block constraints
#[pyclass(module = "sketch_oxide")]
pub struct BlockedBloomFilter {
    inner: RustBlockedBloomFilter,
}

#[pymethods]
impl BlockedBloomFilter {
    /// Create a new Blocked Bloom Filter
    ///
    /// Args:
    ///     n: Expected number of elements
    ///     fpr: False positive rate (0.0 to 1.0), default 0.01 (1%)
    ///
    /// Raises:
    ///     ValueError: If n <= 0 or fpr not in (0, 1)
    #[new]
    #[pyo3(signature = (n, fpr=0.01))]
    fn new(n: usize, fpr: f64) -> PyResult<Self> {
        if n == 0 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Expected number of elements must be > 0",
            ));
        }
        if fpr <= 0.0 || fpr >= 1.0 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "False positive rate must be in (0, 1)",
            ));
        }
        Ok(Self {
            inner: RustBlockedBloomFilter::new(n, fpr),
        })
    }

    /// Insert a key into the filter
    ///
    /// Args:
    ///     key: Bytes to insert
    fn insert(&mut self, key: &[u8]) {
        self.inner.insert(key);
    }

    /// Check if a key might be in the set
    ///
    /// Args:
    ///     key: Bytes to check
    ///
    /// Returns:
    ///     bool: True if key might be in set (possible false positives),
    ///           False if key is definitely not in set (no false negatives)
    fn contains(&self, key: &[u8]) -> bool {
        self.inner.contains(key)
    }

    /// Clear all bits in the filter
    fn clear(&mut self) {
        self.inner.clear();
    }

    /// Merge another Blocked Bloom filter into this one (union operation)
    ///
    /// Args:
    ///     other: Another BlockedBloomFilter with same parameters
    ///
    /// Raises:
    ///     ValueError: If filters have different sizes
    fn merge(&mut self, other: &BlockedBloomFilter) -> PyResult<()> {
        if self.inner.params() != other.inner.params() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Blocked Bloom filters must have same parameters to merge",
            ));
        }
        self.inner.merge(&other.inner);
        Ok(())
    }

    /// Check if the filter is empty
    ///
    /// Returns:
    ///     bool: True if no elements have been inserted
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get the estimated number of inserted elements
    ///
    /// Returns:
    ///     int: Estimated number of elements
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Get the theoretical false positive rate
    ///
    /// Returns:
    ///     float: Current false positive rate based on fill ratio
    fn false_positive_rate(&self) -> f64 {
        self.inner.false_positive_rate()
    }

    /// Get memory usage in bytes
    ///
    /// Returns:
    ///     int: Memory usage in bytes
    fn memory_usage(&self) -> usize {
        self.inner.memory_usage()
    }

    /// Serialize the filter to bytes
    ///
    /// Returns:
    ///     bytes: Serialized filter
    fn to_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new_bound(py, &self.inner.to_bytes())
    }

    /// Deserialize a filter from bytes
    ///
    /// Args:
    ///     data: Serialized filter bytes
    ///
    /// Returns:
    ///     BlockedBloomFilter: Deserialized filter
    ///
    /// Raises:
    ///     ValueError: If bytes are invalid
    #[staticmethod]
    fn from_bytes(data: &[u8]) -> PyResult<Self> {
        RustBlockedBloomFilter::from_bytes(data)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        let (n, num_blocks, k) = self.inner.params();
        format!(
            "BlockedBloomFilter(n={}, blocks={}, hashes={}, fp_rate={:.4}%)",
            n,
            num_blocks,
            k,
            self.inner.false_positive_rate() * 100.0
        )
    }

    fn __str__(&self) -> String {
        let (_, num_blocks, _) = self.inner.params();
        format!("BlockedBloomFilter({} blocks)", num_blocks)
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn __contains__(&self, key: &[u8]) -> bool {
        self.inner.contains(key)
    }
}
