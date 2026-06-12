//! Python bindings for the deterministic WAVE sliding-window bit counter.

use pyo3::prelude::*;
use sketch_oxide::streaming::DeterministicWave as RustDeterministicWave;

/// DeterministicWave — deterministic sliding-window counting of 1-bits over a
/// binary stream (Gibbons–Tirthapura WAVE), answering "how many 1s in the last
/// `window` bits?" within relative error `epsilon`.
///
/// Args:
///     max_window (int): largest window queryable.
///     epsilon (float): relative error bound.
#[pyclass(module = "sketch_oxide")]
pub struct DeterministicWave {
    inner: RustDeterministicWave,
}

#[pymethods]
impl DeterministicWave {
    #[new]
    fn new(max_window: u64, epsilon: f64) -> PyResult<Self> {
        RustDeterministicWave::new(max_window, epsilon)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Appends one bit to the stream.
    fn update(&mut self, bit: bool) {
        self.inner.update(bit);
    }

    /// Estimated number of 1-bits within the most recent `window` positions.
    fn estimate(&self, window: u64) -> u64 {
        self.inner.estimate(window)
    }

    /// Total number of bits processed.
    fn position(&self) -> u64 {
        self.inner.position()
    }

    /// Total number of 1-bits seen.
    fn ones(&self) -> u64 {
        self.inner.ones()
    }

    fn __repr__(&self) -> String {
        format!(
            "DeterministicWave(position={}, ones={})",
            self.inner.position(),
            self.inner.ones()
        )
    }
}
