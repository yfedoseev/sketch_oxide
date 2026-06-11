//! Python bindings for the HyperBitBit cardinality estimator.

use crate::common::python_item_to_hash;
use pyo3::prelude::*;
use sketch_oxide::cardinality::HyperBitBit as RustHyperBitBit;

/// HyperBitBit — Sedgewick's tiny cardinality estimator (a handful of words of state).
///
/// Example:
///     >>> hbb = HyperBitBit()
///     >>> for i in range(10000):
///     ...     hbb.update(i)
///     >>> hbb.estimate() > 0
///     True
#[pyclass(module = "sketch_oxide")]
pub struct HyperBitBit {
    inner: RustHyperBitBit,
}

#[pymethods]
impl HyperBitBit {
    #[new]
    fn new() -> Self {
        Self {
            inner: RustHyperBitBit::new(),
        }
    }

    /// Adds an item (int, str, bytes, or float).
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        let h = python_item_to_hash(item)?;
        self.inner.add_hash(h);
        Ok(())
    }

    /// Estimated number of distinct items.
    fn estimate(&self) -> f64 {
        self.inner.estimate()
    }

    fn __repr__(&self) -> String {
        format!("HyperBitBit(estimate={:.0})", self.inner.estimate())
    }
}
