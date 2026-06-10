//! Sandwiched Learned Bloom Filter.
//!
//! A learned Bloom filter replaces part of a Bloom filter with a model that predicts
//! membership; the "sandwich" construction (Mitzenmacher, "A Model for Learned Bloom
//! Filters and Optimizing by Sandwiching", NeurIPS 2018) wraps that model between two
//! ordinary Bloom filters and is provably the optimal way to spend the bit budget:
//!
//! 1. an **initial** filter over the positive set screens out most true negatives;
//! 2. the **model** ([`Oracle`](crate::learned::Oracle)) classifies the survivors;
//! 3. a **backup** filter holds exactly the positives the model misclassifies as negative,
//!    so there are **no false negatives**.
//!
//! It is a static filter, built once from a known positive set and a trained oracle.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use crate::learned::{Oracle, Score};

/// A minimal internal Bloom filter (bit array + k hashes).
#[derive(Debug, Clone)]
struct Bloom {
    bits: Vec<u64>,
    num_bits: usize,
    k: u32,
    /// Hash seed, so the two sandwich layers are independent.
    seed: u64,
}

impl Bloom {
    /// Sizes a Bloom filter for `n` items at false-positive rate `fp` (clamped to sane
    /// minimums so it is always constructible). `seed` decorrelates layers.
    fn new(n: usize, fp: f64, seed: u64) -> Self {
        let n = n.max(1);
        let fp = fp.clamp(1e-9, 0.5);
        let ln2 = std::f64::consts::LN_2;
        let m = (-(n as f64) * fp.ln() / (ln2 * ln2)).ceil().max(64.0) as usize;
        let k = ((m as f64 / n as f64) * ln2).round().max(1.0) as u32;
        Bloom {
            bits: vec![0u64; m.div_ceil(64)],
            num_bits: m,
            k,
            seed,
        }
    }

    fn positions(&self, key: &[u8]) -> impl Iterator<Item = usize> + '_ {
        let h1 = xxhash(key, self.seed);
        let h2 = xxhash(key, self.seed.wrapping_add(1));
        (0..self.k).map(move |i| {
            (h1.wrapping_add((i as u64).wrapping_mul(h2)) % self.num_bits as u64) as usize
        })
    }

    fn insert(&mut self, key: &[u8]) {
        let positions: Vec<usize> = self.positions(key).collect();
        for p in positions {
            self.bits[p / 64] |= 1 << (p % 64);
        }
    }

    fn contains(&self, key: &[u8]) -> bool {
        self.positions(key)
            .all(|p| self.bits[p / 64] & (1 << (p % 64)) != 0)
    }

    fn size_bits(&self) -> usize {
        self.num_bits
    }
}

/// A sandwiched learned Bloom filter over a fixed positive set.
///
/// # Example
/// ```
/// use sketch_oxide::learned::{SandwichedLearnedBloom, ClosureOracle};
///
/// let positives: Vec<&[u8]> = vec![b"apple", b"banana", b"cherry"];
/// // A perfect oracle scores positives high; build with threshold 0.5.
/// let oracle = ClosureOracle::new(|k: &[u8]| {
///     if k == b"apple" || k == b"banana" || k == b"cherry" { 1.0 } else { 0.0 }
/// });
/// let f = SandwichedLearnedBloom::build(&positives, &oracle, 0.5, 0.01).unwrap();
///
/// // No false negatives: every positive is reported present.
/// for p in &positives {
///     assert!(f.contains(p, &oracle));
/// }
/// assert!(!f.contains(b"durian", &oracle)); // a true negative
/// ```
#[derive(Debug, Clone)]
pub struct SandwichedLearnedBloom {
    initial: Bloom,
    backup: Bloom,
    threshold: Score,
}

