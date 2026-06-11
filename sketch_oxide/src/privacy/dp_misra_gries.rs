//! Differentially private heavy hitters via the Misra–Gries sketch (Lebeda & Tětek, PODS 2023).
//!
//! A Misra–Gries sketch finds the `k` approximate heavy hitters of a stream, but releasing its
//! counters directly leaks individual records: the sketch's `ℓ1`-sensitivity is `k` (one record can
//! shift every counter), so the naive global-sensitivity mechanism must add `Θ(k/ε)` noise to each
//! count. Lebeda & Tětek ("Better Differentially Private Approximate Histograms and Heavy Hitters
//! using the Misra-Gries Sketch") reduce this to `O(1/ε)` by exploiting the *structure* of the
//! sketch: represented as `(counts − mean, mean)`, two neighbouring sketches differ by less than 2 in
//! `ℓ1`. Concretely they release each counter as
//!
//! ```text
//! ĉ_x = c_x + Lap_x + Lap_shared
//! ```
//!
//! where `Lap_x` is independent per-counter noise and `Lap_shared` is a *single* value added to every
//! counter, both of scale `1/ε`, and then drop counters below a threshold `τ` (so absent keys are not
//! revealed). This module uses the **discrete** Laplace (two-sided geometric) mechanism — the
//! attack-resistant integer variant the paper recommends in §5.2 — with the corresponding threshold
//! `τ = 1 + 2⌈ln(6e^ε / ((e^ε + 1)·δ))⌉ / ε`, giving `(ε, δ)`-differential privacy.
//!
//! As with every mechanism here, pass a CSPRNG ([`secure_rng`](crate::privacy::mechanisms::secure_rng))
//! in production; tests may seed for reproducibility.

use crate::common::{Result, SketchError};
use crate::privacy::mechanisms::discrete_laplace;
use rand::Rng;
use std::collections::HashMap;
use std::hash::Hash;

/// A Misra–Gries heavy-hitter sketch with an `(ε, δ)`-differentially private release.
///
/// # Example
/// ```
/// use sketch_oxide::privacy::DpMisraGries;
/// use rand::{rngs::StdRng, SeedableRng};
///
/// let mut rng = StdRng::seed_from_u64(7);
/// let mut mg = DpMisraGries::new(64, 1.0, 1e-5).unwrap();
/// // Item 0 is a clear heavy hitter; the rest is a light tail.
/// for i in 0..100_000u64 {
///     mg.update(if i < 40_000 { 0 } else { 1 + i % 5000 });
/// }
/// let released = mg.release(&mut rng);
/// // The heavy hitter survives the noise and threshold. The count is the Misra-Gries estimate
/// // (which underestimates by up to N/k) plus small DP noise.
/// let (_, est) = released.iter().find(|(k, _)| *k == 0).expect("heavy hitter present");
/// assert!(*est > 38_000 && *est < 41_000, "estimate {est}");
/// ```
#[derive(Debug, Clone)]
pub struct DpMisraGries<T: Hash + Eq + Clone> {
    k: usize,
    epsilon: f64,
    delta: f64,
    /// Misra–Gries counters (every stored count is `≥ 1`).
    counters: HashMap<T, i64>,
}

