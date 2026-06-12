//! Python bindings for the ThinkD fully-dynamic triangle estimator.

use pyo3::prelude::*;
use sketch_oxide::graph::ThinkD as RustThinkD;

/// ThinkD — "Think before you Discard": fully-dynamic (insert/delete) triangle
/// counting that samples edges with rate `r` and gives unbiased global and local
/// triangle estimates.
///
/// Args:
///     r (float): edge sampling rate.
///     seed (int, optional): RNG seed for reproducibility.
#[pyclass(module = "sketch_oxide")]
pub struct ThinkD {
    inner: RustThinkD,
}

#[pymethods]
impl ThinkD {
    #[new]
    #[pyo3(signature = (r, seed=None))]
    fn new(r: f64, seed: Option<u64>) -> PyResult<Self> {
        let res = match seed {
            Some(s) => RustThinkD::with_seed(r, s),
            None => RustThinkD::new(r),
        };
        res.map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Inserts an (undirected) edge between integer vertices `u` and `v`.
    fn add_edge(&mut self, u: u64, v: u64) {
        self.inner.add_edge(u, v);
    }

    /// Deletes an (undirected) edge between integer vertices `u` and `v`.
    fn remove_edge(&mut self, u: u64, v: u64) {
        self.inner.remove_edge(u, v);
    }

    /// Estimated global triangle count.
    fn global_count(&self) -> f64 {
        self.inner.global_count()
    }

    /// Estimated local triangle count incident to `node`.
    fn local_count(&self, node: u64) -> f64 {
        self.inner.local_count(node)
    }

    /// Edge sampling rate.
    fn probability(&self) -> f64 {
        self.inner.probability()
    }

    /// Number of sampled edges.
    fn sampled_edges(&self) -> usize {
        self.inner.sampled_edges()
    }

    fn __repr__(&self) -> String {
        format!(
            "ThinkD(sampled_edges={}, global_count={:.1})",
            self.inner.sampled_edges(),
            self.inner.global_count()
        )
    }
}
