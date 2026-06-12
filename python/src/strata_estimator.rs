//! Python bindings for the strata estimator (set-difference size estimation).

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::reconciliation::StrataEstimator as RustStrataEstimator;

/// StrataEstimator — estimates the size of the symmetric difference between two
/// sets (Eppstein et al.) by partitioning elements into geometrically-sized
/// strata, each a small IBLT, before committing to a full reconciliation.
///
/// Args:
///     num_strata (int): number of strata (covers differences up to ~2**num_strata).
#[pyclass(module = "sketch_oxide")]
pub struct StrataEstimator {
    inner: RustStrataEstimator,
}

#[pymethods]
impl StrataEstimator {
    #[new]
    fn new(num_strata: usize) -> PyResult<Self> {
        RustStrataEstimator::new(num_strata)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Inserts a key (int, str, bytes, or float).
    fn insert(&mut self, key: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.insert(&python_item_to_bytes(key)?);
        Ok(())
    }

    /// Estimates the symmetric-difference size against another estimator.
    fn estimate_difference(&self, other: &StrataEstimator) -> PyResult<usize> {
        self.inner
            .estimate_difference(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Number of strata.
    fn num_strata(&self) -> usize {
        self.inner.num_strata()
    }

    fn __repr__(&self) -> String {
        format!("StrataEstimator(num_strata={})", self.inner.num_strata())
    }
}
