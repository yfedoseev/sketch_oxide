//! Python bindings for the k-ary sketch (change detection / heavy changers).

use pyo3::prelude::*;
use sketch_oxide::statistics::KArySketch as RustKArySketch;

/// KArySketch — a Count-Sketch-style structure over integer keys with signed
/// values, supporting unbiased point estimates, sketch differencing across
/// epochs, and heavy-changer detection.
///
/// Args:
///     depth (int): number of hash rows (median estimators).
///     width (int): counters per row.
#[pyclass(module = "sketch_oxide")]
pub struct KArySketch {
    inner: RustKArySketch,
}

#[pymethods]
impl KArySketch {
    #[new]
    fn new(depth: usize, width: usize) -> PyResult<Self> {
        RustKArySketch::new(depth, width)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Adds signed `value` to integer `key`.
    fn update(&mut self, key: u64, value: i64) {
        self.inner.update(key, value);
    }

    /// Unbiased estimate of the accumulated value for `key`.
    fn estimate(&self, key: u64) -> f64 {
        self.inner.estimate(key)
    }

    /// Returns a new sketch equal to `self - other` (per-counter difference).
    fn difference(&self, other: &KArySketch) -> PyResult<KArySketch> {
        self.inner
            .difference(&other.inner)
            .map(|inner| KArySketch { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Among `candidates`, returns `(key, estimate)` for those whose estimated
    /// magnitude meets `threshold`.
    fn heavy_changers(&self, candidates: Vec<u64>, threshold: f64) -> Vec<(u64, f64)> {
        self.inner.heavy_changers(&candidates, threshold)
    }

    /// Number of hash rows.
    fn depth(&self) -> usize {
        self.inner.depth()
    }

    /// Counters per row.
    fn width(&self) -> usize {
        self.inner.width()
    }

    /// Total signed mass inserted.
    fn total(&self) -> i64 {
        self.inner.total()
    }

    fn __repr__(&self) -> String {
        format!(
            "KArySketch(depth={}, width={}, total={})",
            self.inner.depth(),
            self.inner.width(),
            self.inner.total()
        )
    }
}
