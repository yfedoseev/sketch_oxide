//! Python bindings for the statistical moments sketch (descriptive statistics).

use pyo3::prelude::*;
use sketch_oxide::statistics::MomentsSketch as RustMomentsStats;

/// MomentsStatistics — single-pass descriptive statistics (mean, variance,
/// skewness, kurtosis, min, max) computed online from streaming values via
/// numerically-stable moment accumulation.
///
/// (This is the descriptive-statistics moments sketch; for moments-based quantile
/// estimation see :class:`MomentsSketch`.)
#[pyclass(module = "sketch_oxide")]
pub struct MomentsStatistics {
    inner: RustMomentsStats,
}

#[pymethods]
impl MomentsStatistics {
    #[new]
    fn new() -> Self {
        Self {
            inner: RustMomentsStats::new(),
        }
    }

    /// Adds a value.
    fn update(&mut self, x: f64) {
        self.inner.update(x);
    }

    /// Number of values observed.
    fn count(&self) -> u64 {
        self.inner.count()
    }

    /// Whether no values have been observed.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Mean of observed values.
    fn mean(&self) -> f64 {
        self.inner.mean()
    }

    /// Variance of observed values.
    fn variance(&self) -> f64 {
        self.inner.variance()
    }

    /// Standard deviation of observed values.
    fn std_dev(&self) -> f64 {
        self.inner.std_dev()
    }

    /// Skewness of observed values.
    fn skewness(&self) -> f64 {
        self.inner.skewness()
    }

    /// Excess kurtosis of observed values.
    fn kurtosis(&self) -> f64 {
        self.inner.kurtosis()
    }

    /// Minimum observed value, or None if empty.
    fn min(&self) -> Option<f64> {
        self.inner.min()
    }

    /// Maximum observed value, or None if empty.
    fn max(&self) -> Option<f64> {
        self.inner.max()
    }

    fn __repr__(&self) -> String {
        format!(
            "MomentsStatistics(count={}, mean={:.3})",
            self.inner.count(),
            self.inner.mean()
        )
    }
}
