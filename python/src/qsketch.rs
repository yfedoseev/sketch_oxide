//! Python bindings for QSketch weighted cardinality estimation

use pyo3::prelude::*;
use sketch_oxide::cardinality::QSketch as RustQSketch;
use sketch_oxide::Sketch;

use crate::common::python_item_to_hash;

/// QSketch for weighted cardinality estimation
///
/// QSketch is a state-of-the-art algorithm for estimating the cardinality of weighted elements,
/// combining the benefits of probabilistic sampling with weight-aware estimation.
///
/// Unlike standard cardinality sketches that treat all items equally, QSketch accounts
/// for item weights in the cardinality estimate, making it ideal for scenarios where
/// elements have varying importance or cost.
///
/// Args:
///     max_samples (int): Maximum number of samples to maintain (default: 256).
///         - Must be >= 32 for meaningful estimation
///         - Larger values increase accuracy but use more memory
///         - Common values: 64, 128, 256, 512
///
/// Example:
///     >>> qsketch = QSketch(max_samples=256)
///     >>> qsketch.update("user_123", 100.0)  # User worth 100 units
///     >>> qsketch.update("user_456", 250.0)  # User worth 250 units
///     >>> estimate, error = qsketch.estimate_weighted_cardinality()
///     >>> print(f"Estimate: {estimate:.0f} ± {error:.0f}")
///
/// Notes:
///     - Supports any hashable Python types: int, str, bytes, float
///     - All weights must be positive finite numbers
///     - Mergeable with other QSketch sketches of same max_samples
///     - Standard error scales with sqrt(sample_size)
#[pyclass(module = "sketch_oxide")]
pub struct QSketch {
    inner: RustQSketch,
}

