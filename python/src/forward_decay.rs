//! Python bindings for forward-decay time-weighted aggregates.

use pyo3::prelude::*;
use sketch_oxide::streaming::{
    ForwardDecay as RustForwardDecay, PolynomialForwardDecay as RustPolynomialForwardDecay,
};

/// ForwardDecay — exponential forward-decay aggregation (Cormode–Shkapenyuk–
/// Srivastava–Xu), giving time-weighted count/sum/average that emphasise recent
/// values without per-item rescaling.
///
/// Args:
///     rate (float): exponential decay rate.
#[pyclass(module = "sketch_oxide")]
pub struct ForwardDecay {
    inner: RustForwardDecay,
}

#[pymethods]
impl ForwardDecay {
    #[new]
    fn new(rate: f64) -> PyResult<Self> {
        RustForwardDecay::new(rate)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Records a value observed at the given timestamp.
    fn update(&mut self, value: f64, timestamp: u64) {
        self.inner.update(value, timestamp);
    }

    /// Time-decayed count as of time `now`.
    fn decayed_count(&self, now: u64) -> f64 {
        self.inner.decayed_count(now)
    }

    /// Time-decayed sum as of time `now`.
    fn decayed_sum(&self, now: u64) -> f64 {
        self.inner.decayed_sum(now)
    }

    /// Decay-weighted average, or None if empty.
    fn average(&self) -> Option<f64> {
        self.inner.average()
    }

    /// Number of values observed.
    fn count(&self) -> u64 {
        self.inner.count()
    }

    fn __repr__(&self) -> String {
        format!("ForwardDecay(count={})", self.inner.count())
    }
}

/// PolynomialForwardDecay — forward decay with a polynomial weight `(t - L)^beta`
/// relative to a fixed landmark, for sub-exponential ageing.
///
/// Args:
///     beta (float): polynomial decay exponent.
///     landmark (int): landmark time the decay is measured from.
#[pyclass(module = "sketch_oxide")]
pub struct PolynomialForwardDecay {
    inner: RustPolynomialForwardDecay,
}

#[pymethods]
impl PolynomialForwardDecay {
    #[new]
    fn new(beta: f64, landmark: u64) -> PyResult<Self> {
        RustPolynomialForwardDecay::new(beta, landmark)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Records a value observed at the given timestamp.
    fn update(&mut self, value: f64, timestamp: u64) {
        self.inner.update(value, timestamp);
    }

    /// Polynomially decayed count as of time `now`.
    fn decayed_count(&self, now: u64) -> f64 {
        self.inner.decayed_count(now)
    }

    /// Decay-weighted average, or None if empty.
    fn average(&self) -> Option<f64> {
        self.inner.average()
    }

    /// Number of values observed.
    fn count(&self) -> u64 {
        self.inner.count()
    }

    fn __repr__(&self) -> String {
        format!("PolynomialForwardDecay(count={})", self.inner.count())
    }
}
