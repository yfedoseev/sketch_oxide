//! Python bindings for the Frequent Directions matrix sketch.

use pyo3::prelude::*;
use sketch_oxide::matrix::FrequentDirections as RustFrequentDirections;

/// FrequentDirections — a deterministic matrix sketch (Liberty 2013) that
/// approximates the covariance `Aᵀ A` of a streamed `d`-column matrix using only
/// `ell` rows, the matrix analogue of frequent-items.
///
/// Args:
///     ell (int): number of sketch rows retained.
///     d (int): number of columns (row dimension).
#[pyclass(module = "sketch_oxide")]
pub struct FrequentDirections {
    inner: RustFrequentDirections,
}

#[pymethods]
impl FrequentDirections {
    #[new]
    fn new(ell: usize, d: usize) -> PyResult<Self> {
        RustFrequentDirections::new(ell, d)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Appends a `d`-dimensional row to the sketch.
    fn append(&mut self, row: Vec<f64>) {
        self.inner.append(&row);
    }

    /// The current `ell × d` sketch matrix.
    fn sketch(&self) -> Vec<Vec<f64>> {
        self.inner.sketch().to_vec()
    }

    /// Approximated covariance matrix `Aᵀ A` (`d × d`).
    fn covariance(&self) -> Vec<Vec<f64>> {
        self.inner.covariance()
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
            "FrequentDirections(ell={}, dim={})",
            self.inner.ell(),
            self.inner.dim()
        )
    }
}
