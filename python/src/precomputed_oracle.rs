//! Python bindings for a precomputed score oracle (for learned sketches).

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::learned::PrecomputedOracle as RustPrecomputedOracle;

/// PrecomputedOracle — a lookup-table score oracle for the learned sketches: it
/// maps known keys to predicted "heaviness" scores (e.g. from an offline model),
/// returning a `default` score for unknown keys.
///
/// Args:
///     default (float): score returned for keys not explicitly set.
#[pyclass(module = "sketch_oxide")]
pub struct PrecomputedOracle {
    pub(crate) inner: RustPrecomputedOracle,
}

#[pymethods]
impl PrecomputedOracle {
    #[new]
    fn new(default: f64) -> Self {
        Self {
            inner: RustPrecomputedOracle::new(default),
        }
    }

    /// Sets the predicted score for `key` (int, str, bytes, or float).
    fn set(&mut self, key: &Bound<'_, PyAny>, score: f64) -> PyResult<()> {
        self.inner.set(&python_item_to_bytes(key)?, score);
        Ok(())
    }

    /// The default score for unknown keys.
    fn default_score(&self) -> f64 {
        self.inner.default_score()
    }

    /// Number of keys with an explicit score.
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether no explicit scores have been set.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn __repr__(&self) -> String {
        format!("PrecomputedOracle(len={})", self.inner.len())
    }
}
