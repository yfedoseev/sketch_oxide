//! Python bindings for Bloom-RF (range-capable Bloom filter).

use pyo3::prelude::*;
use sketch_oxide::range_filters::BloomRf as RustBloomRf;

/// BloomRf — a Bloom filter extended with prefix levels so it supports both point
/// and range membership queries on integer keys.
///
/// Args:
///     num_bits (int): total bit-array size.
///     num_hashes (int): hash functions per insert/query.
///     min_level (int): lowest prefix level indexed.
///     level_step (int): bit step between indexed levels.
#[pyclass(module = "sketch_oxide")]
pub struct BloomRf {
    inner: RustBloomRf,
}

#[pymethods]
impl BloomRf {
    #[new]
    fn new(num_bits: usize, num_hashes: u32, min_level: u32, level_step: u32) -> PyResult<Self> {
        RustBloomRf::new(num_bits, num_hashes, min_level, level_step)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Inserts an integer key.
    fn insert(&mut self, key: u64) {
        self.inner.insert(key);
    }

    /// Whether the point `key` might be present.
    fn contains(&self, key: u64) -> bool {
        self.inner.contains(key)
    }

    /// Whether the inclusive range `[lo, hi]` might contain a key (no false negatives).
    fn range_query(&self, lo: u64, hi: u64) -> bool {
        self.inner.range_query(lo, hi)
    }

    fn __repr__(&self) -> String {
        "BloomRf()".to_string()
    }
}
