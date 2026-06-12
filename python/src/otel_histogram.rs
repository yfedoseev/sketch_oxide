//! Python bindings for the OpenTelemetry exponential histogram.
use pyo3::prelude::*;
use sketch_oxide::quantiles::OtelExponentialHistogram as RustOtel;

/// OTel Exponential Histogram — the OpenTelemetry base-2 exponential bucket histogram.
#[pyclass(module = "sketch_oxide")]
pub struct OtelExponentialHistogram {
    inner: RustOtel,
}

#[pymethods]
impl OtelExponentialHistogram {
    #[new]
    fn new(scale: i32, max_buckets: usize) -> PyResult<Self> {
        RustOtel::new(scale, max_buckets)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn update(&mut self, value: f64) {
        self.inner.record(value);
    }
    fn quantile(&self, q: f64) -> Option<f64> {
        self.inner.quantile(q)
    }
    fn count(&self) -> u64 {
        self.inner.count()
    }
    fn min(&self) -> Option<f64> {
        self.inner.min()
    }
    fn max(&self) -> Option<f64> {
        self.inner.max()
    }
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    fn __repr__(&self) -> String {
        format!("OtelExponentialHistogram(count={})", self.inner.count())
    }
}
