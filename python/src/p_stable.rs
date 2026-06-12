//! Python bindings for the p-stable Lp-norm sketch.

use pyo3::prelude::*;
use sketch_oxide::statistics::PStableLpSketch as RustPStableLpSketch;

/// PStableLpSketch — estimates the Lp norm of a dynamic vector (for `0 < p <= 2`)
/// using p-stable random projections, supporting signed coordinate updates.
///
/// Args:
///     p (float): norm parameter in `(0, 2]`.
///     d (int): number of projections (accuracy/space trade-off).
///     seed (int, optional): RNG seed for reproducibility.
#[pyclass(module = "sketch_oxide")]
pub struct PStableLpSketch {
    inner: RustPStableLpSketch,
}

#[pymethods]
impl PStableLpSketch {
    #[new]
    #[pyo3(signature = (p, d, seed=None))]
    fn new(p: f64, d: usize, seed: Option<u64>) -> PyResult<Self> {
        let res = match seed {
            Some(s) => RustPStableLpSketch::with_seed(p, d, s),
            None => RustPStableLpSketch::new(p, d),
        };
        res.map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Applies a signed `delta` to coordinate `coord`.
    fn update(&mut self, coord: u64, delta: f64) {
        self.inner.update(coord, delta);
    }

    /// Estimated Lp norm of the current vector.
    fn estimate(&self) -> f64 {
        self.inner.estimate()
    }

    /// Norm parameter `p`.
    fn p(&self) -> f64 {
        self.inner.p()
    }

    /// Number of projections.
    fn width(&self) -> usize {
        self.inner.width()
    }

    /// Merges another sketch (same parameters) into this one.
    fn merge(&mut self, other: &PStableLpSketch) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "PStableLpSketch(p={:.2}, estimate={:.3})",
            self.inner.p(),
            self.inner.estimate()
        )
    }
}
