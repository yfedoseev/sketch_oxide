//! MinHash-LSH banding index — sublinear near-duplicate search.
//!
//! MinHash lets you *score* the Jaccard similarity of two sets, but scoring every pair is
//! quadratic. Locality-Sensitive Hashing makes it sublinear: split each `b·r`-length MinHash
//! signature into `b` bands of `r` rows, hash each band, and index items by their band
//! buckets. Two items collide in a band's bucket only if all `r` of that band's hashes
//! agree, so similar items (high Jaccard) share at least one band with high probability while
//! dissimilar items almost never do. A query returns the union of its bands' buckets — a
//! small candidate set to verify exactly.
//!
//! The classic `(b, r)` tuning gives an S-curve with threshold `≈ (1/b)^(1/r)`: pairs above
//! it are likely candidates, pairs below it are likely filtered.
//!
//! This index is agnostic to how signatures are produced — pass any `&[u64]` MinHash
//! signature of length `b·r`.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// An LSH banding index over MinHash signatures, mapping band buckets to item ids of type
/// `Id`.
///
/// # Example
/// ```
/// use sketch_oxide::similarity::MinHashLsh;
///
/// // 16 bands × 4 rows => signatures of length 64.
/// let mut lsh: MinHashLsh<u32> = MinHashLsh::new(16, 4).unwrap();
/// let sig_a = vec![7u64; 64];
/// let mut sig_b = sig_a.clone();
/// sig_b[0] = 99; // differs in one position => still shares 15 bands
/// lsh.insert(1, &sig_a).unwrap();
/// lsh.insert(2, &sig_b).unwrap();
///
/// let candidates = lsh.query(&sig_a).unwrap();
/// assert!(candidates.contains(&2), "near-duplicate should be a candidate");
/// ```
#[derive(Debug, Clone)]
pub struct MinHashLsh<Id> {
    num_bands: usize,
    rows_per_band: usize,
    /// One bucket map per band: band-hash → ids.
    bands: Vec<HashMap<u64, Vec<Id>>>,
}

impl<Id: Clone + Eq + Hash> MinHashLsh<Id> {
    /// Creates an index for signatures of length `num_bands · rows_per_band`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `num_bands` or `rows_per_band` is 0.
    pub fn new(num_bands: usize, rows_per_band: usize) -> Result<Self> {
        if num_bands == 0 || rows_per_band == 0 {
            return Err(SketchError::InvalidParameter {
                param: if num_bands == 0 {
                    "num_bands"
                } else {
                    "rows_per_band"
                }
                .to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            num_bands,
            rows_per_band,
            bands: vec![HashMap::new(); num_bands],
        })
    }

    /// Required signature length (`num_bands · rows_per_band`).
    #[inline]
    pub fn signature_len(&self) -> usize {
        self.num_bands * self.rows_per_band
    }

    /// The approximate similarity threshold of the S-curve, `(1/b)^(1/r)`.
    pub fn threshold(&self) -> f64 {
        (1.0 / self.num_bands as f64).powf(1.0 / self.rows_per_band as f64)
    }

    /// Hash of band `b`'s slice of `signature`.
    fn band_hash(&self, signature: &[u64], b: usize) -> u64 {
        let start = b * self.rows_per_band;
        let slice = &signature[start..start + self.rows_per_band];
        // Mix the row values (and the band index, so equal rows in different bands differ).
        let mut bytes = Vec::with_capacity(8 * (self.rows_per_band + 1));
        bytes.extend_from_slice(&(b as u64).to_le_bytes());
        for &v in slice {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        xxhash(&bytes, 0)
    }

    fn check_len(&self, signature: &[u64]) -> Result<()> {
        if signature.len() != self.signature_len() {
            return Err(SketchError::InvalidParameter {
                param: "signature".to_string(),
                value: format!("len {}", signature.len()),
                constraint: format!("must have length {}", self.signature_len()),
            });
        }
        Ok(())
    }

    /// Indexes `id` under its band buckets.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `signature` has the wrong length.
    pub fn insert(&mut self, id: Id, signature: &[u64]) -> Result<()> {
        self.check_len(signature)?;
        for b in 0..self.num_bands {
            let h = self.band_hash(signature, b);
            self.bands[b].entry(h).or_default().push(id.clone());
        }
        Ok(())
    }

    /// Returns the candidate ids that share at least one band bucket with `signature`
    /// (deduplicated). These are the items worth verifying with an exact similarity.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `signature` has the wrong length.
    pub fn query(&self, signature: &[u64]) -> Result<Vec<Id>> {
        self.check_len(signature)?;
        let mut seen: HashSet<Id> = HashSet::new();
        for b in 0..self.num_bands {
            let h = self.band_hash(signature, b);
            if let Some(ids) = self.bands[b].get(&h) {
                for id in ids {
                    seen.insert(id.clone());
                }
            }
        }
        Ok(seen.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_params() {
        assert!(MinHashLsh::<u32>::new(0, 4).is_err());
        assert!(MinHashLsh::<u32>::new(4, 0).is_err());
        assert!(MinHashLsh::<u32>::new(4, 4).is_ok());
    }

    #[test]
    fn wrong_length_errors() {
        let mut lsh: MinHashLsh<u32> = MinHashLsh::new(4, 4).unwrap();
        assert!(lsh.insert(1, &vec![0u64; 10]).is_err());
        assert!(lsh.query(&vec![0u64; 10]).is_err());
    }

    #[test]
    fn identical_signatures_are_candidates() {
        let mut lsh: MinHashLsh<u32> = MinHashLsh::new(8, 4).unwrap();
        let sig = (0..32u64).collect::<Vec<_>>();
        lsh.insert(1, &sig).unwrap();
        let c = lsh.query(&sig).unwrap();
        assert!(c.contains(&1));
    }

    #[test]
    fn near_duplicate_shares_a_band() {
        let mut lsh: MinHashLsh<u32> = MinHashLsh::new(16, 4).unwrap();
        let sig_a: Vec<u64> = (0..64).collect();
        let mut sig_b = sig_a.clone();
        sig_b[0] = 9999; // differs in band 0 only; 15 bands still match exactly
        lsh.insert(2, &sig_b).unwrap();
        let c = lsh.query(&sig_a).unwrap();
        assert!(c.contains(&2), "near-duplicate must be a candidate");
    }

    #[test]
    fn dissimilar_signatures_rarely_collide() {
        let mut lsh: MinHashLsh<u32> = MinHashLsh::new(16, 4).unwrap();
        let sig_a: Vec<u64> = (0..64).collect();
        let sig_b: Vec<u64> = (1000..1064).collect(); // entirely different
        lsh.insert(2, &sig_b).unwrap();
        let c = lsh.query(&sig_a).unwrap();
        assert!(
            !c.contains(&2),
            "completely different signatures should not collide"
        );
    }

    #[test]
    fn threshold_matches_formula() {
        let lsh: MinHashLsh<u32> = MinHashLsh::new(16, 4).unwrap();
        let expected = (1.0_f64 / 16.0).powf(1.0 / 4.0);
        assert!((lsh.threshold() - expected).abs() < 1e-12);
    }

    #[test]
    fn query_deduplicates_candidates() {
        let mut lsh: MinHashLsh<u32> = MinHashLsh::new(8, 2).unwrap();
        let sig: Vec<u64> = (0..16).collect();
        // Same id inserted once shares all 8 bands; must appear once, not 8 times.
        lsh.insert(7, &sig).unwrap();
        let c = lsh.query(&sig).unwrap();
        assert_eq!(c.iter().filter(|&&x| x == 7).count(), 1);
    }
}
