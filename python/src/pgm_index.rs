//! Python bindings for the PGM-index (Piecewise Geometric Model index).

use pyo3::prelude::*;
use sketch_oxide::range_filters::PgmIndex as RustPgmIndex;

/// PgmIndex — a learned index that fits piecewise-linear models to a sorted key
/// array, giving rank/lookup with an `epsilon`-bounded position error.
///
/// Args:
///     keys (list[int]): sorted integer keys.
///     epsilon (int): position-error bound for each linear segment.
#[pyclass(module = "sketch_oxide")]
pub struct PgmIndex {
    inner: RustPgmIndex,
}

#[pymethods]
impl PgmIndex {
    #[new]
    fn new(keys: Vec<u64>, epsilon: usize) -> PyResult<Self> {
        RustPgmIndex::new(&keys, epsilon)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Number of keys less than `key` (its rank in the sorted array).
    fn rank(&self, key: u64) -> usize {
        self.inner.rank(key)
    }

    /// Whether `key` is present.
    fn contains(&self, key: u64) -> bool {
        self.inner.contains(key)
    }

    /// Number of indexed keys.
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the index is empty.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Number of piecewise-linear segments.
    fn num_segments(&self) -> usize {
        self.inner.num_segments()
    }

    fn __repr__(&self) -> String {
        format!(
            "PgmIndex(len={}, segments={})",
            self.inner.len(),
            self.inner.num_segments()
        )
    }
}
