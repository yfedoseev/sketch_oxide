//! Python bindings for the L0 (distinct-element) sampler over a dynamic stream.

use pyo3::prelude::*;
use sketch_oxide::sampling::L0Sampler as RustL0Sampler;

/// L0Sampler — samples a near-uniform key from the support of a dynamic
/// (insert/delete) integer-keyed stream.
#[pyclass(module = "sketch_oxide")]
pub struct L0Sampler {
    inner: RustL0Sampler,
}

#[pymethods]
impl L0Sampler {
    #[new]
    fn new() -> Self {
        Self {
            inner: RustL0Sampler::new(),
        }
    }

    /// Applies a signed update of `delta` to integer `key`.
    fn update(&mut self, key: u64, delta: i64) {
        self.inner.update(key, delta);
    }

    /// Inserts one occurrence of `key` (delta = +1).
    fn insert(&mut self, key: u64) {
        self.inner.insert(key);
    }

    /// Deletes one occurrence of `key` (delta = -1).
    fn delete(&mut self, key: u64) {
        self.inner.delete(key);
    }

    /// Returns a sampled key from the current support, or None if empty.
    fn sample(&self) -> Option<u64> {
        self.inner.sample()
    }

    /// Whether the current support appears empty.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn __repr__(&self) -> String {
        format!("L0Sampler(is_empty={})", self.inner.is_empty())
    }
}