#[pymethods]
impl QSketch {
    /// Create a new QSketch with specified maximum sample size
    ///
    /// Args:
    ///     max_samples: Maximum number of samples to maintain (must be >= 32)
    ///
    /// Raises:
    ///     ValueError: If max_samples < 32
    ///
    /// Example:
    ///     >>> qsketch = QSketch(max_samples=256)
    #[new]
    fn new(max_samples: usize) -> PyResult<Self> {
        if max_samples < 32 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "max_samples must be at least 32",
            ));
        }
        Ok(Self {
            inner: RustQSketch::new(max_samples),
        })
    }

    /// Update the sketch with a weighted item
    ///
    /// Args:
    ///     item: Item to add (int, str, bytes, or float)
    ///     weight: Weight of the item (must be positive and finite)
    ///
    /// Raises:
    ///     TypeError: If item type is not supported
    ///     ValueError: If weight is not positive or is NaN/infinite
    ///
    /// Example:
    ///     >>> qsketch = QSketch(max_samples=256)
    ///     >>> qsketch.update("user_123", 100.0)
    ///     >>> qsketch.update(456, 50.5)
    ///     >>> qsketch.update(b"binary_data", 75.0)
    fn update(&mut self, item: &Bound<'_, PyAny>, weight: f64) -> PyResult<()> {
        // Validate weight
        if !weight.is_finite() || weight <= 0.0 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Weight must be positive and finite, got {}",
                weight
            )));
        }

        // Convert Python item to hash
        let hash_val = python_item_to_hash(item)?;

        // Update the sketch
        // Note: We need to work around the hash-based interface
        // by creating a temporary item for hashing purposes
        self.inner.update(&hash_val.to_le_bytes(), weight);

        Ok(())
    }

    /// Estimate the weighted cardinality with confidence bounds
    ///
    /// Returns a tuple (estimate, error_bound) where error_bound represents
    /// the radius of a 95% confidence interval around the estimate.
    ///
    /// Returns:
    ///     tuple: (estimate, error_bound) where both are floats
    ///         - estimate: Estimated weighted cardinality
    ///         - error_bound: Radius of 95% confidence interval
    ///
    /// Example:
    ///     >>> qsketch = QSketch(max_samples=256)
    ///     >>> for i in range(1000):
    ///     ...     qsketch.update(f"item_{i}", 1.0 + i * 0.1)
    ///     >>> estimate, error = qsketch.estimate_weighted_cardinality()
    ///     >>> print(f"Cardinality: {estimate:.0f} ± {error:.0f}")
    ///     >>> assert estimate > 0.0
    fn estimate_weighted_cardinality(&self) -> (f64, f64) {
        self.inner.estimate_weighted_cardinality()
    }

    /// Estimate the number of distinct elements
    ///
    /// Returns the estimated number of distinct items that have been added,
    /// regardless of their weights.
    ///
    /// Returns:
    ///     int: Estimated count of distinct items
    ///
    /// Example:
    ///     >>> qsketch = QSketch(max_samples=256)
    ///     >>> qsketch.update("item_1", 10.0)
    ///     >>> qsketch.update("item_2", 20.0)
    ///     >>> qsketch.update("item_1", 5.0)  # Duplicate - weight adds up
    ///     >>> assert qsketch.estimate_distinct_elements() == 2
    fn estimate_distinct_elements(&self) -> u64 {
        self.inner.estimate_distinct_elements()
    }

    /// Get the total sum of all weights added to the sketch
    ///
    /// Returns:
    ///     float: Sum of all weights of all items seen (not just sampled items)
    ///
    /// Example:
    ///     >>> qsketch = QSketch(max_samples=256)
    ///     >>> qsketch.update("item_1", 10.5)
    ///     >>> qsketch.update("item_2", 20.5)
    ///     >>> assert abs(qsketch.total_weight() - 31.0) < 0.001
    fn total_weight(&self) -> f64 {
        self.inner.total_weight()
    }

    /// Get the maximum number of samples this sketch can maintain
    ///
    /// Returns:
    ///     int: Maximum sample size
    ///
    /// Example:
    ///     >>> qsketch = QSketch(max_samples=256)
    ///     >>> assert qsketch.max_samples() == 256
    fn max_samples(&self) -> usize {
        self.inner.max_samples()
    }

    /// Get the current number of samples in the sketch
    ///
    /// Returns:
    ///     int: Current sample count (0 to max_samples)
    ///
    /// Example:
    ///     >>> qsketch = QSketch(max_samples=256)
    ///     >>> qsketch.update("item_1", 10.0)
    ///     >>> qsketch.update("item_2", 20.0)
    ///     >>> assert qsketch.sample_count() == 2
    fn sample_count(&self) -> usize {
        self.inner.sample_count()
    }

    /// Merge another QSketch into this one
    ///
    /// Combines the weighted cardinality estimates from both sketches.
    /// Both sketches must have the same max_samples for optimal results.
    ///
    /// Args:
    ///     other: Another QSketch with the same max_samples
    ///
    /// Raises:
    ///     ValueError: If sketches have different max_samples
    ///
    /// Example:
    ///     >>> qsketch1 = QSketch(max_samples=256)
    ///     >>> qsketch2 = QSketch(max_samples=256)
    ///     >>> for i in range(500):
    ///     ...     qsketch1.update(f"item_{i}", 1.0)
    ///     >>> for i in range(500, 1000):
    ///     ...     qsketch2.update(f"item_{i}", 1.0)
    ///     >>> qsketch1.merge(qsketch2)
    ///     >>> # Now qsketch1 has estimates for ~1000 items
    fn merge(&mut self, other: &QSketch) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Reset the sketch to an empty state
    ///
    /// Clears all samples and weights. This is useful for reusing the sketch
    /// or when starting a new analysis phase.
    ///
    /// Example:
    ///     >>> qsketch = QSketch(max_samples=256)
    ///     >>> qsketch.update("item_1", 10.0)
    ///     >>> assert not qsketch.is_empty()
    ///     >>> qsketch.reset()
    ///     >>> assert qsketch.is_empty()
    fn reset(&mut self) {
        self.inner.reset()
    }

    /// Check if the sketch is empty
    ///
    /// Returns:
    ///     bool: True if no items have been added
    ///
    /// Example:
    ///     >>> qsketch = QSketch(max_samples=256)
    ///     >>> assert qsketch.is_empty()
    ///     >>> qsketch.update("item", 1.0)
    ///     >>> assert not qsketch.is_empty()
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Serialize the sketch to bytes
    ///
    /// Returns:
    ///     bytes: Serialized sketch data that can be stored or transmitted
    ///
    /// Example:
    ///     >>> qsketch = QSketch(max_samples=256)
    ///     >>> qsketch.update("item_1", 10.0)
    ///     >>> qsketch.update("item_2", 20.0)
    ///     >>> data = qsketch.serialize()
    ///     >>> assert isinstance(data, bytes)
    ///     >>> assert len(data) > 0
    fn serialize(&self, py: Python) -> PyResult<Py<pyo3::types::PyBytes>> {
        let data = self.inner.serialize();
        Ok(pyo3::types::PyBytes::new_bound(py, &data).into())
    }

    /// Deserialize a sketch from bytes
    ///
    /// Creates a new QSketch from serialized data that was previously
    /// created with serialize().
    ///
    /// Args:
    ///     data: Bytes to deserialize
    ///
    /// Returns:
    ///     QSketch: Reconstructed sketch
    ///
    /// Raises:
    ///     ValueError: If data is invalid or corrupted
    ///
    /// Example:
    ///     >>> qsketch1 = QSketch(max_samples=256)
    ///     >>> qsketch1.update("item_1", 10.0)
    ///     >>> qsketch1.update("item_2", 20.0)
    ///     >>> data = qsketch1.serialize()
    ///     >>> qsketch2 = QSketch.deserialize(data)
    ///     >>> assert qsketch2.estimate_distinct_elements() == qsketch1.estimate_distinct_elements()
    #[staticmethod]
    fn deserialize(data: &[u8]) -> PyResult<Self> {
        RustQSketch::deserialize(data)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "QSketch(max_samples={}, sample_count={}, distinct={}, total_weight={:.2})",
            self.inner.max_samples(),
            self.inner.sample_count(),
            self.inner.estimate_distinct_elements(),
            self.inner.total_weight()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "QSketch(distinct_estimate={}, total_weight={:.2})",
            self.inner.estimate_distinct_elements(),
            self.inner.total_weight()
        )
    }
}

#[cfg(test)]
mod tests {
    // Note: These tests require Python runtime, so they're primarily tested
    // through integration tests. Unit tests here are basic compatibility checks.

    #[test]
    fn test_module_integration() {
        // Verify the PyO3 class can be referenced
        // Actual runtime tests are in Python integration tests
    }
}
