//! Weighted MinHash — Improved Consistent Weighted Sampling (ICWS).
//!
//! Plain MinHash estimates the Jaccard similarity of *unweighted* sets. Many real similarities are
//! **weighted** — term frequencies, traffic volumes, histogram bins — where the natural measure is
//! the *generalized* (weighted) Jaccard `Σ min(w_A, w_B) / Σ max(w_A, w_B)`. ICWS (Ioffe, "Improved
//! Consistent Sampling, Weighted Minhash and L1 Sketching", ICDM 2010) extends MinHash to this
//! setting: it draws, per hash and per element, a *consistent* sample such that two weighted sets
//! produce the same signature component with probability *exactly* their weighted Jaccard. So the
//! fraction of matching signature components is an unbiased estimate of the weighted Jaccard.
//!
//! A weighted set is a list of `(element_id, weight)` with positive weights; the signature is a
//! fixed-length vector of `(chosen_element, level)` pairs.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

/// A Weighted MinHash (ICWS) signer producing `num_hashes`-component signatures.
///
/// # Example
/// ```
/// use sketch_oxide::similarity::WeightedMinHash;
///
/// let wmh = WeightedMinHash::new(256).unwrap();
/// // A and B share elements 2 and 3 (weight 2 each); each has one unique element.
/// let a = [(1u64, 2.0), (2, 2.0), (3, 2.0)];
/// let b = [(2u64, 2.0), (3, 2.0), (4, 2.0)];
/// let est = wmh.jaccard(&wmh.signature(&a), &wmh.signature(&b));
/// // True weighted Jaccard = 4/8 = 0.5.
/// assert!((est - 0.5).abs() < 0.1, "weighted jaccard {est}");
/// ```
#[derive(Debug, Clone)]
pub struct WeightedMinHash {
    num_hashes: usize,
}

