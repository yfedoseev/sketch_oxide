//! Python bindings for the Bloomier Filter (approximate key→value map).
use pyo3::prelude::*;
use sketch_oxide::membership::BloomierFilter as RustBloomier;

/// Bloomier Filter — a static, compact approximate map from `int` keys to `int` values.
#[pyclass(module = "sketch_oxide")]
pub struct BloomierFilter {
    inner: RustBloomier,
}

#[pymethods]
impl BloomierFilter {
    /// Builds from a list of `(key, value)` int pairs; `value_bits` sizes the value field.
    #[new]
    fn new(pairs: Vec<(u64, u64)>, value_bits: u8, fingerprint_bits: u8) -> PyResult<Self> {
        RustBloomier::from_pairs(&pairs, value_bits, fingerprint_bits)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    /// Returns the value mapped to `key`, or None if `key` was not in the build set.
    fn get(&self, key: u64) -> Option<u64> {
        self.inner.get(key)
    }
    fn __len__(&self) -> usize {
        self.inner.len()
    }
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    fn __repr__(&self) -> String {
        format!("BloomierFilter(len={})", self.inner.len())
    }
}
