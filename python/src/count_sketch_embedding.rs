//! Python bindings for the CountSketch matrix embedding (sparse JL).

use pyo3::prelude::*;
use sketch_oxide::matrix::CountSketchEmbedding as RustCountSketchEmbedding;

/// CountSketchEmbedding — a sparse oblivious subspace embedding (the CountSketch /
/// "CountSketch transform") that maps vectors/matrices to `rows` dimensions in
/// input-sparsity time, used for fast sketched least squares.
///
/// Args:
///     rows (int): target (sketch) dimension.
///     seed (int): RNG seed for the hash/sign functions.
#[pyclass(module = "sketch_oxide")]
pub struct CountSketchEmbedding {
    inner: RustCountSketchEmbedding,
}

#[pymethods]
impl CountSketchEmbedding {
    #[new]
    fn new(rows: usize, seed: u64) -> PyResult<Self> {
        RustCountSketchEmbedding::new(rows, seed)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Sketches a single vector `y` down to `rows` dimensions.
    fn apply_vec(&self, y: Vec<f64>) -> Vec<f64> {
        self.inner.apply_vec(&y)
    }

    /// Sketches a matrix `a` (list of equal-length rows) down to `rows` rows.
    fn apply(&self, a: Vec<Vec<f64>>) -> Vec<Vec<f64>> {
        self.inner.apply(&a)
    }

    /// Solves the sketched least-squares problem `min ||S A x - S b||`.
    fn sketched_least_squares(&self, a: Vec<Vec<f64>>, b: Vec<f64>) -> PyResult<Vec<f64>> {
        self.inner
            .sketched_least_squares(&a, &b)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Target sketch dimension.
    fn rows(&self) -> usize {
        self.inner.rows()
    }

    fn __repr__(&self) -> String {
        format!("CountSketchEmbedding(rows={})", self.inner.rows())
    }
}
