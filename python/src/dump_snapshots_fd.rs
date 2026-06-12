//! Python bindings for Dump-Snapshots Frequent Directions (sliding window).

use pyo3::prelude::*;
use sketch_oxide::matrix::DumpSnapshotsFd as RustDumpSnapshotsFd;

/// DumpSnapshotsFd — sliding-window Frequent Directions via the "dump snapshots"
/// scheme: periodic snapshots let the covariance of the last `window` rows be
/// recovered within relative error `eps`.
///
/// Args:
///     d (int): number of columns (row dimension).
///     eps (float): relative error target.
///     window (int): sliding window length in rows.
#[pyclass(module = "sketch_oxide")]
pub struct DumpSnapshotsFd {
    inner: RustDumpSnapshotsFd,
}

#[pymethods]
impl DumpSnapshotsFd {
    #[new]
    fn new(d: usize, eps: f64, window: u64) -> PyResult<Self> {
        RustDumpSnapshotsFd::new(d, eps, window)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Appends a `d`-dimensional row to the window.
    fn update(&mut self, row: Vec<f64>) -> PyResult<()> {
        self.inner
            .update(&row)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Approximated covariance matrix `Aᵀ A` over the current window (`d × d`).
    fn covariance(&self) -> Vec<Vec<f64>> {
        self.inner.covariance()
    }

    /// Number of sketch rows.
    fn ell(&self) -> usize {
        self.inner.ell()
    }

    /// Row dimension.
    fn dim(&self) -> usize {
        self.inner.dim()
    }

    fn __repr__(&self) -> String {
        format!(
            "DumpSnapshotsFd(ell={}, dim={})",
            self.inner.ell(),
            self.inner.dim()
        )
    }
}
