//! Python bindings for Space-Saving heavy hitter detection sketch

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::frequency::SpaceSaving as RustSpaceSaving;
use sketch_oxide::Sketch;

/// Space-Saving Sketch for heavy hitter detection (Metwally et al., 2005)
///
/// A deterministic streaming algorithm for finding the most frequent items (heavy hitters)
/// with guaranteed error bounds. Unlike probabilistic sketches, Space-Saving:
/// - Guarantees no false negatives for items with frequency > epsilon*N
/// - Provides deterministic error bounds for each item
/// - Uses exactly O(1/epsilon) space
/// - Deterministic (no randomness involved)
///
/// Args:
///     epsilon (float): Error threshold (0 < ε < 1). Items with frequency > ε*N
///         are guaranteed to be tracked.
///         - 0.01: Track items > 1% frequency (100 items for 1M stream)
///         - 0.1: Track items > 10% frequency (10 items)
///         - 0.001: Track items > 0.1% frequency (1000 items)
///
/// Example:
///     >>> ss = SpaceSaving(epsilon=0.01)
///     >>> for word in document.split():
///     ...     ss.update(word)
///     >>> # Get top-10 most frequent words
///     >>> for word, lower, upper in ss.heavy_hitters(0.05)[:10]:
///     ...     print(f"{word}: [{lower}, {upper}]")
///
/// Notes:
///     - Deterministic error bounds (not probabilistic)
///     - No false negatives: All items with frequency > epsilon*N are found
///     - Possible false positives: Some items below threshold may appear
///     - True frequency always in [lower_bound, upper_bound]
///     - Space: O(1/epsilon) counters
///
///     - Supports int, str, and bytes types for items
#[pyclass(module = "sketch_oxide")]
pub struct SpaceSaving {
    inner: RustSpaceSaving<Vec<u8>>,
}

