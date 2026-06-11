//! ProbMinHash — locality-sensitive hashing for the probability Jaccard similarity (Ertl, 2019).
//!
//! [`WeightedMinHash`](crate::similarity::WeightedMinHash) (ICWS) targets the *generalized* (weighted)
//! Jaccard. ProbMinHash (Otmar Ertl, "ProbMinHash – A Class of Locality-Sensitive Hash Algorithms for
//! the (Probability) Jaccard Similarity", IEEE TKDE 2019) targets the **probability Jaccard**
//! `J_P` — the natural similarity of two discrete probability distributions — and, for binary weights,
//! reduces exactly to ordinary MinHash. Each element `d` with weight `w(d)` emits an ascending Poisson
//! process of "hash points" `h` at rate `w(d)` (spacings `Exp(1)/w(d)`); each point lands in a uniformly
//! random register and claims it if it beats the register's current minimum. A point beyond the current
//! maximum-of-minima `q_max` can improve nothing, so processing of an element stops there — the early
//! termination that makes the expected cost `O(1)` per element when inputs are balanced.
//!
//! The signature records, per register, *which element* won it; two signatures agree on a register
//! with probability exactly `J_P`, so the fraction of agreeing registers is an unbiased estimate. This
//! is the ProbMinHash1 variant (Algorithm 5 in the paper), transcribed faithfully.

use crate::common::{Result, SketchError};
use rand::{Rng, SeedableRng};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// A ProbMinHash1 signer over `m` registers for weighted elements of type `T`.
///
/// # Example
/// ```
/// use sketch_oxide::similarity::ProbMinHash;
///
/// // Binary weights ⇒ probability Jaccard equals the ordinary Jaccard.
/// let mut a = ProbMinHash::new(512).unwrap();
/// let mut b = ProbMinHash::new(512).unwrap();
/// for i in 0..1000u64 { a.add(i, 1.0); }       // A = {0..1000}
/// for i in 500..1500u64 { b.add(i, 1.0); }     // B = {500..1500}
///
/// // True Jaccard = 500/1500 ≈ 0.333.
/// let est = a.jaccard(&b).unwrap();
/// assert!((est - 0.3333).abs() < 0.06, "jaccard {est}");
/// ```
#[derive(Debug, Clone)]
pub struct ProbMinHash<T: Hash + Eq + Clone> {
    m: usize,
    /// Per-register minimum hash value.
    q: Vec<f64>,
    /// Per-register winning element.
    z: Vec<Option<T>>,
    /// Maximum of the register minima (`max q_k`) — the stop limit.
    q_max: f64,
}

