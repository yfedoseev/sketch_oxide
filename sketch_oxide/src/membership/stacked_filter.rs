//! Stacked Filters — meta-filter composition for skewed / known-negative workloads.
//!
//! A single Bloom filter wastes space when many of the *negatives* that will be queried are
//! known in advance (frequent non-members). Stacked Filters (Deeds, Hentschel & Idreos,
//! "Stacked Filters: Learning to Filter by Structure", VLDB 2020) layer alternating filters:
//! layer 0 holds the positives, layer 1 holds the known negatives that layer 0 falsely
//! admits, layer 2 holds the positives that layer 1 falsely admits, and so on. A query walks
//! down the stack; an item rejected at any layer takes that layer's verdict. This turns prior
//! knowledge of the negative set into far lower effective false-positive rates than a single
//! filter of the same size.
//!
//! This is a three-layer static construction built from the positive set and a sample of
//! known negatives.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

/// A compact internal Bloom filter.
#[derive(Debug, Clone)]
struct Bloom {
    bits: Vec<u64>,
    m: usize,
    k: u32,
    seed: u64,
}

impl Bloom {
    fn new(n: usize, fp: f64, seed: u64) -> Self {
        let n = n.max(1);
        let fp = fp.clamp(1e-9, 0.5);
        let ln2 = std::f64::consts::LN_2;
        let m = (-(n as f64) * fp.ln() / (ln2 * ln2)).ceil().max(64.0) as usize;
        let k = ((m as f64 / n as f64) * ln2).round().clamp(1.0, 30.0) as u32;
        Self {
            bits: vec![0u64; m.div_ceil(64)],
            m,
            k,
            seed,
        }
    }

    fn positions(&self, key: &[u8]) -> impl Iterator<Item = usize> + '_ {
        let h1 = xxhash(key, self.seed);
        let h2 = xxhash(key, self.seed.wrapping_add(1));
        (0..self.k)
            .map(move |i| (h1.wrapping_add((i as u64).wrapping_mul(h2)) % self.m as u64) as usize)
    }

    fn insert(&mut self, key: &[u8]) {
        let ps: Vec<usize> = self.positions(key).collect();
        for p in ps {
            self.bits[p / 64] |= 1 << (p % 64);
        }
    }

    fn contains(&self, key: &[u8]) -> bool {
        self.positions(key)
            .all(|p| self.bits[p / 64] & (1 << (p % 64)) != 0)
    }

    fn size_bits(&self) -> usize {
        self.m
    }
}

/// A three-layer Stacked Filter built from positives and known negatives.
///
/// # Example
/// ```
/// use sketch_oxide::membership::StackedFilter;
///
/// let positives: Vec<&[u8]> = vec![b"a", b"b", b"c"];
/// // Frequent known negatives we want to reject cheaply.
/// let known_negs: Vec<&[u8]> = (0..2000u32).map(|_| b"".as_slice()).collect();
/// let neg_keys: Vec<Vec<u8>> = (0..2000u64).map(|i| i.to_le_bytes().to_vec()).collect();
/// let neg_refs: Vec<&[u8]> = neg_keys.iter().map(|k| k.as_slice()).collect();
/// let _ = known_negs;
///
/// let f = StackedFilter::build(&positives, &neg_refs, 0.1).unwrap();
/// for p in &positives { assert!(f.contains(p)); } // no false negatives
/// ```
#[derive(Debug, Clone)]
pub struct StackedFilter {
    /// Layer 0: positives. Layer 1: known negatives in layer-0 false positives.
    /// Layer 2: positives in layer-1 false positives.
    layers: [Bloom; 3],
}

