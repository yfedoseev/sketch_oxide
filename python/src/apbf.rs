//! Python bindings for the Age-Partitioned Bloom Filter (APBF).

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::streaming::Apbf as RustApbf;

/// Apbf — Age-Partitioned Bloom Filter: sliding-window membership over the last
/// `window` insertions with a bounded false-positive rate.
///
/// Args:
///     window (int): number of recent items to retain.
///     fpr (float): target false-positive rate.
#[pyclass(module = "sketch_oxide")]
pub struct Apbf {
    inner: RustApbf,
}

#[pymethods]
impl Apbf {
    #[new]
    fn new(window: u64, fpr: f64) -> PyResult<Self> {
        RustApbf::new(window, fpr)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Inserts an item (int, str, bytes, or float), ageing out the oldest slice.
    fn insert(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.insert(&python_item_to_bytes(item)?);
        Ok(())
    }

    /// Whether an item is (probably) in the recent window.
    fn contains(&self, item: &Bound<'_, PyAny>) -> PyResult<bool> {
        Ok(self.inner.contains(&python_item_to_bytes(item)?))
    }

    /// Number of age slices.
    fn num_slices(&self) -> usize {
        self.inner.num_slices()
    }

    /// Number of hash functions per query.
    fn k(&self) -> usize {
        self.inner.k()
    }

    fn __repr__(&self) -> String {
        format!(
            "Apbf(num_slices={}, k={})",
            self.inner.num_slices(),
            self.inner.k()
        )
    }
}
