//! Python bindings for the DPSW (differentially-private sliding-window) sketch.

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;
use sketch_oxide::privacy::DpswSketch as RustDpswSketch;

/// DpswSketch — a differentially-private sliding-window frequency sketch: it
/// answers per-item frequency over a window of `w` events while keeping each
/// event ε-DP, splitting the privacy budget across overlapping substreams. Uses
/// an OS-seeded discrete-noise CSPRNG.
///
/// Args:
///     w (int): sliding window length (events).
///     rho (float): substream overlap parameter.
///     alpha (float): budget-split parameter.
///     beta (float): failure-probability parameter.
///     depth (int): Count-Min hash rows.
///     width (int): Count-Min counters per row.
#[pyclass(module = "sketch_oxide")]
pub struct DpswSketch {
    inner: RustDpswSketch,
    rng: StdRng,
}

#[pymethods]
impl DpswSketch {
    #[new]
    fn new(w: u64, rho: f64, alpha: f64, beta: f64, depth: usize, width: usize) -> PyResult<Self> {
        RustDpswSketch::new(w, rho, alpha, beta, depth, width)
            .map(|inner| Self {
                inner,
                rng: StdRng::from_os_rng(),
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Records one occurrence of `item` (int, str, bytes, or float) into the window.
    fn insert(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner
            .insert(&python_item_to_bytes(item)?, &mut self.rng)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Differentially-private frequency estimate of `item` over the window.
    fn query(&self, item: &Bound<'_, PyAny>) -> PyResult<i64> {
        Ok(self.inner.query(&python_item_to_bytes(item)?))
    }

    /// Total privacy budget.
    fn total_budget(&self) -> f64 {
        self.inner.total_budget()
    }

    /// Size of each substream (events).
    fn substream_size(&self) -> u64 {
        self.inner.substream_size()
    }

    /// Substream overlap parameter.
    fn rho(&self) -> f64 {
        self.inner.rho()
    }

    /// Budget-split parameter.
    fn alpha(&self) -> f64 {
        self.inner.alpha()
    }

    /// Number of events currently in the window.
    fn len(&self) -> u64 {
        self.inner.len()
    }

    /// Whether the window is empty.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn __repr__(&self) -> String {
        format!("DpswSketch(len={})", self.inner.len())
    }
}
