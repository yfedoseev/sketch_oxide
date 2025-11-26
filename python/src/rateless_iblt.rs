//! Python bindings for Rateless IBLT set reconciliation

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::common::Reconcilable;
use sketch_oxide::reconciliation::RatelessIBLT as RustRatelessIBLT;

/// Rateless IBLT: Efficient Set Reconciliation (Goodrich & Mitzenmacher, 2011)
///
/// A probabilistic data structure for efficiently computing the symmetric difference
/// between two sets in distributed systems. Used in blockchain synchronization,
/// P2P networks, and distributed caching (Ethereum: 5.6x faster than naive approaches).
///
/// Args:
///     expected_diff (int): Expected size of set difference (> 0)
///     cell_size (int): Maximum size for cell data in bytes (>= 8)
///
/// Example:
///     >>> # Create IBLTs for Alice and Bob
///     >>> alice = RatelessIBLT(expected_diff=100, cell_size=32)
///     >>> bob = RatelessIBLT(expected_diff=100, cell_size=32)
///     >>>
///     >>> # Both insert shared items
///     >>> alice.insert(b"shared1", b"value1")
///     >>> bob.insert(b"shared1", b"value1")
///     >>>
///     >>> # Alice has unique item
///     >>> alice.insert(b"alice_only", b"alice_value")
///     >>>
///     >>> # Bob has unique item
///     >>> bob.insert(b"bob_only", b"bob_value")
///     >>>
///     >>> # Compute difference
///     >>> alice.subtract(bob)
///     >>> result = alice.decode()
///     >>> print(f"Items to insert: {len(result['to_insert'])}")
///     >>> print(f"Items to remove: {len(result['to_remove'])}")
///
/// Notes:
///     - Space: O(c × d) where c ≈ 2.0, d = expected difference size
///     - Insert/Delete: O(k) where k = 3 hash functions
///     - Decode: O(d × k) via iterative peeling algorithm
///     - No false negatives, but may fail to decode if capacity exceeded
#[pyclass(module = "sketch_oxide")]
pub struct RatelessIBLT {
    inner: RustRatelessIBLT,
}

