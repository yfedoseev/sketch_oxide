//! Python bindings for the TCM (Triangular Count Matrix / graph sketch).

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::graph::TcmSketch as RustTcmSketch;

/// TcmSketch — Graphical Sketch (TCM): a stack of `depth` hashed adjacency
/// matrices of side `width` supporting edge-weight, out-degree, and in-degree
/// queries over a directed weighted graph stream.
///
/// Args:
///     depth (int): number of independent hash matrices.
///     width (int): side length of each matrix.
#[pyclass(module = "sketch_oxide")]
pub struct TcmSketch {
    inner: RustTcmSketch,
}

#[pymethods]
impl TcmSketch {
    #[new]
    fn new(depth: usize, width: usize) -> PyResult<Self> {
        RustTcmSketch::new(depth, width)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Adds `weight` to the directed edge `src -> dst` (vertices are int/str/bytes/float).
    fn add_edge(
        &mut self,
        src: &Bound<'_, PyAny>,
        dst: &Bound<'_, PyAny>,
        weight: u64,
    ) -> PyResult<()> {
        let s = python_item_to_bytes(src)?;
        let d = python_item_to_bytes(dst)?;
        self.inner.add_edge(&s, &d, weight);
        Ok(())
    }

    /// Estimated weight of the edge `src -> dst`.
    fn edge_weight(&self, src: &Bound<'_, PyAny>, dst: &Bound<'_, PyAny>) -> PyResult<u64> {
        let s = python_item_to_bytes(src)?;
        let d = python_item_to_bytes(dst)?;
        Ok(self.inner.edge_weight(&s, &d))
    }

    /// Estimated total out-weight of vertex `src`.
    fn out_degree(&self, src: &Bound<'_, PyAny>) -> PyResult<u64> {
        Ok(self.inner.out_degree(&python_item_to_bytes(src)?))
    }

    /// Estimated total in-weight of vertex `dst`.
    fn in_degree(&self, dst: &Bound<'_, PyAny>) -> PyResult<u64> {
        Ok(self.inner.in_degree(&python_item_to_bytes(dst)?))
    }

    /// Number of hash matrices.
    fn depth(&self) -> usize {
        self.inner.depth()
    }

    /// Side length of each matrix.
    fn width(&self) -> usize {
        self.inner.width()
    }

    fn __repr__(&self) -> String {
        format!(
            "TcmSketch(depth={}, width={})",
            self.inner.depth(),
            self.inner.width()
        )
    }
}