#[pymethods]
impl SpaceSaving {
    /// Create a new Space-Saving sketch with error bound epsilon
    ///
    /// Args:
    ///     epsilon: Error threshold (0 < ε < 1)
    ///
    /// Raises:
    ///     ValueError: If epsilon is not in valid range
    #[new]
    fn new(epsilon: f64) -> PyResult<Self> {
        RustSpaceSaving::new(epsilon)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Create a new Space-Saving sketch with explicit capacity
    ///
    /// Args:
    ///     capacity: Maximum number of items to track (must be >= 2)
    ///
    /// Raises:
    ///     ValueError: If capacity is less than 2
    #[staticmethod]
    fn with_capacity(capacity: usize) -> PyResult<SpaceSaving> {
        RustSpaceSaving::with_capacity(capacity)
            .map(|inner| SpaceSaving { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Update the sketch with a single occurrence of an item
    ///
    /// Args:
    ///     item: Item to add (int, str, or bytes)
    ///
    /// Example:
    ///     >>> ss = SpaceSaving(epsilon=0.1)
    ///     >>> ss.update("apple")
    ///     >>> ss.update("banana")
    ///     >>> ss.update("apple")
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        let key = self.py_item_to_bytes(item)?;
        self.inner.update(key);
        Ok(())
    }

    /// Estimate the frequency bounds of an item
    ///
    /// Args:
    ///     item: Item to query (int, str, or bytes)
    ///
    /// Returns:
    ///     tuple: (lower_bound, upper_bound) if item is tracked, None otherwise
    ///     The true frequency is guaranteed to be in this range.
    ///
    /// Example:
    ///     >>> ss = SpaceSaving(epsilon=0.1)
    ///     >>> for _ in range(42):
    ///     ...     ss.update("test")
    ///     >>> bounds = ss.estimate("test")
    ///     >>> if bounds:
    ///     ...     lower, upper = bounds
    ///     ...     assert lower <= 42 <= upper
    fn estimate(&self, item: &Bound<'_, PyAny>) -> PyResult<Option<(u64, u64)>> {
        let key = self.py_item_to_bytes(item)?;
        Ok(self.inner.estimate(&key))
    }

    /// Get all items that may be heavy hitters above a frequency threshold
    ///
    /// A heavy hitter is an item whose true frequency exceeds threshold * stream_length.
    ///
    /// Args:
    ///     threshold: Frequency threshold in (0, 1)
    ///
    /// Returns:
    ///     list: List of (item, lower_bound, upper_bound) tuples, sorted by
    ///     estimated count descending. Items are returned as bytes.
    ///
    /// Example:
    ///     >>> ss = SpaceSaving(epsilon=0.01)
    ///     >>> for _ in range(1000):
    ///     ...     ss.update("common")
    ///     >>> for _ in range(10):
    ///     ...     ss.update("rare")
    ///     >>> # Find items with > 5% frequency
    ///     >>> heavy = ss.heavy_hitters(0.05)
    ///     >>> for item, lower, upper in heavy:
    ///     ...     print(f"{item}: [{lower}, {upper}]")
    fn heavy_hitters(&self, threshold: f64) -> PyResult<Vec<(PyObject, u64, u64)>> {
        let results = self.inner.heavy_hitters(threshold);
        let py = unsafe { pyo3::Python::assume_gil_acquired() };

        Ok(results
            .into_iter()
            .map(|(key, lower, upper)| {
                let py_bytes = PyBytes::new_bound(py, &key);
                (py_bytes.into(), lower, upper)
            })
            .collect())
    }

    /// Get the top-k most frequent items
    ///
    /// Args:
    ///     k: Number of items to return
    ///
    /// Returns:
    ///     list: List of at most k (item, lower_bound, upper_bound) tuples
    ///     sorted by estimated count (upper_bound) descending.
    ///     Items are returned as bytes.
    ///
    /// Example:
    ///     >>> ss = SpaceSaving(epsilon=0.01)
    ///     >>> for i in range(100):
    ///     ...     for _ in range(i + 1):
    ///     ...         ss.update(str(i))
    ///     >>> top10 = ss.top_k(10)
    ///     >>> assert len(top10) <= 10
    fn top_k(&self, k: usize) -> PyResult<Vec<(PyObject, u64, u64)>> {
        let results = self.inner.top_k(k);
        let py = unsafe { pyo3::Python::assume_gil_acquired() };

        Ok(results
            .into_iter()
            .map(|(key, lower, upper)| {
                let py_bytes = PyBytes::new_bound(py, &key);
                (py_bytes.into(), lower, upper)
            })
            .collect())
    }

    /// Merge another Space-Saving sketch into this one
    ///
    /// Args:
    ///     other: Another SpaceSaving with same epsilon/capacity
    ///
    /// Raises:
    ///     ValueError: If sketches have incompatible dimensions
    ///
    /// Example:
    ///     >>> ss1 = SpaceSaving(epsilon=0.1)
    ///     >>> ss2 = SpaceSaving(epsilon=0.1)
    ///     >>> ss1.update("item")
    ///     >>> ss2.update("item")
    ///     >>> ss1.merge(ss2)
    fn merge(&mut self, other: &SpaceSaving) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Serialize the sketch to bytes
    ///
    /// Returns:
    ///     bytes: Serialized sketch data
    fn serialize(&self) -> PyObject {
        let py = unsafe { pyo3::Python::assume_gil_acquired() };
        PyBytes::new_bound(py, &self.inner.serialize()).into()
    }

    /// Deserialize a sketch from bytes
    ///
    /// Args:
    ///     data: Serialized sketch data (from empty sketch only)
    ///
    /// Returns:
    ///     SpaceSaving: Deserialized sketch
    ///
    /// Raises:
    ///     ValueError: If data is invalid
    #[staticmethod]
    fn deserialize(data: &Bound<'_, PyAny>) -> PyResult<SpaceSaving> {
        let bytes = if let Ok(b) = data.downcast::<PyBytes>() {
            b.as_bytes().to_vec()
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "data must be bytes",
            ));
        };

        RustSpaceSaving::deserialize(&bytes)
            .map(|inner| SpaceSaving { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Check if the sketch is empty
    ///
    /// Returns:
    ///     bool: True if no items have been added
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get the capacity (maximum number of items tracked)
    ///
    /// Returns:
    ///     int: Capacity = ceil(1/epsilon)
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Get the epsilon parameter
    ///
    /// Returns:
    ///     float: Error threshold parameter
    fn epsilon(&self) -> f64 {
        self.inner.epsilon()
    }

    /// Get the total number of items seen in the stream
    ///
    /// Returns:
    ///     int: Stream length
    fn stream_length(&self) -> u64 {
        self.inner.stream_length()
    }

    /// Get the number of items currently tracked
    ///
    /// Returns:
    ///     int: Number of items in the sketch
    fn num_items(&self) -> usize {
        self.inner.num_items()
    }

    /// Get the maximum possible error for any item
    ///
    /// Returns:
    ///     int: Maximum error = ceil(stream_length * epsilon)
    fn max_error(&self) -> u64 {
        self.inner.max_error()
    }

    fn __repr__(&self) -> String {
        format!(
            "SpaceSaving(capacity={}, tracking={}, stream_length={}, epsilon={})",
            self.inner.capacity(),
            self.inner.num_items(),
            self.inner.stream_length(),
            self.inner.epsilon()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "SpaceSaving({} items tracked, {} seen)",
            self.inner.num_items(),
            self.inner.stream_length()
        )
    }

    fn __len__(&self) -> usize {
        self.inner.num_items()
    }
}

impl SpaceSaving {
    /// Helper to convert Python items to bytes
    fn py_item_to_bytes(&self, item: &Bound<'_, PyAny>) -> PyResult<Vec<u8>> {
        if let Ok(val) = item.extract::<i64>() {
            Ok(val.to_le_bytes().to_vec())
        } else if let Ok(val) = item.extract::<u64>() {
            Ok(val.to_le_bytes().to_vec())
        } else if let Ok(val) = item.extract::<String>() {
            Ok(val.into_bytes())
        } else if let Ok(b) = item.downcast::<PyBytes>() {
            Ok(b.as_bytes().to_vec())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Item must be int, str, or bytes",
            ))
        }
    }
}
