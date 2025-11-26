//! Python bindings for UltraLogLog cardinality estimation

use pyo3::prelude::*;
use pyo3::types::PyList;
use sketch_oxide::cardinality::UltraLogLog as RustUltraLogLog;
use sketch_oxide::common::hash::xxhash;
use sketch_oxide::{Mergeable, Sketch};

use crate::common::python_item_to_hash;

/// UltraLogLog sketch for cardinality estimation (VLDB 2024)
///
/// 28% more space-efficient than HyperLogLog with the same accuracy.
///
/// Args:
///     precision (int): Precision parameter (4-18), higher = more accurate but more memory.
///         - precision 4: 16 bytes
///         - precision 8: 256 bytes
///         - precision 12: 4 KB (recommended)
///         - precision 16: 64 KB
///         - precision 18: 256 KB
///
/// Example:
///     >>> ull = UltraLogLog(precision=12)
///     >>> for item in data:
///     ...     ull.update(item)
///     >>> print(f"Cardinality: {ull.estimate()}")
///
/// Notes:
///     - Supports int, str, bytes, and float types
///     - Mergeable with other UltraLogLog sketches of same precision
///     - Standard error: ~1.04 / sqrt(2^precision)
#[pyclass(module = "sketch_oxide")]
pub struct UltraLogLog {
    inner: RustUltraLogLog,
}

#[pymethods]
impl UltraLogLog {
    /// Create a new UltraLogLog sketch
    ///
    /// Args:
    ///     precision: Precision parameter (4-18)
    ///
    /// Raises:
    ///     ValueError: If precision is not in range [4, 18]
    #[new]
    fn new(precision: u8) -> PyResult<Self> {
        RustUltraLogLog::new(precision)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Update the sketch with a single item
    ///
    /// Args:
    ///     item: Item to add (int, str, bytes, or float)
    ///
    /// Example:
    ///     >>> ull = UltraLogLog(precision=12)
    ///     >>> ull.update(123)
    ///     >>> ull.update("user_id_456")
    ///     >>> ull.update(b"binary_data")
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        let hash_val = python_item_to_hash(item)?;
        self.inner.update(&hash_val);
        Ok(())
    }

    /// Update with an integer item (no type detection)
    ///
    /// Args:
    ///     value: Integer value to count
    ///
    /// Notes:
    ///     This is faster than update() for integer items as it skips type detection.
    fn update_int(&mut self, value: i64) -> PyResult<()> {
        let hash_val = xxhash(&value.to_le_bytes(), 0);
        self.inner.update(&hash_val);
        Ok(())
    }

    /// Update with a string item (no type detection)
    ///
    /// Args:
    ///     value: String value to count
    ///
    /// Notes:
    ///     This is faster than update() for string items as it skips type detection.
    fn update_str(&mut self, value: String) -> PyResult<()> {
        let hash_val = xxhash(value.as_bytes(), 0);
        self.inner.update(&hash_val);
        Ok(())
    }

    /// Update with a bytes item (no type detection)
    ///
    /// Args:
    ///     value: Bytes value to count
    ///
    /// Notes:
    ///     This is faster than update() for bytes items as it skips type detection.
    fn update_bytes(&mut self, value: &[u8]) -> PyResult<()> {
        let hash_val = xxhash(value, 0);
        self.inner.update(&hash_val);
        Ok(())
    }

    /// Get the estimated cardinality
    ///
    /// Returns:
    ///     float: Estimated number of unique items
    ///
    /// Example:
    ///     >>> ull = UltraLogLog(precision=12)
    ///     >>> for i in range(10000):
    ///     ...     ull.update(i)
    ///     >>> estimate = ull.estimate()
    ///     >>> print(f"Estimate: {estimate:.0f}")  # Should be close to 10000
    fn estimate(&self) -> f64 {
        self.inner.estimate()
    }

    /// Merge another UltraLogLog sketch into this one
    ///
    /// Args:
    ///     other: Another UltraLogLog sketch with the same precision
    ///
    /// Raises:
    ///     ValueError: If sketches have different precisions
    ///
    /// Example:
    ///     >>> ull1 = UltraLogLog(precision=12)
    ///     >>> ull2 = UltraLogLog(precision=12)
    ///     >>> for i in range(1000):
    ///     ...     ull1.update(i)
    ///     >>> for i in range(1000, 2000):
    ///     ...     ull2.update(i)
    ///     >>> ull1.merge(ull2)
    ///     >>> print(ull1.estimate())  # Should be close to 2000
    fn merge(&mut self, other: &UltraLogLog) -> PyResult<()> {
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

    /// Update with a batch of items (optimized for throughput)
    ///
    /// Processes multiple items in a single call, amortizing FFI overhead.
    /// This is significantly faster than calling update() multiple times.
    ///
    /// Args:
    ///     items: Iterable of items to add (int, str, bytes, or float types)
    ///
    /// Example:
    ///     >>> ull = UltraLogLog(precision=12)
    ///     >>> ull.update_batch([1, 2, 3, "user_123", b"data"])
    ///     >>> print(f"Estimate: {ull.estimate():.0f}")
    fn update_batch(&mut self, items: &Bound<'_, PyAny>) -> PyResult<()> {
        let items_list: &Bound<'_, PyList> = items.downcast()?;
        for item in items_list {
            let hash_val = python_item_to_hash(&item)?;
            self.inner.update(&hash_val);
        }
        Ok(())
    }

    /// Update batch with pre-computed integer hashes (fastest)
    ///
    /// For maximum performance when you have integer hashes already computed.
    /// Completely skips type detection and hashing overhead.
    ///
    /// Args:
    ///     hashes: Iterable of integer hash values
    fn update_batch_hashes(&mut self, hashes: &Bound<'_, PyAny>) -> PyResult<()> {
        let hashes_list: &Bound<'_, PyList> = hashes.downcast()?;
        for item in hashes_list {
            let hash_val: u64 = item.extract()?;
            self.inner.update(&hash_val);
        }
        Ok(())
    }

    fn __repr__(&self) -> String {
        format!("UltraLogLog(estimate={:.0})", self.inner.estimate())
    }

    fn __str__(&self) -> String {
        format!("UltraLogLog(estimate={:.0})", self.inner.estimate())
    }
}
