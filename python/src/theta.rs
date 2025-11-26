//! Python bindings for Theta Sketch set operations

use pyo3::prelude::*;
use sketch_oxide::cardinality::ThetaSketch as RustThetaSketch;

use crate::with_python_item;

/// Theta Sketch for cardinality estimation with set operations
///
/// **The only sketch supporting intersection/difference operations.**
/// Production-proven by LinkedIn, ClickHouse 24.1+, Yahoo.
///
/// Args:
///     lg_k (int): log2(k) parameter (4-26). k = 2^lg_k is the number of entries.
///         - lg_k=8: 256 entries, ~3% error
///         - lg_k=12: 4096 entries, ~1.6% error (recommended)
///         - lg_k=16: 65536 entries, ~0.4% error
///
/// Example:
///     >>> sketch_a = ThetaSketch(lg_k=12)
///     >>> sketch_b = ThetaSketch(lg_k=12)
///     >>> # ... add items ...
///     >>> union = sketch_a.union(sketch_b)
///     >>> intersection = sketch_a.intersect(sketch_b)
///     >>> difference = sketch_a.difference(sketch_b)
///
/// Notes:
///     - Supports int, str, and bytes types
///     - Exact mode when n < k (no sampling)
///     - Set operations: union, intersection, difference
#[pyclass(module = "sketch_oxide")]
pub struct ThetaSketch {
    inner: RustThetaSketch,
}

#[pymethods]
impl ThetaSketch {
    /// Create a new Theta Sketch
    ///
    /// Args:
    ///     lg_k: log2(k) parameter (4-26)
    ///
    /// Raises:
    ///     ValueError: If lg_k is not in range [4, 26]
    #[new]
    fn new(lg_k: u8) -> PyResult<Self> {
        RustThetaSketch::new(lg_k)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Update the sketch with a single item
    ///
    /// Args:
    ///     item: Item to add (int, str, or bytes)
    ///
    /// Example:
    ///     >>> theta = ThetaSketch(lg_k=12)
    ///     >>> theta.update("user123")
    ///     >>> theta.update(42)
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        with_python_item!(item, |rust_item| {
            self.inner.update(rust_item);
        })
    }

    /// Get the estimated cardinality
    ///
    /// Returns:
    ///     float: Estimated number of unique items
    fn estimate(&self) -> f64 {
        self.inner.estimate()
    }

    /// Compute the union of two sketches (|A ∪ B|)
    ///
    /// Args:
    ///     other: Another ThetaSketch
    ///
    /// Returns:
    ///     ThetaSketch: New sketch representing the union
    ///
    /// Example:
    ///     >>> sketch_a = ThetaSketch(lg_k=12)
    ///     >>> sketch_b = ThetaSketch(lg_k=12)
    ///     >>> # ... add items ...
    ///     >>> union = sketch_a.union(sketch_b)
    ///     >>> print(f"Union cardinality: {union.estimate()}")
    fn union(&self, other: &ThetaSketch) -> PyResult<Self> {
        self.inner
            .union(&other.inner)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Compute the intersection of two sketches (|A ∩ B|)
    ///
    /// Args:
    ///     other: Another ThetaSketch
    ///
    /// Returns:
    ///     ThetaSketch: New sketch representing the intersection
    ///
    /// Example:
    ///     >>> intersection = sketch_a.intersect(sketch_b)
    ///     >>> print(f"Intersection cardinality: {intersection.estimate()}")
    fn intersect(&self, other: &ThetaSketch) -> PyResult<Self> {
        self.inner
            .intersect(&other.inner)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Compute the difference of two sketches (|A - B|)
    ///
    /// Args:
    ///     other: Another ThetaSketch
    ///
    /// Returns:
    ///     ThetaSketch: New sketch representing A minus B
    ///
    /// Example:
    ///     >>> difference = sketch_a.difference(sketch_b)
    ///     >>> print(f"Difference cardinality: {difference.estimate()}")
    fn difference(&self, other: &ThetaSketch) -> PyResult<Self> {
        self.inner
            .difference(&other.inner)
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

    fn __repr__(&self) -> String {
        format!("ThetaSketch(estimate={:.0})", self.inner.estimate())
    }

    fn __str__(&self) -> String {
        format!("ThetaSketch(estimate={:.0})", self.inner.estimate())
    }
}
