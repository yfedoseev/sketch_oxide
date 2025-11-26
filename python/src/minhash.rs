//! Python bindings for MinHash similarity estimation

use pyo3::prelude::*;
use sketch_oxide::similarity::MinHash as RustMinHash;
use sketch_oxide::{Mergeable, Sketch};

use crate::with_python_item;

/// MinHash sketch for Jaccard similarity estimation (Broder 1997)
///
/// Estimates Jaccard similarity |A ∩ B| / |A ∪ B| between sets.
///
/// Args:
///     num_perm (int): Number of hash functions (≥16). Higher = more accurate.
///         - 64: ~12.5% error
///         - 128: ~8.8% error (recommended)
///         - 256: ~6.25% error
///
/// Example:
///     >>> mh1 = MinHash(num_perm=128)
///     >>> mh2 = MinHash(num_perm=128)
///     >>> for item in set1:
///     ...     mh1.update(item)
///     >>> for item in set2:
///     ...     mh2.update(item)
///     >>> similarity = mh1.jaccard_similarity(mh2)
///     >>> print(f"Jaccard: {similarity:.2%}")
///
/// Notes:
///     - Supports int, str, and bytes types
///     - Standard error ≈ 1/√num_perm
///     - Used in: LSH, deduplication, near-duplicate detection
#[pyclass(module = "sketch_oxide")]
pub struct MinHash {
    inner: RustMinHash,
}

#[pymethods]
impl MinHash {
    /// Create a new MinHash sketch
    ///
    /// Args:
    ///     num_perm: Number of hash functions (≥16)
    ///
    /// Raises:
    ///     ValueError: If num_perm < 16
    #[new]
    fn new(num_perm: usize) -> PyResult<Self> {
        RustMinHash::new(num_perm)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Update the sketch with a single item
    ///
    /// Args:
    ///     item: Item to add (int, str, or bytes)
    ///
    /// Example:
    ///     >>> mh = MinHash(num_perm=128)
    ///     >>> mh.update("user123")
    ///     >>> mh.update(42)
    fn update(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        with_python_item!(item, |rust_item| {
            self.inner.update(rust_item);
        })
    }

    /// Estimate Jaccard similarity with another MinHash
    ///
    /// Args:
    ///     other: Another MinHash sketch with same num_perm
    ///
    /// Returns:
    ///     float: Estimated Jaccard similarity (0.0 to 1.0)
    ///
    /// Raises:
    ///     ValueError: If sketches have different num_perm
    ///
    /// Example:
    ///     >>> mh1 = MinHash(num_perm=128)
    ///     >>> mh2 = MinHash(num_perm=128)
    ///     >>> # ... add data ...
    ///     >>> similarity = mh1.jaccard_similarity(mh2)
    fn jaccard_similarity(&self, other: &MinHash) -> PyResult<f64> {
        self.inner
            .jaccard_similarity(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Merge another MinHash into this one (union operation)
    ///
    /// Args:
    ///     other: Another MinHash with same num_perm
    ///
    /// Raises:
    ///     ValueError: If sketches have different num_perm
    ///
    /// Example:
    ///     >>> mh_union = MinHash(num_perm=128)
    ///     >>> # ... create mh1 and mh2 ...
    ///     >>> mh_union.merge(mh1)
    ///     >>> mh_union.merge(mh2)
    fn merge(&mut self, other: &MinHash) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Check if the sketch is empty
    ///
    /// Returns:
    ///     bool: True if no items have been added
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get the number of permutations
    ///
    /// Returns:
    ///     int: Number of hash functions
    fn num_perm(&self) -> usize {
        self.inner.num_perm()
    }

    fn __repr__(&self) -> String {
        format!("MinHash(num_perm={})", self.inner.num_perm())
    }

    fn __str__(&self) -> String {
        format!("MinHash({} permutations)", self.inner.num_perm())
    }
}
