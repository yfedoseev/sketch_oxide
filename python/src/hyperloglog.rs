//! Python bindings for HyperLogLog cardinality estimation

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList};
use sketch_oxide::cardinality::HyperLogLog as RustHyperLogLog;
use sketch_oxide::{Mergeable, Sketch};

use crate::common::python_item_to_hash;
use crate::with_python_item;

/// HyperLogLog for cardinality estimation (Flajolet 2007)
///
/// Industry-standard algorithm compatible with Redis, PostgreSQL, and Druid.
///
/// Args:
///     precision (int): Precision parameter (4-18), higher = more accurate
///         - 12: ~1.6% error, 4KB memory (recommended)
///         - 14: ~0.8% error, 16KB memory
///
/// Example:
///     >>> hll = HyperLogLog(12)
///     >>> for i in range(10000):
///     ...     hll.update(f"user_{i}")
///     >>> count = hll.estimate()
///     >>> print(f"Unique users: {count:.0f}")
///
/// Notes:
///     - For new applications, consider UltraLogLog (28% more efficient)
///     - Use for ecosystem interoperability with Redis/Druid
#[pyclass(module = "sketch_oxide")]
pub struct HyperLogLog {
    inner: RustHyperLogLog,
}

#[pymethods]
impl HyperLogLog {
    /// Create a new HyperLogLog
    ///
    /// Args:
    ///     precision: Precision parameter (4-18)
    ///
    /// Raises:
    ///     ValueError: If precision is out of range
    #[new]
    #[pyo3(signature = (precision=12))]
    fn new(precision: u8) -> PyResult<Self> {
        RustHyperLogLog::new(precision)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Update with a hashable item
    ///
    /// Args:
    ///     item: Item to count (int, str, or bytes)
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        with_python_item!(item, |rust_item| {
            self.inner.update(rust_item);
        })
    }

    /// Update with an integer item (no type detection)
    ///
    /// Args:
    ///     value: Integer value to count
    ///
    /// Notes:
    ///     This is faster than update() for integer items as it skips type detection.
    fn update_int(&mut self, value: i64) -> PyResult<()> {
        self.inner.update(&value);
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
        self.inner.update(&value);
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
        self.inner.update(&value);
        Ok(())
    }

    /// Estimate the number of unique items
    ///
    /// Returns:
    ///     float: Estimated cardinality
    fn estimate(&self) -> f64 {
        self.inner.estimate()
    }

    /// Merge another HyperLogLog into this one
    ///
    /// Args:
    ///     other: Another HyperLogLog with same precision
    ///
    /// Raises:
    ///     ValueError: If precisions don't match
    fn merge(&mut self, other: &HyperLogLog) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Get the precision parameter
    fn precision(&self) -> u8 {
        self.inner.precision()
    }

    /// Get the number of registers
    fn num_registers(&self) -> usize {
        self.inner.num_registers()
    }

    /// Get the standard error
    fn standard_error(&self) -> f64 {
        self.inner.standard_error()
    }

    /// Check if empty
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Serialize to bytes
    fn serialize<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new_bound(py, &self.inner.to_bytes())
    }

    /// Deserialize from bytes
    #[staticmethod]
    fn from_bytes(bytes: &[u8]) -> PyResult<Self> {
        RustHyperLogLog::from_bytes(bytes)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
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
    ///     >>> hll = HyperLogLog(precision=12)
    ///     >>> hll.update_batch([1, 2, 3, "user_123", b"data"])
    ///     >>> print(f"Estimate: {hll.estimate():.0f}")
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
        format!(
            "HyperLogLog(precision={}, estimate={:.0})",
            self.inner.precision(),
            self.inner.estimate()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "HyperLogLog: precision={}, estimated cardinality={:.0}",
            self.inner.precision(),
            self.inner.estimate()
        )
    }
}
