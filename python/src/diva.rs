//! Python bindings for the DIVA range filter.

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::range_filters::DivaFilter as RustDivaFilter;

/// DivaFilter — a dynamic, deletable range filter over byte-string keys that
/// supports point and range membership with a target false-positive rate.
///
/// Args:
///     fpr (float): target false-positive rate.
#[pyclass(module = "sketch_oxide")]
pub struct DivaFilter {
    inner: RustDivaFilter,
}

#[pymethods]
impl DivaFilter {
    #[new]
    fn new(fpr: f64) -> PyResult<Self> {
        RustDivaFilter::new(fpr)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Constructs a filter with a fixed partition resolution instead of a target FPR.
    #[staticmethod]
    fn with_resolution(resolution: usize) -> Self {
        Self {
            inner: RustDivaFilter::with_resolution(resolution),
        }
    }

    /// Inserts a key (int, str, bytes, or float).
    fn insert(&mut self, key: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.insert(&python_item_to_bytes(key)?);
        Ok(())
    }

    /// Removes a key; returns whether it was present.
    fn remove(&mut self, key: &Bound<'_, PyAny>) -> PyResult<bool> {
        Ok(self.inner.remove(&python_item_to_bytes(key)?))
    }

    /// Whether the point `key` might be present.
    fn may_contain(&self, key: &Bound<'_, PyAny>) -> PyResult<bool> {
        Ok(self.inner.may_contain(&python_item_to_bytes(key)?))
    }

    /// Whether the inclusive byte-range `[low, high]` might contain a key.
    fn may_contain_range(&self, low: &Bound<'_, PyAny>, high: &Bound<'_, PyAny>) -> PyResult<bool> {
        let lo = python_item_to_bytes(low)?;
        let hi = python_item_to_bytes(high)?;
        Ok(self.inner.may_contain_range(&lo, &hi))
    }

    /// Number of keys stored.
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the filter is empty.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Number of partitions.
    fn num_partitions(&self) -> usize {
        self.inner.num_partitions()
    }

    /// Partition resolution.
    fn resolution(&self) -> usize {
        self.inner.resolution()
    }

    fn __repr__(&self) -> String {
        format!(
            "DivaFilter(len={}, partitions={})",
            self.inner.len(),
            self.inner.num_partitions()
        )
    }
}
