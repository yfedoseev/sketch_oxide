//! Python bindings for b-bit MinHash.
use pyo3::prelude::*;
use sketch_oxide::similarity::BBitMinHash as RustBBitMinHash;

/// b-bit MinHash — compresses a MinHash signature to b bits per hash (Li & König 2010).
///
/// Build from a MinHash signature (a list of ints) and `bits` bits per hash.
#[pyclass(module = "sketch_oxide")]
pub struct BBitMinHash {
    inner: RustBBitMinHash,
}

#[pymethods]
impl BBitMinHash {
    #[new]
    fn new(signature: Vec<u64>, bits: u8) -> PyResult<Self> {
        RustBBitMinHash::from_signature(&signature, bits)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn jaccard(&self, other: &BBitMinHash) -> PyResult<f64> {
        self.inner
            .jaccard(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }
    fn bits(&self) -> u8 {
        self.inner.bits()
    }
    fn __len__(&self) -> usize {
        self.inner.len()
    }
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    fn __repr__(&self) -> String {
        format!(
            "BBitMinHash(bits={}, len={})",
            self.inner.bits(),
            self.inner.len()
        )
    }
}
