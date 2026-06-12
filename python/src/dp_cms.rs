//! Python bindings for the Count-Mean-Sketch local-DP frequency oracle.

use pyo3::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;
use sketch_oxide::privacy::CountMeanSketch as RustCountMeanSketch;

/// CountMeanSketch — the local-differential-privacy frequency oracle behind
/// Apple's private telemetry: each client privatizes a value into a noisy
/// `k × m` report, and the aggregator estimates value frequencies. Uses an
/// OS-seeded CSPRNG for the randomized response.
///
/// Args:
///     k (int): number of hash functions (rows).
///     m (int): sketch width per row.
///     epsilon (float): local-DP privacy parameter (> 0).
#[pyclass(module = "sketch_oxide")]
pub struct CountMeanSketch {
    inner: RustCountMeanSketch,
    rng: StdRng,
}

#[pymethods]
impl CountMeanSketch {
    #[new]
    fn new(k: usize, m: usize, epsilon: f64) -> PyResult<Self> {
        RustCountMeanSketch::new(k, m, epsilon)
            .map(|inner| Self {
                inner,
                rng: StdRng::from_os_rng(),
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Client side: privatizes `value` into a noisy report `(vector, row_index)`.
    fn privatize(&mut self, value: usize) -> (Vec<i8>, usize) {
        self.inner.privatize(value, &mut self.rng)
    }

    /// Aggregator side: incorporates a previously-produced report.
    fn observe(&mut self, report: (Vec<i8>, usize)) {
        self.inner.observe(report);
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

    /// Number of rows.
    fn rows(&self) -> usize {
        self.inner.rows()
    }

    /// Sketch width per row.
    fn width(&self) -> usize {
        self.inner.width()
    }

    /// Local-DP privacy parameter.
    fn epsilon(&self) -> f64 {
        self.inner.epsilon()
    }

    fn __repr__(&self) -> String {
        format!(
            "CountMeanSketch(rows={}, width={}, total={})",
            self.inner.rows(),
            self.inner.width(),
            self.inner.total()
        )
    }
}