#[pymethods]
impl RatelessIBLT {
    /// Create a new Rateless IBLT
    ///
    /// Args:
    ///     expected_diff: Expected size of set difference
    ///     cell_size: Maximum size for cell data in bytes
    ///
    /// Raises:
    ///     ValueError: If parameters are invalid
    ///
    /// Example:
    ///     >>> iblt = RatelessIBLT(expected_diff=100, cell_size=32)
    #[new]
    fn new(expected_diff: usize, cell_size: usize) -> PyResult<Self> {
        RustRatelessIBLT::new(expected_diff, cell_size)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Insert a key-value pair into the IBLT
    ///
    /// Args:
    ///     key: The key to insert (bytes)
    ///     value: The value associated with the key (bytes)
    ///
    /// Example:
    ///     >>> iblt = RatelessIBLT(expected_diff=100, cell_size=32)
    ///     >>> iblt.insert(b"my_key", b"my_value")
    fn insert(&mut self, key: &[u8], value: &[u8]) -> PyResult<()> {
        self.inner
            .insert(key, value)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Delete a key-value pair from the IBLT
    ///
    /// Args:
    ///     key: The key to delete (bytes)
    ///     value: The value associated with the key (bytes)
    ///
    /// Example:
    ///     >>> iblt = RatelessIBLT(expected_diff=100, cell_size=32)
    ///     >>> iblt.insert(b"key", b"value")
    ///     >>> iblt.delete(b"key", b"value")
    fn delete(&mut self, key: &[u8], value: &[u8]) -> PyResult<()> {
        self.inner
            .delete(key, value)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Subtract another IBLT from this one
    ///
    /// Computes element-wise XOR difference, leaving cells that contain
    /// the symmetric difference between the two sets.
    ///
    /// Args:
    ///     other: Another RatelessIBLT with same configuration
    ///
    /// Raises:
    ///     ValueError: If IBLTs have incompatible configurations
    ///
    /// Example:
    ///     >>> alice = RatelessIBLT(expected_diff=100, cell_size=32)
    ///     >>> bob = RatelessIBLT(expected_diff=100, cell_size=32)
    ///     >>> alice.insert(b"a", b"1")
    ///     >>> bob.insert(b"b", b"2")
    ///     >>> alice.subtract(bob)
    fn subtract(&mut self, other: &RatelessIBLT) -> PyResult<()> {
        self.inner
            .subtract(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Decode the IBLT to recover set differences
    ///
    /// Uses the "peeling" algorithm to iteratively extract singleton cells
    /// and recover the key-value pairs in the symmetric difference.
    ///
    /// Returns:
    ///     dict: Dictionary with 'to_insert' and 'to_remove' lists
    ///         - to_insert: Items with positive count (in this IBLT but not other)
    ///         - to_remove: Items with negative count (in other IBLT but not this)
    ///
    /// Raises:
    ///     ValueError: If decoding fails (too many items, corruption, etc.)
    ///
    /// Example:
    ///     >>> iblt = RatelessIBLT(expected_diff=10, cell_size=32)
    ///     >>> iblt.insert(b"key1", b"value1")
    ///     >>> result = iblt.decode()
    ///     >>> assert len(result['to_insert']) == 1
    fn decode(&self) -> PyResult<Py<PyAny>> {
        let diff = self
            .inner
            .decode()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        Python::with_gil(|py| {
            let dict = pyo3::types::PyDict::new_bound(py);

            // Convert to_insert list
            let to_insert_list = pyo3::types::PyList::empty_bound(py);
            for (key, value) in diff.to_insert {
                let key_bytes = PyBytes::new_bound(py, &key);
                let value_bytes = PyBytes::new_bound(py, &value);
                let tuple = pyo3::types::PyTuple::new_bound(
                    py,
                    &[key_bytes.as_any(), value_bytes.as_any()],
                );
                to_insert_list.append(tuple)?;
            }
            dict.set_item("to_insert", to_insert_list)?;

            // Convert to_remove list
            let to_remove_list = pyo3::types::PyList::empty_bound(py);
            for (key, value) in diff.to_remove {
                let key_bytes = PyBytes::new_bound(py, &key);
                let value_bytes = PyBytes::new_bound(py, &value);
                let tuple = pyo3::types::PyTuple::new_bound(
                    py,
                    &[key_bytes.as_any(), value_bytes.as_any()],
                );
                to_remove_list.append(tuple)?;
            }
            dict.set_item("to_remove", to_remove_list)?;

            Ok(dict.into())
        })
    }

    /// Get statistics about the IBLT
    ///
    /// Returns:
    ///     dict: Dictionary with statistics
    ///         - num_cells: Number of cells in the IBLT
    ///         - cell_size: Size of each cell in bytes
    ///
    /// Example:
    ///     >>> iblt = RatelessIBLT(expected_diff=100, cell_size=32)
    ///     >>> stats = iblt.stats()
    ///     >>> print(f"Cells: {stats['num_cells']}, Size: {stats['cell_size']}")
    fn stats(&self) -> PyResult<Py<PyAny>> {
        let stats = self.inner.stats();
        Python::with_gil(|py| {
            let dict = pyo3::types::PyDict::new_bound(py);
            dict.set_item("num_cells", stats.num_cells)?;
            dict.set_item("cell_size", stats.cell_size)?;
            Ok(dict.into())
        })
    }

    fn __repr__(&self) -> String {
        let stats = self.inner.stats();
        format!(
            "RatelessIBLT(num_cells={}, cell_size={})",
            stats.num_cells, stats.cell_size
        )
    }

    fn __str__(&self) -> String {
        let stats = self.inner.stats();
        format!(
            "RatelessIBLT({} cells × {} bytes)",
            stats.num_cells, stats.cell_size
        )
    }
}
