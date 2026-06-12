//! Python bindings for the GSS (Graph Stream Sketch).

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::graph::GssSketch as RustGssSketch;

/// GssSketch — Graph Stream Sketch: a compact adjacency-matrix sketch supporting
/// edge-weight, out-degree, and in-degree queries over a directed weighted graph
/// stream, with a small overflow buffer for collisions.
///
/// Args:
///     side (int): hash-matrix side length.
///     room (int): slots per matrix cell.
#[pyclass(module = "sketch_oxide")]
pub struct GssSketch {
    inner: RustGssSketch,
}

#[pymethods]
impl GssSketch {
    #[new]
    fn new(side: usize, room: usize) -> PyResult<Self> {
        RustGssSketch::new(side, room)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Adds `weight` to the directed edge `s -> d` (vertices are int/str/bytes/float).
    fn add_edge(
        &mut self,
        s: &Bound<'_, PyAny>,
        d: &Bound<'_, PyAny>,
        weight: u64,
    ) -> PyResult<()> {
        let sb = python_item_to_bytes(s)?;
        let db = python_item_to_bytes(d)?;
        self.inner.add_edge(&sb, &db, weight);
        Ok(())
    }

    /// Estimated weight of the edge `s -> d`.
    fn edge_weight(&self, s: &Bound<'_, PyAny>, d: &Bound<'_, PyAny>) -> PyResult<u64> {
        let sb = python_item_to_bytes(s)?;
        let db = python_item_to_bytes(d)?;
        Ok(self.inner.edge_weight(&sb, &db))
    }

    /// Estimated total out-weight of vertex `s`.
    fn out_degree(&self, s: &Bound<'_, PyAny>) -> PyResult<u64> {
        Ok(self.inner.out_degree(&python_item_to_bytes(s)?))
    }

    /// Estimated total in-weight of vertex `d`.
    fn in_degree(&self, d: &Bound<'_, PyAny>) -> PyResult<u64> {
        Ok(self.inner.in_degree(&python_item_to_bytes(d)?))
    }

    /// Number of edges held in the overflow buffer.
    fn buffer_len(&self) -> usize {
        self.inner.buffer_len()
    }

    /// Matrix side length.
    fn side(&self) -> usize {
        self.inner.side()
    }

    /// Slots per matrix cell.
    fn room(&self) -> usize {
        self.inner.room()
    }

    fn __repr__(&self) -> String {
        format!(
            "GssSketch(side={}, room={}, buffer_len={})",
            self.inner.side(),
            self.inner.room(),
            self.inner.buffer_len()
        )
    }
}
