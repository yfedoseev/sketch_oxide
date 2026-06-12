//! Python bindings for Doulion triangle-count sparsification.

use pyo3::prelude::*;
use sketch_oxide::graph::Doulion as RustDoulion;

/// Doulion — triangle counting by edge sparsification: each edge is kept with
/// probability `keep_prob`, and the triangle count is scaled back by `1/p**3`.
///
/// Args:
///     keep_prob (float): per-edge retention probability.
///     seed (int): RNG seed.
#[pyclass(module = "sketch_oxide")]
pub struct Doulion {
    inner: RustDoulion,
}

#[pymethods]
impl Doulion {
    #[new]
    fn new(keep_prob: f64, seed: u64) -> PyResult<Self> {
        RustDoulion::new(keep_prob, seed)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Offers an (undirected) edge between integer vertices `u` and `v`.
    fn add_edge(&mut self, u: u64, v: u64) {
        self.inner.add_edge(u, v);
    }

    /// Estimated number of triangles in the full graph.
    fn estimate_triangles(&self) -> f64 {
        self.inner.estimate_triangles()
    }

    /// Number of edges actually kept.
    fn kept_edges(&self) -> usize {
        self.inner.kept_edges()
    }

    /// Per-edge retention probability.
    fn keep_prob(&self) -> f64 {
        self.inner.keep_prob()
    }

    fn __repr__(&self) -> String {
        format!(
            "Doulion(kept_edges={}, estimate={:.1})",
            self.inner.kept_edges(),
            self.inner.estimate_triangles()
        )
    }
}
