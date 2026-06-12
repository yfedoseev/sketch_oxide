//! Python bindings for the OmniSketch (multi-attribute query sketch).

use pyo3::prelude::*;
use sketch_oxide::universal::OmniSketch as RustOmniSketch;

/// OmniSketch — a sketch for approximate multi-predicate (conjunctive) cardinality
/// queries over a stream of records, each tagged with several integer attributes;
/// it estimates how many records match a set of `(attribute_index, value)` filters.
///
/// Args:
///     num_attrs (int): number of attributes per record.
///     d (int): number of hash rows per attribute.
///     w (int): width (buckets) per row.
///     b (int): sample bound per bucket.
#[pyclass(module = "sketch_oxide")]
pub struct OmniSketch {
    inner: RustOmniSketch,
}

#[pymethods]
impl OmniSketch {
    #[new]
    fn new(num_attrs: usize, d: usize, w: usize, b: usize) -> PyResult<Self> {
        RustOmniSketch::new(num_attrs, d, w, b)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Inserts a record with integer attribute values `attrs` and record id `rid`.
    fn insert(&mut self, attrs: Vec<u64>, rid: u64) {
        self.inner.insert(&attrs, rid);
    }

    /// Estimated number of records matching all `(attr_index, value)` predicates.
    fn query(&self, predicates: Vec<(usize, u64)>) -> f64 {
        self.inner.query(&predicates)
    }

    /// Number of attributes per record.
    fn num_attrs(&self) -> usize {
        self.inner.num_attrs()
    }

    fn __repr__(&self) -> String {
        format!("OmniSketch(num_attrs={})", self.inner.num_attrs())
    }
}
