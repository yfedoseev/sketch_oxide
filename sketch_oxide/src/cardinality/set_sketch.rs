//! SetSketch — a single sketch for both cardinality and Jaccard similarity (Ertl, VLDB 2021).
//!
//! [`HyperLogLog`](crate::cardinality::HyperLogLog) estimates set cardinality; MinHash estimates
//! Jaccard similarity; SetSketch (Otmar Ertl, "SetSketch: Filling the Gap between MinHash and
//! HyperLogLog") does **both** in one mergeable structure that interpolates between them via a base
//! parameter `b` (`b → 1` behaves like MinHash, `b = 2` like HyperLogLog). Each register `K_i` holds
//! `max_{d∈S} ⌊1 − log_b(h_i(d))⌋` for exponentially-distributed hashes `h_i(d) ~ Exp(a)`; larger sets
//! push the registers higher.
//!
//! Inserts use the paper's Algorithm 1 (the SetSketch1 variant, with exponential spacings): an element
//! emits an ascending exponential point process, each point updating a register drawn without
//! replacement, stopping once a point can no longer beat the running lower bound `K_low` — giving
//! `O(1)` amortized inserts for large sets. Cardinality uses the closed-form estimator
//! `n̂ = m(1 − 1/b) / (a·ln(b)·Σ_i b^{−K_i})` (Eq. 12); Jaccard uses inclusion–exclusion over the
//! mergeable cardinality estimates (Eq. 13). Faithfully transcribed from the paper.

use crate::common::{Result, SketchError};
use rand::{Rng, SeedableRng};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// A SetSketch over `m` registers with base `b`, rate `a`, and maximum register value `q`.
///
/// # Example
/// ```
/// use sketch_oxide::cardinality::SetSketch;
///
/// let mut s = SetSketch::new(4096).unwrap();
/// for i in 0..100_000u64 {
///     s.add(&i);
/// }
/// let est = s.estimate_cardinality();
/// assert!((est - 100_000.0).abs() < 0.1 * 100_000.0, "cardinality {est}");
/// ```
#[derive(Debug, Clone)]
pub struct SetSketch {
    m: usize,
    b: f64,
    a: f64,
    q: i64,
    /// Register values `K_1..K_m`.
    k: Vec<i64>,
    /// Lower bound `min_i K_i` (tracked lazily for early termination).
    k_low: i64,
    /// Register updates since the last `k_low` refresh.
    w: usize,
    /// Lazy Fisher–Yates permutation buffer and per-element version markers.
    perm: Vec<usize>,
    perm_ver: Vec<i64>,
    /// Monotonic element counter (the Fisher–Yates version marker).
    counter: i64,
}

impl SetSketch {
    /// Creates a SetSketch with `m` registers and default parameters (`b = 2`, `a = 20`,
    /// `q = 2^16 − 2`), suitable for cardinalities up to ~`10^18`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `m` is 0.
    pub fn new(m: usize) -> Result<Self> {
        Self::with_params(m, 2.0, 20.0, 65534)
    }

