//! Python bindings for weighted reservoir sampling (A-Res).

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::sampling::WeightedReservoirSampling as RustWeightedReservoirSampling;

/// WeightedReservoirSampling — Efraimidis–Spirakis A-Res weighted sampling
/// without replacement.
///
/// Args:
///     k (int): reservoir size.
///     seed (int, optional): RNG seed for reproducibility.
#[pyclass(module = "sketch_oxide")]
pub struct WeightedReservoirSampling {
    inner: RustWeightedReservoirSampling<Vec<u8>>,
}

#[pymethods]
impl WeightedReservoirSampling {
    #[new]
    #[pyo3(signature = (k, seed=None))]
    fn new(k: usize, seed: Option<u64>) -> PyResult<Self> {
        let res = match seed {
            Some(s) => RustWeightedReservoirSampling::with_seed(k, s),
            None => RustWeightedReservoirSampling::new(k),
        };
        res.map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Offers a weighted item (int, str, bytes, or float) to the reservoir.
    fn update(&mut self, item: &Bound<'_, PyAny>, weight: f64) -> PyResult<()> {
        self.inner.update(python_item_to_bytes(item)?, weight);
        Ok(())
    }

    /// Current reservoir contents, each item returned as `bytes`.
    fn sample(&self, py: Python<'_>) -> Vec<Py<PyBytes>> {
        self.inner
            .sample()
            .into_iter()
            .map(|v| PyBytes::new(py, v).unbind())
            .collect()
    }

    /// Number of items currently in the reservoir.
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the reservoir is empty.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Reservoir capacity.
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Total number of items seen.
    fn count(&self) -> u64 {
        self.inner.count()
    }

    /// Clears the reservoir.
    fn clear(&mut self) {
        self.inner.clear();
    }

    /// Merges another reservoir of the same capacity into this one.
    fn merge(&mut self, other: &WeightedReservoirSampling) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "WeightedReservoirSampling(len={}, count={})",
            self.inner.len(),
            self.inner.count()
        )
    }
}
