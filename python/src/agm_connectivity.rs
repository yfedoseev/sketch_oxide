//! Python bindings for AGM-sketch graph connectivity.

use pyo3::prelude::*;
use sketch_oxide::graph::AgmConnectivity as RustAgmConnectivity;

/// AgmConnectivity — graph connectivity over a dynamic edge stream using the
/// Ahn–Guha–McGregor linear sketch, recovering connected components in sublinear
/// space for a graph on `n` vertices.
///
/// Args:
///     n (int): number of vertices (vertices are `0 .. n-1`).
///     seed (int): hashing seed.
#[pyclass(module = "sketch_oxide")]
pub struct AgmConnectivity {
    inner: RustAgmConnectivity,
}

#[pymethods]
impl AgmConnectivity {
    #[new]
    fn new(n: usize, seed: u64) -> PyResult<Self> {
        RustAgmConnectivity::new(n, seed)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Adds an (undirected) edge between vertices `u` and `v`.
    fn add_edge(&mut self, u: usize, v: usize) {
        self.inner.add_edge(u, v);
    }

    /// Component label for each vertex (vertices sharing a label are connected).
    fn components(&self) -> Vec<usize> {
        self.inner.components()
    }

    /// Number of connected components.
    fn num_components(&self) -> usize {
        self.inner.num_components()
    }

    /// Whether `u` and `v` are in the same connected component.
    fn connected(&self, u: usize, v: usize) -> bool {
        self.inner.connected(u, v)
    }

    fn __repr__(&self) -> String {
        format!(
            "AgmConnectivity(num_components={})",
            self.inner.num_components()
        )
    }
}
