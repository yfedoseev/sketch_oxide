//! Python bindings for the Sandwiched Learned Bloom filter.

use crate::common::python_item_to_bytes;
use crate::precomputed_oracle::PrecomputedOracle;
use pyo3::prelude::*;
use sketch_oxide::learned::SandwichedLearnedBloom as RustSandwichedLearnedBloom;

/// SandwichedLearnedBloom — a learned Bloom filter "sandwiched" between two
/// classical Bloom layers (Mitzenmacher 2018): an initial Bloom filter, then a
/// learned-oracle classifier, then a backup Bloom filter, giving lower
/// false-positive rates than a learned filter alone for a given size.
///
/// Construct with :meth:`build` from the positive key set and a score oracle.
#[pyclass(module = "sketch_oxide")]
pub struct SandwichedLearnedBloom {
    inner: RustSandwichedLearnedBloom,
}

#[pymethods]
impl SandwichedLearnedBloom {
    /// Builds the filter from `positives` (int/str/bytes/float keys), a score
    /// `oracle`, the classifier `threshold`, and target false-positive rate `fp_target`.
    #[staticmethod]
    fn build(
        positives: Vec<Bound<'_, PyAny>>,
        oracle: &PrecomputedOracle,
        threshold: f64,
        fp_target: f64,
    ) -> PyResult<Self> {
        let owned: Vec<Vec<u8>> = positives
            .iter()
            .map(python_item_to_bytes)
            .collect::<PyResult<_>>()?;
        let refs: Vec<&[u8]> = owned.iter().map(|v| v.as_slice()).collect();
        RustSandwichedLearnedBloom::build(&refs, &oracle.inner, threshold, fp_target)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Whether `key` might be a member, using the same `oracle` as at build time.
    fn contains(&self, key: &Bound<'_, PyAny>, oracle: &PrecomputedOracle) -> PyResult<bool> {
        Ok(self
            .inner
            .contains(&python_item_to_bytes(key)?, &oracle.inner))
    }

    /// Total size of the filter in bits.
    fn size_bits(&self) -> usize {
        self.inner.size_bits()
    }

    fn __repr__(&self) -> String {
        format!(
            "SandwichedLearnedBloom(size_bits={})",
            self.inner.size_bits()
        )
    }
}
