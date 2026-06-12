//! Python bindings for the Taffy Cuckoo Filter.
use crate::common::python_item_to_bytes;
use pyo3::prelude::*;
use sketch_oxide::membership::TaffyCuckooFilter as RustTaffy;

/// Taffy Cuckoo Filter — a cuckoo filter that grows on demand without inflating the FPR (SP&E 2022).
#[pyclass(module = "sketch_oxide")]
pub struct TaffyCuckooFilter {
    inner: RustTaffy,
}

#[pymethods]
impl TaffyCuckooFilter {
    #[new]
    #[pyo3(signature = (seed = None))]
    fn new(seed: Option<u64>) -> Self {
        let inner = match seed {
            Some(s) => RustTaffy::with_seed(s),
            None => RustTaffy::new(),
        };
        Self { inner }
    }
    fn insert(&mut self, item: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner.insert(&python_item_to_bytes(item)?);
        Ok(())
    }
    fn contains(&self, item: &Bound<'_, PyAny>) -> PyResult<bool> {
        Ok(self.inner.contains(&python_item_to_bytes(item)?))
    }
    fn __len__(&self) -> usize {
        self.inner.len()
    }
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    fn __repr__(&self) -> String {
        format!("TaffyCuckooFilter(len={})", self.inner.len())
    }
}
