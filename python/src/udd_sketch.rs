//! Python bindings for the UDDSketch quantile estimator.
use pyo3::prelude::*;
use sketch_oxide::quantiles::UddSketch as RustUdd;

/// UDDSketch — uniform DDSketch with guaranteed relative error even after compaction (2020).
#[pyclass(module = "sketch_oxide")]
pub struct UddSketch {
    inner: RustUdd,
}

#[pymethods]
impl UddSketch {
    #[new]
    fn new(alpha: f64, max_buckets: usize) -> PyResult<Self> {
        RustUdd::new(alpha, max_buckets)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn update(&mut self, value: f64) {
        self.inner.add(value);
    }
    fn quantile(&self, q: f64) -> Option<f64> {
        self.inner.quantile(q)
    }
    fn count(&self) -> u64 {
        self.inner.count()
    }
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    fn __repr__(&self) -> String {
        format!("UddSketch(count={})", self.inner.count())
    }
}
