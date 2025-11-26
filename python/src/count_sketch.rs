//! Python bindings for Count Sketch frequency estimation

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList};
use sketch_oxide::frequency::CountSketch as RustCountSketch;
use sketch_oxide::{Mergeable, Sketch};

use crate::with_python_item;

/// Count Sketch for unbiased frequency estimation (Charikar, Chen, Farach-Colton, 2002)
///
/// A linear sketch that provides unbiased frequency estimates using sign hashing.
/// Unlike Count-Min Sketch, Count Sketch:
/// - Can estimate both positive and negative frequencies
/// - Has E[estimate] = true_count (unbiased)
/// - Provides L2 error guarantees: error <= epsilon * ||f||_2
/// - Supports inner product estimation
///
/// Args:
///     epsilon (float): L2 error bound (0 < ε < 1). Error <= ε * ||frequency_vector||_2
///         - 0.01: High accuracy, more memory
///         - 0.1: Balanced (recommended)
///         - 0.5: Low accuracy, less memory
///     delta (float): Failure probability (0 < δ < 1). Guarantee holds with prob 1-δ.
///         - 0.01: 99% confidence (recommended)
///         - 0.001: 99.9% confidence
///
/// Example:
///     >>> cs = CountSketch(epsilon=0.1, delta=0.01)
///     >>> cs.update("apple", 5)
///     >>> cs.update("apple", -2)  # Decrements supported
///     >>> estimate = cs.estimate("apple")
///     >>> assert abs(estimate - 3) <= 2  # Unbiased, allows variance
///
/// Notes:
///     - Supports int, str, and bytes types
///     - Unbiased: E[estimate] = true count
///     - Can estimate negative frequencies (deletions)
///     - Better L2 error bounds than Count-Min for skewed distributions
///     - Space: O((1/ε²) * ln(1/δ))
#[pyclass(module = "sketch_oxide")]
pub struct CountSketch {
    inner: RustCountSketch,
}

