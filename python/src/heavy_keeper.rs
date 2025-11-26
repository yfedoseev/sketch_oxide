//! Python bindings for HeavyKeeper top-k frequency estimation

use pyo3::prelude::*;
use pyo3::types::PyList;
use sketch_oxide::frequency::HeavyKeeper as RustHeavyKeeper;

/// HeavyKeeper: High-Precision Heavy Hitter Detection (USENIX ATC 2018)
///
/// Identifies top-k most frequent items with high precision using exponential decay.
/// Up to 10x more accurate than Space-Saving algorithm for heavy hitter detection.
///
/// Args:
///     k (int): Number of top items to track (must be > 0)
///     epsilon (float): Error bound in (0, 1). Smaller = higher accuracy, more space.
///         - 0.001: High accuracy (recommended)
///         - 0.01: Balanced
///         - 0.1: Low accuracy, less memory
///     delta (float): Failure probability in (0, 1). Smaller = higher confidence.
///         - 0.01: 99% confidence (recommended)
///         - 0.001: 99.9% confidence
///
/// Example:
///     >>> hk = HeavyKeeper(k=100, epsilon=0.001, delta=0.01)
///     >>> for i in range(1000):
///     ...     hk.update(f"item_{i % 10}")
///     >>> top_items = hk.top_k()
///     >>> print(f"Top 5 items: {top_items[:5]}")
///     >>> freq = hk.estimate("item_5")
///     >>> print(f"Frequency of item_5: {freq}")
///
/// Notes:
///     - Supports int, str, and bytes types
///     - Uses exponential decay (1.08 factor) to age old items
///     - Space: O((e/ε) × ln(1/δ)) counters + O(k) heap
///     - Returns (hash, count) tuples for top-k items
#[pyclass(module = "sketch_oxide")]
pub struct HeavyKeeper {
    inner: RustHeavyKeeper,
}

