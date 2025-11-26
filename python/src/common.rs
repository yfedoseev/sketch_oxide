//! Common utilities for Python bindings
//!
//! This module provides shared functionality for converting Python types
//! to Rust types across all sketch implementations.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::common::hash::xxhash;

/// Convert a Python item to a hash value for hash-based sketch algorithms.
///
/// This function handles conversion of common Python types (int, str, bytes, float)
/// to a consistent u64 hash value using XXHash.
///
/// # Arguments
///
/// * `item` - A Python object that can be an int, str, bytes, or float
///
/// # Returns
///
/// * `Ok(u64)` - The hash value of the item
/// * `Err(PyErr)` - If the item type is not supported
///
/// # Supported Types
///
/// - `int` (i64 or u64)
/// - `str` (String)
/// - `bytes` (PyBytes)
/// - `float` (f64)
///
/// # Example
///
/// ```rust,ignore
/// fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
///     let hash_val = python_item_to_hash(item)?;
///     self.inner.update(&hash_val);
///     Ok(())
/// }
/// ```
pub fn python_item_to_hash(item: &Bound<'_, PyAny>) -> PyResult<u64> {
    if let Ok(val) = item.extract::<i64>() {
        Ok(xxhash(&val.to_le_bytes(), 0))
    } else if let Ok(val) = item.extract::<u64>() {
        Ok(xxhash(&val.to_le_bytes(), 0))
    } else if let Ok(val) = item.extract::<String>() {
        Ok(xxhash(val.as_bytes(), 0))
    } else if let Ok(b) = item.downcast::<PyBytes>() {
        let val = b.as_bytes();
        Ok(xxhash(val, 0))
    } else if let Ok(val) = item.extract::<f64>() {
        Ok(xxhash(&val.to_bits().to_le_bytes(), 0))
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Item must be int, str, bytes, or float",
        ))
    }
}

/// Macro to execute a closure with a Python item converted to a Rust type.
///
/// This macro handles conversion of Python types (int, str, bytes) to their
/// Rust equivalents and calls the provided expression with each type.
///
/// # Example
///
/// ```rust,ignore
/// fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
///     with_python_item!(item, |rust_item| {
///         self.inner.update(rust_item);
///     })
/// }
/// ```
#[macro_export]
macro_rules! with_python_item {
    ($item:expr, $closure:expr) => {{
        use pyo3::types::PyBytes;

        if let Ok(val) = $item.extract::<i64>() {
            Ok($closure(&val))
        } else if let Ok(val) = $item.extract::<u64>() {
            Ok($closure(&val))
        } else if let Ok(val) = $item.extract::<String>() {
            Ok($closure(&val))
        } else if let Ok(b) = $item.downcast::<PyBytes>() {
            let val = b.as_bytes();
            Ok($closure(&val))
        } else {
            Err(pyo3::exceptions::PyTypeError::new_err(
                "Item must be int, str, or bytes",
            ))
        }
    }};
}

#[cfg(test)]
mod tests {

    // Note: These tests would require a Python runtime to be initialized
    // In practice, they're tested through the integration tests
}
