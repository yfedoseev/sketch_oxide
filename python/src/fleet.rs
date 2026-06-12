//! Python bindings for the Fleet streaming triangle estimator.

use pyo3::prelude::*;
use sketch_oxide::graph::Fleet as RustFleet;

/// Fleet — a fully-dynamic reservoir-based triangle-count estimator over a stream
/// of edges, balancing a reservoir of size `max_reservoir` against a sampling
/// rate controlled by `gamma`.
///
/// Args:
///     max_reservoir (int): maximum edges retained.
///     gamma (float): sampling-rate parameter.
///     seed (int): RNG seed.
#[pyclass(module = "sketch_oxide")]
pub struct Fleet {
    inner: RustFleet,
}

#[pymethods]
impl Fleet {
    #[new]
    fn new(max_reservoir: usize, gamma: f64, seed: u64) -> PyResult<Self> {
        RustFleet::new(max_reservoir, gamma, seed)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Offers an (undirected) edge between integer vertices `l` and `r`.
    fn add_edge(&mut self, l: u64, r: u64) {
        self.inner.add_edge(l, r);
    }

    /// Estimated number of triangles.
    fn estimate(&self) -> f64 {
        self.inner.estimate()
    }

    /// Current reservoir occupancy.
    fn reservoir_size(&self) -> usize {
        self.inner.reservoir_size()
    }

    /// Current sampling probability.
    fn sampling_probability(&self) -> f64 {
        self.inner.sampling_probability()
    }

    fn __repr__(&self) -> String {
        format!(
            "Fleet(reservoir_size={}, estimate={:.1})",
            self.inner.reservoir_size(),
            self.inner.estimate()
        )
    }
}
