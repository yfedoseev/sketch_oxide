//! Python bindings for the gradient sketch (sketched SGD / communication-efficient SGD).

use pyo3::prelude::*;
use sketch_oxide::learned::GradientSketch as RustGradientSketch;

/// GradientSketch — a Count-Sketch over gradient coordinates for communication-
/// efficient / federated SGD: gradients are sketched into `depth × width`
/// counters, top-k coordinates recovered, and sketches merged/scaled additively.
///
/// Args:
///     depth (int): number of hash rows.
///     width (int): counters per row.
#[pyclass(module = "sketch_oxide")]
pub struct GradientSketch {
    inner: RustGradientSketch,
}

#[pymethods]
impl GradientSketch {
    #[new]
    fn new(depth: usize, width: usize) -> PyResult<Self> {
        RustGradientSketch::new(depth, width)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Adds `value` to gradient coordinate `coord`.
    fn add(&mut self, coord: u64, value: f64) {
        self.inner.add(coord, value);
    }

    /// Sketches a full dense `gradient` vector (coordinate = index).
    fn accumulate(&mut self, gradient: Vec<f64>) {
        self.inner.accumulate(&gradient);
    }

    /// Unbiased estimate of gradient coordinate `coord`.
    fn estimate(&self, coord: u64) -> f64 {
        self.inner.estimate(coord)
    }

    /// Top-`k` coordinates by magnitude over `dimension` coordinates, as `(index, value)`.
    fn top_k(&self, dimension: usize, k: usize) -> Vec<(usize, f64)> {
        self.inner.top_k(dimension, k)
    }

    /// Recovers the full dense estimate over `dimension` coordinates.
    fn unsketch(&self, dimension: usize) -> Vec<f64> {
        self.inner.unsketch(dimension)
    }

    /// Number of hash rows.
    fn depth(&self) -> usize {
        self.inner.depth()
    }

    /// Counters per row.
    fn width(&self) -> usize {
        self.inner.width()
    }

    /// Adds another gradient sketch (same shape) into this one.
    fn merge(&mut self, other: &GradientSketch) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Scales all counters by `factor` (e.g. for averaging).
    fn scale(&mut self, factor: f64) {
        self.inner.scale(factor);
    }

    fn __repr__(&self) -> String {
        format!(
            "GradientSketch(depth={}, width={})",
            self.inner.depth(),
            self.inner.width()
        )
    }
}
