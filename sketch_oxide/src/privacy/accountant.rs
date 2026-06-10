//! Privacy budget and `(ε, δ)` composition accounting.
//!
//! Differential privacy composes: running several DP mechanisms on the same data spends
//! cumulative privacy. [`Accountant`] tracks an `(ε, δ)` budget and refuses spends that
//! would exceed it, so a pipeline cannot silently leak more than intended.
//!
//! # Composition model
//!
//! This implements **basic (sequential) composition**: total cost is the sum of the
//! per-query `ε` and `δ`. That bound is always valid and never under-counts. Tighter
//! advanced/Rényi composition (which can permit more queries for the same budget) is left
//! for a later iteration; using the conservative bound now never overstates privacy.

use crate::error::{Result, SketchError};

/// An `(ε, δ)` differential-privacy cost or budget.
///
/// `epsilon >= 0` and `delta` in `[0, 1]`. `delta = 0` denotes pure `ε`-DP.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrivacyParams {
    /// The `ε` (epsilon) privacy-loss parameter.
    pub epsilon: f64,
    /// The `δ` (delta) failure-probability parameter, in `[0, 1]`.
    pub delta: f64,
}

impl PrivacyParams {
    /// Creates an `(ε, δ)` pair, validating `epsilon >= 0` and `delta` in `[0, 1]`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if either parameter is out of range or non-finite.
    pub fn new(epsilon: f64, delta: f64) -> Result<Self> {
        if !epsilon.is_finite() || epsilon < 0.0 {
            return Err(SketchError::InvalidParameter {
                param: "epsilon".to_string(),
                value: epsilon.to_string(),
                constraint: "must be a finite value >= 0".to_string(),
            });
        }
        if !delta.is_finite() || !(0.0..=1.0).contains(&delta) {
            return Err(SketchError::InvalidParameter {
                param: "delta".to_string(),
                value: delta.to_string(),
                constraint: "must be in [0, 1]".to_string(),
            });
        }
        Ok(Self { epsilon, delta })
    }

    /// Pure `ε`-DP: `(ε, 0)`.
    pub fn pure(epsilon: f64) -> Result<Self> {
        Self::new(epsilon, 0.0)
    }
}

/// Tracks cumulative `(ε, δ)` spend against a fixed budget under sequential composition.
///
/// # Example
/// ```
/// use sketch_oxide::privacy::{Accountant, PrivacyParams};
///
/// let mut acct = Accountant::new(PrivacyParams::new(1.0, 1e-5).unwrap());
///
/// // Spend on two queries.
/// acct.spend(PrivacyParams::pure(0.4).unwrap()).unwrap();
/// acct.spend(PrivacyParams::pure(0.4).unwrap()).unwrap();
///
/// // 0.4 + 0.4 = 0.8 spent; a third 0.4 query (total 1.2 > 1.0) is refused.
/// assert!(acct.spend(PrivacyParams::pure(0.4).unwrap()).is_err());
/// assert!((acct.spent().epsilon - 0.8).abs() < 1e-9);
/// ```
#[derive(Debug, Clone)]
pub struct Accountant {
    budget: PrivacyParams,
    spent: PrivacyParams,
}

impl Accountant {
    /// Creates an accountant with the given total `(ε, δ)` budget.
    pub fn new(budget: PrivacyParams) -> Self {
        Self {
            budget,
            spent: PrivacyParams {
                epsilon: 0.0,
                delta: 0.0,
            },
        }
    }

    /// Records the cost of a mechanism, refusing it if it would exceed the budget.
    ///
    /// On error the spend is **not** applied, so the accountant stays consistent and the
    /// caller can choose a cheaper mechanism.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if the spend would push cumulative `ε` or `δ` past
    /// the budget (under sequential composition).
    pub fn spend(&mut self, cost: PrivacyParams) -> Result<()> {
        // A tiny tolerance absorbs floating-point summation error at the boundary.
        const TOL: f64 = 1e-9;
        let new_eps = self.spent.epsilon + cost.epsilon;
        let new_delta = self.spent.delta + cost.delta;
        if new_eps > self.budget.epsilon + TOL || new_delta > self.budget.delta + TOL {
            return Err(SketchError::InvalidParameter {
                param: "privacy_budget".to_string(),
                value: format!("(ε={new_eps:.6}, δ={new_delta:.3e})"),
                constraint: format!(
                    "would exceed budget (ε={:.6}, δ={:.3e})",
                    self.budget.epsilon, self.budget.delta
                ),
            });
        }
        self.spent = PrivacyParams {
            epsilon: new_eps,
            delta: new_delta,
        };
        Ok(())
    }

    /// Whether a spend of `cost` would currently be permitted (no state change).
    pub fn can_spend(&self, cost: PrivacyParams) -> bool {
        const TOL: f64 = 1e-9;
        self.spent.epsilon + cost.epsilon <= self.budget.epsilon + TOL
            && self.spent.delta + cost.delta <= self.budget.delta + TOL
    }

    /// The total budget.
    pub fn budget(&self) -> PrivacyParams {
        self.budget
    }

    /// The cumulative spend so far.
    pub fn spent(&self) -> PrivacyParams {
        self.spent
    }

    /// The remaining `(ε, δ)` headroom (clamped at 0).
    pub fn remaining(&self) -> PrivacyParams {
        PrivacyParams {
            epsilon: (self.budget.epsilon - self.spent.epsilon).max(0.0),
            delta: (self.budget.delta - self.spent.delta).max(0.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn params_validate() {
        assert!(PrivacyParams::new(-0.1, 0.0).is_err());
        assert!(PrivacyParams::new(1.0, 1.5).is_err());
        assert!(PrivacyParams::new(1.0, -0.01).is_err());
        assert!(PrivacyParams::pure(1.0).unwrap().delta == 0.0);
    }

    #[test]
    fn sequential_composition_sums() {
        let mut acct = Accountant::new(PrivacyParams::new(1.0, 1e-5).unwrap());
        acct.spend(PrivacyParams::new(0.3, 4e-6).unwrap()).unwrap();
        acct.spend(PrivacyParams::new(0.3, 4e-6).unwrap()).unwrap();
        assert!((acct.spent().epsilon - 0.6).abs() < 1e-9);
        assert!((acct.spent().delta - 8e-6).abs() < 1e-12);
        assert!((acct.remaining().epsilon - 0.4).abs() < 1e-9);
    }

    #[test]
    fn refuses_over_budget_without_applying() {
        let mut acct = Accountant::new(PrivacyParams::pure(1.0).unwrap());
        acct.spend(PrivacyParams::pure(0.8).unwrap()).unwrap();
        assert!(acct.spend(PrivacyParams::pure(0.5).unwrap()).is_err());
        // The failed spend must not have been applied.
        assert!((acct.spent().epsilon - 0.8).abs() < 1e-9);
        assert!(acct.can_spend(PrivacyParams::pure(0.2).unwrap()));
        assert!(!acct.can_spend(PrivacyParams::pure(0.3).unwrap()));
    }

    #[test]
    fn delta_budget_enforced_independently() {
        let mut acct = Accountant::new(PrivacyParams::new(10.0, 1e-6).unwrap());
        // Epsilon has lots of room, but delta is tight.
        acct.spend(PrivacyParams::new(0.1, 9e-7).unwrap()).unwrap();
        assert!(acct.spend(PrivacyParams::new(0.1, 2e-7).unwrap()).is_err());
    }
}
