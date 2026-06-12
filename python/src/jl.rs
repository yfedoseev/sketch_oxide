//! Python bindings for the Johnson–Lindenstrauss random projection.

use pyo3::prelude::*;
use sketch_oxide::matrix::JohnsonLindenstrauss as RustJohnsonLindenstrauss;

/// JohnsonLindenstrauss — a random linear projection from `input_dim` to
/// `output_dim` dimensions that approximately preserves pairwise Euclidean
/// distances (the JL lemma), for dimensionality reduction.
///
/// Args:
///     input_dim (int): source dimensionality.
///     output_dim (int): target (reduced) dimensionality.
///     seed (int): RNG seed for the projection matrix.
#[pyclass(module = "sketch_oxide")]
pub struct JohnsonLindenstrauss {
    inner: RustJohnsonLindenstrauss,
}

#[pymethods]
impl JohnsonLindenstrauss {
    #[new]
    fn new(input_dim: usize, output_dim: usize, seed: u64) -> PyResult<Self> {
        RustJohnsonLindenstrauss::new(input_dim, output_dim, seed)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Projects an `input_dim`-vector to an `output_dim`-vector.
    fn project(&self, x: Vec<f64>) -> PyResult<Vec<f64>> {
        self.inner
            .project(&x)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Source dimensionality.
    fn input_dim(&self) -> usize {
        self.inner.input_dim()
    }

    /// Target dimensionality.
    fn output_dim(&self) -> usize {
        self.inner.output_dim()
    }

    fn __repr__(&self) -> String {
        format!(
            "JohnsonLindenstrauss(input_dim={}, output_dim={})",
            self.inner.input_dim(),
            self.inner.output_dim()
        )
    }
}
