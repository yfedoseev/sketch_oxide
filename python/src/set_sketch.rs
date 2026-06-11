//! Python bindings for SetSketch cardinality estimation.

use crate::common::python_item_to_hash;
use pyo3::prelude::*;
use sketch_oxide::cardinality::SetSketch as RustSetSketch;

/// SetSketch — a locality-sensitive cardinality/similarity sketch (a HLL/MinHash unifier).
///
/// Args:
///     m (int): number of registers.
#[pyclass(module = "sketch_oxide")]
pub struct SetSketch {
    inner: RustSetSketch,
}

#[pymethods]
impl SetSketch {
    #[new]
    fn new(m: usize) -> PyResult<Self> {
        RustSetSketch::new(m)
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
        self.inner.estimate_cardinality()
    }

    /// Merges another sketch with the same number of registers.
    fn merge(&mut self, other: &SetSketch) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "SetSketch(estimate={:.0})",
            self.inner.estimate_cardinality()
        )
    }
}
