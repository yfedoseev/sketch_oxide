//! Python bindings for the Moments Sketch (max-entropy quantiles).
use pyo3::prelude::*;
use sketch_oxide::quantiles::MomentsSketch as RustMoments;

/// Moments Sketch — mergeable quantiles from power moments via maximum entropy (SIGMOD 2018).
#[pyclass(module = "sketch_oxide")]
pub struct MomentsSketch {
    inner: RustMoments,
}

#[pymethods]
impl MomentsSketch {
    #[new]
    fn new(k: usize) -> PyResult<Self> {
        RustMoments::new(k)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn update(&mut self, value: f64) {
        self.inner.add(value);
    }
    fn quantile(&self, phi: f64) -> Option<f64> {
        self.inner.quantile(phi)
    }
    fn merge(&mut self, other: &MomentsSketch) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
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
        format!("MomentsSketch(count={})", self.inner.count())
    }
}
