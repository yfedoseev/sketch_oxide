//! Python bindings for ConservativeCountMin

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList};
use sketch_oxide::frequency::ConservativeCountMin as RustConservativeCountMin;
use std::hash::{Hash, Hasher};

/// Conservative Update Count-Min Sketch
///
/// Improved accuracy variant of Count-Min Sketch that reduces overestimation
/// by only incrementing counters to the minimum necessary value.
///
/// Args:
///     epsilon: Error bound (0.0 to 1.0), default 0.01 (1%)
///     delta: Failure probability (0.0 to 1.0), default 0.01 (1%)
///
/// Example:
///     >>> cms = ConservativeCountMin(epsilon=0.01, delta=0.01)
///     >>> cms.update(b"apple")
///     >>> cms.update(b"apple")
///     >>> cms.update(b"banana")
///     >>> print(cms.estimate(b"apple"))  # >= 2
///     >>> print(cms.estimate(b"banana"))  # >= 1
///
/// Trade-offs vs Standard CountMinSketch:
///     - Up to 10x better accuracy
///     - Cannot support deletions or negative updates
///     - Same space complexity
///
/// Notes:
///     - Never underestimates (always returns >= true count)
///     - Error bounded by εN with probability 1-δ (N = total count)
#[pyclass(module = "sketch_oxide")]
pub struct ConservativeCountMin {
    inner: RustConservativeCountMin,
}

/// Wrapper for hashable Python bytes
struct HashableBytes<'a>(&'a [u8]);

impl Hash for HashableBytes<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

#[pymethods]
impl ConservativeCountMin {
    /// Create a new Conservative Update Count-Min Sketch
    ///
    /// Args:
    ///     epsilon: Error bound (0.0 to 1.0), default 0.01 (1%)
    ///     delta: Failure probability (0.0 to 1.0), default 0.01 (1%)
    ///
    /// Raises:
    ///     ValueError: If parameters are invalid
    #[new]
    #[pyo3(signature = (epsilon=0.01, delta=0.01))]
    fn new(epsilon: f64, delta: f64) -> PyResult<Self> {
        RustConservativeCountMin::new(epsilon, delta)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Create a sketch with specific dimensions
    ///
    /// Args:
    ///     width: Width of the table
    ///     depth: Number of hash functions
    #[staticmethod]
    fn with_dimensions(width: usize, depth: usize) -> PyResult<Self> {
        RustConservativeCountMin::with_dimensions(width, depth)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Update the sketch with an item
    ///
    /// Args:
    ///     item: Bytes to add to the sketch
    fn update(&mut self, item: &[u8]) {
        self.inner.update(&HashableBytes(item));
    }

    /// Update with a specific count
    ///
    /// Args:
    ///     item: Bytes to add
    ///     count: Number of occurrences to add
    fn update_count(&mut self, item: &[u8], count: u64) {
        self.inner.update_count(&HashableBytes(item), count);
    }

    /// Estimate the frequency of an item
    ///
    /// Args:
    ///     item: Bytes to query
    ///
    /// Returns:
    ///     int: Estimated frequency (always >= true frequency)
    fn estimate(&self, item: &[u8]) -> u64 {
        self.inner.estimate(&HashableBytes(item))
    }

    /// Get the width of the table
    fn width(&self) -> usize {
        self.inner.width()
    }

    /// Get the depth (number of hash functions)
    fn depth(&self) -> usize {
        self.inner.depth()
    }

    /// Get the epsilon parameter
    fn epsilon(&self) -> f64 {
        self.inner.epsilon()
    }

    /// Get the delta parameter
    fn delta(&self) -> f64 {
        self.inner.delta()
    }

    /// Get the total count of all updates
    fn total_count(&self) -> u64 {
        self.inner.total_count()
    }

    /// Clear all counters
    fn clear(&mut self) {
        self.inner.clear();
    }

    /// Get memory usage in bytes
    fn memory_usage(&self) -> usize {
        self.inner.memory_usage()
    }

    /// Merge another sketch into this one
    ///
    /// Args:
    ///     other: Another ConservativeCountMin with same dimensions
    ///
    /// Raises:
    ///     ValueError: If dimensions don't match
    fn merge(&mut self, other: &ConservativeCountMin) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Serialize the sketch to bytes
    fn to_bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new_bound(py, &self.inner.to_bytes())
    }

    /// Deserialize a sketch from bytes
    #[staticmethod]
    fn from_bytes(data: &[u8]) -> PyResult<Self> {
        RustConservativeCountMin::from_bytes(data)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ConservativeCountMin({}x{}, epsilon={:.2}%, total={})",
            self.inner.width(),
            self.inner.depth(),
            self.inner.epsilon() * 100.0,
            self.inner.total_count()
        )
    }

    /// Update the sketch with multiple items in a single call (optimized for throughput).
    ///
    /// Batch updates are significantly faster than multiple individual update() calls
    /// because they amortize the FFI (Foreign Function Interface) overhead across
    /// many items. This is the preferred method when adding large quantities of data.
    ///
    /// Args:
    ///     items: Iterable of byte strings to add
    fn update_batch(&mut self, items: &Bound<'_, PyAny>) -> PyResult<()> {
        let items_list: &Bound<'_, PyList> = items.downcast()?;
        for item in items_list {
            let item_bytes: &[u8] = item.extract()?;
            self.update(item_bytes);
        }
        Ok(())
    }

    /// Estimate frequencies of multiple items in a single call (optimized for lookups).
    ///
    /// Batch frequency lookups are faster than multiple individual estimate() calls.
    ///
    /// Args:
    ///     items: Iterable of byte strings to query
    ///
    /// Returns:
    ///     list: List of estimated frequencies (>= true frequencies), one per item
    fn estimate_batch(&self, items: &Bound<'_, PyAny>) -> PyResult<Vec<u64>> {
        let items_list: &Bound<'_, PyList> = items.downcast()?;
        let mut estimates = Vec::new();
        for item in items_list {
            let item_bytes: &[u8] = item.extract()?;
            estimates.push(self.estimate(item_bytes));
        }
        Ok(estimates)
    }
}
