//! Python bindings for the Morton Filter.
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::membership::MortonFilter as RustMortonFilter;

/// Morton Filter — a compressed, cache-efficient cuckoo filter (Breslow & Jayasena 2018).
#[pyclass(module = "sketch_oxide")]
pub struct MortonFilter {
    inner: RustMortonFilter,
}

#[pymethods]
impl MortonFilter {
    #[new]
    fn new(
        num_blocks: usize,
        buckets_per_block: usize,
        slots_per_bucket: usize,
        fsa_capacity: usize,
        fingerprint_bits: u32,
        ota_bits: usize,
    ) -> PyResult<Self> {
        RustMortonFilter::new(
            num_blocks,
            buckets_per_block,
            slots_per_bucket,
            fsa_capacity,
            fingerprint_bits,
            ota_bits,
        )
        .map(|inner| Self { inner })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    /// Inserts a key; returns False if the filter is full.
    fn insert(&mut self, item: &Bound<'_, PyAny>) -> PyResult<bool> {
        Ok(self.inner.insert(&python_item_to_bytes(item)?))
    }
    fn contains(&self, item: &Bound<'_, PyAny>) -> PyResult<bool> {
        Ok(self.inner.contains(&python_item_to_bytes(item)?))
    }
    fn remove(&mut self, item: &Bound<'_, PyAny>) -> PyResult<bool> {
        Ok(self.inner.remove(&python_item_to_bytes(item)?))
    }
    fn __len__(&self) -> usize {
        self.inner.len()
    }
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    fn __repr__(&self) -> String {
        format!("MortonFilter(len={})", self.inner.len())
    }
}
