//! Python bindings for the PeriodicSketch periodic-item detector.

use crate::common::python_item_to_hash;
use pyo3::prelude::*;
use sketch_oxide::streaming::PeriodicSketch as RustPeriodicSketch;

/// PeriodicSketch — finds items that recur with a stable period, pairing a
/// Count-Min sketch with a grouped sum-update structure over time buckets.
///
/// Args:
///     cm_width (int): Count-Min counters per row.
///     cm_depth (int): Count-Min hash rows.
///     gsu_buckets (int): grouped-sum-update bucket count.
///     cells_per_bucket (int): cells per GSU bucket.
///     delta_t (int): time-bucket width.
///     seed (int): hashing seed.
#[pyclass(module = "sketch_oxide")]
pub struct PeriodicSketch {
    inner: RustPeriodicSketch,
}

#[pymethods]
impl PeriodicSketch {
    #[new]
    fn new(
        cm_width: usize,
        cm_depth: usize,
        gsu_buckets: usize,
        cells_per_bucket: usize,
        delta_t: u64,
        seed: u64,
    ) -> PyResult<Self> {
        RustPeriodicSketch::new(
            cm_width,
            cm_depth,
            gsu_buckets,
            cells_per_bucket,
            delta_t,
            seed,
        )
        .map(|inner| Self { inner })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Records an item (int, str, bytes, or float) seen at the given timestamp.
    fn insert(&mut self, item: &Bound<'_, PyAny>, timestamp: u64) -> PyResult<()> {
        self.inner.insert(python_item_to_hash(item)?, timestamp);
        Ok(())
    }

    /// Top-`k` periodic items as `(item_hash, period, count)` tuples.
    fn top_k(&self, k: usize) -> Vec<(u64, u64, u64)> {
        self.inner.top_k(k)
    }

    /// Configured time-bucket width.
    fn delta_t(&self) -> u64 {
        self.inner.delta_t()
    }

    fn __repr__(&self) -> String {
        format!("PeriodicSketch(delta_t={})", self.inner.delta_t())
    }
}
