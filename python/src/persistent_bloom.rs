//! Python bindings for the persistent (durability) Bloom filter.

use crate::common::python_item_to_hash;
use pyo3::prelude::*;
use sketch_oxide::streaming::PersistentBloomFilter as RustPersistentBloomFilter;

/// PersistentBloomFilter — answers "was this element present continuously over a
/// time range?" by layering Bloom filters across time granularities.
///
/// Args:
///     t_max (int): maximum tracked timestamp.
///     granularity (int): time-bucket width.
///     bits_per_filter (int): bits in each per-bucket filter.
///     num_hashes (int): hash functions per filter.
#[pyclass(module = "sketch_oxide")]
pub struct PersistentBloomFilter {
    inner: RustPersistentBloomFilter,
}

#[pymethods]
impl PersistentBloomFilter {
    #[new]
    fn new(
        t_max: u64,
        granularity: u64,
        bits_per_filter: usize,
        num_hashes: u32,
    ) -> PyResult<Self> {
        RustPersistentBloomFilter::new(t_max, granularity, bits_per_filter, num_hashes)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Records an element (int, str, bytes, or float) present at the timestamp.
    fn insert(&mut self, element: &Bound<'_, PyAny>, timestamp: u64) -> PyResult<()> {
        self.inner.insert(python_item_to_hash(element)?, timestamp);
        Ok(())
    }

    /// Whether the element was (probably) present throughout `[start, end]`.
    fn query(&self, element: &Bound<'_, PyAny>, start: u64, end: u64) -> PyResult<bool> {
        Ok(self.inner.query(python_item_to_hash(element)?, start, end))
    }

    /// Maximum tracked timestamp.
    fn t_max(&self) -> u64 {
        self.inner.t_max()
    }

    /// Number of per-bucket filters.
    fn num_filters(&self) -> usize {
        self.inner.num_filters()
    }

    fn __repr__(&self) -> String {
        format!(
            "PersistentBloomFilter(t_max={}, num_filters={})",
            self.inner.t_max(),
            self.inner.num_filters()
        )
    }
}
