//! Python bindings for the Invertible Bloom Lookup Table (IBLT).

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::common::Reconcilable;
use sketch_oxide::reconciliation::Iblt as RustIblt;

/// Iblt — Invertible Bloom Lookup Table for key/value set reconciliation. Two
/// peers each build an IBLT over their key/value pairs; subtracting one from the
/// other and decoding recovers the symmetric difference.
///
/// Args:
///     expected_diff (int): expected symmetric-difference size (> 0).
///     cell_size (int): maximum key/value byte size per cell (>= 8).
#[pyclass(module = "sketch_oxide")]
pub struct Iblt {
    inner: RustIblt,
}

#[pymethods]
impl Iblt {
    #[new]
    fn new(expected_diff: usize, cell_size: usize) -> PyResult<Self> {
        RustIblt::new(expected_diff, cell_size)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Inserts a key/value pair (each int, str, bytes, or float).
    fn insert(&mut self, key: &Bound<'_, PyAny>, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let k = python_item_to_bytes(key)?;
        let v = python_item_to_bytes(value)?;
        self.inner
            .insert(&k, &v)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Deletes a previously inserted key/value pair.
    fn delete(&mut self, key: &Bound<'_, PyAny>, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let k = python_item_to_bytes(key)?;
        let v = python_item_to_bytes(value)?;
        self.inner
            .delete(&k, &v)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Subtracts another IBLT (same parameters) from this one in place, leaving
    /// the symmetric difference encoded.
    fn subtract(&mut self, other: &Iblt) -> PyResult<()> {
        self.inner
            .subtract(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Decodes the encoded difference, returning `(to_insert, to_remove)` where
    /// each is a list of `(key_bytes, value_bytes)` pairs.
    #[allow(clippy::type_complexity)]
    fn decode(
        &self,
        py: Python<'_>,
    ) -> PyResult<(
        Vec<(Py<PyBytes>, Py<PyBytes>)>,
        Vec<(Py<PyBytes>, Py<PyBytes>)>,
    )> {
        let diff = self
            .inner
            .decode()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        let conv = |pairs: Vec<(Vec<u8>, Vec<u8>)>| {
            pairs
                .into_iter()
                .map(|(k, v)| (PyBytes::new(py, &k).unbind(), PyBytes::new(py, &v).unbind()))
                .collect::<Vec<_>>()
        };
        Ok((conv(diff.to_insert), conv(diff.to_remove)))
    }

    /// Returns `(num_cells, cell_size)`.
    fn stats(&self) -> (usize, usize) {
        let s = self.inner.stats();
        (s.num_cells, s.cell_size)
    }

    fn __repr__(&self) -> String {
        let s = self.inner.stats();
        format!("Iblt(num_cells={}, cell_size={})", s.num_cells, s.cell_size)
    }
}
