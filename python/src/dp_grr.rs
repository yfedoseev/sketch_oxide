//! Python bindings for the Generalized Randomized Response local-DP oracle.

use pyo3::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;
use sketch_oxide::privacy::GrrFrequencyOracle as RustGrrFrequencyOracle;

/// GrrFrequencyOracle — Generalized Randomized Response: each client reports its
/// true value with a tuned probability and a uniformly-random other value
/// otherwise, giving ε-local-DP frequency estimation over a `domain`-sized
/// categorical domain. Uses an OS-seeded CSPRNG.
///
/// Args:
///     domain (int): number of distinct categories (values are `0 .. domain-1`).
///     epsilon (float): local-DP privacy parameter (> 0).
#[pyclass(module = "sketch_oxide")]
pub struct GrrFrequencyOracle {
    inner: RustGrrFrequencyOracle,
    rng: StdRng,
}

#[pymethods]
impl GrrFrequencyOracle {
    #[new]
    fn new(domain: usize, epsilon: f64) -> PyResult<Self> {
        RustGrrFrequencyOracle::new(domain, epsilon)
            .map(|inner| Self {
                inner,
                rng: StdRng::from_os_rng(),
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Client side: privatizes `value` into a reported category.
    fn privatize(&mut self, value: usize) -> usize {
        self.inner.privatize(value, &mut self.rng)
    }

    /// Aggregator side: incorporates a previously-reported category.
    fn observe(&mut self, reported: usize) {
        self.inner.observe(reported);
    }

    /// Convenience: privatizes `true_value` and incorporates it in one step.
    fn submit(&mut self, true_value: usize) {
        self.inner.submit(true_value, &mut self.rng);
    }

    /// Estimated (debiased) count of `value`.
    fn estimate(&self, value: usize) -> f64 {
        self.inner.estimate(value)
    }

    /// Estimated relative frequency of `value`.
    fn frequency(&self, value: usize) -> f64 {
        self.inner.frequency(value)
    }

    /// Total number of reports observed.
    fn total(&self) -> u64 {
        self.inner.total()
    }

    /// Local-DP privacy parameter.
    fn epsilon(&self) -> f64 {
        self.inner.epsilon()
    }

    /// Domain size.
    fn domain(&self) -> usize {
        self.inner.domain()
    }

    fn __repr__(&self) -> String {
        format!(
            "GrrFrequencyOracle(domain={}, total={})",
            self.inner.domain(),
            self.inner.total()
        )
    }
}
