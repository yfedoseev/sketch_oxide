//! Python bindings for the JoinSketch (join-size / inner-product estimator).

use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::statistics::JoinSketch as RustJoinSketch;

/// JoinSketch — estimates join sizes and self-join (inner-product) sizes over a
/// stream by combining a frequent-item part with an infrequent Count-Min part,
/// reducing variance versus a plain sketch on skewed data.
///
/// Args:
///     fp_buckets (int): frequent-part bucket count.
///     fp_entries (int): frequent-part entries per bucket.
///     mp_buckets (int): mid-part bucket count.
///     mp_entries (int): mid-part entries per bucket.
///     ifp_depth (int): infrequent-part Count-Min depth.
///     ifp_width (int): infrequent-part Count-Min width.
///     threshold (int): frequency threshold separating frequent from infrequent.
#[pyclass(module = "sketch_oxide")]
pub struct JoinSketch {
    inner: RustJoinSketch,
}

#[pymethods]
impl JoinSketch {
    #[new]
    #[allow(clippy::too_many_arguments)]
    fn new(
        fp_buckets: usize,
        fp_entries: usize,
        mp_buckets: usize,
        mp_entries: usize,
        ifp_depth: usize,
        ifp_width: usize,
        threshold: u64,
    ) -> PyResult<Self> {
        RustJoinSketch::new(
            fp_buckets, fp_entries, mp_buckets, mp_entries, ifp_depth, ifp_width, threshold,
        )
        .map(|inner| Self { inner })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Records one occurrence of an item (int, str, bytes, or float).
    fn insert(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.insert(&python_item_to_bytes(item)?);
        Ok(())
    }

    /// Estimated frequency of an item.
    fn estimate(&self, item: &Bound<'_, PyAny>) -> PyResult<i64> {
        Ok(self.inner.estimate(&python_item_to_bytes(item)?))
    }

    /// Estimated inner product (join size) with another JoinSketch.
    fn inner_product(&self, other: &JoinSketch) -> PyResult<i64> {
        self.inner
            .inner_product(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        "JoinSketch()".to_string()
    }
}
