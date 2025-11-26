//! Python bindings for CPC Sketch cardinality estimation

use pyo3::prelude::*;
use sketch_oxide::cardinality::CpcSketch as RustCpcSketch;
use sketch_oxide::{Mergeable, Sketch};

use crate::common::python_item_to_hash;

/// CPC (Compressed Probabilistic Counting) Sketch for cardinality estimation
///
/// **30-40% more space-efficient than HyperLogLog** for the same accuracy.
/// Achieves maximum space efficiency through adaptive compression.
///
/// Args:
///     lg_k (int): log2(k) parameter (4-26). k = 2^lg_k is the number of virtual slots.
///         - lg_k=8: ~1.5KB, ~3% error
///         - lg_k=11: ~4KB, ~1.5% error (recommended)
///         - lg_k=14: ~16KB, ~0.75% error
///
/// Example:
///     >>> cpc = CpcSketch(lg_k=11)
///     >>> for user_id in user_ids:
///     ...     cpc.update(user_id)
///     >>> print(f"Unique users: {cpc.estimate():.0f}")
///
/// Notes:
///     - Most space-efficient cardinality sketch available
///     - Adaptive: Uses sparse mode for low cardinality, compressed for high
///     - Supports int, str, and bytes types
///     - Mergeable with other CPC sketches of same lg_k
#[pyclass(module = "sketch_oxide")]
pub struct CpcSketch {
    inner: RustCpcSketch,
}

#[pymethods]
impl CpcSketch {
    /// Create a new CPC Sketch
    ///
    /// Args:
    ///     lg_k: log2(k) parameter (4-26)
    ///
    /// Raises:
    ///     ValueError: If lg_k is not in range [4, 26]
    #[new]
    fn new(lg_k: u8) -> PyResult<Self> {
        RustCpcSketch::new(lg_k)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Update the sketch with a single item
    ///
    /// Args:
    ///     item: Item to add (int, str, or bytes)
    ///
    /// Example:
    ///     >>> cpc = CpcSketch(lg_k=11)
    ///     >>> cpc.update("user123")
    ///     >>> cpc.update(42)
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        let hash_val = python_item_to_hash(item)?;
        self.inner.update(&hash_val);
        Ok(())
    }

    /// Get the estimated cardinality
    ///
    /// Returns:
    ///     float: Estimated number of unique items
    ///
    /// Example:
    ///     >>> cpc = CpcSketch(lg_k=11)
    ///     >>> for i in range(10000):
    ///     ...     cpc.update(i)
    ///     >>> estimate = cpc.estimate()
    ///     >>> print(f"Estimate: {estimate:.0f}")  # Should be close to 10000
    fn estimate(&self) -> f64 {
        self.inner.estimate()
    }

    /// Merge another CPC Sketch into this one
    ///
    /// Args:
    ///     other: Another CpcSketch with the same lg_k
    ///
    /// Raises:
    ///     ValueError: If sketches have different lg_k values
    ///
    /// Example:
    ///     >>> cpc1 = CpcSketch(lg_k=11)
    ///     >>> cpc2 = CpcSketch(lg_k=11)
    ///     >>> # ... add data to both ...
    ///     >>> cpc1.merge(cpc2)
    fn merge(&mut self, other: &CpcSketch) -> PyResult<()> {
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

    /// Clear all data from the sketch
    fn clear(&mut self) {
        self.inner.clear();
    }

    /// Get the lg_k parameter
    ///
    /// Returns:
    ///     int: log2(k) parameter
    fn lg_k(&self) -> u8 {
        self.inner.lg_k()
    }

    fn __repr__(&self) -> String {
        format!(
            "CpcSketch(lg_k={}, estimate={:.0})",
            self.inner.lg_k(),
            self.inner.estimate()
        )
    }

    fn __str__(&self) -> String {
        format!("CpcSketch(estimate={:.0})", self.inner.estimate())
    }
}
