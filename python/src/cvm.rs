//! Python bindings for the CVM distinct-counting sketch.

use crate::common::python_item_to_hash;
use pyo3::prelude::*;
use sketch_oxide::cardinality::CvmSketch as RustCvmSketch;

/// CVM — the simple, provable distinct-counting algorithm (Chakraborty, Vinodchandran & Meel).
///
/// Args:
///     capacity (int): buffer size; larger gives a smaller relative error.
#[pyclass(module = "sketch_oxide")]
pub struct CvmSketch {
    inner: RustCvmSketch<u64>,
}

#[pymethods]
impl CvmSketch {
    #[new]
    fn new(capacity: usize) -> PyResult<Self> {
        RustCvmSketch::new(capacity)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Adds an item (int, str, bytes, or float).
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        let h = python_item_to_hash(item)?;
        self.inner.insert(h);
        Ok(())
    }

    /// Estimated number of distinct items.
    fn estimate(&self) -> f64 {
        self.inner.estimate_distinct()
    }

    /// Whether no items have been added.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn __repr__(&self) -> String {
        format!("CvmSketch(estimate={:.0})", self.inner.estimate_distinct())
    }
}
