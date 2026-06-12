//! Python bindings for HyperANF neighbourhood-function estimation.

use pyo3::prelude::*;
use sketch_oxide::graph::HyperAnf as RustHyperAnf;

/// HyperAnf — approximate neighbourhood function: uses HyperLogLog counters per
/// vertex to estimate, for each distance `t`, how many pairs `(u, v)` are within
/// `t` hops — yielding distance distributions and effective diameter.
///
/// Args:
///     precision (int): HyperLogLog precision (register count = 2**precision).
#[pyclass(module = "sketch_oxide")]
pub struct HyperAnf {
    inner: RustHyperAnf,
}

#[pymethods]
impl HyperAnf {
    #[new]
    fn new(precision: u8) -> PyResult<Self> {
        RustHyperAnf::new(precision)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Adds a directed edge `u -> v` (integer vertices).
    fn add_edge(&mut self, u: u64, v: u64) {
        self.inner.add_edge(u, v);
    }

    /// Neighbourhood function values `N(t)` for `t` in `0 ..= max_distance`.
    fn neighborhood_function(&self, max_distance: usize) -> Vec<f64> {
        self.inner.neighborhood_function(max_distance)
    }

    /// Estimated size of the ball of radius `t` around vertex `v`.
    fn ball_size(&self, v: u64, t: usize) -> f64 {
        self.inner.ball_size(v, t)
    }

    /// Number of vertices observed.
    fn num_vertices(&self) -> usize {
        self.inner.num_vertices()
    }

    fn __repr__(&self) -> String {
        format!("HyperAnf(num_vertices={})", self.inner.num_vertices())
    }
}
