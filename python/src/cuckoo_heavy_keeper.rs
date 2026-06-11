//! Python bindings for the Cuckoo Heavy Keeper (top-k).
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::frequency::CuckooHeavyKeeper as RustCuckooHeavyKeeper;

/// Cuckoo Heavy Keeper — a cuckoo-hashed HeavyKeeper for accurate top-k under tight memory.
#[pyclass(module = "sketch_oxide")]
pub struct CuckooHeavyKeeper {
    inner: RustCuckooHeavyKeeper,
}

#[pymethods]
impl CuckooHeavyKeeper {
    #[new]
    fn new(k: usize, num_buckets: usize, slots_per_bucket: usize) -> PyResult<Self> {
        RustCuckooHeavyKeeper::new(k, num_buckets, slots_per_bucket)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.update(&python_item_to_bytes(item)?);
        Ok(())
    }
    fn estimate(&self, item: &Bound<'_, PyAny>) -> PyResult<u32> {
        Ok(self.inner.estimate(&python_item_to_bytes(item)?))
    }
    /// Top-k items as (fingerprint, count).
    fn top_k(&self) -> Vec<(u64, u32)> {
        self.inner.top_k()
    }
    fn __repr__(&self) -> String {
        "CuckooHeavyKeeper()".to_string()
    }
}
