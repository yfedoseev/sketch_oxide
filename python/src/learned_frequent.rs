//! Python bindings for the Learned Frequent (heavy-hitter) sketch.

use crate::common::python_item_to_bytes;
use crate::precomputed_oracle::PrecomputedOracle;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::learned::LearnedFrequent as RustLearnedFrequent;
use sketch_oxide::learned::PrecomputedOracle as RustPrecomputedOracle;

/// LearnedFrequent — a learned heavy-hitter detector: keys the oracle predicts to
/// be frequent (score ≥ `threshold`) are tracked exactly, while a bounded
/// `tail_capacity` summary covers the rest, improving recall on skewed streams.
///
/// Args:
///     oracle (PrecomputedOracle): score oracle (copied at construction).
///     threshold (float): oracle score above which a key is tracked exactly.
///     tail_capacity (int): capacity of the tail (non-heavy) summary.
#[pyclass(module = "sketch_oxide")]
pub struct LearnedFrequent {
    inner: RustLearnedFrequent<RustPrecomputedOracle>,
}

#[pymethods]
impl LearnedFrequent {
    #[new]
    fn new(oracle: &PrecomputedOracle, threshold: f64, tail_capacity: usize) -> PyResult<Self> {
        RustLearnedFrequent::new(oracle.inner.clone(), threshold, tail_capacity)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Records one occurrence of `key` (int, str, bytes, or float).
    fn update(&mut self, key: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.update(&python_item_to_bytes(key)?);
        Ok(())
    }

    /// Estimated frequency of `key`.
    fn estimate(&self, key: &Bound<'_, PyAny>) -> PyResult<u64> {
        Ok(self.inner.estimate(&python_item_to_bytes(key)?))
    }

    /// Heavy hitters as a list of `(key_bytes, estimated_count)` pairs.
    fn heavy_hitters(&self, py: Python<'_>) -> Vec<(Py<PyBytes>, u64)> {
        self.inner
            .heavy_hitters()
            .into_iter()
            .map(|(k, c)| (PyBytes::new_bound(py, &k).unbind(), c))
            .collect()
    }

    /// Number of keys tracked as heavy.
    fn num_heavy(&self) -> usize {
        self.inner.num_heavy()
    }

    /// Total number of updates processed.
    fn total_updates(&self) -> u64 {
        self.inner.total_updates()
    }

    fn __repr__(&self) -> String {
        format!(
            "LearnedFrequent(num_heavy={}, total_updates={})",
            self.inner.num_heavy(),
            self.inner.total_updates()
        )
    }
}
