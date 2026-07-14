//! SuperMinHash — lower-variance MinHash for Jaccard estimation (Ertl, 2017).
//!
//! Plain [`MinHash`](crate::similarity::MinHash) draws `m` *independent* minima, so its Jaccard
//! estimator has variance `J(1−J)/m`. SuperMinHash (Otmar Ertl, "SuperMinHash – A New Minwise Hashing
//! Algorithm for Jaccard Similarity Estimation", 2017) produces `m` registers whose collision
//! probabilities are each exactly the Jaccard index — so the estimator is still unbiased — but with
//! *negatively correlated* registers, cutting the variance by up to a factor of two (`J(1−J)/m ·
//! α(m, u)` with `α ≤ 1`). It is a single-pass streaming algorithm with the same sketch size as
//! MinHash.
//!
//! Each element is processed by a Fisher–Yates-style partial permutation that assigns it a value
//! `r + j` (level `j`, fraction `r ∈ [0,1)`) in one register at a time, keeping the per-register
//! minimum; a level histogram allows early termination once no register can improve. The estimator is
//! simply the fraction of registers two signatures agree on, exactly as in MinHash. This is the
//! optimized Algorithm 4 from the paper.

use crate::common::{Result, SketchError};
use rand::{Rng, SeedableRng};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// A SuperMinHash signer maintaining `m` registers.
///
/// # Example
/// ```
/// use sketch_oxide::similarity::SuperMinHash;
///
/// let mut a = SuperMinHash::new(256).unwrap();
/// let mut b = SuperMinHash::new(256).unwrap();
/// for i in 0..1000u64 { a.add(&i); }        // A = {0..1000}
/// for i in 500..1500u64 { b.add(&i); }      // B = {500..1500}
///
/// // True Jaccard = |∩|/|∪| = 500/1500 ≈ 0.333.
/// let est = a.jaccard(&b).unwrap();
/// assert!((est - 0.3333).abs() < 0.07, "jaccard {est}");
/// ```
#[derive(Debug, Clone)]
pub struct SuperMinHash {
    m: usize,
    /// Register values `h ∈ [0, m)` (∞ until set).
    h: Vec<f64>,
    /// Scratch permutation array.
    p: Vec<usize>,
    /// Version markers: `q[j] == i` means `p[j]` was set while processing element `i`.
    q: Vec<i64>,
    /// Level histogram: `b[k]` = registers whose `⌊h⌋ == k` (`b[m-1]` also counts ∞).
    b: Vec<i64>,
    /// Highest level that still has a register (early-termination bound).
    a: usize,
    /// Monotonic element counter (used only as a version marker).
    i: i64,
}

