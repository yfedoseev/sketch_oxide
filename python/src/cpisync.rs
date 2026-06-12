//! Python bindings for CPISync set reconciliation.

use pyo3::prelude::*;
use sketch_oxide::reconciliation::CpiSync as RustCpiSync;

/// CpiSync — Characteristic-Polynomial Interpolation set reconciliation: recovers
/// the symmetric difference of two integer sets of size ≤ `capacity` using a
/// number of evaluation points proportional to the difference, not the set size.
///
/// Args:
///     capacity (int): maximum recoverable symmetric-difference size.
#[pyclass(module = "sketch_oxide")]
pub struct CpiSync {
    inner: RustCpiSync,
}

#[pymethods]
impl CpiSync {
    #[new]
    fn new(capacity: usize) -> PyResult<Self> {
        RustCpiSync::new(capacity)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Reconciles two integer key sets `a` and `b`, returning `(a_only, b_only)`:
    /// elements present in exactly one of the two sets.
    fn reconcile(&self, a: Vec<u64>, b: Vec<u64>) -> PyResult<(Vec<u64>, Vec<u64>)> {
        self.inner
            .reconcile(&a, &b)
            .map(|d| (d.a_only, d.b_only))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Maximum recoverable symmetric-difference size.
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    fn __repr__(&self) -> String {
        format!("CpiSync(capacity={})", self.inner.capacity())
    }
}
