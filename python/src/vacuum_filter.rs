//! Python bindings for Vacuum Filter - Best-in-class dynamic membership filter

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::membership::VacuumFilter as RustVacuumFilter;

/// VacuumFilter: Best-in-class dynamic membership filter (VLDB 2020)
///
/// Space-efficient filter supporting insertions, deletions, and queries with
/// <15 bits/item at 1% FPR. Combines the space efficiency of static filters
/// with the flexibility of dynamic operations.
///
/// Args:
///     capacity (int): Expected number of elements (must be > 0)
///     fpr (float): Target false positive rate (0.0 < fpr < 1.0)
///         - 0.001: 0.1% FPR (~15-16 bits/item)
///         - 0.01: 1% FPR (~12-14 bits/item, recommended)
///         - 0.05: 5% FPR (~10-11 bits/item)
///
/// Example:
///     >>> vf = VacuumFilter(capacity=1000, fpr=0.01)
///     >>> vf.insert(b"hello")
///     >>> assert vf.contains(b"hello")
///     >>> vf.delete(b"hello")
///     >>> assert not vf.contains(b"hello")
///
/// Key Advantages:
///     - Best space efficiency among dynamic filters (<15 bits/item)
///     - True deletions (no false negatives after deletion)
///     - Cache-optimized semi-sorted buckets
///     - Predictable performance (no cuckoo evictions)
///     - Configurable FPR through fingerprint sizing
///
/// Comparison:
///     - vs Bloom: Supports deletions, better space efficiency
///     - vs Cuckoo: Similar space, better predictability
///     - vs Binary Fuse: Supports deletions (Fuse is static only)
///     - vs Counting Bloom: 3-4x better space efficiency
///
/// Notes:
///     - Supports int, str, and bytes types
///     - Automatic rehashing when load factor exceeds 95%
///     - No false negatives (if contains() returns False, item definitely not present)
///     - May have false positives at configured FPR
#[pyclass(module = "sketch_oxide")]
pub struct VacuumFilter {
    inner: RustVacuumFilter,
}

