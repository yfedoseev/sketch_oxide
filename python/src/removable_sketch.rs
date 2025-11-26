//! Python bindings for RemovableUniversalSketch

use pyo3::prelude::*;
use sketch_oxide::frequency::RemovableUniversalSketch as RustRemovableUniversalSketch;

/// RemovableUniversalSketch for frequency estimation with deletions
///
/// A state-of-the-art algorithm that supports both insertions and deletions (turnstile streams)
/// for accurate frequency estimation. Computes frequency moments and handles skewed distributions
/// efficiently.
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
///     >>> rus = RemovableUniversalSketch(epsilon=0.01, delta=0.01)
///     >>> rus.update(b"item", 100)  # Insert 100 occurrences
///     >>> rus.update(b"item", -30)  # Delete 30 occurrences
///     >>> freq = rus.estimate(b"item")
///     >>> assert freq >= 70
///     >>> l2 = rus.l2_norm()  # Get L2 norm of frequency vector
///
/// Key Features:
///     - **Turnstile Streams**: Supports positive and negative updates
///     - **Frequency Moments**: Computes L2 norm for analysis
///     - **Space Efficient**: More efficient than Count Sketch for deletions
///     - **Heavy Hitters**: Works well with skewed distributions
///
/// Notes:
///     - Space: O(log(1/δ) * log(max_update_value)) + moment sketch overhead
///     - Time: O(depth) per operation
///     - Supports negative frequencies for deletion tracking
#[pyclass(module = "sketch_oxide")]
pub struct RemovableUniversalSketch {
    inner: RustRemovableUniversalSketch,
}

#[pymethods]
impl RemovableUniversalSketch {
    /// Create a new Removable Universal Sketch
    ///
    /// Args:
    ///     epsilon (float): Error bound (0 < ε < 1)
    ///     delta (float): Failure probability (0 < δ < 1)
    ///
    /// Returns:
    ///     RemovableUniversalSketch: A new sketch instance
    ///
    /// Raises:
    ///     ValueError: If parameters are not in valid range
    ///
    /// Example:
    ///     >>> rus = RemovableUniversalSketch(0.01, 0.01)
    ///     >>> assert rus.width() > 0
    ///     >>> assert rus.depth() > 0
    #[new]
    fn new(epsilon: f64, delta: f64) -> PyResult<Self> {
        RustRemovableUniversalSketch::new(epsilon, delta)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Update the sketch with a signed frequency
    ///
    /// Supports both positive updates (insertions) and negative updates (deletions).
    /// This enables tracking of frequency changes in turnstile streams.
    ///
    /// Args:
    ///     item (bytes): The item to update
    ///     delta (int): Frequency change (positive for insert, negative for delete)
    ///
    /// Example:
    ///     >>> rus = RemovableUniversalSketch(0.01, 0.01)
    ///     >>> rus.update(b"page1", 100)      # View page 100 times
    ///     >>> rus.update(b"page1", -30)      # Delete 30 views (e.g., invalid traffic)
    ///     >>> freq = rus.estimate(b"page1")  # Get current estimate
    ///     >>> assert freq >= 70
    fn update(&mut self, item: &[u8], delta: i32) {
        self.inner.update(&item.to_vec(), delta);
    }

    /// Estimate the frequency of an item
    ///
    /// Can return negative values for items with more deletions than insertions.
    ///
    /// Args:
    ///     item (bytes): The item to query
    ///
    /// Returns:
    ///     int: Estimated frequency (can be negative due to deletions)
    ///
    /// Example:
    ///     >>> rus = RemovableUniversalSketch(0.01, 0.01)
    ///     >>> rus.update(b"item", 100)
    ///     >>> rus.update(b"item", -50)
    ///     >>> freq = rus.estimate(b"item")
    ///     >>> assert freq >= 50
    fn estimate(&self, item: &[u8]) -> i64 {
        self.inner.estimate(&item.to_vec())
    }

    /// Compute the L2 norm of the frequency vector
    ///
    /// The L2 norm is the square root of the sum of squares of all frequencies.
    /// It provides a measure of the total "energy" in the frequency distribution.
    ///
    /// Returns:
    ///     float: L2 norm estimate (always non-negative)
    ///
    /// Example:
    ///     >>> rus = RemovableUniversalSketch(0.01, 0.01)
    ///     >>> rus.update(b"item1", 100)
    ///     >>> rus.update(b"item2", 50)
    ///     >>> rus.update(b"item3", 25)
    ///     >>> l2 = rus.l2_norm()
    ///     >>> assert l2 > 0.0
    ///     >>> assert l2 >= 100.0  # At least the largest frequency
    fn l2_norm(&self) -> f64 {
        self.inner.l2_norm()
    }

    /// Merge another RemovableUniversalSketch into this one
    ///
    /// Args:
    ///     other (RemovableUniversalSketch): Another sketch with same epsilon and delta
    ///
    /// Raises:
    ///     ValueError: If sketches have incompatible parameters
    ///
    /// Example:
    ///     >>> rus1 = RemovableUniversalSketch(0.01, 0.01)
    ///     >>> rus2 = RemovableUniversalSketch(0.01, 0.01)
    ///     >>> rus1.update(b"item", 100)
    ///     >>> rus2.update(b"item", 50)
    ///     >>> rus1.merge(rus2)
    ///     >>> freq = rus1.estimate(b"item")
    ///     >>> assert freq >= 150
    fn merge(&mut self, other: &RemovableUniversalSketch) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
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
    ///     int: Width of the underlying sketch
    fn width(&self) -> usize {
        self.inner.width()
    }

    /// Get the depth (number of hash functions)
    ///
    /// Returns:
    ///     int: Depth of the underlying sketch
    fn depth(&self) -> usize {
        self.inner.depth()
    }

    fn __repr__(&self) -> String {
        format!(
            "RemovableUniversalSketch(width={}, depth={}, epsilon={:.4}, delta={:.4})",
            self.inner.width(),
            self.inner.depth(),
            self.inner.epsilon(),
            self.inner.delta()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "RemovableUniversalSketch({}x{}, turnstile-enabled)",
            self.inner.depth(),
            self.inner.width()
        )
    }
}
