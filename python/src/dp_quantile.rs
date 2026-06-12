//! Python bindings for the differentially-private quantile (exponential mechanism).

use pyo3::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;
use sketch_oxide::privacy::DpQuantile as RustDpQuantile;

/// DpQuantile — releases a quantile of a dataset under ε-differential privacy via
/// the exponential mechanism over the bounded range `[lo, hi]`. Uses an OS-seeded
/// CSPRNG.
///
/// Args:
///     epsilon (float): privacy parameter (> 0).
///     lo (float): lower bound of the value range.
///     hi (float): upper bound of the value range.
#[pyclass(module = "sketch_oxide")]
pub struct DpQuantile {
    inner: RustDpQuantile,
    rng: StdRng,
}

#[pymethods]
impl DpQuantile {
    #[new]
    fn new(epsilon: f64, lo: f64, hi: f64) -> PyResult<Self> {
        RustDpQuantile::new(epsilon, lo, hi)
            .map(|inner| Self {
                inner,
                rng: StdRng::from_os_rng(),
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Differentially-private estimate of the `phi`-quantile of `data` (phi in [0, 1]).
    fn quantile(&mut self, data: Vec<f64>, phi: f64) -> f64 {
        self.inner.quantile(&data, phi, &mut self.rng)
    }

    /// Privacy parameter.
    fn epsilon(&self) -> f64 {
        self.inner.epsilon()
    }

    fn __repr__(&self) -> String {
        format!("DpQuantile(epsilon={})", self.inner.epsilon())
    }
}