#[pymethods]
impl VacuumFilter {
    /// Create a new Vacuum Filter
    ///
    /// Args:
    ///     capacity: Expected number of elements
    ///     fpr: Target false positive rate (0.0 < fpr < 1.0)
    ///
    /// Raises:
    ///     ValueError: If parameters are invalid
    ///
    /// Example:
    ///     >>> vf = VacuumFilter(capacity=1000, fpr=0.01)
    #[new]
    fn new(capacity: usize, fpr: f64) -> PyResult<Self> {
        RustVacuumFilter::new(capacity, fpr)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Insert an element into the filter
    ///
    /// Args:
    ///     item: Item to insert (int, str, or bytes)
    ///
    /// Raises:
    ///     ValueError: If filter is at capacity and cannot rehash
    ///
    /// Example:
    ///     >>> vf = VacuumFilter(capacity=100, fpr=0.01)
    ///     >>> vf.insert(b"hello")
    ///     >>> vf.insert("world")
    ///     >>> vf.insert(42)
    fn insert(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        if let Ok(val) = item.extract::<i64>() {
            self.inner
                .insert(&val.to_le_bytes())
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        } else if let Ok(val) = item.extract::<u64>() {
            self.inner
                .insert(&val.to_le_bytes())
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        } else if let Ok(val) = item.extract::<String>() {
            self.inner
                .insert(val.as_bytes())
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        } else if let Ok(b) = item.downcast::<PyBytes>() {
            self.inner
                .insert(b.as_bytes())
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Item must be int, str, or bytes",
            ))
        }
    }

    /// Check if an element might be in the filter
    ///
    /// Args:
    ///     item: Item to check (int, str, or bytes)
    ///
    /// Returns:
    ///     bool: True if might be present (with FPR probability of false positive),
    ///           False if definitely not present
    ///
    /// Example:
    ///     >>> vf = VacuumFilter(capacity=100, fpr=0.01)
    ///     >>> vf.insert(b"hello")
    ///     >>> assert vf.contains(b"hello")  # True positive
    ///     >>> assert not vf.contains(b"world")  # True negative (likely)
    fn contains(&self, item: &Bound<'_, PyAny>) -> PyResult<bool> {
        if let Ok(val) = item.extract::<i64>() {
            Ok(self.inner.contains(&val.to_le_bytes()))
        } else if let Ok(val) = item.extract::<u64>() {
            Ok(self.inner.contains(&val.to_le_bytes()))
        } else if let Ok(val) = item.extract::<String>() {
            Ok(self.inner.contains(val.as_bytes()))
        } else if let Ok(b) = item.downcast::<PyBytes>() {
            Ok(self.inner.contains(b.as_bytes()))
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Item must be int, str, or bytes",
            ))
        }
    }

    /// Delete an element from the filter
    ///
    /// Args:
    ///     item: Item to delete (int, str, or bytes)
    ///
    /// Returns:
    ///     bool: True if found and removed, False if not present
    ///
    /// Example:
    ///     >>> vf = VacuumFilter(capacity=100, fpr=0.01)
    ///     >>> vf.insert(b"hello")
    ///     >>> assert vf.delete(b"hello")
    ///     >>> assert not vf.contains(b"hello")
    ///     >>> assert not vf.delete(b"hello")  # Already deleted
    fn delete(&mut self, item: &Bound<'_, PyAny>) -> PyResult<bool> {
        if let Ok(val) = item.extract::<i64>() {
            self.inner
                .delete(&val.to_le_bytes())
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        } else if let Ok(val) = item.extract::<u64>() {
            self.inner
                .delete(&val.to_le_bytes())
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        } else if let Ok(val) = item.extract::<String>() {
            self.inner
                .delete(val.as_bytes())
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        } else if let Ok(b) = item.downcast::<PyBytes>() {
            self.inner
                .delete(b.as_bytes())
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Item must be int, str, or bytes",
            ))
        }
    }

    /// Get the current load factor (0.0 to 1.0)
    ///
    /// Returns:
    ///     float: Current load factor (num_items / capacity)
    ///
    /// Example:
    ///     >>> vf = VacuumFilter(capacity=100, fpr=0.01)
    ///     >>> vf.insert(b"item1")
    ///     >>> load = vf.load_factor()
    ///     >>> assert 0.0 < load < 1.0
    fn load_factor(&self) -> f64 {
        self.inner.load_factor()
    }

    /// Get the total capacity (maximum items before rehashing)
    ///
    /// Returns:
    ///     int: Total capacity
    ///
    /// Example:
    ///     >>> vf = VacuumFilter(capacity=1000, fpr=0.01)
    ///     >>> cap = vf.capacity()
    ///     >>> assert cap >= 1000  # May be rounded up to power of 2
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Get the number of items currently stored
    ///
    /// Returns:
    ///     int: Number of items
    ///
    /// Example:
    ///     >>> vf = VacuumFilter(capacity=100, fpr=0.01)
    ///     >>> assert vf.len() == 0
    ///     >>> vf.insert(b"item")
    ///     >>> assert vf.len() == 1
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if the filter is empty
    ///
    /// Returns:
    ///     bool: True if no items have been inserted
    ///
    /// Example:
    ///     >>> vf = VacuumFilter(capacity=100, fpr=0.01)
    ///     >>> assert vf.is_empty()
    ///     >>> vf.insert(b"item")
    ///     >>> assert not vf.is_empty()
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Clear all items from the filter
    ///
    /// Example:
    ///     >>> vf = VacuumFilter(capacity=100, fpr=0.01)
    ///     >>> vf.insert(b"item")
    ///     >>> vf.clear()
    ///     >>> assert vf.is_empty()
    fn clear(&mut self) {
        self.inner.clear();
    }

    /// Get the memory usage in bytes
    ///
    /// Returns:
    ///     int: Memory usage in bytes
    ///
    /// Example:
    ///     >>> vf = VacuumFilter(capacity=1000, fpr=0.01)
    ///     >>> mem = vf.memory_usage()
    ///     >>> print(f"Filter uses {mem} bytes")
    fn memory_usage(&self) -> usize {
        self.inner.memory_usage()
    }

    /// Get statistics about the filter
    ///
    /// Returns:
    ///     dict: Dictionary with statistics
    ///         - capacity: Total capacity
    ///         - num_items: Current number of items
    ///         - load_factor: Current load factor (0.0 to 1.0)
    ///         - memory_bits: Total memory in bits
    ///         - fingerprint_bits: Fingerprint size in bits
    ///
    /// Example:
    ///     >>> vf = VacuumFilter(capacity=1000, fpr=0.01)
    ///     >>> stats = vf.stats()
    ///     >>> print(f"Load: {stats['load_factor']:.2%}")
    ///     >>> print(f"Memory: {stats['memory_bits'] // 8} bytes")
    fn stats(&self) -> PyResult<Py<PyAny>> {
        let stats = self.inner.stats();
        Python::with_gil(|py| {
            let dict = pyo3::types::PyDict::new_bound(py);
            dict.set_item("capacity", stats.capacity)?;
            dict.set_item("num_items", stats.num_items)?;
            dict.set_item("load_factor", stats.load_factor)?;
            dict.set_item("memory_bits", stats.memory_bits)?;
            dict.set_item("fingerprint_bits", stats.fingerprint_bits)?;
            Ok(dict.into())
        })
    }

    fn __repr__(&self) -> String {
        let stats = self.inner.stats();
        format!(
            "VacuumFilter(capacity={}, num_items={}, load_factor={:.2}%)",
            stats.capacity,
            stats.num_items,
            stats.load_factor * 100.0
        )
    }

    fn __str__(&self) -> String {
        let stats = self.inner.stats();
        format!(
            "VacuumFilter({}/{} items, {:.2}% full)",
            stats.num_items,
            stats.capacity,
            stats.load_factor * 100.0
        )
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }
}
