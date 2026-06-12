//! Python bindings for the HyperCalm periodic-batch detection sketch.

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::streaming::HyperCalm as RustHyperCalm;

/// HyperCalm — detects periodic batches (items recurring at a fixed period) in a
/// stream, combining a HyperBloomFilter for period estimation with a CALM
/// space-saving top-k.
///
/// Args:
///     t_threshold (int): minimum inter-arrival gap to treat as periodic.
///     d (int): HyperBloomFilter rows.
///     m (int): HyperBloomFilter columns.
///     l (int): bits per cell.
///     recorder_cap (int): per-item recorder capacity.
///     lru_w (int): LRU width.
///     promotion (int): promotion threshold.
///     ss_size (int): space-saving table size.
///     k (int): number of top periodic items to track.
#[pyclass(module = "sketch_oxide")]
pub struct HyperCalm {
    inner: RustHyperCalm,
}

#[pymethods]
impl HyperCalm {
    #[new]
    #[allow(clippy::too_many_arguments)]
    fn new(
        t_threshold: u64,
        d: usize,
        m: usize,
        l: usize,
        recorder_cap: usize,
        lru_w: usize,
        promotion: u64,
        ss_size: usize,
        k: usize,
    ) -> PyResult<Self> {
        RustHyperCalm::new(
            t_threshold,
            d,
            m,
            l,
            recorder_cap,
            lru_w,
            promotion,
            ss_size,
            k,
        )
        .map(|inner| Self { inner })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Records an item (int, str, bytes, or float) seen at the given timestamp.
    fn insert(&mut self, item: &Bound<'_, PyAny>, timestamp: u64) -> PyResult<()> {
        self.inner.insert(&python_item_to_bytes(item)?, timestamp);
        Ok(())
    }

    /// Top periodic items as a list of `((item_bytes, period), count)` tuples.
    fn top_k_periodic(&self, py: Python<'_>) -> Vec<((Py<PyBytes>, u64), u64)> {
        self.inner
            .top_k_periodic()
            .into_iter()
            .map(|((item, period), count)| {
                ((PyBytes::new_bound(py, &item).unbind(), period), count)
            })
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "HyperCalm(periodic_items={})",
            self.inner.top_k_periodic().len()
        )
    }
}
