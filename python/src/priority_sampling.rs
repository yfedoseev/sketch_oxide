//! Python bindings for priority sampling (weighted subset-sum estimation).

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::sampling::PrioritySampling as RustPrioritySampling;

/// PrioritySampling — a bounded weighted sample supporting unbiased subset-sum
/// and total-weight estimation.
///
/// Args:
///     k (int): sample size.
///     seed (int, optional): RNG seed for reproducibility.
#[pyclass(module = "sketch_oxide")]
pub struct PrioritySampling {
    inner: RustPrioritySampling<Vec<u8>>,
}

#[pymethods]
impl PrioritySampling {
    #[new]
    #[pyo3(signature = (k, seed=None))]
    fn new(k: usize, seed: Option<u64>) -> PyResult<Self> {
        let res = match seed {
            Some(s) => RustPrioritySampling::with_seed(k, s),
            None => RustPrioritySampling::new(k),
        };
        res.map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Adds a weighted item (int, str, bytes, or float).
    fn update(&mut self, item: &Bound<'_, PyAny>, weight: f64) -> PyResult<()> {
        self.inner.update(python_item_to_bytes(item)?, weight);
        Ok(())
    }

    /// Current sample as a list of `(bytes, adjusted_weight)` pairs.
    fn sample(&self, py: Python<'_>) -> Vec<(Py<PyBytes>, f64)> {
        self.inner
            .sample()
            .into_iter()
            .map(|(v, w)| (PyBytes::new(py, &v).unbind(), w))
            .collect()
    }

    /// Unbiased estimate of the total weight of the stream.
    fn estimated_total(&self) -> f64 {
        self.inner.estimated_total()
    }

    /// Number of items currently in the sample.
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the sample is empty.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Maximum sample size.
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    fn __repr__(&self) -> String {
        format!(
            "PrioritySampling(len={}, estimated_total={:.3})",
            self.inner.len(),
            self.inner.estimated_total()
        )
    }
}
