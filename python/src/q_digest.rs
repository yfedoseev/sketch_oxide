//! Python bindings for the Q-Digest quantile sketch.
use pyo3::prelude::*;
use sketch_oxide::quantiles::QDigest as RustQDigest;

/// Q-Digest — mergeable quantiles over a fixed integer universe (Shrivastava 2004).
#[pyclass(module = "sketch_oxide")]
pub struct QDigest {
    inner: RustQDigest,
}

#[pymethods]
impl QDigest {
    #[new]
    fn new(levels: u32, compression: u64) -> PyResult<Self> {
        RustQDigest::new(levels, compression)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn update(&mut self, value: u64) -> PyResult<()> {
        self.inner
            .insert(value)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn quantile(&mut self, phi: f64) -> Option<u64> {
        self.inner.quantile(phi)
    }
    /// Estimated number of items at or below `value`.
    fn rank(&self, value: u64) -> u64 {
        self.inner.rank(value)
    }
    fn merge(&mut self, other: &QDigest) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn count(&self) -> u64 {
        self.inner.count()
    }
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    fn __repr__(&self) -> String {
        format!("QDigest(count={})", self.inner.count())
    }
}
