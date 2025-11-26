//! Python bindings for T-Digest quantile estimation

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::quantiles::TDigest as RustTDigest;
use sketch_oxide::{Mergeable, Sketch};

/// T-Digest for quantile estimation (Dunning 2019)
///
/// Provides high accuracy at distribution tails (p99, p99.9).
/// Used by Netflix, Microsoft, Elasticsearch, and Prometheus.
///
/// Args:
///     compression (float): Controls accuracy/memory tradeoff (default: 100)
///         - 100: Good balance, ~500 centroids max
///         - 200: Higher accuracy, ~1000 centroids max
///
/// Example:
///     >>> td = TDigest(100.0)
///     >>> for latency in latencies:
///     ...     td.update(latency)
///     >>> p99 = td.quantile(0.99)
///     >>> p999 = td.quantile(0.999)
///
/// Notes:
///     - Best for extreme percentiles (p99+)
///     - Smaller memory than KLL for same accuracy at tails
#[pyclass(module = "sketch_oxide")]
pub struct TDigest {
    inner: RustTDigest,
}

#[pymethods]
impl TDigest {
    /// Create a new T-Digest
    ///
    /// Args:
    ///     compression: Controls accuracy/memory (default: 100.0)
    #[new]
    #[pyo3(signature = (compression=100.0))]
    fn new(compression: f64) -> Self {
        Self {
            inner: RustTDigest::new(compression),
        }
    }

    /// Update with a value
    ///
    /// Args:
    ///     value: Value to add
    fn update(&mut self, value: f64) {
        self.inner.update(value);
    }

    /// Update with a weighted value
    ///
    /// Args:
    ///     value: Value to add
    ///     weight: Weight of this value
    fn update_weighted(&mut self, value: f64, weight: f64) {
        self.inner.update_weighted(value, weight);
    }

    /// Update with multiple values
    ///
    /// Args:
    ///     values: List of values to add
    fn update_batch(&mut self, values: Vec<f64>) {
        self.inner.update_batch(&values);
    }

    /// Get the quantile value
    ///
    /// Args:
    ///     q: Quantile (0.0 to 1.0)
    ///
    /// Returns:
    ///     float: Estimated value at that quantile
    fn quantile(&mut self, q: f64) -> f64 {
        self.inner.quantile(q)
    }

    /// Get the CDF (cumulative distribution) value
    ///
    /// Args:
    ///     value: Value to find CDF for
    ///
    /// Returns:
    ///     float: Fraction of values <= given value
    fn cdf(&mut self, value: f64) -> f64 {
        self.inner.cdf(value)
    }

    /// Get the trimmed mean between two quantiles
    ///
    /// Args:
    ///     low: Lower quantile bound
    ///     high: Upper quantile bound
    ///
    /// Returns:
    ///     float: Mean of values between the quantiles
    fn trimmed_mean(&mut self, low: f64, high: f64) -> f64 {
        self.inner.trimmed_mean(low, high)
    }

    /// Merge another T-Digest into this one
    ///
    /// Args:
    ///     other: Another T-Digest
    fn merge(&mut self, other: &TDigest) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Get the compression parameter
    fn compression(&self) -> f64 {
        self.inner.compression()
    }

    /// Get the number of centroids
    fn centroid_count(&self) -> usize {
        self.inner.centroid_count()
    }

    /// Get the total count
    fn count(&self) -> f64 {
        self.inner.count()
    }

    /// Get the minimum value
    fn min(&self) -> f64 {
        self.inner.min()
    }

    /// Get the maximum value
    fn max(&self) -> f64 {
        self.inner.max()
    }

    /// Check if empty
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Serialize to bytes
    fn serialize<'py>(&mut self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new_bound(py, &self.inner.to_bytes())
    }

    /// Deserialize from bytes
    #[staticmethod]
    fn from_bytes(bytes: &[u8]) -> PyResult<Self> {
        RustTDigest::from_bytes(bytes)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "TDigest(compression={}, count={})",
            self.inner.compression(),
            self.inner.count()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "TDigest: compression={}, {} values, {} centroids",
            self.inner.compression(),
            self.inner.count(),
            self.inner.centroid_count()
        )
    }
}
