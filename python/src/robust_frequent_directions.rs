//! Python bindings for Robust Frequent Directions.

use pyo3::prelude::*;
use sketch_oxide::matrix::RobustFrequentDirections as RustRobustFrequentDirections;

/// RobustFrequentDirections — a Frequent Directions variant that tracks the
/// accumulated shrinkage as a regularizer, giving tighter covariance estimates
/// with the same `ell × d` footprint.
///
/// Args:
///     ell (int): number of sketch rows retained.
///     d (int): number of columns (row dimension).
#[pyclass(module = "sketch_oxide")]
pub struct RobustFrequentDirections {
    inner: RustRobustFrequentDirections,
}

#[pymethods]
impl RobustFrequentDirections {
    #[new]
    fn new(ell: usize, d: usize) -> PyResult<Self> {
        RustRobustFrequentDirections::new(ell, d)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Appends a `d`-dimensional row to the sketch.
    fn append(&mut self, row: Vec<f64>) {
        self.inner.append(&row);
    }

    /// Approximated covariance matrix `Aᵀ A` (`d × d`).
    fn covariance(&self) -> Vec<Vec<f64>> {
        self.inner.covariance()
    }

    /// Accumulated regularizer (total shrinkage).
    fn regularizer(&self) -> f64 {
        self.inner.regularizer()
    }

    /// Number of sketch rows.
    fn ell(&self) -> usize {
        self.inner.ell()
    }

    /// Row dimension.
    fn dim(&self) -> usize {
        self.inner.dim()
    }

    fn __repr__(&self) -> String {
        format!(
            "RobustFrequentDirections(ell={}, dim={})",
            self.inner.ell(),
            self.inner.dim()
        )
    }
}