impl SandwichedLearnedBloom {
    /// Builds the filter from the `positives` set and a trained `oracle`.
    ///
    /// * `threshold` — oracle score at/above which a key is predicted positive.
    /// * `fp_target` — target false-positive rate used to size the two Bloom filters.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `positives` is empty.
    pub fn build<O: Oracle + ?Sized>(
        positives: &[&[u8]],
        oracle: &O,
        threshold: Score,
        fp_target: f64,
    ) -> Result<Self> {
        if positives.is_empty() {
            return Err(SketchError::InvalidParameter {
                param: "positives".to_string(),
                value: "empty".to_string(),
                constraint: "must be non-empty".to_string(),
            });
        }

        // Initial filter over all positives.
        let mut initial = Bloom::new(positives.len(), fp_target.sqrt(), 0x1111);
        for p in positives {
            initial.insert(p);
        }

        // Backup filter over positives the oracle would wrongly call negative.
        let misses: Vec<&&[u8]> = positives
            .iter()
            .filter(|p| oracle.score(p) < threshold)
            .collect();
        let mut backup = Bloom::new(misses.len().max(1), fp_target.sqrt(), 0x9999);
        for p in &misses {
            backup.insert(p);
        }

        Ok(Self {
            initial,
            backup,
            threshold,
        })
    }

    /// Tests membership. Never returns a false negative for a key in the original positive
    /// set.
    pub fn contains<O: Oracle + ?Sized>(&self, key: &[u8], oracle: &O) -> bool {
        // Screened out by the initial filter => definitely not a positive.
        if !self.initial.contains(key) {
            return false;
        }
        // Trust the model when it predicts positive; otherwise fall back to the backup filter
        // that holds the model's false negatives.
        if oracle.score(key) >= self.threshold {
            true
        } else {
            self.backup.contains(key)
        }
    }

    /// Total size of the two Bloom filters in bits.
    pub fn size_bits(&self) -> usize {
        self.initial.size_bits() + self.backup.size_bits()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learned::{ClosureOracle, PrecomputedOracle};

    #[test]
    fn rejects_empty_positive_set() {
        let oracle = ClosureOracle::new(|_: &[u8]| 1.0);
        assert!(SandwichedLearnedBloom::build(&[], &oracle, 0.5, 0.01).is_err());
    }

    #[test]
    fn no_false_negatives_with_perfect_oracle() {
        let positives: Vec<&[u8]> = vec![b"a", b"b", b"c", b"d"];
        let oracle = ClosureOracle::new(|_: &[u8]| 1.0); // says everything positive
        let f = SandwichedLearnedBloom::build(&positives, &oracle, 0.5, 0.01).unwrap();
        for p in &positives {
            assert!(f.contains(p, &oracle), "missing positive");
        }
    }

    #[test]
    fn no_false_negatives_with_imperfect_oracle() {
        // Oracle misclassifies half the positives as negative; backup must catch them.
        let positives: Vec<&[u8]> = vec![b"a", b"b", b"c", b"d", b"e", b"f"];
        let oracle = PrecomputedOracle::from_pairs(
            [
                (b"a".to_vec(), 1.0),
                (b"b".to_vec(), 1.0),
                (b"c".to_vec(), 1.0),
                // d, e, f get the default 0.0 => predicted negative
            ],
            0.0,
        );
        let f = SandwichedLearnedBloom::build(&positives, &oracle, 0.5, 0.01).unwrap();
        for p in &positives {
            assert!(f.contains(p, &oracle), "false negative for a positive key");
        }
    }

    #[test]
    fn rejects_clear_negatives() {
        let keys: Vec<Vec<u8>> = (0..200u64).map(|i| i.to_le_bytes().to_vec()).collect();
        let pos_refs: Vec<&[u8]> = keys.iter().map(|k| k.as_slice()).collect();
        let oracle = ClosureOracle::new(|_: &[u8]| 1.0);
        let f = SandwichedLearnedBloom::build(&pos_refs, &oracle, 2.0, 0.01).unwrap();
        // threshold 2.0 > oracle max 1.0 => model never predicts positive => pure Bloom.
        // Far-away negatives should be rejected (allowing the configured FP rate).
        let mut false_pos = 0;
        for i in 1000..1200u64 {
            if f.contains(&i.to_le_bytes(), &oracle) {
                false_pos += 1;
            }
        }
        assert!(false_pos < 20, "too many false positives: {false_pos}/200");
    }

    #[test]
    fn reports_size() {
        let positives: Vec<&[u8]> = vec![b"a", b"b"];
        let oracle = ClosureOracle::new(|_: &[u8]| 1.0);
        let f = SandwichedLearnedBloom::build(&positives, &oracle, 0.5, 0.01).unwrap();
        assert!(f.size_bits() > 0);
    }
}
