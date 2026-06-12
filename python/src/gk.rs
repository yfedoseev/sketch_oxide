//! Python bindings for the Greenwald-Khanna quantile sketch.
use pyo3::prelude::*;
use sketch_oxide::quantiles::GreenwaldKhanna as RustGk;

/// Greenwald-Khanna — deterministic ε-approximate quantiles with no failure probability (2001).
#[pyclass(module = "sketch_oxide")]
pub struct GreenwaldKhanna {
    inner: RustGk,
}

#[pymethods]
impl GreenwaldKhanna {
    #[new]
    fn new(epsilon: f64) -> PyResult<Self> {
        RustGk::new(epsilon)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    /// Adds a numeric value.
    fn update(&mut self, value: f64) {
        self.inner.insert(value);
    }
    /// Approximate φ-quantile (φ in [0, 1]).
    fn quantile(&self, phi: f64) -> Option<f64> {
        self.inner.quantile(phi)
    }
    fn min(&self) -> Option<f64> {
        self.inner.min()
    }
    fn max(&self) -> Option<f64> {
        self.inner.max()
    }
    fn count(&self) -> usize {
        self.inner.count()
    }
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    fn __repr__(&self) -> String {
        format!("GreenwaldKhanna(count={})", self.inner.count())
    }
}
