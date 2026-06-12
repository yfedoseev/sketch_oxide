//! Python bindings for stratified reservoir sampling.

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::sampling::StratifiedReservoir as RustStratifiedReservoir;

/// StratifiedReservoir — maintains an independent size-`k` uniform reservoir per
/// stratum (group key), enabling per-group and overall estimation.
///
/// Args:
///     k (int): reservoir size per stratum.
///     seed (int, optional): RNG seed for reproducibility.
#[pyclass(module = "sketch_oxide")]
pub struct StratifiedReservoir {
    inner: RustStratifiedReservoir<Vec<u8>, Vec<u8>>,
}

#[pymethods]
impl StratifiedReservoir {
    #[new]
    #[pyo3(signature = (k, seed=None))]
    fn new(k: usize, seed: Option<u64>) -> PyResult<Self> {
        let inner = match seed {
            Some(s) => RustStratifiedReservoir::with_seed(k, s),
            None => RustStratifiedReservoir::checked_new(k)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?,
        };
        Ok(Self { inner })
    }

    /// Offers an item (int, str, bytes, or float) into the reservoir for `stratum`.
    fn add(&mut self, stratum: &Bound<'_, PyAny>, item: &Bound<'_, PyAny>) -> PyResult<()> {
        let s = python_item_to_bytes(stratum)?;
        let i = python_item_to_bytes(item)?;
        self.inner.add(s, i);
        Ok(())
    }

    /// Current reservoir contents for `stratum` as a list of `bytes`, or None if unseen.
    fn sample(
        &self,
        py: Python<'_>,
        stratum: &Bound<'_, PyAny>,
    ) -> PyResult<Option<Vec<Py<PyBytes>>>> {
        let s = python_item_to_bytes(stratum)?;
        Ok(self.inner.sample(&s).map(|items| {
            items
                .iter()
                .map(|v| PyBytes::new_bound(py, v).unbind())
                .collect()
        }))
    }

    /// Reservoir capacity per stratum.
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Total number of items seen across all strata.
    fn total_seen(&self) -> u64 {
        self.inner.total_seen()
    }

    /// Number of distinct strata observed.
    fn num_strata(&self) -> usize {
        self.inner.num_strata()
    }

    /// Number of items seen for a given stratum.
    fn stratum_count(&self, stratum: &Bound<'_, PyAny>) -> PyResult<u64> {
        let s = python_item_to_bytes(stratum)?;
        Ok(self.inner.stratum_count(&s))
    }

    fn __repr__(&self) -> String {
        format!(
            "StratifiedReservoir(num_strata={}, total_seen={})",
            self.inner.num_strata(),
            self.inner.total_seen()
        )
    }
}
