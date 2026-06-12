//! Python bindings for distinct (L0) sampling.

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::sampling::DistinctSampling as RustDistinctSampling;

/// DistinctSampling — a sample drawn uniformly over the set of *distinct* items.
///
/// Args:
///     capacity (int): maximum number of distinct items retained.
#[pyclass(module = "sketch_oxide")]
pub struct DistinctSampling {
    inner: RustDistinctSampling<Vec<u8>>,
}

#[pymethods]
impl DistinctSampling {
    #[new]
    fn new(capacity: usize) -> PyResult<Self> {
        RustDistinctSampling::new(capacity)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Inserts an item (int, str, bytes, or float).
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.insert(python_item_to_bytes(item)?);
        Ok(())
    }

    /// Estimated number of distinct items seen.
    fn estimate_distinct(&self) -> f64 {
        self.inner.estimate_distinct()
    }

    /// Current sample of distinct items, each returned as `bytes`.
    fn sample(&self, py: Python<'_>) -> Vec<Py<PyBytes>> {
        self.inner
            .sample()
            .iter()
            .map(|v| PyBytes::new_bound(py, v).unbind())
            .collect()
    }

    /// Number of items currently retained.
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether no items have been retained.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Current sampling level (number of halvings).
    fn level(&self) -> u32 {
        self.inner.level()
    }

    /// Maximum number of distinct items retained.
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    fn __repr__(&self) -> String {
        format!(
            "DistinctSampling(estimate={:.0}, retained={})",
            self.inner.estimate_distinct(),
            self.inner.len()
        )
    }
}
