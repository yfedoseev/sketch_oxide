//! Prefix Filter — a fast two-level approximate-membership filter.
//!
//! The Prefix Filter (Even, Even & Morrison, "The Prefix Filter: Practically and Theoretically
//! Better Than Bloom", VLDB 2023) is a cache-friendly Bloom alternative built from two levels.
//! Each key is routed by part of its hash to one of many **bins**; the bin stores a small set of
//! fingerprints (the paper's compact "Pocket Dictionary"). The clever part is what happens on
//! overflow: instead of growing every bin for the worst case, a bin that fills sends its extra
//! fingerprints to a shared second-level **spare** filter. Because overflow is rare, the bins stay
//! tiny and cache-local — giving Bloom-beating speed and space with **no false negatives** and a
//! bounded false-positive rate.
//!
//! # Layout note
//!
//! Bins here are explicit fingerprint vectors and the spare is an exact `(bin, fingerprint)` set —
//! the clear, verifiable form of the two-level contract. The paper's bit-packed Pocket Dictionary
//! and a cuckoo/quotient spare are space optimizations over this same contract and are documented
//! follow-ups; they shrink bytes, not the set of keys accepted.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use std::collections::HashSet;

/// A Prefix Filter over `bin_capacity`-slot bins with a shared spare.
///
/// # Example
/// ```
/// use sketch_oxide::membership::PrefixFilter;
///
/// let mut pf = PrefixFilter::new(1000, 12, 25).unwrap();
/// for i in 0..1000u64 { pf.insert(&i.to_le_bytes()); }
/// for i in 0..1000u64 { assert!(pf.contains(&i.to_le_bytes())); } // no false negatives
/// assert!(!pf.contains(b"definitely absent"));
/// ```
#[derive(Debug, Clone)]
pub struct PrefixFilter {
    num_bins: usize,
    bin_capacity: usize,
    fp_mask: u16,
    /// Per-bin fingerprint lists (first level).
    bins: Vec<Vec<u16>>,
    /// Overflow fingerprints keyed by `(bin, fingerprint)` (second level).
    spare: HashSet<(u32, u16)>,
    len: usize,
}

impl PrefixFilter {
    /// Creates a filter sized for `expected_keys`, with `fp_bits`-bit fingerprints (`2..=16`) and
    /// `bin_capacity` fingerprints per bin before overflow.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `expected_keys`/`bin_capacity` is 0 or `fp_bits` is
    /// outside `2..=16`.
    pub fn new(expected_keys: usize, fp_bits: u32, bin_capacity: usize) -> Result<Self> {
        if expected_keys == 0 || bin_capacity == 0 {
            return Err(SketchError::InvalidParameter {
                param: "expected_keys/bin_capacity".to_string(),
                value: format!("{expected_keys}/{bin_capacity}"),
                constraint: "both must be > 0".to_string(),
            });
        }
        if !(2..=16).contains(&fp_bits) {
            return Err(SketchError::InvalidParameter {
                param: "fp_bits".to_string(),
                value: fp_bits.to_string(),
                constraint: "must be in 2..=16".to_string(),
            });
        }
        // Aim for ~bin_capacity keys per bin so overflow is rare.
        let num_bins = expected_keys.div_ceil(bin_capacity).max(1);
        let fp_mask = if fp_bits == 16 {
            u16::MAX
        } else {
            (1u16 << fp_bits) - 1
        };
        Ok(Self {
            num_bins,
            bin_capacity,
            fp_mask,
            bins: vec![Vec::new(); num_bins],
            spare: HashSet::new(),
            len: 0,
        })
    }

    /// `(bin, fingerprint)` for a key.
    #[inline]
    fn locate(&self, key: &[u8]) -> (usize, u16) {
        let h = xxhash(key, 0);
        let bin = (h % self.num_bins as u64) as usize;
        // Use a different part of the hash for the fingerprint, forced nonzero.
        let fp = (((h >> 40) as u16) & self.fp_mask) | 1;
        (bin, fp)
    }

