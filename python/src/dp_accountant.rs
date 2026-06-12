//! Python bindings for the differential-privacy budget accountant.

use pyo3::prelude::*;
use sketch_oxide::privacy::{Accountant as RustAccountant, PrivacyParams as RustPrivacyParams};

/// PrivacyParams — an `(epsilon, delta)` differential-privacy budget / cost.
///
/// Args:
///     epsilon (float): the ε privacy parameter (> 0).
///     delta (float): the δ privacy parameter (>= 0); use 0 for pure ε-DP.
#[pyclass(module = "sketch_oxide")]
#[derive(Clone)]
pub struct PrivacyParams {
    pub(crate) inner: RustPrivacyParams,
}

#[pymethods]
impl PrivacyParams {
    #[new]
    #[pyo3(signature = (epsilon, delta=0.0))]
    fn new(epsilon: f64, delta: f64) -> PyResult<Self> {
        let inner = if delta == 0.0 {
            RustPrivacyParams::pure(epsilon)
        } else {
            RustPrivacyParams::new(epsilon, delta)
        }
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(Self { inner })
    }

    /// The ε privacy parameter.
    #[getter]
    fn epsilon(&self) -> f64 {
        self.inner.epsilon
    }

    /// The δ privacy parameter.
    #[getter]
    fn delta(&self) -> f64 {
        self.inner.delta
    }

    fn __repr__(&self) -> String {
        format!(
            "PrivacyParams(epsilon={}, delta={})",
            self.inner.epsilon, self.inner.delta
        )
    }
}

/// Accountant — tracks a differential-privacy budget, allowing composed
/// mechanisms to "spend" `(epsilon, delta)` until the budget is exhausted.
///
/// Args:
///     budget (PrivacyParams): the total privacy budget.
#[pyclass(module = "sketch_oxide")]
pub struct Accountant {
    inner: RustAccountant,
}

#[pymethods]
impl Accountant {
    #[new]
    fn new(budget: &PrivacyParams) -> Self {
        Self {
            inner: RustAccountant::new(budget.inner),
        }
    }

    /// Spends `cost` from the budget; raises if it would exceed the budget.
    fn spend(&mut self, cost: &PrivacyParams) -> PyResult<()> {
        self.inner
            .spend(cost.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Whether `cost` could be spent without exceeding the budget.
    fn can_spend(&self, cost: &PrivacyParams) -> bool {
        self.inner.can_spend(cost.inner)
    }

    /// The total budget.
    fn budget(&self) -> PrivacyParams {
        PrivacyParams {
            inner: self.inner.budget(),
        }
    }

    /// The amount spent so far.
    fn spent(&self) -> PrivacyParams {
        PrivacyParams {
            inner: self.inner.spent(),
        }
    }

    /// The remaining budget.
    fn remaining(&self) -> PrivacyParams {
        PrivacyParams {
            inner: self.inner.remaining(),
        }
    }

    fn __repr__(&self) -> String {
        let s = self.inner.spent();
        let b = self.inner.budget();
        format!(
            "Accountant(spent_epsilon={}, budget_epsilon={})",
            s.epsilon, b.epsilon
        )
    }
}
