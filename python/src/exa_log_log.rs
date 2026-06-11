//! Python bindings for ExaLogLog cardinality estimation.

use crate::common::python_item_to_hash;
use pyo3::prelude::*;
use sketch_oxide::cardinality::ExaLogLog as RustExaLogLog;

/// ExaLogLog — a near-optimal cardinality sketch with tunable precision/tail parameters.
///
/// Args:
///     p (int): register precision.
///     t (int): tail length parameter.
///     d (int): extra-bits parameter.
///
/// Example:
///     >>> ell = ExaLogLog(12, 2, 20)
///     >>> for i in range(100000):
///     ...     ell.update(i)
///     >>> ell.estimate() > 0
///     True
#[pyclass(module = "sketch_oxide")]
pub struct ExaLogLog {
    inner: RustExaLogLog,
}

#[pymethods]
impl ExaLogLog {
    #[new]
    fn new(p: u32, t: u32, d: u32) -> PyResult<Self> {
        RustExaLogLog::new(p, t, d)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Adds an item (int, str, bytes, or float).
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        let h = python_item_to_hash(item)?;
        self.inner.add(&h);
        Ok(())
    }

    /// Estimated number of distinct items.
    fn estimate(&self) -> f64 {
        self.inner.estimate()
    }

    fn __repr__(&self) -> String {
        format!("ExaLogLog(estimate={:.0})", self.inner.estimate())
    }
}
