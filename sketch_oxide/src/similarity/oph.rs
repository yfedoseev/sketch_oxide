//! One-Permutation Hashing (OPH) with densification — fast MinHash.
//!
//! Classic MinHash applies `k` independent hash permutations to every element, costing
//! `O(k)` per element. One-Permutation Hashing (Li, Owen & Zhang, NeurIPS 2012; densified by
//! Shrivastava & Li) applies a *single* hash, splits its range into `k` bins, and keeps the
//! minimum in each bin — `O(1)` amortized per element. Bins left empty by sparse inputs are
//! filled by **densification**: an empty bin copies a value from a non-empty bin found by a
//! fixed probe sequence (mixed with the offset so copies stay decorrelated). The resulting
//! `k`-length signature estimates Jaccard exactly as MinHash does — the fraction of agreeing
//! bins — but builds far faster, the default fast path in large-scale dedup pipelines.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use std::hash::Hash;

/// A one-permutation MinHash sketch with `k` bins.
///
/// # Example
/// ```
/// use sketch_oxide::similarity::OnePermutationHash;
///
/// let mut a = OnePermutationHash::new(256).unwrap();
/// let mut b = OnePermutationHash::new(256).unwrap();
/// for i in 0..1000u64 { a.update(&i); }
/// for i in 500..1500u64 { b.update(&i); } // Jaccard = 500/1500 ≈ 0.333
/// let j = a.jaccard(&b).unwrap();
/// assert!((j - 0.333).abs() < 0.1, "estimated Jaccard {j}");
/// ```
#[derive(Debug, Clone)]
pub struct OnePermutationHash {
    k: usize,
    /// Per-bin minimum value, `None` if the bin is empty.
    bins: Vec<Option<u64>>,
}

impl OnePermutationHash {
    /// Creates a sketch with `k` bins (the signature length).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `k` is 0.
    pub fn new(k: usize) -> Result<Self> {
        if k == 0 {
            return Err(SketchError::InvalidParameter {
                param: "k".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            k,
            bins: vec![None; k],
        })
    }

    /// Adds an item to the set.
    pub fn update<T: Hash>(&mut self, item: &T) {
        // One hash, split: high bits choose the bin, the whole hash is the bin value.
        let h = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::Hasher;
            let mut hasher = DefaultHasher::new();
            item.hash(&mut hasher);
            hasher.finish()
        };
        let bin = (h % self.k as u64) as usize;
        // Re-mix so the bin value is independent of the bin choice.
        let value = xxhash(&h.to_le_bytes(), 0x4F50_4831);
        let slot = &mut self.bins[bin];
        *slot = Some(slot.map_or(value, |cur| cur.min(value)));
    }

    /// Mixes a copied densification value with the probe offset so different empty bins that
    /// copy the same source bin still get decorrelated values.
    #[inline]
    fn densify_mix(value: u64, offset: usize) -> u64 {
        xxhash(&value.to_le_bytes(), offset as u64 + 1)
    }

    /// Produces the densified `k`-length signature: empty bins are filled from the nearest
    /// non-empty bin in a fixed forward probe order.
    pub fn signature(&self) -> Vec<u64> {
        let mut sig = vec![0u64; self.k];
        for (j, slot) in sig.iter_mut().enumerate() {
            if let Some(v) = self.bins[j] {
                *slot = v;
            } else {
                // Probe forward (wrapping) for the first non-empty bin.
                for step in 1..=self.k {
                    if let Some(v) = self.bins[(j + step) % self.k] {
                        *slot = Self::densify_mix(v, step);
                        break;
                    }
                }
                // `*slot` stays 0 only if the sketch is entirely empty.
            }
        }
        sig
    }

    /// Estimates the Jaccard similarity with another sketch (fraction of agreeing bins after
    /// densification). Both must have the same `k`.
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the bin counts differ.
    pub fn jaccard(&self, other: &Self) -> Result<f64> {
        if self.k != other.k {
            return Err(SketchError::IncompatibleSketches {
                reason: format!("bin count mismatch: {} vs {}", self.k, other.k),
            });
        }
        let a = self.signature();
        let b = other.signature();
        let matches = a.iter().zip(&b).filter(|(x, y)| x == y).count();
        Ok(matches as f64 / self.k as f64)
    }

    /// Number of bins (signature length).
    #[inline]
    pub fn num_bins(&self) -> usize {
        self.k
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(range: std::ops::Range<u64>, k: usize) -> OnePermutationHash {
        let mut o = OnePermutationHash::new(k).unwrap();
        for i in range {
            o.update(&i);
        }
        o
    }

    #[test]
    fn rejects_zero_k() {
        assert!(OnePermutationHash::new(0).is_err());
        assert!(OnePermutationHash::new(64).is_ok());
    }

    #[test]
    fn identical_sets_give_one() {
        let a = build(0..1000, 256);
        let b = build(0..1000, 256);
        assert!((a.jaccard(&b).unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn disjoint_sets_give_near_zero() {
        let a = build(0..1000, 256);
        let b = build(10_000..11_000, 256);
        assert!(a.jaccard(&b).unwrap() < 0.1, "disjoint jaccard");
    }

    #[test]
    fn partial_overlap_estimated() {
        // |A∩B| = 500, |A∪B| = 1500 => J ≈ 0.333.
        let a = build(0..1000, 512);
        let b = build(500..1500, 512);
        let j = a.jaccard(&b).unwrap();
        assert!((j - 0.333).abs() < 0.07, "estimated {j}");
    }

    #[test]
    fn densification_handles_sparse_sets() {
        // Far fewer items than bins => many empty bins, exercised by densification.
        let a = build(0..10, 256);
        let b = build(0..10, 256);
        // Identical sparse sets must still estimate Jaccard 1 after densification.
        assert!((a.jaccard(&b).unwrap() - 1.0).abs() < 1e-9);
        // Signature is fully filled (no leftover zeros from a non-empty sketch).
        assert!(a.signature().iter().all(|&v| v != 0));
    }

    #[test]
    fn incompatible_bins_error() {
        let a = OnePermutationHash::new(64).unwrap();
        let b = OnePermutationHash::new(128).unwrap();
        assert!(a.jaccard(&b).is_err());
    }
}

// ---------------------------------------------------------------------------
// Capability-trait adoptions (fable5 doc 01 F3 "split the `Sketch` trait").
// One-permutation hashing ingests any hashable item, so it satisfies `Update`.
// It has no `Sketch` impl (no serialize), no cardinality/quantile/point/
// membership semantics (it estimates Jaccard between two sketches), so only
// `Update` applies.
// ---------------------------------------------------------------------------
use crate::common::Update;

impl<T: Hash> Update<T> for OnePermutationHash {
    fn update(&mut self, item: &T) {
        OnePermutationHash::update(self, item);
    }
}
