//! Python bindings for the Morris approximate counter.

use pyo3::prelude::*;
use sketch_oxide::statistics::MorrisCounter as RustMorrisCounter;

/// MorrisCounter — probabilistic approximate counting (Morris 1978): counts up to
/// very large totals in a few bits by storing the exponent of a `base`-power
/// estimate and incrementing it probabilistically.
///
/// Args:
///     base (float): counter base (closer to 1 gives higher accuracy, more bits).
///     seed (int, optional): RNG seed for reproducibility.
#[pyclass(module = "sketch_oxide")]
pub struct MorrisCounter {
    inner: RustMorrisCounter,
}

#[pymethods]
impl MorrisCounter {
    #[new]
    #[pyo3(signature = (base, seed=None))]
    fn new(base: f64, seed: Option<u64>) -> PyResult<Self> {
        let res = match seed {
            Some(s) => RustMorrisCounter::with_seed(base, s),
            None => RustMorrisCounter::new(base),
        };
        res.map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Registers one event (probabilistically bumps the internal register).
    fn increment(&mut self) {
        self.inner.increment();
    }

    /// Estimated total count.
    fn estimate(&self) -> f64 {
        self.inner.estimate()
    }

    /// Current internal register value.
    fn register(&self) -> u32 {
        self.inner.register()
    }

    /// Counter base.
    fn base(&self) -> f64 {
        self.inner.base()
    }

    fn __repr__(&self) -> String {
        format!("MorrisCounter(estimate={:.1})", self.inner.estimate())
    }
}
