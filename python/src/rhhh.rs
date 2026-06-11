//! Python bindings for RHHH (Randomized Hierarchical Heavy Hitters).
use crate::common::python_item_to_hash;
use pyo3::prelude::*;
use sketch_oxide::frequency::Rhhh as RustRhhh;

/// RHHH — randomized hierarchical heavy hitters with O(1) per-update cost.
#[pyclass(module = "sketch_oxide")]
pub struct Rhhh {
    inner: RustRhhh,
}

#[pymethods]
impl Rhhh {
    #[new]
    fn new(num_levels: usize, bits_per_level: u32, capacity: usize) -> PyResult<Self> {
        RustRhhh::new(num_levels, bits_per_level, capacity)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.update(python_item_to_hash(item)?);
        Ok(())
    }
    /// Estimated frequency of `item` at hierarchy `level`.
    fn estimate(&self, item: &Bound<'_, PyAny>, level: usize) -> PyResult<f64> {
        Ok(self.inner.estimate(python_item_to_hash(item)?, level))
    }
    /// Hierarchical heavy hitters above `threshold`, as (level, prefix, frequency).
    fn heavy_hitters(&self, threshold: f64) -> Vec<(usize, u64, f64)> {
        self.inner.heavy_hitters(threshold)
    }
    fn __repr__(&self) -> String {
        "Rhhh()".to_string()
    }
}
