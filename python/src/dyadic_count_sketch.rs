//! Python bindings for the Dyadic Count Sketch (turnstile quantiles).
use pyo3::prelude::*;
use sketch_oxide::quantiles::DyadicCountSketch as RustDcs;

/// Dyadic Count Sketch — approximate quantiles over turnstile streams (insertions and deletions).
///
/// Operates on integer values in [0, 2^universe_bits).
#[pyclass(module = "sketch_oxide")]
pub struct DyadicCountSketch {
    inner: RustDcs,
}

#[pymethods]
impl DyadicCountSketch {
    #[new]
    fn new(universe_bits: u32, width: usize, depth: usize, seed: u64) -> PyResult<Self> {
        RustDcs::new(universe_bits, width, depth, seed)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    /// Applies a signed weight change to value `x` (turnstile update).
    fn update(&mut self, x: u64, delta: i64) -> PyResult<()> {
        self.inner
            .update(x, delta)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    /// Inserts one occurrence of `x`.
    fn add(&mut self, x: u64) {
        self.inner.add(x);
    }
    /// Estimated number of items strictly less than `x`.
    fn rank(&self, x: u64) -> f64 {
        self.inner.rank(x)
    }
    /// Approximate φ-quantile value.
    fn quantile(&self, phi: f64) -> u64 {
        self.inner.quantile(phi)
    }
    /// Estimated total weight.
    fn total(&self) -> f64 {
        self.inner.total()
    }
    /// Estimated point frequency of value `x`.
    fn count(&self, x: u64) -> f64 {
        self.inner.count(x)
    }
    fn __repr__(&self) -> String {
        format!("DyadicCountSketch(total={:.0})", self.inner.total())
    }
}
