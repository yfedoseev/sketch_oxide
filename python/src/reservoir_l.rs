//! Python bindings for Algorithm L reservoir sampling.

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::sampling::ReservoirSamplingL as RustReservoirSamplingL;

/// ReservoirSamplingL — uniform reservoir sampling via Li's Algorithm L
/// (skip-based, O(k log(n/k)) randomness).
///
/// Args:
///     k (int): reservoir size.
///     seed (int, optional): RNG seed for reproducibility.
#[pyclass(module = "sketch_oxide")]
pub struct ReservoirSamplingL {
    inner: RustReservoirSamplingL<Vec<u8>>,
}

#[pymethods]
impl ReservoirSamplingL {
    #[new]
    #[pyo3(signature = (k, seed=None))]
    fn new(k: usize, seed: Option<u64>) -> PyResult<Self> {
        let res = match seed {
            Some(s) => RustReservoirSamplingL::with_seed(k, s),
            None => RustReservoirSamplingL::new(k),
        };
        res.map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Offers an item (int, str, bytes, or float) to the reservoir.
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.update(python_item_to_bytes(item)?);
        Ok(())
    }

    /// Current reservoir contents, each item returned as `bytes`.
    fn sample(&self, py: Python<'_>) -> Vec<Py<PyBytes>> {
        self.inner
            .sample()
            .iter()
            .map(|v| PyBytes::new(py, v).unbind())
            .collect()
    }

    /// Whether the reservoir is empty.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Number of items currently in the reservoir.
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Reservoir capacity.
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Total number of items seen.
    fn count(&self) -> u64 {
        self.inner.count()
    }

    /// Per-item inclusion probability at the current count.
    fn inclusion_probability(&self) -> f64 {
        self.inner.inclusion_probability()
    }

    /// Clears the reservoir.
    fn clear(&mut self) {
        self.inner.clear();
    }

    fn __repr__(&self) -> String {
        format!(
            "ReservoirSamplingL(len={}, count={})",
            self.inner.len(),
            self.inner.count()
        )
    }
}
