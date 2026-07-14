//! Python bindings for the differentially-private Misra–Gries heavy hitters.

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use rand::rngs::StdRng;
use rand::SeedableRng;
use sketch_oxide::privacy::DpMisraGries as RustDpMisraGries;

/// DpMisraGries — the Misra–Gries heavy-hitters summary with a differentially-
/// private release (Lebeda & Tětek): keeps the top-`k` counters and adds
/// calibrated discrete noise at release time so the published heavy hitters are
/// `(epsilon, delta)`-DP. Uses an OS-seeded CSPRNG.
///
/// Args:
///     k (int): number of counters (summary capacity).
///     epsilon (float): privacy parameter (> 0).
///     delta (float): privacy parameter (>= 0).
#[pyclass(module = "sketch_oxide")]
pub struct DpMisraGries {
    inner: RustDpMisraGries<Vec<u8>>,
    rng: StdRng,
}

#[pymethods]
impl DpMisraGries {
    #[new]
    fn new(k: usize, epsilon: f64, delta: f64) -> PyResult<Self> {
        RustDpMisraGries::new(k, epsilon, delta)
            .map(|inner| Self {
                inner,
                rng: StdRng::from_os_rng(),
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Records one occurrence of `item` (int, str, bytes, or float).
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.update(python_item_to_bytes(item)?);
        Ok(())
    }

    /// Differentially-private release as a list of `(item_bytes, noisy_count)` pairs.
    fn release(&mut self, py: Python<'_>) -> Vec<(Py<PyBytes>, i64)> {
        self.inner
            .release(&mut self.rng)
            .into_iter()
            .map(|(k, c)| (PyBytes::new(py, &k).unbind(), c))
            .collect()
    }

    /// Summary capacity (number of counters).
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    fn __repr__(&self) -> String {
        format!("DpMisraGries(capacity={})", self.inner.capacity())
    }
}
