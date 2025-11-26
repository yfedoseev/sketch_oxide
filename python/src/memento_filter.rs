//! Python bindings for Memento Filter dynamic range filter

use pyo3::prelude::*;
use sketch_oxide::range_filters::MementoFilter as RustMementoFilter;

/// MementoFilter: Dynamic Range Filter with FPR Guarantees (2025)
///
/// The first dynamic range filter supporting insertions while maintaining
/// false positive rate guarantees. Adapts structure during insertions via
/// quotient filter integration.
///
/// Args:
///     expected_elements (int): Expected number of elements to store (> 0)
///     fpr (float): Target false positive rate in (0.0, 1.0)
///
/// Example:
///     >>> filter = MementoFilter(expected_elements=1000, fpr=0.01)
///     >>>
///     >>> # Insert key-value pairs dynamically
///     >>> filter.insert(42, b"value1")
///     >>> filter.insert(100, b"value2")
///     >>> filter.insert(250, b"value3")
///     >>>
///     >>> # Query ranges - maintains FPR guarantees
///     >>> assert filter.may_contain_range(40, 50)  # Contains 42
///     >>> assert filter.may_contain_range(95, 105)  # Contains 100
///     >>>
///     >>> # Get current range bounds
///     >>> range_bounds = filter.range()
///     >>> print(f"Current range: {range_bounds}")
///
/// Production Use (2025):
///     - MongoDB WiredTiger integration
///     - RocksDB block filters
///     - Dynamic database indexes
///     - Log systems with streaming data
///     - Time-series with growing ranges
///
/// Performance:
///     - Insertion: O(1) amortized, <200ns
///     - Query: O(1), <150ns
///     - Space: ~10 bits per element with 1% FPR
///     - FPR: Stays below configured target even with dynamic insertions
#[pyclass(module = "sketch_oxide")]
pub struct MementoFilter {
    inner: RustMementoFilter,
}

#[pymethods]
impl MementoFilter {
    /// Create a new Memento Filter
    ///
    /// Args:
    ///     expected_elements: Expected number of elements to store
    ///     fpr: Target false positive rate (0.0, 1.0)
    ///
    /// Raises:
    ///     ValueError: If parameters are invalid
    ///
    /// Example:
    ///     >>> filter = MementoFilter(expected_elements=1000, fpr=0.01)
    #[new]
    fn new(expected_elements: usize, fpr: f64) -> PyResult<Self> {
        RustMementoFilter::new(expected_elements, fpr)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Insert a key-value pair into the filter
    ///
    /// Args:
    ///     key: The key to insert (u64)
    ///     value: The value associated with the key (bytes)
    ///
    /// Raises:
    ///     ValueError: If capacity is exceeded
    ///
    /// Example:
    ///     >>> filter = MementoFilter(expected_elements=1000, fpr=0.01)
    ///     >>> filter.insert(42, b"value")
    fn insert(&mut self, key: u64, value: &[u8]) -> PyResult<()> {
        self.inner
            .insert(key, value)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Check if a range might contain elements
    ///
    /// Args:
    ///     low: Lower bound of range (inclusive)
    ///     high: Upper bound of range (inclusive)
    ///
    /// Returns:
    ///     bool: True if range might contain elements, False if definitely does not
    ///
    /// Guarantees:
    ///     - No false negatives
    ///     - False positive rate bounded by configured FPR
    ///
    /// Example:
    ///     >>> filter = MementoFilter(expected_elements=1000, fpr=0.01)
    ///     >>> filter.insert(50, b"value")
    ///     >>> assert filter.may_contain_range(45, 55)  # Contains 50
    fn may_contain_range(&self, low: u64, high: u64) -> bool {
        self.inner.may_contain_range(low, high)
    }

    /// Get the number of elements in the filter
    ///
    /// Returns:
    ///     int: Number of elements currently stored
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if the filter is empty
    ///
    /// Returns:
    ///     bool: True if no elements have been inserted
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get the current range bounds
    ///
    /// Returns:
    ///     tuple or None: (min, max) if filter is non-empty, None if empty
    ///
    /// Example:
    ///     >>> filter = MementoFilter(expected_elements=1000, fpr=0.01)
    ///     >>> filter.insert(10, b"value1")
    ///     >>> filter.insert(100, b"value2")
    ///     >>> range_bounds = filter.range()
    ///     >>> assert range_bounds == (10, 100)
    fn range(&self) -> Option<(u64, u64)> {
        self.inner.range()
    }

    /// Get statistics about the filter
    ///
    /// Returns:
    ///     dict: Dictionary with statistics
    ///         - num_elements: Number of elements currently stored
    ///         - capacity: Maximum capacity
    ///         - fpr_target: Target false positive rate
    ///         - num_expansions: Number of range expansions
    ///         - load_factor: Current load factor (num_elements / capacity)
    ///
    /// Example:
    ///     >>> filter = MementoFilter(expected_elements=1000, fpr=0.01)
    ///     >>> filter.insert(42, b"value")
    ///     >>> stats = filter.stats()
    ///     >>> print(f"Load factor: {stats['load_factor']:.2%}")
    fn stats(&self) -> PyResult<Py<PyAny>> {
        let stats = self.inner.stats();
        Python::with_gil(|py| {
            let dict = pyo3::types::PyDict::new_bound(py);
            dict.set_item("num_elements", stats.num_elements)?;
            dict.set_item("capacity", stats.capacity)?;
            dict.set_item("fpr_target", stats.fpr_target)?;
            dict.set_item("num_expansions", stats.num_expansions)?;
            dict.set_item("load_factor", stats.load_factor)?;
            Ok(dict.into())
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "MementoFilter(len={}, capacity={})",
            self.inner.len(),
            self.inner.stats().capacity
        )
    }

    fn __str__(&self) -> String {
        let stats = self.inner.stats();
        format!(
            "MementoFilter({} / {} elements, {:.2}% FPR)",
            self.inner.len(),
            stats.capacity,
            stats.fpr_target * 100.0
        )
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }
}
