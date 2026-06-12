//! Python bindings for the Proteus learned range filter.

use pyo3::prelude::*;
use sketch_oxide::range_filters::Proteus as RustProteus;

/// Proteus — a learned range filter that, given a key set, a memory budget, and a
/// sample of expected range queries, automatically tunes a trie-plus-Bloom design
/// to minimise false positives for that workload.
///
/// Args:
///     keys (list[int]): sorted integer keys.
///     mem_bits (int): memory budget in bits.
///     sample_queries (list[tuple[int, int]]): representative `(lo, hi)` query ranges.
#[pyclass(module = "sketch_oxide")]
pub struct Proteus {
    inner: RustProteus,
}

#[pymethods]
impl Proteus {
    #[new]
    fn new(keys: Vec<u64>, mem_bits: usize, sample_queries: Vec<(u64, u64)>) -> PyResult<Self> {
        RustProteus::build(&keys, mem_bits, &sample_queries)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Whether the inclusive range `[lo, hi]` might contain a key (no false negatives).
    fn range_query(&self, lo: u64, hi: u64) -> bool {
        self.inner.range_query(lo, hi)
    }

    /// Chosen trie depth.
    fn trie_depth(&self) -> u32 {
        self.inner.trie_depth()
    }

    /// Length of the backing Bloom filter (bits).
    fn bloom_len(&self) -> u32 {
        self.inner.bloom_len()
    }

    fn __repr__(&self) -> String {
        format!(
            "Proteus(trie_depth={}, bloom_len={})",
            self.inner.trie_depth(),
            self.inner.bloom_len()
        )
    }
}
