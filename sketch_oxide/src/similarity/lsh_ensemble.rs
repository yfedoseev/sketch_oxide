//! LSH Ensemble — domain search by set **containment** (Zhu, Nargesian, Pu, Miller & Çetintemel,
//! "LSH Ensemble: Internet-Scale Domain Search", VLDB 2016).
//!
//! Jaccard-based MinHash LSH searches for sets *similar* to a query `Q`. Many applications instead
//! want high **containment** `t(Q, X) = |Q ∩ X| / |Q|` — "which indexed sets contain most of my query
//! set?" — which is asymmetric and depends on the indexed set's size `|X|`. A single Jaccard LSH is
//! badly biased across set sizes, because containment `t` and Jaccard `J` relate through the sizes:
//! `J = t·|Q| / (|X| + (1 − t)·|Q|)`. **LSH Ensemble** fixes this by **partitioning the indexed sets
//! by size** into log-scaled bands, each band a MinHash LSH index; a containment query probes every
//! band, gathers candidates, and keeps those whose estimated containment clears the threshold.
//!
//! Sets are supplied as MinHash signatures (e.g. from [`MinHash`](crate::similarity::MinHash)) plus
//! their cardinalities. Containment is estimated from the signature Jaccard and the two sizes.
//!
//! # Implementation note
//!
//! High-containment sets can have *low* Jaccard (a big `X` that contains a small `Q`), so this
//! reference uses a **maximal-recall LSH** within each size partition — each MinHash position is its
//! own band, so any set sharing a hash with `Q` becomes a candidate — and then keeps those whose
//! *estimated containment* clears the threshold. The paper's per-partition *optimal* band tuning
//! (which bounds the candidate count from above for a target threshold) is a follow-up.

use crate::common::{Result, SketchError};
use crate::similarity::MinHashLsh;

/// One indexed set: id, signature, and cardinality.
#[derive(Debug, Clone)]
struct Indexed<Id> {
    id: Id,
    sig: Vec<u64>,
    size: usize,
}

/// A size-partitioned LSH index for containment search.
struct Partition<Id> {
    lsh: MinHashLsh<usize>,
    sets: Vec<Indexed<Id>>,
}

impl<Id> std::fmt::Debug for Partition<Id> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Partition")
            .field("sets", &self.sets.len())
            .finish()
    }
}

/// An LSH Ensemble for set-containment search over MinHash signatures.
///
/// # Example
/// ```
/// use sketch_oxide::similarity::{LshEnsemble, MinHash};
///
/// fn sig(items: impl IntoIterator<Item = u32>, num_perm: usize) -> Vec<u64> {
///     let mut m = MinHash::new(num_perm).unwrap();
///     for i in items { m.update(&i); }
///     m.hashes().to_vec()
/// }
///
/// let num_perm = 128;
/// let mut ens = LshEnsemble::new(num_perm, 24).unwrap();
/// // A big set that fully contains the query, and an unrelated set.
/// ens.add("contains", &sig(0..5000u32, num_perm), 5000).unwrap();
/// ens.add("unrelated", &sig(10_000..10_500u32, num_perm), 500).unwrap();
///
/// // Query = {0..500}; "contains" has containment 1.0, "unrelated" 0.0.
/// let hits = ens.query(&sig(0..500u32, num_perm), 500, 0.8).unwrap();
/// assert!(hits.contains(&"contains"));
/// assert!(!hits.contains(&"unrelated"));
/// ```
#[derive(Debug)]
pub struct LshEnsemble<Id> {
    num_perm: usize,
    partitions: Vec<Partition<Id>>,
}

