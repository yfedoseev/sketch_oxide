//! Python bindings for Linear Counting cardinality estimation.

use crate::common::python_item_to_hash;
use pyo3::prelude::*;
use sketch_oxide::cardinality::LinearCounting as RustLinearCounting;

/// Linear Counting — a bit-array distinct counter, accurate for low/moderate cardinality.
///
/// Args:
///     num_bits (int): size of the bit array.
#[pyclass(module = "sketch_oxide")]
pub struct LinearCounting {
    inner: RustLinearCounting,
}

#[pymethods]
impl LinearCounting {
    #[new]
    fn new(num_bits: usize) -> PyResult<Self> {
        RustLinearCounting::new(num_bits)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Adds an item (int, str, bytes, or float).
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        let h = python_item_to_hash(item)?;
        self.inner.add(&h.to_le_bytes());
        Ok(())
    }

    /// Estimated number of distinct items.
    fn estimate(&self) -> f64 {
        self.inner.estimate()
    }

    /// Whether no items have been added.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Merges another counter of the same size.
    fn merge(&mut self, other: &LinearCounting) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("LinearCounting(estimate={:.0})", self.inner.estimate())
    }
}
