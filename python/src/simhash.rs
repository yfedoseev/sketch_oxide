//! Python bindings for SimHash similarity estimation

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::similarity::SimHash as RustSimHash;

use crate::with_python_item;

/// SimHash for near-duplicate detection (Charikar 2002)
///
/// Creates 64-bit fingerprints for near-duplicate detection via Hamming distance.
/// Used by Google for web crawling and spam detection.
///
/// Example:
///     >>> sh1 = SimHash()
///     >>> sh2 = SimHash()
///     >>> for word in "the quick brown fox".split():
///     ...     sh1.update(word)
///     >>> for word in "the quick brown dog".split():  # "fox" -> "dog"
///     ...     sh2.update(word)
///     >>> distance = sh1.hamming_distance(sh2)
///     >>> print(f"Hamming distance: {distance}")  # Low = similar
///
/// Notes:
///     - O(1) space (64-bit fingerprint)
///     - Best for Hamming distance 3-7 (near-duplicates)
///     - For 5%+ similarity, use MinHash instead
#[pyclass(module = "sketch_oxide")]
pub struct SimHash {
    inner: RustSimHash,
}

#[pymethods]
impl SimHash {
    /// Create a new SimHash
    #[new]
    fn new() -> Self {
        Self {
            inner: RustSimHash::new(),
        }
    }

    /// Update with a feature (weight=1)
    ///
    /// Args:
    ///     feature: Feature to add (int, str, or bytes)
    fn update(&mut self, feature: &Bound<'_, PyAny>) -> PyResult<()> {
        with_python_item!(feature, |rust_item| {
            self.inner.update(rust_item);
        })
    }

    /// Update with a weighted feature
    ///
    /// Args:
    ///     feature: Feature to add (int, str, or bytes)
    ///     weight: Weight of the feature (positive integer)
    fn update_weighted(&mut self, feature: &Bound<'_, PyAny>, weight: i64) -> PyResult<()> {
        with_python_item!(feature, |rust_item| {
            self.inner.update_weighted(rust_item, weight);
        })
    }

    /// Get the 64-bit fingerprint
    ///
    /// Returns:
    ///     int: The 64-bit SimHash fingerprint
    fn fingerprint(&mut self) -> u64 {
        self.inner.fingerprint()
    }

    /// Compute Hamming distance to another SimHash
    ///
    /// Lower distance = more similar documents.
    ///
    /// Args:
    ///     other: Another SimHash
    ///
    /// Returns:
    ///     int: Number of differing bits (0-64)
    fn hamming_distance(&mut self, other: &mut SimHash) -> u32 {
        self.inner.hamming_distance(&mut other.inner)
    }

    /// Compute similarity (64 - hamming_distance) / 64
    ///
    /// Args:
    ///     other: Another SimHash
    ///
    /// Returns:
    ///     float: Similarity score (0.0 to 1.0)
    fn similarity(&mut self, other: &mut SimHash) -> f64 {
        self.inner.similarity(&mut other.inner)
    }

    /// Number of features added
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// Check if empty
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Serialize to bytes
    fn serialize<'py>(&mut self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new_bound(py, &self.inner.to_bytes())
    }

    /// Deserialize from bytes
    #[staticmethod]
    fn from_bytes(bytes: &[u8]) -> PyResult<Self> {
        RustSimHash::from_bytes(bytes)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("SimHash(features={})", self.inner.len())
    }

    fn __str__(&self) -> String {
        format!("SimHash with {} features", self.inner.len())
    }
}
