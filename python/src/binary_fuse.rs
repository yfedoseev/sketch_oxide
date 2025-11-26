//! Python bindings for BinaryFuseFilter membership testing

use pyo3::prelude::*;
use pyo3::types::PyList;
use sketch_oxide::membership::BinaryFuseFilter as RustBinaryFuseFilter;

/// Binary Fuse Filter for probabilistic membership testing (ACM JEA 2022)
///
/// 75% more space-efficient than Bloom filters. This is an immutable data structure
/// that must be built from a complete set of items.
///
/// Args:
///     items: List of integers to insert into the filter
///     bits_per_entry: Bits per entry (8-16), higher = lower false positive rate
///         - 9 bits: ~1% false positive rate (recommended)
///         - 12 bits: ~0.02% false positive rate
///         - 16 bits: ~0.0015% false positive rate
///
/// Example:
///     >>> items = [1, 2, 3, 4, 5, 100, 200, 300]
///     >>> filter = BinaryFuseFilter(items, bits_per_entry=9)
///     >>> assert filter.contains(3)
///     >>> assert filter.contains(100)
///     >>> # False positives possible but rare
///     >>> filter.contains(999)  # Might return True with ~1% probability
///
/// Notes:
///     - Immutable: cannot add items after construction
///     - Very space-efficient: ~9-16 bits per item
///     - Fast queries: O(1) with 3 memory accesses
#[pyclass(module = "sketch_oxide")]
pub struct BinaryFuseFilter {
    inner: RustBinaryFuseFilter,
}

#[pymethods]
impl BinaryFuseFilter {
    /// Create a new Binary Fuse Filter from a list of items
    ///
    /// Args:
    ///     items: List of integers to insert
    ///     bits_per_entry: Bits per entry (8-16), default 9
    ///
    /// Raises:
    ///     ValueError: If bits_per_entry is not in range [8, 16]
    ///     RuntimeError: If filter construction fails
    #[new]
    #[pyo3(signature = (items, bits_per_entry=9))]
    fn new(items: Vec<u64>, bits_per_entry: u8) -> PyResult<Self> {
        RustBinaryFuseFilter::from_items(items.into_iter(), bits_per_entry)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Check if an item might be in the set
    ///
    /// Args:
    ///     item: Integer to check
    ///
    /// Returns:
    ///     bool: True if item might be in set (with possible false positives),
    ///           False if item is definitely not in set
    ///
    /// Example:
    ///     >>> filter = BinaryFuseFilter([1, 2, 3], bits_per_entry=9)
    ///     >>> filter.contains(2)
    ///     True
    ///     >>> filter.contains(999)  # Might be False or True (false positive)
    fn contains(&self, item: u64) -> bool {
        self.inner.contains(&item)
    }

    /// Get the number of items the filter was built from
    ///
    /// Returns:
    ///     int: Number of items
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if the filter is empty
    ///
    /// Returns:
    ///     bool: True if filter contains no items
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get the bits per entry parameter
    ///
    /// Returns:
    ///     float: Bits per entry (8-16)
    fn bits_per_entry(&self) -> f64 {
        self.inner.bits_per_entry()
    }

    /// Get the estimated false positive rate
    ///
    /// Returns:
    ///     float: Expected false positive rate
    fn false_positive_rate(&self) -> f64 {
        self.inner.estimated_fpr()
    }

    fn __repr__(&self) -> String {
        format!(
            "BinaryFuseFilter(items={}, bits_per_entry={:.1}, fp_rate={:.4}%)",
            self.inner.len(),
            self.inner.bits_per_entry(),
            self.false_positive_rate() * 100.0
        )
    }

    fn __str__(&self) -> String {
        format!("BinaryFuseFilter({} items)", self.inner.len())
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn __contains__(&self, item: u64) -> bool {
        self.inner.contains(&item)
    }

    /// Check multiple items with a single call (optimized for lookups).
    ///
    /// Batch contains checks are faster than multiple individual contains() calls.
    ///
    /// Args:
    ///     items: Iterable of integers to check
    ///
    /// Returns:
    ///     list: List of booleans, one for each item
    fn contains_batch(&self, items: &Bound<'_, PyAny>) -> PyResult<Vec<bool>> {
        let items_list: &Bound<'_, PyList> = items.downcast()?;
        let mut results = Vec::new();
        for item in items_list {
            let item_val: u64 = item.extract()?;
            results.push(self.contains(item_val));
        }
        Ok(results)
    }
}
