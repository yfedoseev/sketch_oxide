//! Python bindings for Weighted MinHash (consistent weighted sampling).
use pyo3::prelude::*;
use sketch_oxide::similarity::WeightedMinHash as RustWeightedMinHash;

/// Weighted MinHash — consistent weighted sampling for weighted Jaccard (Ioffe 2010).
///
/// A stateless signer: `signature(weighted_set)` turns a list of `(id, weight)` pairs into a
/// signature, and `jaccard(sig_a, sig_b)` estimates their weighted Jaccard similarity.
#[pyclass(module = "sketch_oxide")]
pub struct WeightedMinHash {
    inner: RustWeightedMinHash,
}

#[pymethods]
impl WeightedMinHash {
    #[new]
    fn new(num_hashes: usize) -> PyResult<Self> {
        RustWeightedMinHash::new(num_hashes)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    /// Signs a weighted set given as a list of (id, weight) pairs.
    fn signature(&self, weighted_set: Vec<(u64, f64)>) -> Vec<(u64, i64)> {
        self.inner.signature(&weighted_set)
    }
    /// Estimated weighted Jaccard between two signatures.
    fn jaccard(&self, a: Vec<(u64, i64)>, b: Vec<(u64, i64)>) -> f64 {
        self.inner.jaccard(&a, &b)
    }
    fn __repr__(&self) -> String {
        "WeightedMinHash()".to_string()
    }
}
