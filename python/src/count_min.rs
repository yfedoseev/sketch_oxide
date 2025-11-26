//! Python bindings for Count-Min Sketch frequency estimation

use pyo3::prelude::*;
use pyo3::types::PyList;
use sketch_oxide::frequency::CountMinSketch as RustCountMinSketch;
use sketch_oxide::{Mergeable, Sketch};

use crate::with_python_item;

/// Count-Min Sketch for frequency estimation (Cormode & Muthukrishnan, 2003)
///
/// A space-efficient probabilistic data structure for estimating item frequencies.
/// Never underestimates (always returns count >= true count).
///
/// Args:
///     epsilon (float): Error bound (0 < ε < 1). Estimates within εN of true value.
///         - 0.001: High accuracy, more memory
///         - 0.01: Balanced (recommended)
///         - 0.1: Low accuracy, less memory
///     delta (float): Failure probability (0 < δ < 1). Guarantee holds with prob 1-δ.
///         - 0.01: 99% confidence (recommended)
///         - 0.001: 99.9% confidence
///
/// Example:
///     >>> cms = CountMinSketch(epsilon=0.01, delta=0.01)
///     >>> cms.update("apple")
///     >>> cms.update("apple")
///     >>> cms.update("banana")
///     >>> assert cms.estimate("apple") >= 2  # Never underestimates
///     >>> assert cms.estimate("banana") >= 1
///
/// Notes:
///     - Supports int, str, and bytes types
///     - Never underestimates frequencies
///     - May overestimate due to hash collisions
///     - Space: O((e/ε) * ln(1/δ)) ≈ O(1/ε * log(1/δ))
#[pyclass(module = "sketch_oxide")]
pub struct CountMinSketch {
    inner: RustCountMinSketch,
}

#[pymethods]
impl CountMinSketch {
    /// Create a new Count-Min Sketch
    ///
    /// Args:
    ///     epsilon: Error bound (0 < ε < 1)
    ///     delta: Failure probability (0 < δ < 1)
    ///
    /// Raises:
    ///     ValueError: If parameters are not in valid range
    #[new]
    fn new(epsilon: f64, delta: f64) -> PyResult<Self> {
        RustCountMinSketch::new(epsilon, delta)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Update the sketch with a single item
    ///
    /// Args:
    ///     item: Item to add (int, str, or bytes)
    ///
    /// Example:
    ///     >>> cms = CountMinSketch(epsilon=0.01, delta=0.01)
    ///     >>> cms.update("user123")
    ///     >>> cms.update("user456")
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        with_python_item!(item, |rust_item| {
            self.inner.update(rust_item);
        })
    }

    /// Estimate the frequency of an item
    ///
    /// Args:
    ///     item: Item to query (int, str, or bytes)
    ///
    /// Returns:
    ///     int: Estimated frequency (>= true frequency)
    ///
    /// Example:
    ///     >>> cms = CountMinSketch(epsilon=0.01, delta=0.01)
    ///     >>> cms.update("apple")
    ///     >>> cms.update("apple")
    ///     >>> freq = cms.estimate("apple")
    ///     >>> assert freq >= 2
    fn estimate(&self, item: &Bound<'_, PyAny>) -> PyResult<u64> {
        with_python_item!(item, |rust_item| { self.inner.estimate(rust_item) })
    }

    /// Merge another Count-Min Sketch into this one
    ///
    /// Args:
    ///     other: Another CountMinSketch with same dimensions
    ///
    /// Raises:
    ///     ValueError: If sketches have incompatible dimensions
    ///
    /// Example:
    ///     >>> cms1 = CountMinSketch(epsilon=0.01, delta=0.01)
    ///     >>> cms2 = CountMinSketch(epsilon=0.01, delta=0.01)
    ///     >>> # ... add data to both ...
    ///     >>> cms1.merge(cms2)
    fn merge(&mut self, other: &CountMinSketch) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
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
    ///     float: Error bound parameter
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

    /// Update the sketch with a batch of items (optimized for throughput)
    ///
    /// Batch updates are significantly faster than multiple individual update() calls
    /// because they amortize the FFI (Foreign Function Interface) overhead across
    /// many items. This is the preferred method when adding large quantities of data.
    ///
    /// Args:
    ///     items: Iterable of items to add (int, str, or bytes types)
    ///
    /// Example:
    ///     >>> cms = CountMinSketch(epsilon=0.01, delta=0.01)
    ///     >>> cms.update_batch(["apple", "banana", "apple"])
    fn update_batch(&mut self, items: &Bound<'_, PyAny>) -> PyResult<()> {
        let items_list: &Bound<'_, PyList> = items.downcast()?;
        for item in items_list {
            self.update(&item)?;
        }
        Ok(())
    }

    /// Estimate frequencies of multiple items in a single call (optimized for lookups)
    ///
    /// Batch frequency lookups are faster than multiple individual estimate() calls.
    ///
    /// Args:
    ///     items: Iterable of items to query (int, str, or bytes types)
    ///
    /// Returns:
    ///     list: List of estimated frequencies (>= true frequencies), one per item
    ///
    /// Example:
    ///     >>> cms = CountMinSketch(epsilon=0.01, delta=0.01)
    ///     >>> estimates = cms.estimate_batch(["apple", "banana"])
    fn estimate_batch(&self, items: &Bound<'_, PyAny>) -> PyResult<Vec<u64>> {
        let items_list: &Bound<'_, PyList> = items.downcast()?;
        let mut estimates = Vec::new();
        for item in items_list {
            estimates.push(self.estimate(&item)?);
        }
        Ok(estimates)
    }

    fn __repr__(&self) -> String {
        format!(
            "CountMinSketch(width={}, depth={}, epsilon={}, delta={})",
            self.inner.width(),
            self.inner.depth(),
            self.inner.epsilon(),
            self.inner.delta()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "CountMinSketch({}x{} table)",
            self.inner.depth(),
            self.inner.width()
        )
    }
}
