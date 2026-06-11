//! KMV — K-Minimum-Values sketch for cardinality and Jaccard similarity.
//!
//! A KMV sketch (Bar-Yossef et al. 2002; Beyer et al., "On synopses for distinct-value
//! estimation under multiset operations", SIGMOD 2007) keeps the `k` smallest hash values
//! seen. Because hashes are uniform on `[0, 1)`, the `k`-th smallest value `v_k` tells you how
//! densely the unit interval is filled: `n̂ = (k − 1) / v_k`. Keeping the actual minima (rather
//! than a register summary) makes set operations exact on the sample — the bottom-`k` of a
//! union is computable directly, giving unbiased **union cardinality** and **Jaccard**
//! estimates. It is the explicit, mergeable cousin of the Theta sketch.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use std::collections::BTreeSet;

/// A K-Minimum-Values cardinality + similarity sketch.
///
/// # Example
/// ```
/// use sketch_oxide::cardinality::KmvSketch;
///
/// let mut a = KmvSketch::new(1024).unwrap();
/// for i in 0..10_000u64 { a.add(&i.to_le_bytes()); }
/// let est = a.estimate();
/// assert!((est - 10_000.0).abs() < 0.1 * 10_000.0, "estimate {est}");
/// ```
#[derive(Debug, Clone)]
pub struct KmvSketch {
    k: usize,
    /// The `k` smallest hash values (ascending).
    mins: BTreeSet<u64>,
}

impl KmvSketch {
    /// Creates a sketch retaining the `k` smallest hash values. Larger `k` is more accurate
    /// (relative error `~1/√k`).
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
            mins: BTreeSet::new(),
        })
    }

    /// Adds an item.
    pub fn add(&mut self, item: &[u8]) {
        let h = xxhash(item, 0);
        self.add_hash(h);
    }

    fn add_hash(&mut self, h: u64) {
        if self.mins.contains(&h) {
            return;
        }
        if self.mins.len() < self.k {
            self.mins.insert(h);
        } else if let Some(&max) = self.mins.iter().next_back() {
            if h < max {
                self.mins.remove(&max);
                self.mins.insert(h);
            }
        }
    }

    /// Estimated number of distinct items.
    pub fn estimate(&self) -> f64 {
        if self.mins.len() < self.k {
            // Below capacity: the retained count is exact.
            self.mins.len() as f64
        } else {
            // (k-1) / (v_k / 2^64), where v_k is the largest retained (the k-th smallest).
            let v_k = *self.mins.iter().next_back().unwrap() as f64;
            (self.k - 1) as f64 * (u64::MAX as f64) / v_k
        }
    }

    /// The bottom-`k` of the union of `self` and `other` (the merged KMV sample).
    fn union_mins(&self, other: &Self) -> BTreeSet<u64> {
        let mut merged: BTreeSet<u64> = self.mins.union(&other.mins).copied().collect();
        while merged.len() > self.k.min(other.k) {
            let max = *merged.iter().next_back().unwrap();
            merged.remove(&max);
        }
        merged
    }

    /// Estimated cardinality of the union with `other`.
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the two sketches have different `k`.
    pub fn union_cardinality(&self, other: &Self) -> Result<f64> {
        if self.k != other.k {
            return Err(SketchError::IncompatibleSketches {
                reason: format!("k mismatch: {} vs {}", self.k, other.k),
            });
        }
        let merged = self.union_mins(other);
        if merged.len() < self.k {
            Ok(merged.len() as f64)
        } else {
            let v_k = *merged.iter().next_back().unwrap() as f64;
            Ok((self.k - 1) as f64 * (u64::MAX as f64) / v_k)
        }
    }

    /// Estimated Jaccard similarity with `other`: the fraction of the merged bottom-`k` that
    /// is present in *both* sketches.
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the two sketches have different `k`.
    pub fn jaccard(&self, other: &Self) -> Result<f64> {
        if self.k != other.k {
            return Err(SketchError::IncompatibleSketches {
                reason: format!("k mismatch: {} vs {}", self.k, other.k),
            });
        }
        let merged = self.union_mins(other);
        if merged.is_empty() {
            return Ok(0.0);
        }
        let both = merged
            .iter()
            .filter(|h| self.mins.contains(h) && other.mins.contains(h))
            .count();
        Ok(both as f64 / merged.len() as f64)
    }

    /// Number of retained minima.
    #[inline]
    pub fn num_retained(&self) -> usize {
        self.mins.len()
    }

    /// Whether nothing has been added.
    pub fn is_empty(&self) -> bool {
        self.mins.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_k() {
        assert!(KmvSketch::new(0).is_err());
        assert!(KmvSketch::new(256).is_ok());
    }

    #[test]
    fn exact_below_capacity() {
        let mut s = KmvSketch::new(1024).unwrap();
        for i in 0..50u64 {
            s.add(&i.to_le_bytes());
        }
        assert_eq!(s.estimate(), 50.0);
    }

    #[test]
    fn estimates_large_cardinality() {
        let mut s = KmvSketch::new(2048).unwrap();
        for i in 0..100_000u64 {
            s.add(&i.to_le_bytes());
        }
        let est = s.estimate();
        assert!((est - 100_000.0).abs() < 0.1 * 100_000.0, "estimate {est}");
    }

    #[test]
    fn duplicates_dont_count() {
        let mut s = KmvSketch::new(256).unwrap();
        for _ in 0..1000 {
            s.add(b"same");
        }
        assert_eq!(s.estimate(), 1.0);
    }

    #[test]
    fn union_cardinality_estimate() {
        let mut a = KmvSketch::new(2048).unwrap();
        let mut b = KmvSketch::new(2048).unwrap();
        for i in 0..50_000u64 {
            a.add(&i.to_le_bytes());
        }
        for i in 25_000..75_000u64 {
            b.add(&i.to_le_bytes());
        }
        // Union = [0,75000) = 75000 distinct.
        let u = a.union_cardinality(&b).unwrap();
        assert!((u - 75_000.0).abs() < 0.1 * 75_000.0, "union {u}");
    }

    #[test]
    fn jaccard_estimate() {
        let mut a = KmvSketch::new(4096).unwrap();
        let mut b = KmvSketch::new(4096).unwrap();
        for i in 0..10_000u64 {
            a.add(&i.to_le_bytes());
        }
        for i in 5_000..15_000u64 {
            b.add(&i.to_le_bytes());
        }
        // |A∩B| = 5000, |A∪B| = 15000 => J = 1/3.
        let j = a.jaccard(&b).unwrap();
        assert!((j - 0.333).abs() < 0.05, "jaccard {j}");
    }

    #[test]
    fn incompatible_k_errors() {
        let a = KmvSketch::new(256).unwrap();
        let b = KmvSketch::new(512).unwrap();
        assert!(a.jaccard(&b).is_err());
        assert!(a.union_cardinality(&b).is_err());
    }
}
