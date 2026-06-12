//! Python bindings for the DensitySketch (streaming kernel density estimate).

use pyo3::prelude::*;
use sketch_oxide::statistics::DensitySketch as RustDensitySketch;

/// DensitySketch — a bounded-memory streaming kernel-density estimator: it keeps
/// a weighted sample of up to `capacity` points and answers density queries with
/// a Gaussian kernel of the given `bandwidth`.
///
/// Args:
///     capacity (int): maximum retained sample points.
///     bandwidth (float): Gaussian kernel bandwidth.
///     seed (int, optional): RNG seed for reproducibility.
#[pyclass(module = "sketch_oxide")]
pub struct DensitySketch {
    inner: RustDensitySketch,
}

#[pymethods]
impl DensitySketch {
    #[new]
    #[pyo3(signature = (capacity, bandwidth, seed=None))]
    fn new(capacity: usize, bandwidth: f64, seed: Option<u64>) -> PyResult<Self> {
        let res = match seed {
            Some(s) => RustDensitySketch::with_seed(capacity, bandwidth, s),
            None => RustDensitySketch::new(capacity, bandwidth),
        };
        res.map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Adds a sample value.
    fn update(&mut self, x: f64) {
        self.inner.update(x);
    }

    /// Estimated probability density at `x`.
    fn density(&self, x: f64) -> f64 {
        self.inner.density(x)
    }

    /// Silverman's rule-of-thumb bandwidth from the retained sample, if available.
    fn silverman_bandwidth(&self) -> Option<f64> {
        self.inner.silverman_bandwidth()
    }

    /// Number of retained sample points.
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the sketch is empty.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Total number of values observed.
    fn count(&self) -> u64 {
        self.inner.count()
    }

    /// Configured kernel bandwidth.
    fn bandwidth(&self) -> f64 {
        self.inner.bandwidth()
    }

    fn __repr__(&self) -> String {
        format!(
            "DensitySketch(len={}, count={})",
            self.inner.len(),
            self.inner.count()
        )
    }
}
