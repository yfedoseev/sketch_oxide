//! Python bindings for the EBPPS weighted sampling sketch.

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::sampling::EbppsSketch as RustEbppsSketch;

/// EbppsSketch — Exact Bounded-size Probability Proportional to Size weighted sampling.
///
/// Args:
///     k (int): target sample size.
///     seed (int, optional): RNG seed for reproducibility.
#[pyclass(module = "sketch_oxide")]
pub struct EbppsSketch {
    inner: RustEbppsSketch<Vec<u8>>,
}

#[pymethods]
impl EbppsSketch {
    #[new]
    #[pyo3(signature = (k, seed=None))]
    fn new(k: usize, seed: Option<u64>) -> PyResult<Self> {
        let res = match seed {
            Some(s) => RustEbppsSketch::with_seed(k, s),
            None => RustEbppsSketch::new(k),
        };
        res.map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Adds a weighted item (int, str, bytes, or float) with a positive weight.
    fn update(&mut self, item: &Bound<'_, PyAny>, weight: f64) -> PyResult<()> {
        self.inner
            .update(python_item_to_bytes(item)?, weight)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Draws the current sample, each item returned as `bytes`, or None if empty.
    fn sample(&mut self, py: Python<'_>) -> Option<Vec<Py<PyBytes>>> {
        self.inner
            .sample()
            .map(|items| items.iter().map(|v| PyBytes::new(py, v).unbind()).collect())
    }

    /// Number of items processed.
    fn n(&self) -> u64 {
        self.inner.n()
    }

    /// Total cumulative weight seen.
    fn cumulative_weight(&self) -> f64 {
        self.inner.cumulative_weight()
    }

    /// Expected (fractional) sample size.
    fn c(&self) -> f64 {
        self.inner.c()
    }

    /// Partial-item inclusion probability.
    fn rho(&self) -> f64 {
        self.inner.rho()
    }

    fn __repr__(&self) -> String {
        format!("EbppsSketch(n={}, c={:.3})", self.inner.n(), self.inner.c())
    }
}
