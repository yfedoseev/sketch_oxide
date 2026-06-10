//! Hamming-LSH banding index for SimHash signatures.
//!
//! SimHash maps a document to a single `b`-bit fingerprint whose Hamming distance tracks
//! cosine distance. To find near-duplicates sublinearly (rather than comparing every pair),
//! this index bands the fingerprint: split the `b` bits into `r` blocks, index items by each
//! block's bit pattern, and a query returns the items that agree on at least one block. Two
//! fingerprints within a small Hamming distance are very likely to share a block (their
//! differing bits land in few blocks), while distant ones rarely do — the Hamming analogue of
//! the MinHash banding S-curve.

use crate::common::{Result, SketchError};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// An LSH index over 64-bit SimHash fingerprints, mapping block patterns to item ids.
///
/// # Example
/// ```
/// use sketch_oxide::similarity::SimHashLsh;
///
/// // 4 blocks of 16 bits each.
/// let mut lsh: SimHashLsh<u32> = SimHashLsh::new(4).unwrap();
/// let sig = 0xABCD_1234_5678_9876u64;
/// lsh.insert(1, sig);
/// lsh.insert(2, sig ^ 0x1); // differs in 1 bit => shares 3 of 4 blocks
///
/// let candidates = lsh.query(sig);
/// assert!(candidates.contains(&2), "near-duplicate is a candidate");
/// ```
#[derive(Debug, Clone)]
pub struct SimHashLsh<Id> {
    num_blocks: usize,
    bits_per_block: u32,
    blocks: Vec<HashMap<u64, Vec<Id>>>,
}

impl<Id: Clone + Eq + Hash> SimHashLsh<Id> {
    /// Creates an index that splits the 64-bit fingerprint into `num_blocks` blocks.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `num_blocks` is 0 or does not divide 64.
    pub fn new(num_blocks: usize) -> Result<Self> {
        if num_blocks == 0 || 64 % num_blocks != 0 {
            return Err(SketchError::InvalidParameter {
                param: "num_blocks".to_string(),
                value: num_blocks.to_string(),
                constraint: "must be a positive divisor of 64".to_string(),
            });
        }
        Ok(Self {
            num_blocks,
            bits_per_block: (64 / num_blocks) as u32,
            blocks: vec![HashMap::new(); num_blocks],
        })
    }

    /// Block `b`'s bit pattern of `sig`.
    #[inline]
    fn block_value(&self, sig: u64, b: usize) -> u64 {
        let shift = b as u32 * self.bits_per_block;
        let mask = if self.bits_per_block == 64 {
            u64::MAX
        } else {
            (1u64 << self.bits_per_block) - 1
        };
        (sig >> shift) & mask
    }

    /// Indexes `id` under its block patterns.
    pub fn insert(&mut self, id: Id, sig: u64) {
        for b in 0..self.num_blocks {
            let v = self.block_value(sig, b);
            self.blocks[b].entry(v).or_default().push(id.clone());
        }
    }

    /// Returns the candidate ids sharing at least one block with `sig` (deduplicated) — the
    /// small set to verify by exact Hamming distance.
    pub fn query(&self, sig: u64) -> Vec<Id> {
        let mut seen: HashSet<Id> = HashSet::new();
        for b in 0..self.num_blocks {
            let v = self.block_value(sig, b);
            if let Some(ids) = self.blocks[b].get(&v) {
                seen.extend(ids.iter().cloned());
            }
        }
        seen.into_iter().collect()
    }

    /// Number of blocks.
    #[inline]
    pub fn num_blocks(&self) -> usize {
        self.num_blocks
    }
}

/// Hamming distance between two 64-bit fingerprints.
pub fn hamming_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_block_count() {
        assert!(SimHashLsh::<u32>::new(0).is_err());
        assert!(SimHashLsh::<u32>::new(5).is_err()); // 5 does not divide 64
        assert!(SimHashLsh::<u32>::new(4).is_ok());
        assert!(SimHashLsh::<u32>::new(8).is_ok());
    }

    #[test]
    fn identical_is_candidate() {
        let mut lsh: SimHashLsh<u32> = SimHashLsh::new(4).unwrap();
        let sig = 0x1234_5678_9ABC_DEF0u64;
        lsh.insert(1, sig);
        assert!(lsh.query(sig).contains(&1));
    }

    #[test]
    fn near_duplicate_shares_a_block() {
        let mut lsh: SimHashLsh<u32> = SimHashLsh::new(4).unwrap();
        let sig = 0xFFFF_0000_FFFF_0000u64;
        lsh.insert(2, sig ^ 0b1); // 1-bit difference, all in block 0 => blocks 1..3 match
        let c = lsh.query(sig);
        assert!(
            c.contains(&2),
            "1-bit-different fingerprint must be a candidate"
        );
    }

    #[test]
    fn distant_fingerprints_rarely_collide() {
        let mut lsh: SimHashLsh<u32> = SimHashLsh::new(8).unwrap();
        let sig = 0x0000_0000_0000_0000u64;
        let far = 0xFFFF_FFFF_FFFF_FFFFu64; // all 64 bits differ
        lsh.insert(2, far);
        assert!(
            !lsh.query(sig).contains(&2),
            "fully-different sig should not collide"
        );
    }

    #[test]
    fn query_deduplicates() {
        let mut lsh: SimHashLsh<u32> = SimHashLsh::new(8).unwrap();
        let sig = 0xDEAD_BEEF_CAFE_BABEu64;
        lsh.insert(7, sig); // shares all 8 blocks with itself
        let c = lsh.query(sig);
        assert_eq!(c.iter().filter(|&&x| x == 7).count(), 1);
    }

    #[test]
    fn hamming_distance_works() {
        assert_eq!(hamming_distance(0b1010, 0b1010), 0);
        assert_eq!(hamming_distance(0b1010, 0b0101), 4);
        assert_eq!(hamming_distance(0, u64::MAX), 64);
    }
}
