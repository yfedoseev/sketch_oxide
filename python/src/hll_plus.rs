//! Python bindings for HyperLogLog++ cardinality estimation.

use crate::common::python_item_to_hash;
use pyo3::prelude::*;
use sketch_oxide::cardinality::HyperLogLogPlus as RustHllPlus;

/// HyperLogLog++ — Google's improved HyperLogLog with bias correction and a sparse representation.
///
/// Args:
///     precision (int): register precision (4-18); larger = more accurate, more memory.
///
/// Example:
///     >>> hll = HyperLogLogPlus(14)
///     >>> for i in range(100000):
///     ...     hll.update(i)
///     >>> round(hll.estimate())
#[pyclass(module = "sketch_oxide")]
pub struct HyperLogLogPlus {
    inner: RustHllPlus,
}

#[pymethods]
impl HyperLogLogPlus {
    #[new]
    fn new(precision: u8) -> PyResult<Self> {
        RustHllPlus::new(precision)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Adds an item (int, str, bytes, or float).
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        let h = python_item_to_hash(item)?;
        self.inner.add(&h);
        Ok(())
    }

    /// Estimated number of distinct items.
    fn estimate(&self) -> f64 {
        self.inner.estimate()
    }

    /// Merges another sketch of the same precision into this one.
    fn merge(&mut self, other: &HyperLogLogPlus) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("HyperLogLogPlus(estimate={:.0})", self.inner.estimate())
    }
}
