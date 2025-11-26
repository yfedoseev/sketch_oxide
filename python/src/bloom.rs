//! Python bindings for BloomFilter membership testing

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList};
use sketch_oxide::membership::BloomFilter as RustBloomFilter;

/// Bloom Filter for probabilistic membership testing
///
/// Classic probabilistic data structure for set membership queries.
/// Supports dynamic insertions, making it ideal for streaming data.
///
/// Args:
///     n: Expected number of elements
///     fpr: Desired false positive rate (0.0 to 1.0), default 0.01 (1%)
///
/// Example:
///     >>> filter = BloomFilter(1000, fpr=0.01)
///     >>> filter.insert(b"key1")
///     >>> filter.insert(b"key2")
///     >>> assert filter.contains(b"key1")
///     >>> assert filter.contains(b"key2")
///     >>> # False positives possible but rare
///     >>> filter.contains(b"missing")  # Might return True with ~1% probability
///
/// Notes:
///     - Supports incremental inserts (unlike BinaryFuseFilter)
///     - Zero false negatives guaranteed
///     - Space: ~10 bits per item at 1% FPR
///     - Use BinaryFuseFilter for better space efficiency if all items known upfront
#[pyclass(module = "sketch_oxide")]
pub struct BloomFilter {
    inner: RustBloomFilter,
}

#[pymethods]
impl BloomFilter {
    /// Create a new Bloom Filter
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
            inner: RustBloomFilter::new(n, fpr),
        })
    }

    /// Insert a key into the filter
    ///
    /// Args:
    ///     key: Bytes to insert
    ///
    /// Example:
    ///     >>> filter = BloomFilter(1000)
    ///     >>> filter.insert(b"my_key")
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

    /// Merge another Bloom filter into this one (union operation)
    ///
    /// Args:
    ///     other: Another BloomFilter with same parameters
    ///
    /// Raises:
    ///     ValueError: If filters have different sizes
    fn merge(&mut self, other: &BloomFilter) -> PyResult<()> {
        if self.inner.params() != other.inner.params() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Bloom filters must have same parameters to merge",
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
    ///     BloomFilter: Deserialized filter
    ///
    /// Raises:
    ///     ValueError: If bytes are invalid
    #[staticmethod]
    fn from_bytes(data: &[u8]) -> PyResult<Self> {
        RustBloomFilter::from_bytes(data)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        let (n, m, k) = self.inner.params();
        format!(
            "BloomFilter(n={}, bits={}, hashes={}, fp_rate={:.4}%)",
            n,
            m,
            k,
            self.inner.false_positive_rate() * 100.0
        )
    }

    fn __str__(&self) -> String {
        format!("BloomFilter({} bits)", self.inner.params().1)
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn __contains__(&self, key: &[u8]) -> bool {
        self.inner.contains(key)
    }

    /// Insert multiple keys into the filter in a single call (optimized for throughput)
    ///
    /// Batch inserts are significantly faster than multiple individual insert() calls
    /// because they amortize the FFI (Foreign Function Interface) overhead across
    /// many items. This is the preferred method when adding large quantities of data.
    ///
    /// Args:
    ///     keys: Iterable of byte strings to insert
    ///
    /// Example:
    ///     >>> filter = BloomFilter(1000)
    ///     >>> filter.insert_batch([b"key1", b"key2", b"key3"])
    fn insert_batch(&mut self, keys: &Bound<'_, PyAny>) -> PyResult<()> {
        let keys_list: &Bound<'_, PyList> = keys.downcast()?;
        for key in keys_list {
            let key_bytes: &[u8] = key.extract()?;
            self.inner.insert(key_bytes);
        }
        Ok(())
    }

    /// Check multiple keys with a single call (optimized for lookups)
    ///
    /// Batch contains checks are faster than multiple individual contains() calls.
    ///
    /// Args:
    ///     keys: Iterable of byte strings to check
    ///
    /// Returns:
    ///     list: List of booleans, one for each key
    ///
    /// Example:
    ///     >>> filter = BloomFilter(1000)
    ///     >>> results = filter.contains_batch([b"key1", b"missing"])
    fn contains_batch(&self, keys: &Bound<'_, PyAny>) -> PyResult<Vec<bool>> {
        let keys_list: &Bound<'_, PyList> = keys.downcast()?;
        let mut results = Vec::new();
        for key in keys_list {
            let key_bytes: &[u8] = key.extract()?;
            results.push(self.inner.contains(key_bytes));
        }
        Ok(results)
    }
}
