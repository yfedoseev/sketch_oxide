//! Python bindings for the AMS (Alon–Matias–Szegedy) sketch.

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::statistics::AmsSketch as RustAmsSketch;

/// AmsSketch — the Alon–Matias–Szegedy sketch for estimating the second frequency
/// moment F2 (and inner products) of a stream using `±1` hash projections.
///
/// Args:
///     depth (int): number of independent estimators (median rows).
///     width (int): averaging width per row.
#[pyclass(module = "sketch_oxide")]
pub struct AmsSketch {
    inner: RustAmsSketch,
}

#[pymethods]
impl AmsSketch {
    #[new]
    fn new(depth: usize, width: usize) -> PyResult<Self> {
        RustAmsSketch::new(depth, width)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Adds `count` (possibly negative) to an item (int, str, bytes, or float).
    fn update(&mut self, item: &Bound<'_, PyAny>, count: f64) -> PyResult<()> {
        self.inner.update(&python_item_to_bytes(item)?, count);
        Ok(())
    }

    /// Estimated second frequency moment F2 = sum of squared frequencies.
    fn f2(&self) -> f64 {
        self.inner.f2()
    }

    /// Estimated inner product with another AMS sketch of the same shape.
    fn inner_product(&self, other: &AmsSketch) -> PyResult<f64> {
        self.inner
            .inner_product(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Number of estimator rows.
    fn depth(&self) -> usize {
        self.inner.depth()
    }

    /// Averaging width per row.
    fn width(&self) -> usize {
        self.inner.width()
    }

    fn __repr__(&self) -> String {
        format!("AmsSketch(f2={:.1})", self.inner.f2())
    }
}
