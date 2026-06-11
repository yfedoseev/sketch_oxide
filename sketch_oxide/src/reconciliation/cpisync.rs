//! CPISync — Characteristic Polynomial Interpolation set reconciliation.
//!
//! CPISync (Minsky, Trachtenberg & Zippel, "Set Reconciliation with Nearly Optimal Communication
//! Complexity", IEEE Trans. Inf. Theory 2003) reconciles two sets of integers using communication
//! proportional only to the size of their **symmetric difference**, not the sets. Each party encodes
//! its set as the **power sums** `Σ_{a∈A} a^k` (over a prime field) for `k = 1 … 2m`; subtracting the
//! two parties' power sums gives the *signed* power sums of the difference,
//! `Σ_{d∈A\B} d^k − Σ_{e∈B\A} e^k`, from which the difference elements are recovered as the roots of
//! a small "characteristic" polynomial — found by Berlekamp–Massey, exactly as in BCH decoding.
//! Only `2m` field elements cross the wire regardless of set size.
//!
//! This is the GF(p) analogue of [`PinSketch`](crate::reconciliation::PinSketch) (which works in
//! GF(2^b)), with the +1/−1 multiplicities that distinguish the two sides. The function here models
//! the protocol in-process (both sets available); a distributed deployment exchanges only the `2m`
//! power sums and recovers roots over the element universe.

use crate::common::{Result, SketchError};
use std::collections::HashSet;

/// A Mersenne prime field `GF(2^31 − 1)`; set elements must be in `0 .. p − 1`.
const P: u64 = (1 << 31) - 1;

/// The symmetric difference recovered by [`CpiSync::reconcile`], as sorted element lists.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CpiDiff {
    /// Elements in the first set but not the second.
    pub a_only: Vec<u64>,
    /// Elements in the second set but not the first.
    pub b_only: Vec<u64>,
}

/// A CPISync reconciler that can recover symmetric differences of up to `capacity` elements.
///
/// # Example
/// ```
/// use sketch_oxide::reconciliation::CpiSync;
///
/// let cpi = CpiSync::new(8).unwrap();
/// let a = [1u64, 2, 3, 4, 5, 100];
/// let b = [1u64, 2, 3, 4, 5, 200];
/// let diff = cpi.reconcile(&a, &b).unwrap();
/// assert_eq!(diff.a_only, vec![100]);
/// assert_eq!(diff.b_only, vec![200]);
/// ```
#[derive(Debug, Clone)]
pub struct CpiSync {
    capacity: usize,
}

