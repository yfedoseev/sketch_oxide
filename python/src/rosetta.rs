//! Python bindings for the Rosetta range filter.

use pyo3::prelude::*;
use sketch_oxide::range_filters::Rosetta as RustRosetta;

/// Rosetta — a range filter built from a hierarchy of Bloom filters over key
/// prefixes (the SIGMOD'20 design), answering range-emptiness with a target FPR.
///
/// Args:
///     expected_keys (int): expected number of keys (sizes the filters).
///     bits (int): key bit-width.
///     fpr (float): target false-positive rate.
#[pyclass(module = "sketch_oxide")]
pub struct Rosetta {
    inner: RustRosetta,
}

#[pymethods]
impl Rosetta {
    #[new]
    fn new(expected_keys: usize, bits: u32, fpr: f64) -> PyResult<Self> {
        RustRosetta::new(expected_keys, bits, fpr)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Inserts an integer key.
    fn insert(&mut self, key: u64) {
        self.inner.insert(key);
    }

    /// Whether the inclusive range `[low, high]` might contain a key (no false negatives).
    fn range_query(&self, low: u64, high: u64) -> bool {
        self.inner.range_query(low, high)
    }

    /// Number of keys inserted.
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the filter is empty.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Number of hash functions per level.
    fn num_hashes(&self) -> u32 {
        self.inner.num_hashes()
    }

    fn __repr__(&self) -> String {
        format!("Rosetta(len={})", self.inner.len())
    }
}