impl WeightedMinHash {
    /// Creates a signer with `num_hashes` signature components (more ⇒ lower variance).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `num_hashes` is 0.
    pub fn new(num_hashes: usize) -> Result<Self> {
        if num_hashes == 0 {
            return Err(SketchError::InvalidParameter {
                param: "num_hashes".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self { num_hashes })
    }

    /// A uniform `(0, 1)` value from element `i`, hash `k`, and a salt.
    #[inline]
    fn uniform(i: u64, k: usize, salt: u64) -> f64 {
        let mut buf = [0u8; 16];
        buf[..8].copy_from_slice(&i.to_le_bytes());
        buf[8..].copy_from_slice(&(k as u64).to_le_bytes());
        let h = xxhash(&buf, salt);
        let u = (h >> 11) as f64 / (1u64 << 53) as f64;
        u.clamp(f64::MIN_POSITIVE, 1.0 - f64::EPSILON)
    }

    /// A `Gamma(2, 1)` variate = sum of two `Exp(1)` = `−ln(u1·u2)`.
    #[inline]
    fn gamma2(i: u64, k: usize, salt: u64) -> f64 {
        let u1 = Self::uniform(i, k, salt);
        let u2 = Self::uniform(i, k, salt ^ 0xA5A5_A5A5_5A5A_5A5A);
        -(u1.ln() + u2.ln())
    }

    /// Computes the ICWS signature of a weighted set (positive weights only).
    pub fn signature(&self, weighted_set: &[(u64, f64)]) -> Vec<(u64, i64)> {
        let mut sig = vec![(0u64, 0i64); self.num_hashes];
        for (k, slot) in sig.iter_mut().enumerate() {
            let mut best_a = f64::INFINITY;
            let mut best = (0u64, 0i64);
            for &(i, w) in weighted_set {
                if w <= 0.0 || !w.is_finite() {
                    continue;
                }
                let r = Self::gamma2(i, k, 1);
                let c = Self::gamma2(i, k, 2);
                let beta = Self::uniform(i, k, 3);
                let t = (w.ln() / r + beta).floor();
                let y = (r * (t - beta)).exp();
                let z = y * r.exp();
                let a = c / z;
                if a < best_a {
                    best_a = a;
                    best = (i, t as i64);
                }
            }
            *slot = best;
        }
        sig
    }

    /// Estimated weighted Jaccard similarity: the fraction of matching signature components.
    pub fn jaccard(&self, a: &[(u64, i64)], b: &[(u64, i64)]) -> f64 {
        let n = a.len().min(b.len());
        if n == 0 {
            return 0.0;
        }
        let matches = a.iter().zip(b).filter(|(x, y)| x == y).count();
        matches as f64 / n as f64
    }

    /// Number of signature components.
    #[inline]
    pub fn num_hashes(&self) -> usize {
        self.num_hashes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exact generalized (weighted) Jaccard for cross-checking.
    fn true_weighted_jaccard(a: &[(u64, f64)], b: &[(u64, f64)]) -> f64 {
        use std::collections::BTreeMap;
        let mut wa: BTreeMap<u64, f64> = BTreeMap::new();
        let mut wb: BTreeMap<u64, f64> = BTreeMap::new();
        for &(i, w) in a {
            *wa.entry(i).or_insert(0.0) += w;
        }
        for &(i, w) in b {
            *wb.entry(i).or_insert(0.0) += w;
        }
        let mut keys: Vec<u64> = wa.keys().chain(wb.keys()).copied().collect();
        keys.sort_unstable();
        keys.dedup();
        let (mut num, mut den) = (0.0, 0.0);
        for k in keys {
            let x = *wa.get(&k).unwrap_or(&0.0);
            let y = *wb.get(&k).unwrap_or(&0.0);
            num += x.min(y);
            den += x.max(y);
        }
        if den == 0.0 {
            0.0
        } else {
            num / den
        }
    }

    #[test]
    fn rejects_zero_hashes() {
        assert!(WeightedMinHash::new(0).is_err());
        assert!(WeightedMinHash::new(128).is_ok());
    }

    #[test]
    fn identical_sets_match() {
        let wmh = WeightedMinHash::new(256).unwrap();
        let a = [(1u64, 3.0), (2, 1.5), (3, 7.0)];
        let est = wmh.jaccard(&wmh.signature(&a), &wmh.signature(&a));
        assert!(est > 0.99, "identical jaccard {est}");
    }

    #[test]
    fn disjoint_sets_dont_match() {
        let wmh = WeightedMinHash::new(256).unwrap();
        let a = [(1u64, 2.0), (2, 2.0)];
        let b = [(100u64, 2.0), (200, 2.0)];
        let est = wmh.jaccard(&wmh.signature(&a), &wmh.signature(&b));
        assert!(est < 0.05, "disjoint jaccard {est}");
    }

    #[test]
    fn matches_true_weighted_jaccard() {
        let wmh = WeightedMinHash::new(1024).unwrap();
        let a = [(1u64, 2.0), (2, 2.0), (3, 2.0)];
        let b = [(2u64, 2.0), (3, 2.0), (4, 2.0)];
        let truth = true_weighted_jaccard(&a, &b); // 4/8 = 0.5
        let est = wmh.jaccard(&wmh.signature(&a), &wmh.signature(&b));
        assert!((est - truth).abs() < 0.07, "est {est} vs truth {truth}");
    }

    #[test]
    fn respects_unequal_weights() {
        // Same support, very different weights → lower weighted Jaccard than the unweighted 1.0.
        let wmh = WeightedMinHash::new(1024).unwrap();
        let a = [(1u64, 10.0), (2, 1.0)];
        let b = [(1u64, 1.0), (2, 10.0)];
        let truth = true_weighted_jaccard(&a, &b); // (1+1)/(10+10) = 0.1
        let est = wmh.jaccard(&wmh.signature(&a), &wmh.signature(&b));
        assert!((est - truth).abs() < 0.07, "est {est} vs truth {truth}");
    }

    #[test]
    fn signature_length_matches_num_hashes() {
        let wmh = WeightedMinHash::new(64).unwrap();
        assert_eq!(wmh.signature(&[(1u64, 1.0)]).len(), 64);
    }
}