impl<T: Hash + Eq + Clone> ProbMinHash<T> {
    /// Creates a signer with `m` registers (larger ⇒ lower variance).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `m` is 0.
    pub fn new(m: usize) -> Result<Self> {
        if m == 0 {
            return Err(SketchError::InvalidParameter {
                param: "m".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            m,
            q: vec![f64::INFINITY; m],
            z: vec![None; m],
            q_max: f64::INFINITY,
        })
    }

    /// Adds element `item` with positive weight `weight`. Non-positive or non-finite weights are
    /// ignored.
    pub fn add(&mut self, item: T, weight: f64) {
        if !(weight.is_finite() && weight > 0.0) {
            return;
        }
        let w_inv = 1.0 / weight;
        let mut hasher = DefaultHasher::new();
        item.hash(&mut hasher);
        let mut rng = rand::rngs::SmallRng::seed_from_u64(hasher.finish());

        // First Poisson point: Exp(1)/w(d).
        let mut h = w_inv * exp1(&mut rng);
        while h < self.q_max {
            let k = rng.random_range(0..self.m);
            if h < self.q[k] {
                self.q[k] = h;
                self.z[k] = Some(item.clone());
                self.q_max = self.recompute_q_max();
                if h >= self.q_max {
                    break;
                }
            }
            h += w_inv * exp1(&mut rng);
        }
    }

    /// Recomputes `max_k q_k`.
    fn recompute_q_max(&self) -> f64 {
        self.q.iter().copied().fold(f64::NEG_INFINITY, f64::max)
    }

    /// Estimated probability Jaccard similarity with `other`: the fraction of registers won by the
    /// same element in both.
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the register counts differ.
    pub fn jaccard(&self, other: &Self) -> Result<f64> {
        if self.m != other.m {
            return Err(SketchError::IncompatibleSketches {
                reason: format!("register counts differ: {} vs {}", self.m, other.m),
            });
        }
        let matches = self
            .z
            .iter()
            .zip(&other.z)
            .filter(|(a, b)| a.is_some() && a == b)
            .count();
        Ok(matches as f64 / self.m as f64)
    }

    /// Merges `other` into `self` (register-wise minimum), giving the signature of the combined
    /// weighted set.
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the register counts differ.
    pub fn merge(&mut self, other: &Self) -> Result<()> {
        if self.m != other.m {
            return Err(SketchError::IncompatibleSketches {
                reason: format!("register counts differ: {} vs {}", self.m, other.m),
            });
        }
        for k in 0..self.m {
            if other.q[k] < self.q[k] {
                self.q[k] = other.q[k];
                self.z[k] = other.z[k].clone();
            }
        }
        self.q_max = self.recompute_q_max();
        Ok(())
    }

    /// Number of registers.
    #[inline]
    pub fn registers(&self) -> usize {
        self.m
    }
}

/// One `Exp(1)` variate.
#[inline]
fn exp1<R: Rng + ?Sized>(rng: &mut R) -> f64 {
    // 1 - U ∈ (0, 1], so ln is finite and the result is ≥ 0.
    -(1.0 - rng.random::<f64>()).ln()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signed(range: std::ops::Range<u64>, m: usize) -> ProbMinHash<u64> {
        let mut s = ProbMinHash::new(m).unwrap();
        for i in range {
            s.add(i, 1.0);
        }
        s
    }

    #[test]
    fn rejects_zero_registers() {
        assert!(ProbMinHash::<u64>::new(0).is_err());
        assert!(ProbMinHash::<u64>::new(256).is_ok());
    }

    #[test]
    fn identical_sets() {
        let a = signed(0..500, 256);
        let b = signed(0..500, 256);
        assert!((a.jaccard(&b).unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn disjoint_sets() {
        let a = signed(0..500, 256);
        let b = signed(10_000..10_500, 256);
        assert!(a.jaccard(&b).unwrap() < 0.03);
    }

    #[test]
    fn binary_weights_match_ordinary_jaccard() {
        let a = signed(0..1000, 512);
        let b = signed(500..1500, 512);
        // True Jaccard = 500/1500 ≈ 0.333.
        let est = a.jaccard(&b).unwrap();
        assert!((est - 0.3333).abs() < 0.05, "jaccard {est}");
    }

    #[test]
    fn order_independent() {
        let a = signed(0..400, 128);
        let mut b = ProbMinHash::new(128).unwrap();
        for i in (0..400u64).rev() {
            b.add(i, 1.0);
        }
        assert!((a.jaccard(&b).unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn weighted_similarity_reflects_overlap() {
        // Two weighted sets sharing the heavy element should be highly similar; differing heavy
        // elements should be dissimilar.
        let mut a = ProbMinHash::new(512).unwrap();
        a.add(0u64, 100.0);
        a.add(1, 1.0);
        let mut b = ProbMinHash::new(512).unwrap();
        b.add(0u64, 100.0);
        b.add(2, 1.0);
        // Both dominated by element 0 (weight 100) ⇒ J_P close to 1.
        assert!(
            a.jaccard(&b).unwrap() > 0.9,
            "sim {}",
            a.jaccard(&b).unwrap()
        );

        let mut c = ProbMinHash::new(512).unwrap();
        c.add(5u64, 100.0);
        c.add(6, 1.0);
        // Disjoint heavy elements ⇒ low similarity.
        assert!(
            a.jaccard(&c).unwrap() < 0.1,
            "sim {}",
            a.jaccard(&c).unwrap()
        );
    }

    #[test]
    fn merge_is_union() {
        let mut a = signed(0..1000, 256);
        let b = signed(500..1500, 256);
        a.merge(&b).unwrap();
        let union = signed(0..1500, 256);
        assert!((a.jaccard(&union).unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn incompatible_sizes_error() {
        let a = ProbMinHash::<u64>::new(128).unwrap();
        let b = ProbMinHash::<u64>::new(256).unwrap();
        assert!(a.jaccard(&b).is_err());
    }
}
