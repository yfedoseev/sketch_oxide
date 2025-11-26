//! Python bindings for REQ Sketch quantile estimation

use numpy::{PyArray1, PyReadonlyArray1};
use pyo3::prelude::*;
use sketch_oxide::quantiles::req::{ReqMode, ReqSketch as RustReqSketch};

/// REQ (Relative Error Quantile) Sketch for tail quantiles (PODS 2021)
///
/// Production implementation used in Google BigQuery and Apache DataSketches.
/// **Key feature: Zero error at p100 (HRA mode) or p0 (LRA mode).**
///
/// Args:
///     k (int): Size parameter (4-1024). Higher = more accurate but more memory.
///         - k=32: Good for most use cases
///         - k=64: High accuracy
///         - k=128: Very high accuracy
///     mode (str): Operating mode, either "HRA" or "LRA"
///         - "HRA": High Rank Accuracy - zero error at p100 (tail quantiles)
///         - "LRA": Low Rank Accuracy - zero error at p0 (low quantiles)
///
/// Example:
///     >>> req = ReqSketch(k=32, mode="HRA")  # Optimized for tail quantiles
///     >>> for latency in latencies:
///     ...     req.update(latency)
///     >>> p100 = req.quantile(1.0)  # Exact maximum
///     >>> p99 = req.quantile(0.99)   # Accurate tail quantile
///
/// Notes:
///     - HRA mode: Perfect for p90, p99, p99.9 (tail latencies)
///     - LRA mode: Perfect for p1, p0.1 (low percentiles)
///     - Space: O(k log(n/k))
#[pyclass(module = "sketch_oxide")]
pub struct ReqSketch {
    inner: RustReqSketch,
}

#[pymethods]
impl ReqSketch {
    /// Create a new REQ Sketch
    ///
    /// Args:
    ///     k: Size parameter (4-1024)
    ///     mode: Operating mode ("HRA" or "LRA")
    ///
    /// Raises:
    ///     ValueError: If k is not in range [4, 1024] or mode is invalid
    #[new]
    fn new(k: usize, mode: &str) -> PyResult<Self> {
        let req_mode = match mode.to_uppercase().as_str() {
            "HRA" => ReqMode::HighRankAccuracy,
            "LRA" => ReqMode::LowRankAccuracy,
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "Mode must be 'HRA' or 'LRA'",
                ))
            }
        };

        RustReqSketch::new(k, req_mode)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Update the sketch with a single value
    ///
    /// Args:
    ///     value: Numeric value to add
    fn update(&mut self, value: f64) {
        self.inner.update(value);
    }

    /// Update the sketch with multiple values (NumPy support)
    ///
    /// Args:
    ///     values: NumPy array of values
    fn update_batch(&mut self, values: PyReadonlyArray1<f64>) -> PyResult<()> {
        for &val in values.as_slice()? {
            self.inner.update(val);
        }
        Ok(())
    }

    /// Get a quantile estimate
    ///
    /// Args:
    ///     q: Quantile to estimate (0.0 to 1.0)
    ///
    /// Returns:
    ///     float: Estimated quantile value, or None if sketch is empty
    ///
    /// Example:
    ///     >>> req = ReqSketch(k=32, mode="HRA")
    ///     >>> # ... add data ...
    ///     >>> p99 = req.quantile(0.99)
    ///     >>> p100 = req.quantile(1.0)  # Exact in HRA mode
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

    /// Get the total count of values added
    ///
    /// Returns:
    ///     int: Number of values added
    fn count(&self) -> u64 {
        self.inner.n()
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
            "ReqSketch(count={}, min={:.2}, max={:.2})",
            self.inner.n(),
            self.inner.min().unwrap_or(0.0),
            self.inner.max().unwrap_or(0.0)
        )
    }

    fn __str__(&self) -> String {
        format!("ReqSketch({} values)", self.inner.n())
    }

    fn __len__(&self) -> usize {
        self.inner.n() as usize
    }
}
