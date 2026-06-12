//! Python bindings for the Optimized Local Hashing local-DP oracle.

use pyo3::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;
use sketch_oxide::privacy::OlhFrequencyOracle as RustOlhFrequencyOracle;

/// OlhFrequencyOracle — Optimized Local Hashing (Wang et al. 2017): each client
/// hashes its value into a small range and applies randomized response to the
/// hashed bucket, giving ε-local-DP frequency estimation that scales better than
/// GRR for large domains. Uses an OS-seeded CSPRNG.
///
/// Args:
///     domain (int): number of distinct categories (values are `0 .. domain-1`).
///     epsilon (float): local-DP privacy parameter (> 0).
#[pyclass(module = "sketch_oxide")]
pub struct OlhFrequencyOracle {
    inner: RustOlhFrequencyOracle,
    rng: StdRng,
}

#[pymethods]
impl OlhFrequencyOracle {
    #[new]
    fn new(domain: usize, epsilon: f64) -> PyResult<Self> {
        RustOlhFrequencyOracle::new(domain, epsilon)
            .map(|inner| Self {
                inner,
                rng: StdRng::from_os_rng(),
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Client side: privatizes `value` into a report `(hash_seed, hashed_bucket)`.
    fn privatize(&mut self, value: usize) -> (u64, u32) {
        self.inner.privatize(value, &mut self.rng)
    }

    /// Aggregator side: incorporates a previously-produced report.
    fn observe(&mut self, report: (u64, u32)) {
        self.inner.observe(report);
    }

    /// Convenience: privatizes `true_value` and incorporates it in one step.
    fn submit(&mut self, true_value: usize) {
        self.inner.submit(true_value, &mut self.rng);
    }

    /// Raw support count (number of reports hashing to `value`).
    fn support(&self, value: usize) -> u64 {
        self.inner.support(value)
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
            "OlhFrequencyOracle(domain={}, total={})",
            self.inner.domain(),
            self.inner.total()
        )
    }
}
