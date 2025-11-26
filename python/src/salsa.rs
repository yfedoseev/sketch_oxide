//! Python bindings for SALSA (Self-Adjusting Counter Sizing Algorithm)

use pyo3::prelude::*;
use sketch_oxide::frequency::SALSA as RustSALSA;

/// SALSA: Self-Adjusting Counter Sizing for frequency estimation
///
/// Wraps CountMinSketch to provide adaptive counter management. When frequencies approach
/// overflow, SALSA automatically adjusts parameters to maintain accuracy. This is essential
/// for production systems handling skewed/heavy-tailed distributions.
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
///     >>> salsa = SALSA(epsilon=0.01, delta=0.01)
///     >>> salsa.update(b"item", 100)
///     >>> estimate, confidence = salsa.estimate(b"item")
///     >>> assert estimate >= 100
///     >>> print(f"Confidence: {confidence}%")
///
/// Key Features:
///     - Automatic counter size adaptation for skewed distributions
///     - Never underestimates (always returns count >= true count)
///     - Confidence metric based on total updates processed
///     - Compatible with CountMinSketch merging
///
/// Notes:
///     - Space: O((e/ε) * ln(1/δ)) initially, grows logarithmically with heavy hitters
///     - Time: O(d) where d is the depth
///     - Best for heavy-tailed/Zipfian distributions
#[allow(clippy::upper_case_acronyms)]
#[pyclass(module = "sketch_oxide")]
pub struct SALSA {
    inner: RustSALSA,
}

#[pymethods]
impl SALSA {
    /// Create a new SALSA sketch with default parameters
    ///
    /// Args:
    ///     epsilon (float): Error bound (0 < ε < 1)
    ///     delta (float): Failure probability (0 < δ < 1)
    ///
    /// Returns:
    ///     SALSA: A new sketch instance
    ///
    /// Raises:
    ///     ValueError: If parameters are not in valid range
    ///
    /// Example:
    ///     >>> salsa = SALSA(0.01, 0.01)
    ///     >>> assert salsa.total_updates() == 0
    ///     >>> assert salsa.adaptation_level() == 0
    #[new]
    fn new(epsilon: f64, delta: f64) -> PyResult<Self> {
        RustSALSA::new(epsilon, delta)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Update the sketch with an item and its frequency
    ///
    /// Args:
    ///     item (bytes): The item to add (feature, user ID, etc.)
    ///     count (int): The frequency/weight to add
    ///
    /// Example:
    ///     >>> salsa = SALSA(0.01, 0.01)
    ///     >>> salsa.update(b"apple", 50)
    ///     >>> salsa.update(b"apple", 50)
    ///     >>> estimate, conf = salsa.estimate(b"apple")
    ///     >>> assert estimate >= 100
    fn update(&mut self, item: &[u8], count: u64) {
        // Convert bytes to a hashable type by creating a key from the bytes
        self.inner.update(&item.to_vec(), count);
    }

    /// Estimate the frequency of an item with confidence
    ///
    /// Args:
    ///     item (bytes): The item to query
    ///
    /// Returns:
    ///     tuple[int, int]: (estimate, confidence) where confidence is 0-100
    ///
    /// Example:
    ///     >>> salsa = SALSA(0.01, 0.01)
    ///     >>> salsa.update(b"apple", 30)
    ///     >>> est, conf = salsa.estimate(b"apple")
    ///     >>> assert est >= 30
    ///     >>> print(f"Confidence: {conf}%")
    fn estimate(&self, item: &[u8]) -> (u64, u64) {
        self.inner.estimate(&item.to_vec())
    }

    /// Merge another SALSA sketch into this one
    ///
    /// Args:
    ///     other (SALSA): Another SALSA with same epsilon and delta
    ///
    /// Raises:
    ///     ValueError: If sketches have incompatible parameters
    ///
    /// Example:
    ///     >>> salsa1 = SALSA(0.01, 0.01)
    ///     >>> salsa2 = SALSA(0.01, 0.01)
    ///     >>> salsa1.update(b"item", 100)
    ///     >>> salsa2.update(b"item", 50)
    ///     >>> salsa1.merge(salsa2)
    ///     >>> est, _ = salsa1.estimate(b"item")
    ///     >>> assert est >= 150
    fn merge(&mut self, other: &SALSA) -> PyResult<()> {
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

    /// Get the maximum frequency observed so far
    ///
    /// Returns:
    ///     int: Maximum frequency in a single update call
    fn max_observed(&self) -> u64 {
        self.inner.max_observed()
    }

    /// Get the total number of updates processed
    ///
    /// Returns:
    ///     int: Sum of all update counts
    fn total_updates(&self) -> u64 {
        self.inner.total_updates()
    }

    /// Get the current adaptation level
    ///
    /// Increments when the sketch detects that frequencies are approaching
    /// overflow and adaptation is triggered.
    ///
    /// Returns:
    ///     int: Number of times the sketch has adapted
    fn adaptation_level(&self) -> u32 {
        self.inner.adaptation_level()
    }

    /// Get the width (number of counters per row) of the sketch
    ///
    /// Returns:
    ///     int: Width parameter
    fn width(&self) -> usize {
        self.inner.width()
    }

    /// Get the depth (number of hash functions)
    ///
    /// Returns:
    ///     int: Depth parameter
    fn depth(&self) -> usize {
        self.inner.depth()
    }

    fn __repr__(&self) -> String {
        format!(
            "SALSA(width={}, depth={}, epsilon={:.4}, delta={:.4}, adaptations={})",
            self.inner.width(),
            self.inner.depth(),
            self.inner.epsilon(),
            self.inner.delta(),
            self.inner.adaptation_level()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "SALSA({}x{} with {} adaptations, total_updates={})",
            self.inner.depth(),
            self.inner.width(),
            self.inner.adaptation_level(),
            self.inner.total_updates()
        )
    }
}
