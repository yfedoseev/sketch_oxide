//! Python bindings for the persistent (time-range) Count-Min sketch.

use crate::common::python_item_to_hash;
use pyo3::prelude::*;
use sketch_oxide::streaming::PersistentCountMin as RustPersistentCountMin;

/// PersistentCountMin — Count-Min variant supporting time-range frequency queries
/// by retaining one sketch per time bucket.
///
/// Args:
///     t_max (int): maximum tracked timestamp.
///     granularity (int): time-bucket width.
///     width (int): counters per row.
///     depth (int): number of hash rows.
///     seed (int): hashing seed.
#[pyclass(module = "sketch_oxide")]
pub struct PersistentCountMin {
    inner: RustPersistentCountMin,
}

#[pymethods]
impl PersistentCountMin {
    #[new]
    fn new(t_max: u64, granularity: u64, width: usize, depth: usize, seed: u64) -> PyResult<Self> {
        RustPersistentCountMin::new(t_max, granularity, width, depth, seed)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Records an item (int, str, bytes, or float) seen at the timestamp.
    fn insert(&mut self, item: &Bound<'_, PyAny>, timestamp: u64) -> PyResult<()> {
        self.inner.insert(python_item_to_hash(item)?, timestamp);
        Ok(())
    }

    /// Estimated frequency of an item within `[start, end]`.
    fn estimate_range(&self, item: &Bound<'_, PyAny>, start: u64, end: u64) -> PyResult<u64> {
        Ok(self
            .inner
            .estimate_range(python_item_to_hash(item)?, start, end))
    }

    /// Maximum tracked timestamp.
    fn t_max(&self) -> u64 {
        self.inner.t_max()
    }

    /// Number of per-bucket sketches.
    fn num_sketches(&self) -> usize {
        self.inner.num_sketches()
    }

    fn __repr__(&self) -> String {
        format!(
            "PersistentCountMin(t_max={}, num_sketches={})",
            self.inner.t_max(),
            self.inner.num_sketches()
        )
    }
}
