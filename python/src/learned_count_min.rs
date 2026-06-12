//! Python bindings for the Learned Count-Min sketch.

use crate::common::python_item_to_bytes;
use crate::precomputed_oracle::PrecomputedOracle;
use pyo3::prelude::*;
use sketch_oxide::learned::LearnedCountMin as RustLearnedCountMin;

/// LearnedCountMin — a Count-Min sketch augmented with a learned oracle: items the
/// oracle predicts to be heavy (score ≥ `threshold`) are tracked in an exact
/// heavy-hitter table, while the rest fall back to the Count-Min counters,
/// reducing error on skewed streams.
///
/// Args:
///     depth (int): Count-Min hash rows.
///     width (int): Count-Min counters per row.
///     threshold (float): oracle score above which an item is treated as heavy.
///     heavy_capacity (int): capacity of the exact heavy-hitter table.
#[pyclass(module = "sketch_oxide")]
pub struct LearnedCountMin {
    inner: RustLearnedCountMin,
}

#[pymethods]
impl LearnedCountMin {
    #[new]
    fn new(depth: usize, width: usize, threshold: f64, heavy_capacity: usize) -> PyResult<Self> {
        RustLearnedCountMin::new(depth, width, threshold, heavy_capacity)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Adds `count` occurrences of `item`, routed by the oracle's predicted score.
    fn update(
        &mut self,
        item: &Bound<'_, PyAny>,
        count: u64,
        oracle: &PrecomputedOracle,
    ) -> PyResult<()> {
        self.inner
            .update(&python_item_to_bytes(item)?, count, &oracle.inner);
        Ok(())
    }

    /// Estimated frequency of `item`.
    fn estimate(&self, item: &Bound<'_, PyAny>) -> PyResult<u64> {
        Ok(self.inner.estimate(&python_item_to_bytes(item)?))
    }

    /// Number of items currently held in the exact heavy-hitter table.
    fn heavy_len(&self) -> usize {
        self.inner.heavy_len()
    }

    fn __repr__(&self) -> String {
        format!("LearnedCountMin(heavy_len={})", self.inner.heavy_len())
    }
}
