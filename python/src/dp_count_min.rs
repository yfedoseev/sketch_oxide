//! Python bindings for the differentially-private Count-Min sketch.

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;
use sketch_oxide::privacy::{DpCountMin as RustDpCountMin, PrivateCountMin as RustPrivateCountMin};

/// PrivateCountMin — a frozen, noise-added Count-Min sketch released under
/// ε-differential privacy. Query-only.
#[pyclass(module = "sketch_oxide")]
pub struct PrivateCountMin {
    inner: RustPrivateCountMin,
}

#[pymethods]
impl PrivateCountMin {
    /// Noisy frequency estimate for `item`.
    fn estimate(&self, item: &Bound<'_, PyAny>) -> PyResult<i64> {
        Ok(self.inner.estimate(&python_item_to_bytes(item)?))
    }

    /// Noisy frequency estimate for `item`, clamped to be non-negative.
    fn estimate_clamped(&self, item: &Bound<'_, PyAny>) -> PyResult<i64> {
        Ok(self.inner.estimate_clamped(&python_item_to_bytes(item)?))
    }

    /// Privacy parameter used for the release.
    fn epsilon(&self) -> f64 {
        self.inner.epsilon()
    }

    fn __repr__(&self) -> String {
        format!("PrivateCountMin(epsilon={})", self.inner.epsilon())
    }
}

/// DpCountMin — a plain (non-private) Count-Min sketch that can be frozen into a
/// differentially-private :class:`PrivateCountMin` release via :meth:`privatize`
/// (discrete-Laplace noise from an OS-seeded CSPRNG).
///
/// Args:
///     depth (int): number of hash rows.
///     width (int): counters per row.
#[pyclass(module = "sketch_oxide")]
pub struct DpCountMin {
    inner: RustDpCountMin,
    rng: StdRng,
}

#[pymethods]
impl DpCountMin {
    #[new]
    fn new(depth: usize, width: usize) -> PyResult<Self> {
        RustDpCountMin::new(depth, width)
            .map(|inner| Self {
                inner,
                rng: StdRng::from_os_rng(),
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Adds `count` (possibly negative) to `item` (int, str, bytes, or float).
    fn update(&mut self, item: &Bound<'_, PyAny>, count: i64) -> PyResult<()> {
        self.inner.update(&python_item_to_bytes(item)?, count);
        Ok(())
    }

    /// Non-private frequency estimate for `item`.
    fn estimate(&self, item: &Bound<'_, PyAny>) -> PyResult<i64> {
        Ok(self.inner.estimate(&python_item_to_bytes(item)?))
    }

    /// Number of hash rows.
    fn depth(&self) -> usize {
        self.inner.depth()
    }

    /// Counters per row.
    fn width(&self) -> usize {
        self.inner.width()
    }

    /// Freezes a differentially-private release at privacy level `epsilon`.
    fn privatize(&mut self, epsilon: f64) -> PyResult<PrivateCountMin> {
        self.inner
            .privatize(epsilon, &mut self.rng)
            .map(|inner| PrivateCountMin { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "DpCountMin(depth={}, width={})",
            self.inner.depth(),
            self.inner.width()
        )
    }
}
