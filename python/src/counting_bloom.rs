//! Python bindings for CountingBloomFilter membership testing with deletions

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList};
use sketch_oxide::membership::CountingBloomFilter as RustCountingBloomFilter;

/// Counting Bloom Filter for probabilistic membership testing with deletions
///
/// Extension of classic Bloom filter that supports deletions using 4-bit counters.
/// Each position has a counter instead of a single bit, allowing decrement on removal.
///
/// Args:
///     n: Expected number of elements
///     fpr: Desired false positive rate (0.0 to 1.0), default 0.01 (1%)
///
/// Example:
///     >>> filter = CountingBloomFilter(1000, fpr=0.01)
///     >>> filter.insert(b"key1")
///     >>> filter.insert(b"key2")
///     >>> assert filter.contains(b"key1")
///     >>> filter.remove(b"key1")
///     >>> assert not filter.contains(b"key1")
///     >>> assert filter.contains(b"key2")
///
/// Notes:
///     - Supports deletions (unlike standard BloomFilter)
///     - Uses ~4x more memory than standard Bloom filter (4 bits vs 1 bit per position)
///     - Counter overflow possible with many collisions (detected via has_overflow())
///     - Use CuckooFilter for better space efficiency with deletions
#[pyclass(module = "sketch_oxide")]
pub struct CountingBloomFilter {
    inner: RustCountingBloomFilter,
}

#[pymethods]
impl CountingBloomFilter {
    /// Create a new Counting Bloom Filter
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
            inner: RustCountingBloomFilter::new(n, fpr),
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
    ///     bool: True if key might be in set, False if definitely not
    fn contains(&self, key: &[u8]) -> bool {
        self.inner.contains(key)
    }

    /// Remove a key from the filter
    ///
    /// Args:
    ///     key: Bytes to remove
    ///
    /// Returns:
    ///     bool: True if key was present and removed
    ///
    /// Warning:
    ///     Removing a key that was never inserted may cause false negatives
    fn remove(&mut self, key: &[u8]) -> bool {
        self.inner.remove(key)
    }

    /// Estimate the count of a key (minimum counter value)
    ///
    /// Args:
    ///     key: Bytes to check
    ///
    /// Returns:
    ///     int: Minimum counter value across all hash positions
    fn count_estimate(&self, key: &[u8]) -> u8 {
        self.inner.count_estimate(key)
    }

    /// Check if any counter has overflowed
    ///
    /// Returns:
    ///     bool: True if any counter reached maximum (15)
    fn has_overflow(&self) -> bool {
        self.inner.has_overflow()
    }

    /// Get the number of elements inserted (minus removals)
    ///
    /// Returns:
    ///     int: Approximate count of elements
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if the filter is empty
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Clear all counters in the filter
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
        RustCountingBloomFilter::from_bytes(data)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "CountingBloomFilter(count={}, overflow={})",
            self.inner.len(),
            self.inner.has_overflow()
        )
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn __contains__(&self, key: &[u8]) -> bool {
        self.inner.contains(key)
    }

    /// Insert multiple keys into the filter in a single call (optimized for throughput).
    ///
    /// Batch inserts are significantly faster than multiple individual insert() calls
    /// because they amortize the FFI (Foreign Function Interface) overhead across
    /// many items. This is the preferred method when adding large quantities of data.
    ///
    /// Args:
    ///     keys: Iterable of byte strings to insert
    fn insert_batch(&mut self, keys: &Bound<'_, PyAny>) -> PyResult<()> {
        let keys_list: &Bound<'_, PyList> = keys.downcast()?;
        for key in keys_list {
            let key_bytes: &[u8] = key.extract()?;
            self.insert(key_bytes);
        }
        Ok(())
    }

    /// Remove multiple keys from the filter in a single call (optimized for throughput).
    ///
    /// Args:
    ///     keys: Iterable of byte strings to remove
    ///
    /// Returns:
    ///     list: List of booleans indicating if each key was found and removed
    fn remove_batch(&mut self, keys: &Bound<'_, PyAny>) -> PyResult<Vec<bool>> {
        let keys_list: &Bound<'_, PyList> = keys.downcast()?;
        let mut results = Vec::new();
        for key in keys_list {
            let key_bytes: &[u8] = key.extract()?;
            results.push(self.remove(key_bytes));
        }
        Ok(results)
    }

    /// Check multiple keys with a single call (optimized for lookups).
    ///
    /// Batch contains checks are faster than multiple individual contains() calls.
    ///
    /// Args:
    ///     keys: Iterable of byte strings to check
    ///
    /// Returns:
    ///     list: List of booleans, one for each key
    fn contains_batch(&self, keys: &Bound<'_, PyAny>) -> PyResult<Vec<bool>> {
        let keys_list: &Bound<'_, PyList> = keys.downcast()?;
        let mut results = Vec::new();
        for key in keys_list {
            let key_bytes: &[u8] = key.extract()?;
            results.push(self.contains(key_bytes));
        }
        Ok(results)
    }

    /// Get count estimates for multiple keys in a single call.
    ///
    /// Args:
    ///     keys: Iterable of byte strings to estimate
    ///
    /// Returns:
    ///     list: List of count estimates, one for each key
    fn count_estimate_batch(&self, keys: &Bound<'_, PyAny>) -> PyResult<Vec<u8>> {
        let keys_list: &Bound<'_, PyList> = keys.downcast()?;
        let mut results = Vec::new();
        for key in keys_list {
            let key_bytes: &[u8] = key.extract()?;
            results.push(self.count_estimate(key_bytes));
        }
        Ok(results)
    }
}
