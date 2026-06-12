//! Python bindings for the signed-update (lp) weighted sampler.

use pyo3::prelude::*;
use sketch_oxide::sampling::SignedUpdateSampler as RustSignedUpdateSampler;

/// SignedUpdateSampler — priority-sampling estimator over a dynamic stream of
/// signed real-valued updates to integer keys, for unbiased total / subset-sum
/// estimation.
///
/// Args:
///     k (int): sample size.
///     seed (int, optional): RNG seed for reproducibility.
#[pyclass(module = "sketch_oxide")]
pub struct SignedUpdateSampler {
    inner: RustSignedUpdateSampler,
}

#[pymethods]
impl SignedUpdateSampler {
    #[new]
    #[pyo3(signature = (k, seed=None))]
    fn new(k: usize, seed: Option<u64>) -> PyResult<Self> {
        let res = match seed {
            Some(s) => RustSignedUpdateSampler::with_seed(k, s),
            None => RustSignedUpdateSampler::new(k),
        };
        res.map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Applies a signed real-valued update of `delta` to integer `key`.
    fn update(&mut self, key: u64, delta: f64) {
        self.inner.update(key, delta);
    }

    /// Unbiased estimate of the total weight (sum over keys).
    fn estimate_total(&self) -> f64 {
        self.inner.estimate_total()
    }

    /// Maximum sample size.
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Number of keys currently in the sample.
    fn sample_size(&self) -> usize {
        self.inner.sample_size()
    }

    fn __repr__(&self) -> String {
        format!(
            "SignedUpdateSampler(sample_size={}, estimate_total={:.3})",
            self.inner.sample_size(),
            self.inner.estimate_total()
        )
    }
}
