//! Python bindings for range-based set reconciliation.

use pyo3::prelude::*;
use sketch_oxide::reconciliation::RangeReconciler as RustRangeReconciler;

/// RangeReconciler — recursive range-based set reconciliation over a sorted
/// `u64` key set (the approach used by Mentat/range-based sync): both peers
/// compare hashes of key ranges, recursing only into ranges that differ.
///
/// Args:
///     keys (list[int]): the local key set (de-duplicated and sorted internally).
#[pyclass(module = "sketch_oxide")]
pub struct RangeReconciler {
    inner: RustRangeReconciler,
}

#[pymethods]
impl RangeReconciler {
    #[new]
    fn new(keys: Vec<u64>) -> Self {
        Self {
            inner: RustRangeReconciler::new(keys),
        }
    }

    /// Sorted, de-duplicated local keys.
    fn keys(&self) -> Vec<u64> {
        self.inner.keys().to_vec()
    }

    /// Reconciles against `other`, returning `(to_insert, to_remove)`: keys the
    /// local side is missing, and keys the local side has that the other lacks.
    /// `split_threshold` controls when a differing range is recursed vs. sent whole.
    fn reconcile(&self, other: &RangeReconciler, split_threshold: usize) -> (Vec<u64>, Vec<u64>) {
        let diff = self.inner.reconcile(&other.inner, split_threshold);
        (diff.to_insert, diff.to_remove)
    }

    fn __repr__(&self) -> String {
        format!("RangeReconciler(keys={})", self.inner.keys().len())
    }
}