impl CpiSync {
    /// Creates a reconciler for symmetric differences of up to `capacity` elements.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `capacity` is 0.
    pub fn new(capacity: usize) -> Result<Self> {
        if capacity == 0 {
            return Err(SketchError::InvalidParameter {
                param: "capacity".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self { capacity })
    }

    #[inline]
    fn mul(a: u64, b: u64) -> u64 {
        a * b % P
    }

    #[inline]
    fn add(a: u64, b: u64) -> u64 {
        (a + b) % P
    }

    #[inline]
    fn sub(a: u64, b: u64) -> u64 {
        (a + P - b % P) % P
    }

    fn pow(mut a: u64, mut e: u64) -> u64 {
        let mut r = 1u64;
        a %= P;
        while e != 0 {
            if e & 1 == 1 {
                r = Self::mul(r, a);
            }
            a = Self::mul(a, a);
            e >>= 1;
        }
        r
    }

    #[inline]
    fn inv(a: u64) -> u64 {
        Self::pow(a, P - 2)
    }

    /// Power sums `Σ x^k` for `k = 1 ..= 2·capacity`, with elements shifted by 1 to avoid the
    /// power-sum blind spot at 0.
    fn power_sums(&self, set: &[u64]) -> Vec<u64> {
        let n = 2 * self.capacity;
        let mut sums = vec![0u64; n];
        for &x in set {
            let v = (x % P + 1) % P; // shift by 1 so element 0 is detectable
            let mut pw = v;
            for s in sums.iter_mut() {
                *s = Self::add(*s, pw);
                pw = Self::mul(pw, v);
            }
        }
        sums
    }

    /// Berlekamp–Massey over GF(p): the connection polynomial (low-order first) of the sequence.
    fn berlekamp_massey(s: &[u64]) -> Vec<u64> {
        let mut cur = vec![1u64];
        let mut prev = vec![1u64];
        let mut l = 0usize;
        let mut m = 1usize;
        let mut b = 1u64;
        for i in 0..s.len() {
            let mut d = s[i];
            for j in 1..=l {
                d = Self::add(d, Self::mul(cur[j], s[i - j]));
            }
            if d == 0 {
                m += 1;
            } else {
                let coef = Self::mul(d, Self::inv(b));
                if cur.len() < prev.len() + m {
                    cur.resize(prev.len() + m, 0);
                }
                let snapshot = cur.clone();
                for (j, &pv) in prev.iter().enumerate() {
                    cur[j + m] = Self::sub(cur[j + m], Self::mul(coef, pv));
                }
                if 2 * l <= i {
                    l = i + 1 - l;
                    prev = snapshot;
                    b = d;
                    m = 1;
                } else {
                    m += 1;
                }
            }
        }
        // The connection polynomial accumulates trailing zeros from the x^m shifts; trim them so the
        // degree reflects the true recurrence order.
        while cur.len() > 1 && *cur.last().unwrap() == 0 {
            cur.pop();
        }
        cur
    }

    /// Evaluates a polynomial (low-order first) at `x`.
    fn eval(coeffs: &[u64], x: u64) -> u64 {
        let mut acc = 0u64;
        for &c in coeffs.iter().rev() {
            acc = Self::add(Self::mul(acc, x), c);
        }
        acc
    }

    /// Reconciles two integer sets, returning their symmetric difference.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if any element is `>= 2^31 − 1`.
    /// [`SketchError::ReconciliationError`] if the symmetric difference exceeds `capacity`.
    pub fn reconcile(&self, a: &[u64], b: &[u64]) -> Result<CpiDiff> {
        if a.iter().chain(b).any(|&x| x >= P) {
            return Err(SketchError::InvalidParameter {
                param: "element".to_string(),
                value: "too large".to_string(),
                constraint: format!("all elements must be < {P}"),
            });
        }
        // Signed power sums of the difference: power_sums(A) − power_sums(B).
        let pa = self.power_sums(a);
        let pb = self.power_sums(b);
        let s: Vec<u64> = pa.iter().zip(&pb).map(|(&x, &y)| Self::sub(x, y)).collect();

        // The locator's roots are the (shifted) difference elements.
        let locator = Self::berlekamp_massey(&s);
        let degree = locator.len() - 1;
        if degree > self.capacity {
            return Err(SketchError::ReconciliationError {
                reason: format!(
                    "symmetric difference exceeds capacity {} (locator degree {degree})",
                    self.capacity
                ),
            });
        }

        // Difference elements are roots of the locator and lie in A ∪ B; classify by membership in B.
        let a_set: HashSet<u64> = a.iter().copied().collect();
        let b_set: HashSet<u64> = b.iter().copied().collect();
        let mut a_only = Vec::new();
        let mut b_only = Vec::new();
        let mut roots_found = 0;
        let mut seen = HashSet::new();
        for &e in a.iter().chain(b) {
            if !seen.insert(e) {
                continue;
            }
            let shifted = (e % P + 1) % P;
            // A difference element d is the *inverse* of a locator root: the recurrence's
            // characteristic polynomial is the reciprocal of the connection polynomial, so check
            // C(1/d) = 0 (as in BCH decoding / PinSketch).
            if Self::eval(&locator, Self::inv(shifted)) == 0 {
                roots_found += 1;
                let in_a = a_set.contains(&e);
                let in_b = b_set.contains(&e);
                if in_a && !in_b {
                    a_only.push(e);
                } else if in_b && !in_a {
                    b_only.push(e);
                }
                // (in both ⇒ not a true difference; a spurious root signals over-capacity.)
            }
        }
        if roots_found != degree {
            return Err(SketchError::ReconciliationError {
                reason: format!(
                    "recovered {roots_found} roots for a degree-{degree} locator; difference likely \
                     exceeds capacity"
                ),
            });
        }
        a_only.sort_unstable();
        b_only.sort_unstable();
        Ok(CpiDiff { a_only, b_only })
    }

    /// Maximum recoverable symmetric-difference size.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_capacity() {
        assert!(CpiSync::new(0).is_err());
        assert!(CpiSync::new(8).is_ok());
    }

    #[test]
    fn identical_sets_have_no_difference() {
        let cpi = CpiSync::new(4).unwrap();
        let a: Vec<u64> = (0..1000).collect();
        let diff = cpi.reconcile(&a, &a).unwrap();
        assert!(diff.a_only.is_empty() && diff.b_only.is_empty());
    }

    #[test]
    fn recovers_small_difference_in_large_sets() {
        let cpi = CpiSync::new(8).unwrap();
        let a: Vec<u64> = (0..100_000).collect();
        let mut b = a.clone();
        b.retain(|&x| x != 5 && x != 42 && x != 99_999);
        b.extend([1_000_000, 2_000_000]);
        let diff = cpi.reconcile(&a, &b).unwrap();
        assert_eq!(diff.a_only, vec![5, 42, 99_999]);
        assert_eq!(diff.b_only, vec![1_000_000, 2_000_000]);
    }

    #[test]
    fn detects_element_zero() {
        // Element 0 is special (0^k = 0); the +1 shift must let it be recovered.
        let cpi = CpiSync::new(4).unwrap();
        let a = [0u64, 1, 2, 3];
        let b = [1u64, 2, 3];
        let diff = cpi.reconcile(&a, &b).unwrap();
        assert_eq!(diff.a_only, vec![0]);
        assert!(diff.b_only.is_empty());
    }

    #[test]
    fn disjoint_sets() {
        let cpi = CpiSync::new(8).unwrap();
        let a: Vec<u64> = (0..4).collect();
        let b: Vec<u64> = (100..104).collect();
        let diff = cpi.reconcile(&a, &b).unwrap();
        assert_eq!(diff.a_only, vec![0, 1, 2, 3]);
        assert_eq!(diff.b_only, vec![100, 101, 102, 103]);
    }

    #[test]
    fn over_capacity_is_reported() {
        let cpi = CpiSync::new(2).unwrap();
        let a: Vec<u64> = (0..1000).collect();
        let b: Vec<u64> = (0..1000).filter(|&x| x % 100 != 0).collect(); // 10 differences
        assert!(matches!(
            cpi.reconcile(&a, &b),
            Err(SketchError::ReconciliationError { .. })
        ));
    }

    #[test]
    fn rejects_out_of_range_elements() {
        let cpi = CpiSync::new(4).unwrap();
        assert!(cpi.reconcile(&[P], &[1]).is_err());
    }
}
