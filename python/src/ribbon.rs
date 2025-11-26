//! Python bindings for RibbonFilter membership testing

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList};
use sketch_oxide::membership::RibbonFilter as RustRibbonFilter;

/// Ribbon Filter for space-efficient membership testing (RocksDB 2021+)
///
/// ~30% more space-efficient than Bloom filter (~7 bits/key @ 1% FPR).
/// Uses Gaussian elimination for construction, requiring finalization before queries.
///
/// Args:
///     n: Expected number of elements
///     fpr: Desired false positive rate (0.0 to 1.0), default 0.01 (1%)
///
/// Example:
///     >>> filter = RibbonFilter(1000, fpr=0.01)
///     >>> filter.insert(b"key1")
///     >>> filter.insert(b"key2")
///     >>> filter.finalize()  # Required before queries!
///     >>> assert filter.contains(b"key1")
///     >>> assert filter.contains(b"key2")
///
/// Use cases:
///     - LSM-tree SSTables (static, write-once)
///     - Space-constrained environments
///     - When construction time is acceptable for space savings
///
/// Notes:
///     - MUST call finalize() after all inserts and before queries
///     - ~30% smaller than Bloom filter
///     - Slower construction (Gaussian elimination) but fast queries
///     - No insertions allowed after finalization
#[pyclass(module = "sketch_oxide")]
pub struct RibbonFilter {
    inner: RustRibbonFilter,
}

#[pymethods]
impl RibbonFilter {
    /// Create a new Ribbon Filter
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
            inner: RustRibbonFilter::new(n, fpr),
        })
    }

    /// Insert a key into the filter
    ///
    /// Args:
    ///     key: Bytes to insert
    ///
    /// Raises:
    ///     RuntimeError: If called after finalization or filter is full
    fn insert(&mut self, key: &[u8]) -> PyResult<()> {
        if self.inner.is_finalized() {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Cannot insert after finalization",
            ));
        }
        self.inner.insert(key);
        Ok(())
    }

    /// Finalize the filter (required before queries)
    ///
    /// Must be called after all inserts and before any contains() calls.
    /// Calling finalize() multiple times is safe (idempotent).
    fn finalize(&mut self) {
        self.inner.finalize();
    }

    /// Check if a key might be in the set
    ///
    /// Args:
    ///     key: Bytes to check
    ///
    /// Returns:
    ///     bool: True if key might be in set (possible false positives),
    ///           False if key is definitely not in set (no false negatives)
    ///
    /// Raises:
    ///     RuntimeError: If called before finalization
    fn contains(&self, key: &[u8]) -> PyResult<bool> {
        if !self.inner.is_finalized() {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Must call finalize() before querying",
            ));
        }
        Ok(self.inner.contains(key))
    }

    /// Check if the filter has been finalized
    ///
    /// Returns:
    ///     bool: True if finalize() has been called
    fn is_finalized(&self) -> bool {
        self.inner.is_finalized()
    }

    /// Check if the filter is empty
    ///
    /// Returns:
    ///     bool: True if no elements have been inserted
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get the number of inserted elements
    ///
    /// Returns:
    ///     int: Number of elements inserted
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Get the theoretical false positive rate
    ///
    /// Returns:
    ///     float: Expected false positive rate
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
    ///
    /// Raises:
    ///     RuntimeError: If called before finalization
    fn to_bytes<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        if !self.inner.is_finalized() {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Must finalize before serialization",
            ));
        }
        Ok(PyBytes::new_bound(py, &self.inner.to_bytes()))
    }

    /// Deserialize a filter from bytes
    ///
    /// Args:
    ///     data: Serialized filter bytes
    ///
    /// Returns:
    ///     RibbonFilter: Deserialized filter
    ///
    /// Raises:
    ///     ValueError: If bytes are invalid
    #[staticmethod]
    fn from_bytes(data: &[u8]) -> PyResult<Self> {
        RustRibbonFilter::from_bytes(data)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        let (n, _, cols) = self.inner.params();
        format!(
            "RibbonFilter(n={}, cols={}, finalized={}, fp_rate={:.4}%)",
            n,
            cols,
            self.inner.is_finalized(),
            self.inner.false_positive_rate() * 100.0
        )
    }

    fn __str__(&self) -> String {
        let status = if self.inner.is_finalized() {
            "finalized"
        } else {
            "pending"
        };
        format!("RibbonFilter({} items, {})", self.inner.len(), status)
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// Insert multiple keys into the filter in a single call (optimized for throughput).
    ///
    /// Batch inserts are significantly faster than multiple individual insert() calls
    /// because they amortize the FFI (Foreign Function Interface) overhead across
    /// many items. This is the preferred method when adding large quantities of data.
    /// Must call finalize() after all inserts.
    ///
    /// Args:
    ///     keys: Iterable of byte strings to insert
    ///
    /// Raises:
    ///     RuntimeError: If called after finalization
    fn insert_batch(&mut self, keys: &Bound<'_, PyAny>) -> PyResult<()> {
        let keys_list: &Bound<'_, PyList> = keys.downcast()?;
        for key in keys_list {
            let key_bytes: &[u8] = key.extract()?;
            self.insert(key_bytes)?;
        }
        Ok(())
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
    ///
    /// Raises:
    ///     RuntimeError: If called before finalization
    fn contains_batch(&self, keys: &Bound<'_, PyAny>) -> PyResult<Vec<bool>> {
        let keys_list: &Bound<'_, PyList> = keys.downcast()?;
        let mut results = Vec::new();
        for key in keys_list {
            let key_bytes: &[u8] = key.extract()?;
            results.push(self.contains(key_bytes)?);
        }
        Ok(results)
    }
}
