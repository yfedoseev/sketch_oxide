//! Python bindings for the Odd Sketch (Jaccard similarity).
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::similarity::OddSketch as RustOddSketch;

/// Odd Sketch — a compact bit-sketch for set similarity (Mitzenmacher 2014).
#[pyclass(module = "sketch_oxide")]
pub struct OddSketch {
    inner: RustOddSketch,
}

#[pymethods]
impl OddSketch {
    #[new]
    fn new(num_bits: usize) -> PyResult<Self> {
        RustOddSketch::new(num_bits)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.insert(&python_item_to_bytes(item)?);
        Ok(())
    }
    /// Estimated Jaccard similarity given the (distinct) set sizes of both sketches.
    fn jaccard(&self, other: &OddSketch, size_a: u64, size_b: u64) -> PyResult<f64> {
        self.inner
            .jaccard(&other.inner, size_a, size_b)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn __repr__(&self) -> String {
        "OddSketch()".to_string()
    }
}
