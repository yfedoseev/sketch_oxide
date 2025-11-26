//! Python bindings for DDSketch quantile estimation

use numpy::{PyArray1, PyReadonlyArray1};
use pyo3::prelude::*;
use sketch_oxide::quantiles::DDSketch as RustDDSketch;
use sketch_oxide::{Mergeable, Sketch};

/// DDSketch for quantile estimation with relative error guarantees (VLDB 2019)
///
/// Production-proven by Datadog, ClickHouse 24.1+, TimescaleDB.
/// Provides relative error guarantees: |estimate - true| <= alpha * true_value.
///
/// Args:
///     relative_accuracy (float): Relative error bound (e.g., 0.01 for 1% error).
///         - 0.001: 0.1% error, more memory
///         - 0.01: 1% error (recommended)
///         - 0.05: 5% error, less memory
///
/// Example:
///     >>> dd = DDSketch(relative_accuracy=0.01)  # 1% error
///     >>> for latency in latencies:
///     ...     dd.update(latency)
///     >>> print(f"p50: {dd.quantile(0.5):.2f}ms")
///     >>> print(f"p99: {dd.quantile(0.99):.2f}ms")
///
/// Notes:
///     - Handles positive, negative, and zero values
///     - Fully mergeable with other DDSketches
///     - Space: O(log(max/min))
///     - Fast: O(1) updates, O(k) quantile queries
#[pyclass(module = "sketch_oxide")]
pub struct DDSketch {
    inner: RustDDSketch,
}

#[pymethods]
impl DDSketch {
    /// Create a new DDSketch with specified relative accuracy
    ///
    /// Args:
    ///     relative_accuracy: Relative error bound (0 < alpha < 1)
    ///
    /// Raises:
    ///     ValueError: If relative_accuracy is not in (0, 1)
    #[new]
    fn new(relative_accuracy: f64) -> PyResult<Self> {
        RustDDSketch::new(relative_accuracy)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Update the sketch with a single value
    ///
    /// Args:
    ///     value: Numeric value to add (int or float)
    ///
    /// Example:
    ///     >>> dd = DDSketch(relative_accuracy=0.01)
    ///     >>> dd.update(42.5)
    ///     >>> dd.update(100)
    fn update(&mut self, value: f64) {
        self.inner.update(&value);
    }

    /// Update the sketch with multiple values (NumPy support)
    ///
    /// Args:
    ///     values: NumPy array of values
    ///
    /// Example:
    ///     >>> import numpy as np
    ///     >>> dd = DDSketch(relative_accuracy=0.01)
    ///     >>> values = np.random.exponential(scale=100, size=10000)
    ///     >>> dd.update_batch(values)
    fn update_batch(&mut self, values: PyReadonlyArray1<f64>) -> PyResult<()> {
        for &val in values.as_slice()? {
            self.inner.update(&val);
        }
        Ok(())
    }

    /// Get a quantile estimate
    ///
    /// Args:
    ///     q: Quantile to estimate (0.0 to 1.0)
    ///         - 0.5: median
    ///         - 0.95: 95th percentile
    ///         - 0.99: 99th percentile
    ///
    /// Returns:
    ///     float: Estimated quantile value, or None if sketch is empty
    ///
    /// Example:
    ///     >>> dd = DDSketch(relative_accuracy=0.01)
    ///     >>> for i in range(1, 1001):
    ///     ...     dd.update(i)
    ///     >>> p50 = dd.quantile(0.5)
    ///     >>> print(f"Median: {p50}")
    fn quantile(&self, q: f64) -> Option<f64> {
        self.inner.quantile(q)
    }

    /// Get multiple quantiles at once (NumPy support)
    ///
    /// Args:
    ///     quantiles: NumPy array of quantiles to estimate
    ///
    /// Returns:
    ///     NumPy array of quantile estimates
    ///
    /// Example:
    ///     >>> import numpy as np
    ///     >>> dd = DDSketch(relative_accuracy=0.01)
    ///     >>> # ... add data ...
    ///     >>> qs = np.array([0.5, 0.95, 0.99, 0.999])
    ///     >>> values = dd.quantiles(qs)
    fn quantiles<'py>(
        &self,
        py: Python<'py>,
        quantiles: PyReadonlyArray1<f64>,
    ) -> PyResult<Bound<'py, PyArray1<f64>>> {
        let results: Vec<f64> = quantiles
            .as_slice()?
            .iter()
            .filter_map(|&q| self.inner.quantile(q))
            .collect();

        Ok(PyArray1::from_vec_bound(py, results))
    }

    /// Merge another DDSketch into this one
    ///
    /// Args:
    ///     other: Another DDSketch with the same relative accuracy
    ///
    /// Raises:
    ///     ValueError: If sketches have different relative accuracies
    ///
    /// Example:
    ///     >>> dd1 = DDSketch(relative_accuracy=0.01)
    ///     >>> dd2 = DDSketch(relative_accuracy=0.01)
    ///     >>> # ... add data to both ...
    ///     >>> dd1.merge(dd2)  # Combine distributions
    fn merge(&mut self, other: &DDSketch) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Get the total count of values added
    ///
    /// Returns:
    ///     int: Number of values added
    fn count(&self) -> u64 {
        self.inner.count()
    }

    /// Get the minimum value seen
    ///
    /// Returns:
    ///     float: Minimum value, or None if empty
    fn min(&self) -> Option<f64> {
        self.inner.min()
    }

    /// Get the maximum value seen
    ///
    /// Returns:
    ///     float: Maximum value, or None if empty
    fn max(&self) -> Option<f64> {
        self.inner.max()
    }

    /// Check if the sketch is empty
    ///
    /// Returns:
    ///     bool: True if no values have been added
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn __repr__(&self) -> String {
        format!(
            "DDSketch(count={}, min={:.2}, max={:.2})",
            self.inner.count(),
            self.inner.min().unwrap_or(0.0),
            self.inner.max().unwrap_or(0.0)
        )
    }

    fn __str__(&self) -> String {
        format!("DDSketch({} values)", self.inner.count())
    }

    fn __len__(&self) -> usize {
        self.inner.count() as usize
    }
}