#[pymethods]
impl CountSketch {
    /// Create a new Count Sketch
    ///
    /// Args:
    ///     epsilon: L2 error bound (0 < ε < 1)
    ///     delta: Failure probability (0 < δ < 1)
    ///
    /// Raises:
    ///     ValueError: If parameters are not in valid range
    #[new]
    fn new(epsilon: f64, delta: f64) -> PyResult<Self> {
        RustCountSketch::new(epsilon, delta)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Update the sketch with an item and delta
    ///
    /// Args:
    ///     item: Item to add (int, str, or bytes)
    ///     delta: Count to add (positive for insertions, negative for deletions)
    ///
    /// Example:
    ///     >>> cs = CountSketch(epsilon=0.1, delta=0.01)
    ///     >>> cs.update("user123", 1)
    ///     >>> cs.update("user123", -1)  # Delete one occurrence
    ///     >>> cs.update("user456", 5)
    #[pyo3(signature = (item, delta=1))]
    fn update(&mut self, item: &Bound<'_, PyAny>, delta: i64) -> PyResult<()> {
        with_python_item!(item, |rust_item| {
            self.inner.update(rust_item, delta);
        })
    }

    /// Estimate the frequency of an item
    ///
    /// Args:
    ///     item: Item to query (int, str, or bytes)
    ///
    /// Returns:
    ///     int: Estimated frequency (can be negative)
    ///
    /// Example:
    ///     >>> cs = CountSketch(epsilon=0.1, delta=0.01)
    ///     >>> cs.update("apple", 10)
    ///     >>> estimate = cs.estimate("apple")
    ///     >>> # estimate is approximately 10 (unbiased, may vary)
    fn estimate(&self, item: &Bound<'_, PyAny>) -> PyResult<i64> {
        with_python_item!(item, |rust_item| { self.inner.estimate(rust_item) })
    }

    /// Estimate the inner product of two frequency vectors
    ///
    /// Given two Count Sketches built from streams A and B, estimates
    /// the inner product sum_x(f_A(x) * f_B(x)).
    ///
    /// Args:
    ///     other: Another CountSketch with same dimensions
    ///
    /// Returns:
    ///     int: Estimated inner product
    ///
    /// Example:
    ///     >>> cs1 = CountSketch(epsilon=0.1, delta=0.01)
    ///     >>> cs2 = CountSketch(epsilon=0.1, delta=0.01)
    ///     >>> cs1.update("a", 3)
    ///     >>> cs1.update("b", 2)
    ///     >>> cs2.update("a", 4)
    ///     >>> cs2.update("b", 5)
    ///     >>> inner = cs1.inner_product(cs2)
    ///     >>> # Expected: ~22 (3*4 + 2*5)
    fn inner_product(&self, other: &CountSketch) -> i64 {
        self.inner.inner_product(&other.inner)
    }

    /// Merge another Count Sketch into this one
    ///
    /// Args:
    ///     other: Another CountSketch with same dimensions
    ///
    /// Raises:
    ///     ValueError: If sketches have incompatible dimensions
    ///
    /// Example:
    ///     >>> cs1 = CountSketch(epsilon=0.1, delta=0.01)
    ///     >>> cs2 = CountSketch(epsilon=0.1, delta=0.01)
    ///     >>> cs1.update("item", 10)
    ///     >>> cs2.update("item", 5)
    ///     >>> cs1.merge(cs2)
    ///     >>> assert cs1.estimate("item") >= 10  # Should be approximately 15
    fn merge(&mut self, other: &CountSketch) -> PyResult<()> {
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
    ///     data: Serialized sketch data
    ///
    /// Returns:
    ///     CountSketch: Deserialized sketch
    ///
    /// Raises:
    ///     ValueError: If data is invalid
    #[staticmethod]
    fn deserialize(data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let bytes = if let Ok(b) = data.downcast::<PyBytes>() {
            b.as_bytes().to_vec()
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "data must be bytes",
            ));
        };

        RustCountSketch::deserialize(&bytes)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Check if the sketch is empty
    ///
    /// Returns:
    ///     bool: True if no items have been added
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get the epsilon parameter
    ///
    /// Returns:
    ///     float: L2 error bound parameter
    fn epsilon(&self) -> f64 {
        self.inner.epsilon()
    }

    /// Get the delta parameter
    ///
    /// Returns:
    ///     float: Failure probability parameter
    fn delta(&self) -> f64 {
        self.inner.delta()
    }

    /// Get the width (number of counters per row)
    ///
    /// Returns:
    ///     int: Width of the sketch
    fn width(&self) -> usize {
        self.inner.width()
    }

    /// Get the depth (number of hash functions)
    ///
    /// Returns:
    ///     int: Depth of the sketch
    fn depth(&self) -> usize {
        self.inner.depth()
    }

    fn __repr__(&self) -> String {
        format!(
            "CountSketch(width={}, depth={}, epsilon={}, delta={})",
            self.inner.width(),
            self.inner.depth(),
            self.inner.epsilon(),
            self.inner.delta()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "CountSketch({}x{} table, unbiased estimation)",
            self.inner.depth(),
            self.inner.width()
        )
    }

    /// Update the sketch with multiple items in a single call (optimized for throughput).
    ///
    /// Batch updates are significantly faster than multiple individual update() calls
    /// because they amortize the FFI (Foreign Function Interface) overhead across
    /// many items. This is the preferred method when adding large quantities of data.
    ///
    /// Args:
    ///     items: Iterable of tuples (item, delta) or items (delta defaults to 1)
    fn update_batch(&mut self, items: &Bound<'_, PyAny>) -> PyResult<()> {
        let items_list: &Bound<'_, PyList> = items.downcast()?;
        for item_tuple in items_list {
            // Try to unpack as (item, delta) tuple first
            if let Ok(tuple) = item_tuple.downcast::<pyo3::types::PyTuple>() {
                if tuple.len() == 2 {
                    let item = &tuple.get_item(0)?;
                    let delta: i64 = tuple.get_item(1)?.extract()?;
                    self.update(item, delta)?;
                    continue;
                }
            }
            // Fall back to single item with delta=1
            self.update(&item_tuple, 1)?;
        }
        Ok(())
    }

    /// Estimate frequencies of multiple items in a single call (optimized for lookups).
    ///
    /// Batch frequency lookups are faster than multiple individual estimate() calls.
    ///
    /// Args:
    ///     items: Iterable of items to query (int, str, or bytes types)
    ///
    /// Returns:
    ///     list: List of estimated frequencies (can be negative), one per item
    fn estimate_batch(&self, items: &Bound<'_, PyAny>) -> PyResult<Vec<i64>> {
        let items_list: &Bound<'_, PyList> = items.downcast()?;
        let mut estimates = Vec::new();
        for item in items_list {
            estimates.push(self.estimate(&item)?);
        }
        Ok(estimates)
    }
}
