//! Python bindings for the Xor Filter.
use crate::common::python_item_to_hash;
use pyo3::prelude::*;
use sketch_oxide::membership::XorFilter as RustXorFilter;

/// Xor Filter — a static AMQ ~25% smaller and faster than Bloom (Graf & Lemire 2020).
#[pyclass(module = "sketch_oxide")]
pub struct XorFilter {
    inner: RustXorFilter,
}

#[pymethods]
impl XorFilter {
    /// Builds from a list of keys (int/str/bytes/float) using `bits`-bit fingerprints (8 or 16).
    #[new]
    fn new(keys: Vec<Bound<'_, PyAny>>, bits: u8) -> PyResult<Self> {
        let hashed: Vec<u64> = keys
            .iter()
            .map(python_item_to_hash)
            .collect::<PyResult<_>>()?;
        RustXorFilter::from_keys(&hashed, bits)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn contains(&self, item: &Bound<'_, PyAny>) -> PyResult<bool> {
        Ok(self.inner.contains(python_item_to_hash(item)?))
    }
    fn __len__(&self) -> usize {
        self.inner.len()
    }
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    fn __repr__(&self) -> String {
        format!("XorFilter(len={})", self.inner.len())
    }
}
