//! Python bindings for CuckooFilter membership testing with deletions

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList};
use sketch_oxide::membership::CuckooFilter as RustCuckooFilter;

/// Cuckoo Filter for space-efficient membership testing with deletions
///
/// Space-efficient alternative to Counting Bloom filters using cuckoo hashing.
/// Supports insertions, deletions, and membership queries with ~12 bits per item.
///
/// Args:
///     capacity: Expected number of elements to store
///
/// Example:
///     >>> filter = CuckooFilter(1000)
///     >>> filter.insert(b"key1")
///     >>> filter.insert(b"key2")
///     >>> assert filter.contains(b"key1")
///     >>> filter.remove(b"key1")
///     >>> assert not filter.contains(b"key1")
///     >>> assert filter.contains(b"key2")
///
/// Notes:
///     - More space-efficient than CountingBloomFilter (~12 bits vs ~40 bits per item)
///     - Supports deletions
///     - Insert may fail if filter is near capacity (raises ValueError)
///     - FPR is determined by fingerprint size, typically ~3%
#[pyclass(module = "sketch_oxide")]
pub struct CuckooFilter {
    inner: RustCuckooFilter,
}

#[pymethods]
impl CuckooFilter {
    /// Create a new Cuckoo Filter
    ///
    /// Args:
    ///     capacity: Expected number of elements
    ///
    /// Raises:
    ///     ValueError: If capacity <= 0
    #[new]
    fn new(capacity: usize) -> PyResult<Self> {
        RustCuckooFilter::new(capacity)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Insert a key into the filter
    ///
    /// Args:
    ///     key: Bytes to insert
    ///
    /// Raises:
    ///     ValueError: If filter is full and cannot accommodate the key
    fn insert(&mut self, key: &[u8]) -> PyResult<()> {
        self.inner
            .insert(key)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
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
    ///     bool: True if key was found and removed
    ///
    /// Warning:
    ///     Removing a key that was never inserted may cause false negatives
    fn remove(&mut self, key: &[u8]) -> bool {
        self.inner.remove(key)
    }

    /// Get the number of elements stored
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if the filter is empty
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get the capacity (maximum elements)
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Get the current load factor
    ///
    /// Returns:
    ///     float: Ratio of stored items to capacity
    fn load_factor(&self) -> f64 {
        self.inner.load_factor()
    }

    /// Clear all items from the filter
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
        RustCuckooFilter::from_bytes(data)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "CuckooFilter(count={}, capacity={}, load={:.1}%)",
            self.inner.len(),
            self.inner.capacity(),
            self.inner.load_factor() * 100.0
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
    ///
    /// Raises:
    ///     ValueError: If filter is full and cannot accommodate any key
    fn insert_batch(&mut self, keys: &Bound<'_, PyAny>) -> PyResult<()> {
        let keys_list: &Bound<'_, PyList> = keys.downcast()?;
        for key in keys_list {
            let key_bytes: &[u8] = key.extract()?;
            self.insert(key_bytes)?;
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
}
