//! Python bindings for Frequent Items sketch

use pyo3::prelude::*;
use sketch_oxide::frequency::frequent::{
    ErrorType as RustErrorType, FrequentItems as RustFrequentItems,
};

/// Frequent Items sketch for finding top-K most frequent items (Misra-Gries algorithm)
///
/// Based on Apache DataSketches and Google BigQuery implementation.
/// Provides **deterministic error bounds** for heavy hitters.
///
/// Args:
///     max_size (int): Maximum number of items to track. Higher = more accurate.
///         - max_size=100: Track top 100 items, ε = 1%
///         - max_size=1000: Track top 1000 items, ε = 0.1%
///
/// Example:
///     >>> fi = FrequentItems(max_size=100)
///     >>> for word in document.split():
///     ...     fi.update(word)
///     >>> # Get top-10 most frequent words
///     >>> for item, count in fi.frequent_items(mode="no_false_positives")[:10]:
///     ...     print(f"{item}: {count}")
///
/// Notes:
///     - Deterministic error bounds (not probabilistic)
///     - Error bounded by: ε = 1 / max_size
///     - Two modes: no false positives (conservative) or no false negatives (inclusive)
#[pyclass(module = "sketch_oxide")]
pub struct FrequentItems {
    inner: RustFrequentItems<String>,
}

#[pymethods]
impl FrequentItems {
    /// Create a new Frequent Items sketch
    ///
    /// Args:
    ///     max_size: Maximum number of items to track
    ///
    /// Raises:
    ///     ValueError: If max_size is 0
    #[new]
    fn new(max_size: usize) -> PyResult<Self> {
        RustFrequentItems::new(max_size)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Update the sketch with a single item
    ///
    /// Args:
    ///     item: Item to add (string)
    ///     count: Number of occurrences to add (default 1)
    ///
    /// Example:
    ///     >>> fi = FrequentItems(max_size=100)
    ///     >>> fi.update("apple")
    ///     >>> fi.update("banana", count=5)
    #[pyo3(signature = (item, count=1))]
    fn update(&mut self, item: &str, count: u64) {
        self.inner.update_by(item.to_string(), count);
    }

    /// Get the estimated frequency bounds for an item
    ///
    /// Args:
    ///     item: Item to query
    ///
    /// Returns:
    ///     tuple: (lower_bound, upper_bound) or None if item not tracked
    ///
    /// Example:
    ///     >>> fi = FrequentItems(max_size=100)
    ///     >>> # ... add data ...
    ///     >>> bounds = fi.get_estimate("apple")
    ///     >>> if bounds:
    ///     ...     lower, upper = bounds
    ///     ...     print(f"Frequency in range [{lower}, {upper}]")
    fn get_estimate(&self, item: &str) -> Option<(u64, u64)> {
        self.inner.get_estimate(&item.to_string())
    }

    /// Get all frequent items above a threshold
    ///
    /// Args:
    ///     mode: Error mode, either "no_false_positives" or "no_false_negatives"
    ///
    /// Returns:
    ///     list: List of (item, lower_bound, upper_bound) tuples, sorted by count descending
    ///
    /// Example:
    ///     >>> fi = FrequentItems(max_size=100)
    ///     >>> # ... add data ...
    ///     >>> items = fi.frequent_items(mode="no_false_positives")
    ///     >>> for item, lower, upper in items[:10]:  # Top 10
    ///     ...     print(f"{item}: [{lower}, {upper}]")
    #[pyo3(signature = (mode="no_false_positives"))]
    fn frequent_items(&self, mode: &str) -> PyResult<Vec<(String, u64, u64)>> {
        let error_type = match mode.to_lowercase().as_str() {
            "no_false_positives" => RustErrorType::NoFalsePositives,
            "no_false_negatives" => RustErrorType::NoFalseNegatives,
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "Mode must be 'no_false_positives' or 'no_false_negatives'",
                ))
            }
        };

        Ok(self.inner.frequent_items(error_type))
    }

    /// Check if the sketch is empty
    ///
    /// Returns:
    ///     bool: True if no items have been added
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get the maximum size parameter
    ///
    /// Returns:
    ///     int: Maximum number of items tracked
    fn max_size(&self) -> usize {
        self.inner.max_size()
    }

    /// Get the current number of items being tracked
    ///
    /// Returns:
    ///     int: Number of items currently in the sketch
    fn num_items(&self) -> usize {
        self.inner.num_items()
    }

    /// Get the error offset
    ///
    /// Returns:
    ///     int: Accumulated error from purged items
    fn offset(&self) -> u64 {
        self.inner.offset()
    }

    fn __repr__(&self) -> String {
        format!(
            "FrequentItems(max_size={}, tracking={}, offset={})",
            self.inner.max_size(),
            self.inner.num_items(),
            self.inner.offset()
        )
    }

    fn __str__(&self) -> String {
        format!("FrequentItems(tracking {} items)", self.inner.num_items())
    }

    fn __len__(&self) -> usize {
        self.inner.num_items()
    }
}