impl<T: Hash + Eq + Clone> DpMisraGries<T> {
    /// Creates a sketch with `k` counters at privacy level `(epsilon, delta)`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `k == 0`, `epsilon` is not positive and finite, or `delta`
    /// is not in `(0, 1)`.
    pub fn new(k: usize, epsilon: f64, delta: f64) -> Result<Self> {
        if k == 0 {
            return Err(SketchError::InvalidParameter {
                param: "k".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        if !(epsilon.is_finite() && epsilon > 0.0) {
            return Err(SketchError::InvalidParameter {
                param: "epsilon".to_string(),
                value: epsilon.to_string(),
                constraint: "must be a positive finite number".to_string(),
            });
        }
        if !(delta > 0.0 && delta < 1.0) {
            return Err(SketchError::InvalidParameter {
                param: "delta".to_string(),
                value: delta.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }
        Ok(Self {
            k,
            epsilon,
            delta,
            counters: HashMap::new(),
        })
    }

    /// Records one occurrence of `item` (standard Misra–Gries update).
    pub fn update(&mut self, item: T) {
        if let Some(c) = self.counters.get_mut(&item) {
            *c += 1;
        } else if self.counters.len() < self.k {
            self.counters.insert(item, 1);
        } else {
            // Decrement every counter; drop those that reach zero (the new item is absorbed).
            self.counters.retain(|_, c| {
                *c -= 1;
                *c > 0
            });
        }
    }

    /// The drop threshold `τ = 1 + 2⌈ln(6e^ε/((e^ε+1)δ))⌉/ε` for the discrete mechanism (§5.2).
    fn threshold(&self) -> f64 {
        let e = self.epsilon.exp();
        let inner = (6.0 * e / ((e + 1.0) * self.delta)).ln().ceil();
        1.0 + 2.0 * inner / self.epsilon
    }

    /// Releases the `(ε, δ)`-differentially private heavy hitters as `(item, noisy_count)` pairs.
    ///
    /// Each counter gets independent discrete-Laplace`(1/ε)` noise plus one shared sample; counters
    /// whose noisy value falls below the threshold are suppressed. Pass a CSPRNG in production.
    pub fn release<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec<(T, i64)> {
        let scale = 1.0 / self.epsilon;
        let shared = discrete_laplace(rng, scale).expect("epsilon validated > 0");
        let tau = self.threshold();
        let mut out = Vec::new();
        for (item, &c) in &self.counters {
            let noisy = c + discrete_laplace(rng, scale).expect("epsilon validated > 0") + shared;
            if noisy as f64 >= tau {
                out.push((item.clone(), noisy));
            }
        }
        out
    }

    /// The raw (non-private) Misra–Gries counts. Exposed for testing/comparison — do **not** release
    /// these directly.
    pub fn raw_counters(&self) -> &HashMap<T, i64> {
        &self.counters
    }

    /// Number of counters `k`.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.k
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn rejects_bad_params() {
        assert!(DpMisraGries::<u64>::new(0, 1.0, 1e-5).is_err());
        assert!(DpMisraGries::<u64>::new(64, 0.0, 1e-5).is_err());
        assert!(DpMisraGries::<u64>::new(64, 1.0, 0.0).is_err());
        assert!(DpMisraGries::<u64>::new(64, 1.0, 1.0).is_err());
        assert!(DpMisraGries::<u64>::new(64, 1.0, 1e-5).is_ok());
    }

    #[test]
    fn misra_gries_keeps_heavy_hitters() {
        // The underlying (non-private) sketch must retain dominant items.
        let mut mg = DpMisraGries::new(64, 1.0, 1e-5).unwrap();
        for i in 0..100_000u64 {
            mg.update(if i < 50_000 { 0 } else { 1 + i % 5000 });
        }
        let c0 = *mg.raw_counters().get(&0).expect("heavy hitter tracked");
        // Misra-Gries underestimates by at most N/k; here that is well under 50_000.
        assert!(c0 > 49_000, "raw count {c0}");
    }

    #[test]
    fn private_release_recovers_heavy_hitter() {
        let mut rng = StdRng::seed_from_u64(11);
        let mut mg = DpMisraGries::new(64, 2.0, 1e-5).unwrap();
        for i in 0..120_000u64 {
            mg.update(if i < 60_000 { 0 } else { 1 + i % 5000 });
        }
        let released = mg.release(&mut rng);
        let hit = released.iter().find(|(k, _)| *k == 0);
        assert!(hit.is_some(), "heavy hitter dropped");
        let (_, est) = hit.unwrap();
        // The count is the Misra-Gries estimate (underestimates by up to N/k = 120_000/64 ≈ 1875)
        // plus small DP noise (ε = 2 ⇒ scale 0.5).
        assert!(*est > 57_500 && *est < 60_100, "estimate {est}");
    }

    #[test]
    fn threshold_grows_as_delta_shrinks() {
        let loose = DpMisraGries::<u64>::new(64, 1.0, 1e-2).unwrap();
        let tight = DpMisraGries::<u64>::new(64, 1.0, 1e-8).unwrap();
        assert!(tight.threshold() > loose.threshold());
        // A smaller ε (more privacy) also raises the threshold.
        let small_eps = DpMisraGries::<u64>::new(64, 0.5, 1e-2).unwrap();
        assert!(small_eps.threshold() > loose.threshold());
    }

    #[test]
    fn suppresses_tiny_counts() {
        // With only light items (each count 1) and a tiny δ, the threshold suppresses everything
        // w.h.p. — released keys, if any, must come from the input.
        let mut rng = StdRng::seed_from_u64(3);
        let mut mg = DpMisraGries::new(64, 1.0, 1e-9).unwrap();
        for i in 0..1000u64 {
            mg.update(i); // all distinct, count 1 each
        }
        let released = mg.release(&mut rng);
        for (k, _) in &released {
            assert!(*k < 1000, "released a key not in the input");
        }
        assert!(
            released.len() < 10,
            "too many tiny counts survived: {}",
            released.len()
        );
    }
}
