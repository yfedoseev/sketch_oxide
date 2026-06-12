//! Python bindings for the TRIÈST streaming triangle estimator.

use pyo3::prelude::*;
use sketch_oxide::graph::Triest as RustTriest;

/// Triest — reservoir-based triangle counting over an edge stream, maintaining a
/// fixed sample of `m` edges and giving an unbiased global triangle estimate.
///
/// Args:
///     m (int): reservoir (sample) size.
///     seed (int, optional): RNG seed for reproducibility.
#[pyclass(module = "sketch_oxide")]
pub struct Triest {
    inner: RustTriest,
}

#[pymethods]
impl Triest {
    #[new]
    #[pyo3(signature = (m, seed=None))]
    fn new(m: usize, seed: Option<u64>) -> PyResult<Self> {
        let res = match seed {
            Some(s) => RustTriest::with_seed(m, s),
            None => RustTriest::new(m),
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

    /// Total number of edges seen.
    fn edges_seen(&self) -> u64 {
        self.inner.edges_seen()
    }

    /// Current reservoir occupancy.
    fn sample_size(&self) -> usize {
        self.inner.sample_size()
    }

    fn __repr__(&self) -> String {
        format!(
            "Triest(sample_size={}, estimate={:.1})",
            self.inner.sample_size(),
            self.inner.estimate()
        )
    }
}
