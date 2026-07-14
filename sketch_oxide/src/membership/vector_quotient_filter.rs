//! Vector Quotient Filter (VQF) — power-of-two-choices block filter with deletes.
//!
//! VQF (Pandey, Conway, Durie, Bender, Farach-Colton & Johnson, "Vector Quotient Filters: Overcoming
//! the Time/Space Trade-Off in Filter Design", SIGMOD 2021) is a fast, deletion-capable
//! approximate-membership filter. Its key idea is **power-of-two-choices** load balancing: each key
//! has *two* candidate blocks, and its tag is stored in whichever of the two is **less full**. That
//! balancing keeps every block far from overflowing, so the filter runs at very high load (≈ 0.95)
//! with small fixed-size blocks — which on real hardware are scanned with a couple of SIMD
//! instructions. This implementation is the portable scalar version of that contract; the SIMD
//! block scan is a drop-in performance optimization that does not change which keys are accepted.
//!
//! Because each tag occupies an explicit slot, deletion is exact (remove one matching tag), unlike
//! a Bloom filter.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

/// A Vector Quotient Filter over `num_blocks` blocks of `block_capacity` 16-bit tags.
///
/// # Example
/// ```
/// use sketch_oxide::membership::VectorQuotientFilter;
///
/// let mut vqf = VectorQuotientFilter::new(256, 48).unwrap();
/// assert!(vqf.insert(b"key"));
/// assert!(vqf.contains(b"key"));   // no false negatives
/// assert!(vqf.remove(b"key"));     // deletable
/// assert!(!vqf.contains(b"key"));
/// ```
#[derive(Debug, Clone)]
pub struct VectorQuotientFilter {
    num_blocks: usize,
    block_capacity: usize,
    /// One tag list per block; a 16-bit nonzero tag per stored key.
    blocks: Vec<Vec<u16>>,
    len: usize,
}

impl VectorQuotientFilter {
    /// Creates a filter with `num_blocks` blocks (rounded up to a power of two) of `block_capacity`
    /// tags each.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `num_blocks` or `block_capacity` is 0.
    pub fn new(num_blocks: usize, block_capacity: usize) -> Result<Self> {
        if num_blocks == 0 || block_capacity == 0 {
            return Err(SketchError::InvalidParameter {
                param: "num_blocks/block_capacity".to_string(),
                value: format!("{num_blocks}/{block_capacity}"),
                constraint: "both must be > 0".to_string(),
            });
        }
        let num_blocks = num_blocks.next_power_of_two();
        Ok(Self {
            num_blocks,
            block_capacity,
            blocks: vec![Vec::new(); num_blocks],
            len: 0,
        })
    }

    /// The two candidate blocks and the 16-bit tag for a key.
    #[inline]
    fn locate(&self, key: &[u8]) -> (usize, usize, u16) {
        let h = xxhash(key, 0);
        let mask = self.num_blocks - 1;
        let b1 = (h as usize) & mask;
        let b2 = ((h >> 32) as usize) & mask;
        let tag = ((h >> 16) as u16) | 1; // nonzero tag
        (b1, b2, tag)
    }

    /// Inserts a key. Returns `false` if both candidate blocks are full (the region is saturated).
    pub fn insert(&mut self, key: &[u8]) -> bool {
        let (b1, b2, tag) = self.locate(key);
        // Power of two choices: store in the less-full of the two candidate blocks.
        let target = if b1 == b2 || self.blocks[b1].len() <= self.blocks[b2].len() {
            b1
        } else {
            b2
        };
        if self.blocks[target].len() >= self.block_capacity {
            // Chosen block full; try the other.
            let other = if target == b1 { b2 } else { b1 };
            if self.blocks[other].len() >= self.block_capacity {
                return false; // both full
            }
            self.blocks[other].push(tag);
        } else {
            self.blocks[target].push(tag);
        }
        self.len += 1;
        true
    }

    /// Whether a key is (probably) present. No false negatives.
    pub fn contains(&self, key: &[u8]) -> bool {
        let (b1, b2, tag) = self.locate(key);
        self.blocks[b1].contains(&tag) || (b1 != b2 && self.blocks[b2].contains(&tag))
    }

