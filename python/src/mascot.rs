//! Python bindings for the MASCOT streaming triangle estimator.

use pyo3::prelude::*;
use sketch_oxide::graph::Mascot as RustMascot;

/// Mascot — Memory-Aware Streaming algorithm for COunting local Triangles:
/// samples each edge independently with probability `p` and estimates global and
/// local triangle counts.
///
/// Args:
///     p (float): per-edge sampling probability.
///     seed (int, optional): RNG seed for reproducibility.
#[pyclass(module = "sketch_oxide")]
pub struct Mascot {
    inner: RustMascot,
}

#[pymethods]
impl Mascot {
    #[new]
    #[pyo3(signature = (p, seed=None))]
    fn new(p: f64, seed: Option<u64>) -> PyResult<Self> {
        let res = match seed {
            Some(s) => RustMascot::with_seed(p, s),
            None => RustMascot::new(p),
        };
        res.map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Offers an (undirected) edge between integer vertices `u` and `v`.
    fn add_edge(&mut self, u: u64, v: u64) {
        self.inner.add_edge(u, v);
    }

    /// Estimated number of triangles.
    fn estimate(&self) -> f64 {
        self.inner.estimate()
    }

    /// Per-edge sampling probability.
    fn probability(&self) -> f64 {
        self.inner.probability()
    }

    /// Number of sampled edges.
    fn sampled_edges(&self) -> usize {
        self.inner.sampled_edges()
    }

    fn __repr__(&self) -> String {
        format!(
            "Mascot(sampled_edges={}, estimate={:.1})",
            self.inner.sampled_edges(),
            self.inner.estimate()
        )
    }
}
