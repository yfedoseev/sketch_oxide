//! Python bindings for Grafite optimal range filter

use pyo3::prelude::*;
use sketch_oxide::range_filters::Grafite as RustGrafite;

/// Grafite: Optimal Range Filter with Robust FPR Bounds (2024)
///
/// The first optimal range filter with adversarial-robust guarantees.
/// Provides FPR = L / 2^(B-2) where L is query range size and B is bits per key.
///
/// Args:
///     keys (list): List of u64 keys to be filtered
///     bits_per_key (int): Number of bits per key (2-16, typically 4-8)
///
/// Example:
///     >>> # Build Grafite from sorted keys
///     >>> keys = [10, 20, 30, 40, 50]
///     >>> filter = Grafite(keys, bits_per_key=6)
///     >>>
///     >>> # Query ranges
///     >>> assert filter.may_contain_range(15, 25)  # Contains key 20
///     >>> assert filter.may_contain_range(10, 10)  # Point query for key 10
///     >>> assert filter.may_contain(20)  # Point query
///     >>>
///     >>> # Check FPR for range width
///     >>> fpr = filter.expected_fpr(10)
///     >>> print(f"FPR for range width 10: {fpr:.4f}")
///
/// Production Use Cases (2025):
///     - LSM-tree range queries (RocksDB, LevelDB)
///     - Database index optimization
///     - Time-series databases
///     - Financial market data (range lookups on timestamps)
///     - Log aggregation systems
///
/// Notes:
///     - Optimal FPR: L / 2^(B-2) for range width L
///     - Adversarial robust: Worst-case bounds hold
///     - No false negatives: Always returns true for ranges containing keys
///     - Space: B bits per key
#[pyclass(module = "sketch_oxide")]
pub struct Grafite {
    inner: RustGrafite,
}

#[pymethods]
impl Grafite {
    /// Build a Grafite filter from a set of keys
    ///
    /// Args:
    ///     keys: List of u64 keys (will be sorted and deduplicated internally)
    ///     bits_per_key: Number of bits per key (2-16)
    ///
    /// Raises:
    ///     ValueError: If parameters are invalid
    ///
    /// Example:
    ///     >>> filter = Grafite([100, 200, 300, 400, 500], bits_per_key=6)
    #[new]
    fn new(keys: Vec<u64>, bits_per_key: usize) -> PyResult<Self> {
        RustGrafite::build(&keys, bits_per_key)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Check if a range may contain keys
    ///
    /// Args:
    ///     low: Lower bound of the range (inclusive)
    ///     high: Upper bound of the range (inclusive)
    ///
    /// Returns:
    ///     bool: True if range may contain keys, False if definitely does not
    ///
    /// Guarantees:
    ///     - No false negatives: If a key exists in [low, high], returns True
    ///     - Bounded false positives: FPR ≤ (high - low + 1) / 2^(bits_per_key - 2)
    ///
    /// Example:
    ///     >>> filter = Grafite([10, 20, 30], bits_per_key=6)
    ///     >>> assert filter.may_contain_range(15, 25)  # Contains key 20
    ///     >>> assert filter.may_contain_range(10, 50)  # Contains all keys
    fn may_contain_range(&self, low: u64, high: u64) -> bool {
        self.inner.may_contain_range(low, high)
    }

    /// Check if a single key may be present
    ///
    /// This is a point query (degenerate range query where low == high).
    ///
    /// Args:
    ///     key: The key to check
    ///
    /// Returns:
    ///     bool: True if key may be present, False if definitely not present
    ///
    /// Example:
    ///     >>> filter = Grafite([100, 200, 300], bits_per_key=6)
    ///     >>> assert filter.may_contain(200)  # Definitely present
    fn may_contain(&self, key: u64) -> bool {
        self.inner.may_contain(key)
    }

    /// Get expected false positive rate for a range of given width
    ///
    /// Grafite provides optimal FPR guarantee: FPR = L / 2^(B-2)
    /// where L is range width and B is bits per key.
    ///
    /// Args:
    ///     range_width: Width of the query range (high - low + 1)
    ///
    /// Returns:
    ///     float: Expected FPR between 0.0 and 1.0
    ///
    /// Example:
    ///     >>> filter = Grafite([1, 2, 3], bits_per_key=6)
    ///     >>> # FPR for range width 10: 10 / 2^4 = 10/16 = 0.625
    ///     >>> fpr = filter.expected_fpr(10)
    ///     >>> assert abs(fpr - 0.625) < 0.001
    fn expected_fpr(&self, range_width: u64) -> f64 {
        self.inner.expected_fpr(range_width)
    }

    /// Get statistics about the filter
    ///
    /// Returns:
    ///     dict: Dictionary with statistics
    ///         - key_count: Number of keys in the filter
    ///         - bits_per_key: Configuration parameter
    ///         - total_bits: Total memory used in bits
    ///
    /// Example:
    ///     >>> filter = Grafite([1, 2, 3, 4, 5], bits_per_key=6)
    ///     >>> stats = filter.stats()
    ///     >>> assert stats['key_count'] == 5
    ///     >>> assert stats['bits_per_key'] == 6
    fn stats(&self) -> PyResult<Py<PyAny>> {
        let stats = self.inner.stats();
        Python::with_gil(|py| {
            let dict = pyo3::types::PyDict::new_bound(py);
            dict.set_item("key_count", stats.key_count)?;
            dict.set_item("bits_per_key", stats.bits_per_key)?;
            dict.set_item("total_bits", stats.total_bits)?;
            Ok(dict.into())
        })
    }

    /// Get the number of keys in the filter
    ///
    /// Returns:
    ///     int: Count of unique keys stored
    fn key_count(&self) -> usize {
        self.inner.key_count()
    }

    /// Get the bits per key configuration
    ///
    /// Returns:
    ///     int: Number of bits allocated per key
    fn bits_per_key(&self) -> usize {
        self.inner.bits_per_key()
    }

    fn __repr__(&self) -> String {
        format!(
            "Grafite(keys={}, bits_per_key={})",
            self.inner.key_count(),
            self.inner.bits_per_key()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Grafite({} keys @ {} bits/key)",
            self.inner.key_count(),
            self.inner.bits_per_key()
        )
    }
}
