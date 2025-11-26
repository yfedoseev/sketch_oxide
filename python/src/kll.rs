//! Python bindings for KLL Sketch quantile estimation

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList};
use sketch_oxide::quantiles::KllSketch as RustKllSketch;
use sketch_oxide::{Mergeable, Sketch};

/// KLL Sketch for quantile estimation (Karnin 2016)
///
/// Standard quantile algorithm in the Apache ecosystem (Druid, Spark, Flink).
/// Provides absolute error guarantees with near-optimal space usage.
///
/// Args:
///     k (int): Accuracy parameter (default: 200)
///         - Higher k = more accurate, more memory
///         - k=200 gives ~1.65% normalized rank error
///         - k=100 gives ~3.3% normalized rank error
///
/// Example:
///     >>> kll = KllSketch(200)
///     >>> for value in data_stream:
///     ...     kll.update(value)
///     >>> median = kll.quantile(0.5)
///     >>> p99 = kll.quantile(0.99)
///
/// Notes:
///     - Absolute error (±ε for all ranks)
///     - Compatible with Apache DataSketches
#[pyclass(module = "sketch_oxide")]
pub struct KllSketch {
    inner: RustKllSketch,
}

#[pymethods]
impl KllSketch {
    /// Create a new KLL Sketch
    ///
    /// Args:
    ///     k: Accuracy parameter (8-65535, default: 200)
    ///
    /// Raises:
    ///     ValueError: If k is too small
    #[new]
    #[pyo3(signature = (k=200))]
    fn new(k: u16) -> PyResult<Self> {
        RustKllSketch::new(k)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Update with a value
    ///
    /// Args:
    ///     value: Value to add (NaN/infinity ignored)
    fn update(&mut self, value: f64) {
        self.inner.update(value);
    }

    /// Get the quantile value
    ///
    /// Args:
    ///     rank: Quantile rank (0.0 to 1.0)
    ///
    /// Returns:
    ///     float or None: Estimated value at that quantile
    fn quantile(&mut self, rank: f64) -> Option<f64> {
        self.inner.quantile(rank)
    }

    /// Get the rank of a value
    ///
    /// Args:
    ///     value: Value to find rank for
    ///
    /// Returns:
    ///     float: Fraction of values <= given value (0.0 to 1.0)
    fn rank(&mut self, value: f64) -> f64 {
        self.inner.rank(value)
    }

    /// Get the CDF (cumulative distribution function)
    ///
    /// Returns:
    ///     list: List of (value, cumulative_rank) tuples
    fn cdf(&mut self) -> Vec<(f64, f64)> {
        self.inner.cdf()
    }

    /// Merge another KLL Sketch into this one
    ///
    /// Args:
    ///     other: Another KLL Sketch with same k
    ///
    /// Raises:
    ///     ValueError: If k values don't match
    fn merge(&mut self, other: &KllSketch) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Get the k parameter
    fn k(&self) -> u16 {
        self.inner.k()
    }

    /// Get the number of items seen
    fn count(&self) -> u64 {
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

    /// Get the normalized rank error bound
    fn normalized_rank_error(&self) -> f64 {
        self.inner.normalized_rank_error()
    }

    /// Get the number of retained items
    fn num_retained(&self) -> usize {
        self.inner.num_retained()
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
        RustKllSketch::from_bytes(bytes)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "KllSketch(k={}, count={})",
            self.inner.k(),
            self.inner.count()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "KllSketch: k={}, {} items, {} retained",
            self.inner.k(),
            self.inner.count(),
            self.inner.num_retained()
        )
    }

    /// Update the sketch with multiple values in a single call (optimized for throughput).
    ///
    /// Batch updates are significantly faster than multiple individual update() calls
    /// because they amortize the FFI (Foreign Function Interface) overhead across
    /// many items. This is the preferred method when adding large quantities of data.
    ///
    /// Args:
    ///     values: Iterable of numeric values to add
    fn update_batch(&mut self, values: &Bound<'_, PyAny>) -> PyResult<()> {
        let values_list: &Bound<'_, PyList> = values.downcast()?;
        for value in values_list {
            let val: f64 = value.extract()?;
            self.update(val);
        }
        Ok(())
    }

    /// Get quantiles for multiple ranks in a single call (optimized for lookups).
    ///
    /// Batch quantile queries are faster than multiple individual quantile() calls.
    ///
    /// Args:
    ///     ranks: Iterable of quantile ranks (0.0 to 1.0)
    ///
    /// Returns:
    ///     list: List of estimated quantile values (or None if sketch is empty)
    fn quantile_batch(&mut self, ranks: &Bound<'_, PyAny>) -> PyResult<Vec<Option<f64>>> {
        let ranks_list: &Bound<'_, PyList> = ranks.downcast()?;
        let mut results = Vec::new();
        for rank in ranks_list {
            let r: f64 = rank.extract()?;
            results.push(self.quantile(r));
        }
        Ok(results)
    }

    /// Get ranks for multiple values in a single call (optimized for lookups).
    ///
    /// Args:
    ///     values: Iterable of values to find ranks for
    ///
    /// Returns:
    ///     list: List of ranks (0.0 to 1.0) for each value
    fn rank_batch(&mut self, values: &Bound<'_, PyAny>) -> PyResult<Vec<f64>> {
        let values_list: &Bound<'_, PyList> = values.downcast()?;
        let mut results = Vec::new();
        for value in values_list {
            let val: f64 = value.extract()?;
            results.push(self.rank(val));
        }
        Ok(results)
    }
}