    /// Creates a SetSketch with explicit parameters: `m` registers, base `b > 1`, rate `a > 0`, and
    /// maximum register value `q ≥ 1`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `m == 0`, `b <= 1`, `a <= 0`, or `q < 1`.
    pub fn with_params(m: usize, b: f64, a: f64, q: i64) -> Result<Self> {
        if m == 0 {
            return Err(SketchError::InvalidParameter {
                param: "m".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        if !(b.is_finite() && b > 1.0) {
            return Err(SketchError::InvalidParameter {
                param: "b".to_string(),
                value: b.to_string(),
                constraint: "must be a finite number > 1".to_string(),
            });
        }
        if !(a.is_finite() && a > 0.0) {
            return Err(SketchError::InvalidParameter {
                param: "a".to_string(),
                value: a.to_string(),
                constraint: "must be a positive finite number".to_string(),
            });
        }
        if q < 1 {
            return Err(SketchError::InvalidParameter {
                param: "q".to_string(),
                value: q.to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        Ok(Self {
            m,
            b,
            a,
            q,
            k: vec![0i64; m],
            k_low: 0,
            w: 0,
            perm: vec![0usize; m],
            perm_ver: vec![-1i64; m],
            counter: 0,
        })
    }

    /// Adds one element to the set.
    pub fn add<T: Hash>(&mut self, item: &T) {
        let elem = self.counter;
        self.counter += 1;
        let mut hasher = DefaultHasher::new();
        item.hash(&mut hasher);
        let mut rng = rand::rngs::SmallRng::seed_from_u64(hasher.finish());

        let mut x = 0.0f64;
        let mut thresh = self.b.powf(-(self.k_low as f64)); // b^{-K_low}
        let ln_b = self.b.ln();
        for jj in 0..self.m {
            // x_j = x_{j-1} + Exp(1) / ((m - jj) · a)  — ascending Exp(a) order statistics.
            x += exp1(&mut rng) / ((self.m - jj) as f64 * self.a);
            if x > thresh {
                break;
            }
            // k = clamp(⌊1 − log_b(x)⌋, 0, q+1)
            let raw = (1.0 - x.ln() / ln_b).floor();
            let kval = raw.clamp(0.0, (self.q + 1) as f64) as i64;
            if kval <= self.k_low {
                break;
            }
            // Draw a register without replacement (lazy Fisher–Yates step jj).
            let r = rng.random_range(jj..self.m);
            if self.perm_ver[jj] != elem {
                self.perm[jj] = jj;
                self.perm_ver[jj] = elem;
            }
            if self.perm_ver[r] != elem {
                self.perm[r] = r;
                self.perm_ver[r] = elem;
            }
            self.perm.swap(jj, r);
            let reg = self.perm[jj];
            if kval > self.k[reg] {
                self.k[reg] = kval;
                self.w += 1;
                if self.w >= self.m {
                    self.k_low = *self.k.iter().min().unwrap();
                    self.w = 0;
                    thresh = self.b.powf(-(self.k_low as f64));
                }
            }
        }
    }

    /// Closed-form cardinality estimate `n̂ = m(1 − 1/b) / (a·ln(b)·Σ_i b^{−K_i})` (Eq. 12).
    pub fn estimate_cardinality(&self) -> f64 {
        let sum: f64 = self.k.iter().map(|&ki| self.b.powf(-(ki as f64))).sum();
        if sum <= 0.0 {
            return f64::INFINITY;
        }
        self.m as f64 * (1.0 - 1.0 / self.b) / (self.a * self.b.ln() * sum)
    }

    /// Whether `other` has matching parameters (`m`, `b`, `a`, `q`).
    fn compatible(&self, other: &Self) -> bool {
        self.m == other.m && self.b == other.b && self.a == other.a && self.q == other.q
    }

    /// Merges `other` into `self` (register-wise maximum = set union).
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the parameters differ.
    pub fn merge(&mut self, other: &Self) -> Result<()> {
        if !self.compatible(other) {
            return Err(SketchError::IncompatibleSketches {
                reason: "SetSketch parameters (m, b, a, q) differ".to_string(),
            });
        }
        for (x, &y) in self.k.iter_mut().zip(&other.k) {
            if y > *x {
                *x = y;
            }
        }
        self.k_low = *self.k.iter().min().unwrap();
        self.w = 0;
        Ok(())
    }

    /// Estimated Jaccard similarity with `other` via inclusion–exclusion over the cardinality
    /// estimates: `Ĵ = (n̂_A + n̂_B − n̂_{A∪B}) / n̂_{A∪B}` (Eq. 13).
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the parameters differ.
    pub fn jaccard(&self, other: &Self) -> Result<f64> {
        if !self.compatible(other) {
            return Err(SketchError::IncompatibleSketches {
                reason: "SetSketch parameters (m, b, a, q) differ".to_string(),
            });
        }
        let n_a = self.estimate_cardinality();
        let n_b = other.estimate_cardinality();
        let mut union = self.clone();
        union.merge(other)?;
        let n_u = union.estimate_cardinality();
        if n_u <= 0.0 || !n_u.is_finite() {
            return Ok(0.0);
        }
        Ok(((n_a + n_b - n_u) / n_u).clamp(0.0, 1.0))
    }

    /// Number of registers.
    #[inline]
    pub fn registers(&self) -> usize {
        self.m
    }

    /// Base `b`.
    #[inline]
    pub fn base(&self) -> f64 {
        self.b
    }
}

/// One `Exp(1)` variate.
#[inline]
fn exp1<R: Rng + ?Sized>(rng: &mut R) -> f64 {
    -(1.0 - rng.random::<f64>()).ln()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(range: std::ops::Range<u64>) -> SetSketch {
        let mut s = SetSketch::new(4096).unwrap();
        for i in range {
            s.add(&i);
        }
        s
    }

    #[test]
    fn rejects_bad_params() {
        assert!(SetSketch::with_params(0, 2.0, 20.0, 100).is_err());
        assert!(SetSketch::with_params(64, 1.0, 20.0, 100).is_err());
        assert!(SetSketch::with_params(64, 2.0, 0.0, 100).is_err());
        assert!(SetSketch::with_params(64, 2.0, 20.0, 0).is_err());
        assert!(SetSketch::new(64).is_ok());
    }

    #[test]
    fn empty_cardinality_near_zero() {
        let s = SetSketch::new(4096).unwrap();
        assert!(s.estimate_cardinality() < 1.0);
    }

    #[test]
    fn estimates_large_cardinality() {
        let s = build(0..100_000);
        let est = s.estimate_cardinality();
        assert!(
            (est - 100_000.0).abs() < 0.1 * 100_000.0,
            "cardinality {est}"
        );
    }

    #[test]
    fn estimates_small_cardinality() {
        let s = build(0..1000);
        let est = s.estimate_cardinality();
        assert!((est - 1000.0).abs() < 0.15 * 1000.0, "cardinality {est}");
    }

    #[test]
    fn merge_is_union_cardinality() {
        let mut a = build(0..50_000);
        let b = build(50_000..100_000);
        a.merge(&b).unwrap();
        let est = a.estimate_cardinality();
        assert!((est - 100_000.0).abs() < 0.1 * 100_000.0, "union {est}");
    }

    #[test]
    fn estimates_jaccard() {
        let a = build(0..100_000);
        let b = build(50_000..150_000);
        // |∩| = 50_000, |∪| = 150_000 ⇒ J = 1/3.
        let est = a.jaccard(&b).unwrap();
        assert!((est - 0.3333).abs() < 0.08, "jaccard {est}");
    }

    #[test]
    fn jaccard_extremes() {
        let a = build(0..20_000);
        let b = build(0..20_000);
        assert!(a.jaccard(&b).unwrap() > 0.9, "identical jaccard");
        let c = build(100_000..120_000);
        assert!(a.jaccard(&c).unwrap() < 0.1, "disjoint jaccard");
    }

    #[test]
    fn incompatible_params_error() {
        let mut a = SetSketch::new(64).unwrap();
        let b = SetSketch::with_params(64, 1.5, 20.0, 100).unwrap();
        assert!(a.jaccard(&b).is_err());
        assert!(a.merge(&b).is_err());
    }
}

/// Capability-trait adoptions (see `crate::common::capabilities`).
mod capability_impls {
    use super::*;
    use crate::common::capabilities::{CardinalityEstimate, Update};
    use std::hash::Hash;

    impl<T: Hash> Update<T> for SetSketch {
        fn update(&mut self, item: &T) {
            self.add(item);
        }
    }

    impl CardinalityEstimate for SetSketch {
        fn estimate_cardinality(&self) -> f64 {
            // Disambiguate from the trait method of the same name.
            SetSketch::estimate_cardinality(self)
        }
    }
}
