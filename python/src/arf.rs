//! Python bindings for the Adaptive Range Filter (ARF).

use pyo3::prelude::*;
use sketch_oxide::range_filters::Arf as RustArf;

/// Arf — Adaptive Range Filter: a learned trie over a `domain_bits`-wide integer
/// domain that answers range-emptiness queries and refines itself from observed
/// empty ranges, bounded to `max_leaves` nodes.
///
/// Args:
///     domain_bits (int): bit-width of the key domain.
///     max_leaves (int): maximum trie leaves (space bound).
#[pyclass(module = "sketch_oxide")]
pub struct Arf {
    inner: RustArf,
}

#[pymethods]
impl Arf {
    #[new]
    fn new(domain_bits: u32, max_leaves: usize) -> PyResult<Self> {
        RustArf::new(domain_bits, max_leaves)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Inserts an integer key.
    fn insert(&mut self, key: u64) {
        self.inner.insert(key);
    }

    /// Whether the inclusive range `[lo, hi]` might contain a key (no false negatives).
    fn query(&mut self, lo: u64, hi: u64) -> bool {
        self.inner.query(lo, hi)
    }

    /// Teaches the filter that `[lo, hi]` is empty, allowing it to refine.
    fn learn_empty(&mut self, lo: u64, hi: u64) {
        self.inner.learn_empty(lo, hi);
    }

    /// Current number of trie leaves.
    fn size(&self) -> usize {
        self.inner.size()
    }

    fn __repr__(&self) -> String {
        format!("Arf(size={})", self.inner.size())
    }
}