impl SuperMinHash {
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
        let mut b = vec![0i64; m];
        b[m - 1] = m as i64;
        Ok(Self {
            m,
            h: vec![f64::INFINITY; m],
            p: vec![0usize; m],
            q: vec![-1i64; m],
            b,
            a: m - 1,
            i: 0,
        })
    }

    /// Adds one element to the set being signed.
    pub fn add<T: Hash>(&mut self, item: &T) {
        let mut hasher = DefaultHasher::new();
        item.hash(&mut hasher);
        let mut rng = rand::rngs::SmallRng::seed_from_u64(hasher.finish());

        let i = self.i;
        let m = self.m;
        let mut j = 0usize;
        while j <= self.a {
            let r: f64 = rng.random::<f64>();
            let k: usize = rng.random_range(j..m);
            if self.q[j] != i {
                self.q[j] = i;
                self.p[j] = j;
            }
            if self.q[k] != i {
                self.q[k] = i;
                self.p[k] = k;
            }
            self.p.swap(j, k);
            let target = self.p[j];
            let value = r + j as f64;
            if value < self.h[target] {
                // Level the register currently occupies (m-1 if it was ∞).
                let j_prime = (self.h[target].floor() as usize).min(m - 1);
                self.h[target] = value;
                if j < j_prime {
                    self.b[j_prime] -= 1;
                    self.b[j] += 1;
                    while self.b[self.a] == 0 {
                        self.a -= 1;
                    }
                }
            }
            j += 1;
        }
        self.i += 1;
    }

    /// Estimated Jaccard similarity with `other`: the fraction of registers that agree.
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
            .h
            .iter()
            .zip(&other.h)
            .filter(|(x, y)| x == y && x.is_finite())
            .count();
        Ok(matches as f64 / self.m as f64)
    }

    /// Merges `other` into `self` (register-wise minimum), giving the signature of the set union.
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the register counts differ.
    pub fn merge(&mut self, other: &Self) -> Result<()> {
        if self.m != other.m {
            return Err(SketchError::IncompatibleSketches {
                reason: format!("register counts differ: {} vs {}", self.m, other.m),
            });
        }
        for (x, &y) in self.h.iter_mut().zip(&other.h) {
            if y < *x {
                *x = y;
            }
        }
        self.recompute_histogram();
        Ok(())
    }

    /// Rebuilds the level histogram and early-termination bound from the current register values
    /// (needed after a merge, so further [`add`](Self::add) calls stay correct).
    fn recompute_histogram(&mut self) {
        for slot in self.b.iter_mut() {
            *slot = 0;
        }
        for &v in &self.h {
            let level = if v.is_finite() {
                (v.floor() as usize).min(self.m - 1)
            } else {
                self.m - 1
            };
            self.b[level] += 1;
        }
        self.a = self.m - 1;
        while self.a > 0 && self.b[self.a] == 0 {
            self.a -= 1;
        }
        // Invalidate version markers so the next element re-initializes the permutation.
        for marker in self.q.iter_mut() {
            *marker = -1;
        }
        self.i = 0;
    }

    /// The raw register signature.
    pub fn signature(&self) -> &[f64] {
        &self.h
    }

    /// Number of registers.
    #[inline]
    pub fn registers(&self) -> usize {
        self.m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signed(range: std::ops::Range<u64>, m: usize) -> SuperMinHash {
        let mut s = SuperMinHash::new(m).unwrap();
        for i in range {
            s.add(&i);
        }
        s
    }

    #[test]
    fn rejects_zero_registers() {
        assert!(SuperMinHash::new(0).is_err());
        assert!(SuperMinHash::new(256).is_ok());
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
    fn estimates_jaccard() {
        let a = signed(0..1000, 512);
        let b = signed(500..1500, 512);
        // True Jaccard = 500/1500 ≈ 0.333.
        let est = a.jaccard(&b).unwrap();
        assert!((est - 0.3333).abs() < 0.05, "jaccard {est}");
    }

    #[test]
    fn order_independent() {
        // The signature must not depend on insertion order.
        let a = signed(0..400, 128);
        let mut b = SuperMinHash::new(128).unwrap();
        for i in (0..400u64).rev() {
            b.add(&i);
        }
        assert_eq!(a.signature(), b.signature());
    }

    #[test]
    fn merge_is_union() {
        let mut a = signed(0..1000, 256);
        let b = signed(500..1500, 256);
        a.merge(&b).unwrap();
        let union = signed(0..1500, 256);
        // Merged signature equals the union's signature register-for-register.
        assert!((a.jaccard(&union).unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn incompatible_sizes_error() {
        let a = SuperMinHash::new(128).unwrap();
        let b = SuperMinHash::new(256).unwrap();
        assert!(a.jaccard(&b).is_err());
    }
}

// ---------------------------------------------------------------------------
// Capability-trait adoptions (fable5 doc 01 F3 "split the `Sketch` trait").
// SuperMinHash ingests any hashable item via `add`, so it satisfies `Update`
// (delegating to `add`). It has no `Sketch` serialize and no cardinality/
// quantile/point/membership semantics (estimates Jaccard), so only `Update`
// applies.
// ---------------------------------------------------------------------------
use crate::common::Update;

impl<T: Hash> Update<T> for SuperMinHash {
    fn update(&mut self, item: &T) {
        self.add(item);
    }
}
