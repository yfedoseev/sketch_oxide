//! Python bindings for PinSketch (BCH-based set reconciliation).

use pyo3::prelude::*;
use sketch_oxide::reconciliation::PinSketch as RustPinSketch;

/// PinSketch — a BCH-code-based set reconciliation sketch over a binary field.
/// The XOR (merge) of two sketches decodes to their symmetric difference, for up
/// to `capacity` differing elements.
///
/// Elements are integers in the field domain `1 ..= 2**field_bits - 1`; values
/// outside that range raise `ValueError`. To reconcile arbitrary objects, hash
/// them into this range before inserting.
///
/// Args:
///     field_bits (int): bit-width of the binary field (element domain = 2**field_bits).
///     capacity (int): maximum recoverable symmetric-difference size.
#[pyclass(module = "sketch_oxide")]
pub struct PinSketch {
    inner: RustPinSketch,
}

#[pymethods]
impl PinSketch {
    #[new]
    fn new(field_bits: u32, capacity: usize) -> PyResult<Self> {
        RustPinSketch::new(field_bits, capacity)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Inserts an integer element in the field domain.
    fn insert(&mut self, element: u64) -> PyResult<()> {
        self.inner
            .insert(element)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Removes a previously inserted integer element.
    fn remove(&mut self, element: u64) -> PyResult<()> {
        self.inner
            .remove(element)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// XOR-merges another sketch (same parameters) into this one, yielding the
    /// symmetric-difference sketch.
    fn merge(&mut self, other: &PinSketch) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Decodes the (field-reduced) symmetric-difference elements.
    fn decode(&self) -> PyResult<Vec<u64>> {
        self.inner
            .decode()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Maximum recoverable symmetric-difference size.
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Field bit-width.
    fn field_bits(&self) -> u32 {
        self.inner.field_bits()
    }

    fn __repr__(&self) -> String {
        format!(
            "PinSketch(field_bits={}, capacity={})",
            self.inner.field_bits(),
            self.inner.capacity()
        )
    }
}