impl StackedFilter {
    /// Builds a stacked filter from the `positives` set and a sample of `known_negatives`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `positives` is empty or `fp` is not in `(0, 1)`.
    pub fn build(positives: &[&[u8]], known_negatives: &[&[u8]], fp: f64) -> Result<Self> {
        if positives.is_empty() {
            return Err(SketchError::InvalidParameter {
                param: "positives".to_string(),
                value: "empty".to_string(),
                constraint: "must be non-empty".to_string(),
            });
        }
        if !(fp > 0.0 && fp < 1.0) {
            return Err(SketchError::InvalidParameter {
                param: "fp".to_string(),
                value: fp.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }

        // Layer 0: all positives.
        let mut l0 = Bloom::new(positives.len(), fp, 0x10);
        for p in positives {
            l0.insert(p);
        }

        // Layer 1: known negatives that layer 0 falsely admits.
        let fp_negs: Vec<&&[u8]> = known_negatives.iter().filter(|n| l0.contains(n)).collect();
        let mut l1 = Bloom::new(fp_negs.len().max(1), fp, 0x20);
        for n in &fp_negs {
            l1.insert(n);
        }

        // Layer 2: positives that layer 1 falsely admits (so they are not wrongly rejected).
        let fp_pos: Vec<&&[u8]> = positives.iter().filter(|p| l1.contains(p)).collect();
        let mut l2 = Bloom::new(fp_pos.len().max(1), fp, 0x30);
        for p in &fp_pos {
            l2.insert(p);
        }

        Ok(Self {
            layers: [l0, l1, l2],
        })
    }

    /// Tests membership. Never a false negative for a key in the original positive set.
    pub fn contains(&self, key: &[u8]) -> bool {
        // Not in layer 0 => definitely negative.
        if !self.layers[0].contains(key) {
            return false;
        }
        // In layer 0 but not layer 1 => positive (layer 1 holds known negatives).
        if !self.layers[1].contains(key) {
            return true;
        }
        // In layers 0 and 1: decide via layer 2 (positives that reach here).
        self.layers[2].contains(key)
    }

    /// Total size of all layers in bits.
    pub fn size_bits(&self) -> usize {
        self.layers.iter().map(Bloom::size_bits).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(range: std::ops::Range<u64>) -> Vec<Vec<u8>> {
        range.map(|i| i.to_le_bytes().to_vec()).collect()
    }

    #[test]
    fn rejects_bad_params() {
        let p: Vec<&[u8]> = vec![b"a"];
        assert!(StackedFilter::build(&[], &[], 0.1).is_err());
        assert!(StackedFilter::build(&p, &[], 0.0).is_err());
        assert!(StackedFilter::build(&p, &[], 1.0).is_err());
        assert!(StackedFilter::build(&p, &[], 0.1).is_ok());
    }

    #[test]
    fn no_false_negatives() {
        let pk = keys(0..500);
        let positives: Vec<&[u8]> = pk.iter().map(|k| k.as_slice()).collect();
        let nk = keys(1_000_000..1_002_000);
        let negs: Vec<&[u8]> = nk.iter().map(|k| k.as_slice()).collect();
        let f = StackedFilter::build(&positives, &negs, 0.1).unwrap();
        for p in &positives {
            assert!(f.contains(p), "false negative");
        }
    }

    #[test]
    fn known_negatives_rejected_better() {
        // The known negatives used in construction should be rejected at a high rate.
        let pk = keys(0..500);
        let positives: Vec<&[u8]> = pk.iter().map(|k| k.as_slice()).collect();
        let nk = keys(1_000_000..1_005_000);
        let negs: Vec<&[u8]> = nk.iter().map(|k| k.as_slice()).collect();
        let f = StackedFilter::build(&positives, &negs, 0.1).unwrap();

        let admitted = negs.iter().filter(|n| f.contains(n)).count();
        // A single 10%-FP Bloom would admit ~500 of 5000; the stack does far better on the
        // known negatives it was built against.
        assert!(admitted < 200, "known negatives admitted {admitted}/5000");
    }

    #[test]
    fn reports_size() {
        let pk = keys(0..100);
        let positives: Vec<&[u8]> = pk.iter().map(|k| k.as_slice()).collect();
        let f = StackedFilter::build(&positives, &[], 0.1).unwrap();
        assert!(f.size_bits() > 0);
    }
}
