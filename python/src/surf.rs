//! Python bindings for SuRF (Succinct Range Filter).

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::common::RangeFilter;
use sketch_oxide::range_filters::Surf as RustSurf;

/// Surf — Succinct Range Filter (SIGMOD'18): a space-efficient trie over a sorted
/// key set that answers point and range membership with no false negatives. Build
/// from a sorted list of integer keys, or via :meth:`from_bytes` for byte keys.
///
/// Args:
///     keys (list[int]): sorted integer keys.
#[pyclass(module = "sketch_oxide")]
pub struct Surf {
    inner: RustSurf,
}

#[pymethods]
impl Surf {
    #[new]
    fn new(keys: Vec<u64>) -> Self {
        Self {
            inner: RustSurf::from_u64(&keys),
        }
    }

    /// Builds a SuRF from a sorted list of byte-string keys (int, str, bytes, or float).
    #[staticmethod]
    fn from_bytes(keys: Vec<Bound<'_, PyAny>>) -> PyResult<Self> {
        let byte_keys: Vec<Vec<u8>> = keys
            .iter()
            .map(python_item_to_bytes)
            .collect::<PyResult<_>>()?;
        Ok(Self {
            inner: RustSurf::build_bytes(&byte_keys),
        })
    }

    /// Whether the integer `key` might be present.
    fn contains_u64(&self, key: u64) -> bool {
        self.inner.contains_u64(key)
    }

    /// Whether the byte-string `key` (int, str, bytes, or float) might be present.
    fn contains_bytes(&self, key: &Bound<'_, PyAny>) -> PyResult<bool> {
        Ok(self.inner.contains_bytes(&python_item_to_bytes(key)?))
    }

    /// Whether the inclusive integer range `[low, high]` might contain a key.
    fn may_contain_range(&self, low: u64, high: u64) -> bool {
        self.inner.may_contain_range(low, high)
    }

    /// Number of keys.
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the filter is empty.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn __repr__(&self) -> String {
        format!("Surf(len={})", self.inner.len())
    }
}