impl<Id: Clone + PartialEq> LshEnsemble<Id> {
    /// Creates an ensemble of `num_partitions` size bands, each a maximal-recall MinHash LSH over
    /// `num_perm`-length signatures.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `num_perm == 0` or `num_partitions == 0`.
    pub fn new(num_perm: usize, num_partitions: usize) -> Result<Self> {
        if num_perm == 0 || num_partitions == 0 {
            return Err(SketchError::InvalidParameter {
                param: "num_perm/num_partitions".to_string(),
                value: format!("{num_perm}/{num_partitions}"),
                constraint: "must be >= 1".to_string(),
            });
        }
        let partitions = (0..num_partitions)
            .map(|_| {
                // Maximal recall: one band per MinHash, so any shared hash makes a candidate.
                Ok(Partition {
                    lsh: MinHashLsh::new(num_perm, 1)?,
                    sets: Vec::new(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            num_perm,
            partitions,
        })
    }

    /// Partition index for a set of cardinality `size` (log-scaled, clamped to the last band).
    fn partition_of(&self, size: usize) -> usize {
        let band = (size.max(1) as f64).log2().floor() as usize;
        band.min(self.partitions.len() - 1)
    }

    /// Adds a set (its `signature` of length `num_perm` and cardinality `size`).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if the signature length differs from `num_perm`.
    pub fn add(&mut self, id: Id, signature: &[u64], size: usize) -> Result<()> {
        if signature.len() != self.num_perm {
            return Err(SketchError::InvalidParameter {
                param: "signature.len".to_string(),
                value: signature.len().to_string(),
                constraint: format!("must equal num_perm = {}", self.num_perm),
            });
        }
        let p = self.partition_of(size);
        let local = self.partitions[p].sets.len();
        self.partitions[p].lsh.insert(local, signature)?;
        self.partitions[p].sets.push(Indexed {
            id,
            sig: signature.to_vec(),
            size,
        });
        Ok(())
    }

    /// Estimates `t(Q, X)` from the signature Jaccard and the two cardinalities.
    fn containment(q_sig: &[u64], q_size: usize, x: &Indexed<Id>) -> f64 {
        let matches = q_sig.iter().zip(&x.sig).filter(|(a, b)| a == b).count();
        let j = matches as f64 / q_sig.len() as f64;
        if j <= 0.0 {
            return 0.0;
        }
        // |Q ∩ X| = J·|Q ∪ X| = J·(|Q| + |X|)/(1 + J); t = |Q ∩ X| / |Q|.
        let inter = j * (q_size + x.size) as f64 / (1.0 + j);
        (inter / q_size as f64).clamp(0.0, 1.0)
    }

    /// Returns the ids of indexed sets whose estimated containment `t(Q, X) ≥ threshold`, retrieving
    /// candidates from every size band's LSH.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `q_signature.len() != num_perm`.
    pub fn query(&self, q_signature: &[u64], q_size: usize, threshold: f64) -> Result<Vec<Id>> {
        if q_signature.len() != self.num_perm {
            return Err(SketchError::InvalidParameter {
                param: "q_signature.len".to_string(),
                value: q_signature.len().to_string(),
                constraint: format!("must equal num_perm = {}", self.num_perm),
            });
        }
        let mut out = Vec::new();
        for part in &self.partitions {
            for local in part.lsh.query(q_signature)? {
                let x = &part.sets[local];
                if Self::containment(q_signature, q_size, x) >= threshold {
                    out.push(x.id.clone());
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::similarity::MinHash;

    fn sig(items: impl IntoIterator<Item = u32>, num_perm: usize) -> Vec<u64> {
        let mut m = MinHash::new(num_perm).unwrap();
        for i in items {
            m.update(&i);
        }
        m.hashes().to_vec()
    }

    #[test]
    fn rejects_bad_params() {
        assert!(LshEnsemble::<u32>::new(0, 4).is_err());
        assert!(LshEnsemble::<u32>::new(128, 0).is_err());
        assert!(LshEnsemble::<u32>::new(128, 24).is_ok());
    }

    #[test]
    fn finds_containing_sets() {
        let np = 128;
        let mut ens = LshEnsemble::new(np, 24).unwrap();
        ens.add("big-contains", &sig(0..5000u32, np), 5000).unwrap();
        ens.add("exact", &sig(0..500u32, np), 500).unwrap();
        ens.add("partial", &sig(250..2000u32, np), 1750).unwrap();
        ens.add("unrelated", &sig(50_000..50_500u32, np), 500)
            .unwrap();

        // Query Q = {0..500}: "big-contains" (t≈1) and "exact" (t=1) clear 0.8; others don't.
        let hits = ens.query(&sig(0..500u32, np), 500, 0.8).unwrap();
        assert!(
            hits.contains(&"big-contains"),
            "missing big-contains: {hits:?}"
        );
        assert!(hits.contains(&"exact"), "missing exact");
        assert!(!hits.contains(&"unrelated"), "unrelated should not match");
    }

    #[test]
    fn rejects_wrong_signature_length() {
        let mut ens = LshEnsemble::<u32>::new(128, 4).unwrap();
        assert!(ens.add(1, &[0u64; 64], 100).is_err());
        assert!(ens.query(&[0u64; 64], 100, 0.5).is_err());
    }
}