    /// Removes one occurrence of a key. Returns `true` if a matching tag was found and removed.
    pub fn remove(&mut self, key: &[u8]) -> bool {
        let (b1, b2, tag) = self.locate(key);
        for b in if b1 == b2 { vec![b1] } else { vec![b1, b2] } {
            if let Some(pos) = self.blocks[b].iter().position(|&t| t == tag) {
                self.blocks[b].swap_remove(pos);
                self.len -= 1;
                return true;
            }
        }
        false
    }

    /// Number of tags currently stored.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the filter is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Total tag capacity (`num_blocks · block_capacity`).
    #[inline]
    pub fn capacity(&self) -> usize {
        self.num_blocks * self.block_capacity
    }

    /// Current load factor.
    pub fn load_factor(&self) -> f64 {
        self.len as f64 / self.capacity() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(VectorQuotientFilter::new(0, 48).is_err());
        assert!(VectorQuotientFilter::new(256, 0).is_err());
        assert!(VectorQuotientFilter::new(256, 48).is_ok());
    }

    #[test]
    fn blocks_rounded_to_power_of_two() {
        let vqf = VectorQuotientFilter::new(200, 48).unwrap();
        assert_eq!(vqf.capacity(), 256 * 48);
    }

    #[test]
    fn no_false_negatives() {
        let mut vqf = VectorQuotientFilter::new(1024, 48).unwrap();
        for i in 0..30_000u64 {
            assert!(vqf.insert(&i.to_le_bytes()), "insert failed at {i}");
        }
        for i in 0..30_000u64 {
            assert!(vqf.contains(&i.to_le_bytes()), "false negative for {i}");
        }
    }

    #[test]
    fn deletes_work() {
        let mut vqf = VectorQuotientFilter::new(256, 48).unwrap();
        vqf.insert(b"x");
        vqf.insert(b"y");
        assert!(vqf.contains(b"x"));
        assert!(vqf.remove(b"x"));
        assert!(!vqf.remove(b"x")); // already gone
        assert!(vqf.contains(b"y"));
        assert_eq!(vqf.len(), 1);
    }

    #[test]
    fn power_of_two_choices_reaches_high_load() {
        // The less-full-of-two rule should let the filter fill to a high load before any failure.
        let mut vqf = VectorQuotientFilter::new(512, 32).unwrap();
        let cap = vqf.capacity();
        let mut inserted = 0;
        for i in 0..cap as u64 {
            if vqf.insert(&i.to_le_bytes()) {
                inserted += 1;
            } else {
                break;
            }
        }
        assert!(
            vqf.load_factor() > 0.85,
            "only reached load {} ({inserted}/{cap})",
            vqf.load_factor()
        );
    }

    #[test]
    fn false_positive_rate_bounded() {
        let mut vqf = VectorQuotientFilter::new(1024, 48).unwrap();
        for i in 0..20_000u64 {
            vqf.insert(&i.to_le_bytes());
        }
        let mut fp = 0;
        for i in 1_000_000..1_010_000u64 {
            if vqf.contains(&i.to_le_bytes()) {
                fp += 1;
            }
        }
        // ~2·tags_per_block / 2^16 per query; comfortably under 5%.
        let rate = fp as f64 / 10_000.0;
        assert!(rate < 0.05, "false-positive rate {rate}");
    }
}

// ---------------------------------------------------------------------------
// Capability-trait adoption (fable5 doc 01 F3): express the inherent API via
// the orthogonal capability traits, delegating to the inherent methods.
// ---------------------------------------------------------------------------
use crate::common::capabilities::*;

impl Update<[u8]> for VectorQuotientFilter {
    fn update(&mut self, item: &[u8]) {
        let _ = self.insert(item);
    }
}

impl Filter<[u8]> for VectorQuotientFilter {
    fn contains(&self, item: &[u8]) -> bool {
        VectorQuotientFilter::contains(self, item)
    }
}
