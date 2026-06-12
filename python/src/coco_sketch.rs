//! Python bindings for the CocoSketch (compressed-counting universal sketch).

use pyo3::prelude::*;
use rand::rngs::SmallRng;
use rand::SeedableRng;
use sketch_oxide::universal::CocoSketch as RustCocoSketch;

/// CocoSketch — a universal sketch for per-flow size estimation that keeps a small
/// set of "winning" flows per bucket via a compressed-counting contest, capturing
/// heavy flows with bounded error.
///
/// Args:
///     d (int): number of hash arrays (rows).
///     l (int): number of buckets per array.
///     seed (int, optional): RNG seed for the sampling contest (default: OS entropy).
#[pyclass(module = "sketch_oxide")]
pub struct CocoSketch {
    inner: RustCocoSketch,
    rng: SmallRng,
}

#[pymethods]
impl CocoSketch {
    #[new]
    #[pyo3(signature = (d, l, seed=None))]
    fn new(d: usize, l: usize, seed: Option<u64>) -> PyResult<Self> {
        let rng = match seed {
            Some(s) => SmallRng::seed_from_u64(s),
            None => SmallRng::from_os_rng(),
        };
        RustCocoSketch::new(d, l)
            .map(|inner| Self { inner, rng })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Adds weight `w` to integer flow `e`.
    fn insert(&mut self, e: u64, w: f64) {
        self.inner.insert(e, w, &mut self.rng);
    }

    /// Estimated total weight of flow `e`.
    fn estimate(&self, e: u64) -> f64 {
        self.inner.estimate(e)
    }

    /// The currently-recorded `(flow, estimated_weight)` winners.
    fn recorded(&self) -> Vec<(u64, f64)> {
        self.inner.recorded()
    }

    /// Number of hash arrays.
    fn arrays(&self) -> usize {
        self.inner.arrays()
    }

    /// Number of buckets per array.
    fn buckets(&self) -> usize {
        self.inner.buckets()
    }

    fn __repr__(&self) -> String {
        format!(
            "CocoSketch(arrays={}, buckets={})",
            self.inner.arrays(),
            self.inner.buckets()
        )
    }
}