    /// Inserts a key.
    pub fn insert(&mut self, key: &[u8]) {
        let (bin, fp) = self.locate(key);
        let slot = &mut self.bins[bin];
        if slot.contains(&fp) {
            return; // fingerprint already present (this key, or a colliding one)
        }
        if slot.len() < self.bin_capacity {
            slot.push(fp);
        } else {
            self.spare.insert((bin as u32, fp));
        }
        self.len += 1;
    }

    /// Whether a key is (probably) present. No false negatives; false positives only on a
    /// fingerprint collision within the key's bin.
    pub fn contains(&self, key: &[u8]) -> bool {
        let (bin, fp) = self.locate(key);
        self.bins[bin].contains(&fp) || self.spare.contains(&(bin as u32, fp))
    }

    /// Number of insertions (including any that collided on a fingerprint).
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether nothing has been inserted.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Number of fingerprints that spilled into the spare (overflow). Useful for tuning.
    #[inline]
    pub fn spare_len(&self) -> usize {
        self.spare.len()
    }

    /// Number of bins.
    #[inline]
    pub fn num_bins(&self) -> usize {
        self.num_bins
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(PrefixFilter::new(0, 12, 10).is_err());
        assert!(PrefixFilter::new(100, 12, 0).is_err());
        assert!(PrefixFilter::new(100, 1, 10).is_err());
        assert!(PrefixFilter::new(100, 17, 10).is_err());
        assert!(PrefixFilter::new(100, 12, 10).is_ok());
    }

    #[test]
    fn no_false_negatives() {
        let mut pf = PrefixFilter::new(5000, 12, 25).unwrap();
        for i in 0..5000u64 {
            pf.insert(&i.to_le_bytes());
        }
        for i in 0..5000u64 {
            assert!(pf.contains(&i.to_le_bytes()), "false negative for {i}");
        }
    }

    #[test]
    fn no_false_negatives_string_keys() {
        let mut pf = PrefixFilter::new(2000, 14, 20).unwrap();
        let keys: Vec<String> = (0..2000).map(|i| format!("user::{i}")).collect();
        for k in &keys {
            pf.insert(k.as_bytes());
        }
        for k in &keys {
            assert!(pf.contains(k.as_bytes()));
        }
    }

    #[test]
    fn false_positive_rate_bounded() {
        let mut pf = PrefixFilter::new(4000, 12, 25).unwrap();
        for i in 0..4000u64 {
            pf.insert(&i.to_le_bytes());
        }
        let mut fp = 0;
        for i in 1_000_000..1_010_000u64 {
            if pf.contains(&i.to_le_bytes()) {
                fp += 1;
            }
        }
        let rate = fp as f64 / 10_000.0;
        assert!(rate < 0.05, "false-positive rate {rate} too high");
    }

    #[test]
    fn overflow_goes_to_spare_not_lost() {
        // Tiny bins force overflow; every key must still be found.
        let mut pf = PrefixFilter::new(20, 14, 2).unwrap(); // ~10 bins, capacity 2
        for i in 0..200u64 {
            pf.insert(&i.to_le_bytes());
        }
        assert!(pf.spare_len() > 0, "expected overflow into the spare");
        for i in 0..200u64 {
            assert!(pf.contains(&i.to_le_bytes()), "lost key {i}");
        }
    }

    #[test]
    fn empty_filter_rejects() {
        let pf = PrefixFilter::new(100, 12, 10).unwrap();
        assert!(pf.is_empty());
        assert!(!pf.contains(b"anything"));
    }
}

// ---------------------------------------------------------------------------
// Capability-trait adoption (fable5 doc 01 F3): express the inherent API via
// the orthogonal capability traits, delegating to the inherent methods.
// ---------------------------------------------------------------------------
use crate::common::capabilities::*;

impl Update<[u8]> for PrefixFilter {
    fn update(&mut self, item: &[u8]) {
        self.insert(item);
    }
}

impl Filter<[u8]> for PrefixFilter {
    fn contains(&self, item: &[u8]) -> bool {
        PrefixFilter::contains(self, item)
    }
}