#[pymethods]
impl HeavyKeeper {
    /// Create a new HeavyKeeper sketch
    ///
    /// Args:
    ///     k: Number of top items to track
    ///     epsilon: Error bound (0 < ε < 1)
    ///     delta: Failure probability (0 < δ < 1)
    ///
    /// Raises:
    ///     ValueError: If parameters are not in valid range
    ///
    /// Example:
    ///     >>> hk = HeavyKeeper(k=100, epsilon=0.001, delta=0.01)
    #[new]
    fn new(k: usize, epsilon: f64, delta: f64) -> PyResult<Self> {
        RustHeavyKeeper::new(k, epsilon, delta)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Update the sketch with a single item
    ///
    /// Args:
    ///     item: Item to add (int, str, or bytes)
    ///
    /// Example:
    ///     >>> hk = HeavyKeeper(k=10, epsilon=0.001, delta=0.01)
    ///     >>> hk.update("user123")
    ///     >>> hk.update(42)
    ///     >>> hk.update(b"binary_data")
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        use pyo3::types::PyBytes;

        if let Ok(val) = item.extract::<i64>() {
            self.inner.update(&val.to_le_bytes());
        } else if let Ok(val) = item.extract::<u64>() {
            self.inner.update(&val.to_le_bytes());
        } else if let Ok(val) = item.extract::<String>() {
            self.inner.update(val.as_bytes());
        } else if let Ok(b) = item.downcast::<PyBytes>() {
            self.inner.update(b.as_bytes());
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Item must be int, str, or bytes",
            ));
        }
        Ok(())
    }

    /// Estimate the frequency of an item
    ///
    /// Args:
    ///     item: Item to query (int, str, or bytes)
    ///
    /// Returns:
    ///     int: Estimated frequency
    ///
    /// Example:
    ///     >>> hk = HeavyKeeper(k=10, epsilon=0.001, delta=0.01)
    ///     >>> for _ in range(100):
    ///     ...     hk.update("apple")
    ///     >>> freq = hk.estimate("apple")
    ///     >>> assert freq >= 90  # May slightly overestimate
    fn estimate(&self, item: &Bound<'_, PyAny>) -> PyResult<u32> {
        use pyo3::types::PyBytes;

        if let Ok(val) = item.extract::<i64>() {
            Ok(self.inner.estimate(&val.to_le_bytes()))
        } else if let Ok(val) = item.extract::<u64>() {
            Ok(self.inner.estimate(&val.to_le_bytes()))
        } else if let Ok(val) = item.extract::<String>() {
            Ok(self.inner.estimate(val.as_bytes()))
        } else if let Ok(b) = item.downcast::<PyBytes>() {
            Ok(self.inner.estimate(b.as_bytes()))
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Item must be int, str, or bytes",
            ))
        }
    }

    /// Get the top-k heavy hitters
    ///
    /// Returns:
    ///     list: List of (item_hash, count) tuples, sorted by count descending
    ///
    /// Example:
    ///     >>> hk = HeavyKeeper(k=5, epsilon=0.001, delta=0.01)
    ///     >>> for i in range(100):
    ///     ...     hk.update(f"item_{i % 10}")
    ///     >>> top_k = hk.top_k()
    ///     >>> assert len(top_k) <= 5
    ///     >>> # Items are sorted by frequency
    fn top_k(&self) -> Vec<(u64, u32)> {
        self.inner.top_k()
    }

    /// Apply exponential decay to all counters
    ///
    /// Divides all counts by decay factor (1.08), effectively applying ~8% decay.
    /// This ages old items and makes room for new heavy hitters.
    ///
    /// Returns:
    ///     None
    ///
    /// Example:
    ///     >>> hk = HeavyKeeper(k=10, epsilon=0.001, delta=0.01)
    ///     >>> hk.update("item")
    ///     >>> before = hk.estimate("item")
    ///     >>> hk.decay()
    ///     >>> after = hk.estimate("item")
    ///     >>> assert after < before
    fn decay(&mut self) {
        self.inner.decay();
    }

    /// Merge another HeavyKeeper into this one
    ///
    /// Args:
    ///     other: Another HeavyKeeper with same parameters
    ///
    /// Raises:
    ///     ValueError: If sketches have incompatible dimensions
    ///
    /// Example:
    ///     >>> hk1 = HeavyKeeper(k=10, epsilon=0.001, delta=0.01)
    ///     >>> hk2 = HeavyKeeper(k=10, epsilon=0.001, delta=0.01)
    ///     >>> hk1.update("item")
    ///     >>> hk2.update("item")
    ///     >>> hk1.merge(hk2)
    fn merge(&mut self, other: &HeavyKeeper) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Check if the sketch is empty
    ///
    /// Returns:
    ///     bool: True if no items have been added
    fn is_empty(&self) -> bool {
        // HeavyKeeper doesn't expose is_empty, check total_updates instead
        self.inner.stats().total_updates == 0
    }

    /// Update with a batch of items (optimized for throughput)
    ///
    /// Batch updates amortize FFI overhead across many items.
    ///
    /// Args:
    ///     items: Iterable of items to add
    ///
    /// Example:
    ///     >>> hk = HeavyKeeper(k=10, epsilon=0.001, delta=0.01)
    ///     >>> hk.update_batch(["apple", "banana", "apple"])
    fn update_batch(&mut self, items: &Bound<'_, PyAny>) -> PyResult<()> {
        let items_list: &Bound<'_, PyList> = items.downcast()?;
        for item in items_list {
            self.update(&item)?;
        }
        Ok(())
    }

    /// Get statistics about the sketch
    ///
    /// Returns:
    ///     dict: Dictionary with statistics
    ///         - total_updates: Total number of items processed
    ///         - k: Number of top items tracked
    ///         - memory_bits: Memory usage in bits
    ///         - depth: Number of hash functions
    ///         - width: Number of buckets per row
    ///
    /// Example:
    ///     >>> hk = HeavyKeeper(k=100, epsilon=0.001, delta=0.01)
    ///     >>> stats = hk.stats()
    ///     >>> print(f"Memory: {stats['memory_bits'] // 8} bytes")
    fn stats(&self) -> PyResult<Py<PyAny>> {
        let stats = self.inner.stats();
        Python::with_gil(|py| {
            let dict = pyo3::types::PyDict::new_bound(py);
            dict.set_item("total_updates", stats.total_updates)?;
            dict.set_item("k", stats.k)?;
            dict.set_item("memory_bits", stats.memory_bits)?;
            dict.set_item("depth", stats.depth)?;
            dict.set_item("width", stats.width)?;
            Ok(dict.into())
        })
    }

    fn __repr__(&self) -> String {
        let stats = self.inner.stats();
        format!(
            "HeavyKeeper(k={}, depth={}, width={})",
            stats.k, stats.depth, stats.width
        )
    }

    fn __str__(&self) -> String {
        let stats = self.inner.stats();
        format!("HeavyKeeper(tracking top-{} items)", stats.k)
    }
}
