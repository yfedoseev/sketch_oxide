//! Aleph Filter — an expandable membership filter that grows to infinity in constant time
//! (Dayan, Bercea & Pagh, VLDB 2024), the successor to InfiniFilter.
//!
//! A static filter must know the data size up front. Expandable filters double their capacity on
//! demand, but doing so cheaply *and* keeping the false-positive rate (FPR) from degrading is hard.
//! Aleph Filter (and its predecessor InfiniFilter) is a quotient filter that supports **variable-
//! length fingerprints**: an item's `B`-bit hash splits into a `k`-bit canonical bucket address (the
//! low bits) and an `F`-bit fingerprint (the next bits). When the filter **doubles** (`k → k+1`),
//! each stored fingerprint donates its least-significant bit to become the new high bit of the bucket
//! address — so every entry migrates to its correct bucket in the larger table while its fingerprint
//! *shrinks by one bit*. New items inserted after an expansion get full `F`-bit fingerprints again.
//!
//! After `F` expansions an old entry exhausts its fingerprint bits and becomes a **void entry**
//! (length 0), which matches every query to its bucket. InfiniFilter banishes void entries to a chain
//! of secondary tables (queries then cost `O(log N / F)`). **Aleph Filter's** insight is to instead
//! *duplicate* each void entry across **both** buckets it could map to on the next expansion; the
//! collective FPR contribution of the duplicates equals that of a single non-void entry (the
//! generations are geometrically distributed), so the FPR matches InfiniFilter while every query
//! costs a single bucket access — **`O(1)`**.
//!
//! # Layout note
//!
//! This is a behaviour-faithful reference layout: each bucket is an explicit list of
//! `(fingerprint, length)` entries, rather than the paper's packed quotient-filter slot array with
//! `occupied`/`runend` metadata bits and run shifting. It reproduces the same membership semantics —
//! no false negatives ever, the same variable-length-fingerprint FPR, the same void-entry
//! duplication and expansion dynamics — while leaving the rank-and-select bit-packing (a space/cache
//! optimisation) as a follow-up, matching this crate's [`CountingQuotientFilter`] approach. This
//! implements the **Fixed-Width** regime; deletion (tombstones + deferred cleanup) is a follow-up.
//!
//! [`CountingQuotientFilter`]: crate::membership::CountingQuotientFilter

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

const ALEPH_SEED: u64 = 0xA1EF_0000_0000_0001;

/// One stored entry: a fingerprint and how many of its bits are still valid (`0` ⇒ a void entry).
#[derive(Debug, Clone, Copy)]
struct Entry {
    fp: u64,
    len: u32,
}

/// An Aleph Filter with `2^k` buckets and `F`-bit fingerprints, doubling in size as it fills.
///
/// # Example
/// ```
/// use sketch_oxide::membership::AlephFilter;
///
/// // Start at 2^8 buckets, 16-bit fingerprints; it expands automatically as items are added.
/// let mut f = AlephFilter::new(8, 16).unwrap();
/// for i in 0..100_000u32 {
///     f.insert(&i.to_le_bytes());
/// }
/// // No false negatives, ever — every inserted item is found, across all the expansions.
/// for i in 0..100_000u32 {
///     assert!(f.contains(&i.to_le_bytes()));
/// }
/// // Absent items are almost never reported (bounded false-positive rate).
/// let fp = (200_000..300_000u32)
///     .filter(|i| f.contains(&i.to_le_bytes()))
///     .count();
/// assert!(fp < 1_000, "false positives: {fp}");
/// ```
#[derive(Debug, Clone)]
pub struct AlephFilter {
    k: u32,
    f: u32,
    buckets: Vec<Vec<Entry>>,
    /// Number of logically-inserted items (void duplicates are not double-counted).
    count: usize,
    seed: u64,
}

impl AlephFilter {
    /// Expand when the load factor (`count / 2^k`) reaches this value.
    const MAX_LOAD: f64 = 0.9;

    /// Creates an Aleph Filter starting with `2^initial_log_buckets` buckets and `fingerprint_bits`
    /// fingerprints.
    ///
    /// `fingerprint_bits` must be in `1..=32`, `initial_log_buckets` in `1..=32`, and their sum at
    /// most 64 (the hash width). The FPR is roughly `2^-fingerprint_bits · (log₂N + 2)`; pick
    /// `fingerprint_bits ≈ log₂(1/target_fpr) + log₂log₂N`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if a parameter is out of range or the sum exceeds 64.
    pub fn new(initial_log_buckets: u32, fingerprint_bits: u32) -> Result<Self> {
        if !(1..=32).contains(&initial_log_buckets) {
            return Err(SketchError::InvalidParameter {
                param: "initial_log_buckets".to_string(),
                value: initial_log_buckets.to_string(),
                constraint: "must be in 1..=32".to_string(),
            });
        }
        if !(1..=32).contains(&fingerprint_bits) {
            return Err(SketchError::InvalidParameter {
                param: "fingerprint_bits".to_string(),
                value: fingerprint_bits.to_string(),
                constraint: "must be in 1..=32".to_string(),
            });
        }
        if initial_log_buckets + fingerprint_bits > 64 {
            return Err(SketchError::InvalidParameter {
                param: "initial_log_buckets + fingerprint_bits".to_string(),
                value: (initial_log_buckets + fingerprint_bits).to_string(),
                constraint: "must be <= 64 (hash width)".to_string(),
            });
        }
        Ok(Self {
            k: initial_log_buckets,
            f: fingerprint_bits,
            buckets: vec![Vec::new(); 1usize << initial_log_buckets],
            count: 0,
            seed: ALEPH_SEED,
        })
    }

    #[inline]
    fn hash(&self, item: &[u8]) -> u64 {
        xxhash(item, self.seed)
    }

    /// Number of buckets (`2^k`).
    #[inline]
    pub fn num_buckets(&self) -> usize {
        self.buckets.len()
    }

    /// Number of items inserted.
    #[inline]
    pub fn len(&self) -> usize {
        self.count
    }

    /// Whether no items have been inserted.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Inserts `item`, doubling the filter first if it has reached its load threshold.
    pub fn insert(&mut self, item: &[u8]) {
        if self.count as f64 >= Self::MAX_LOAD * self.buckets.len() as f64 {
            self.expand();
        }
        let h = self.hash(item);
        let addr = (h & ((1u64 << self.k) - 1)) as usize;
        let fp = (h >> self.k) & ((1u64 << self.f) - 1);
        self.buckets[addr].push(Entry { fp, len: self.f });
        self.count += 1;
    }

    /// Tests membership. Never a false negative; false positives occur with bounded probability.
    pub fn contains(&self, item: &[u8]) -> bool {
        let h = self.hash(item);
        let addr = (h & ((1u64 << self.k) - 1)) as usize;
        let qfp = h >> self.k;
        self.buckets[addr].iter().any(|e| {
            // A void entry (len 0) matches everything in its bucket.
            e.len == 0 || (qfp & ((1u64 << e.len) - 1)) == (e.fp & ((1u64 << e.len) - 1))
        })
    }

    /// Doubles the number of buckets, migrating every entry (shrinking non-void fingerprints by one
    /// bit, duplicating void entries across both candidate buckets).
    fn expand(&mut self) {
        // Out of hash bits: new inserts would have no fingerprint room, so stop growing.
        if self.k + self.f >= 64 {
            return;
        }
        let new_k = self.k + 1;
        let mut nb: Vec<Vec<Entry>> = vec![Vec::new(); 1usize << new_k];
        let high_bit = 1usize << self.k;
        for (addr, bucket) in self.buckets.iter().enumerate() {
            for &e in bucket {
                if e.len == 0 {
                    // Void: duplicate into both buckets it might have mapped to.
                    nb[addr].push(e);
                    nb[addr | high_bit].push(e);
                } else {
                    // Donate the fingerprint's LSB to the new high address bit.
                    let new_addr = addr | (((e.fp & 1) as usize) << self.k);
                    nb[new_addr].push(Entry {
                        fp: e.fp >> 1,
                        len: e.len - 1,
                    });
                }
            }
        }
        self.buckets = nb;
        self.k = new_k;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(AlephFilter::new(0, 16).is_err());
        assert!(AlephFilter::new(8, 0).is_err());
        assert!(AlephFilter::new(8, 33).is_err());
        assert!(AlephFilter::new(40, 30).is_err()); // sum > 64
        assert!(AlephFilter::new(8, 16).is_ok());
    }

    #[test]
    fn empty_contains_nothing_inserted() {
        let f = AlephFilter::new(6, 12).unwrap();
        assert!(f.is_empty());
        // An empty filter may still false-positive in principle, but with no entries it cannot.
        assert!(!f.contains(b"anything"));
    }

    #[test]
    fn no_false_negatives_across_expansions() {
        let mut f = AlephFilter::new(8, 16).unwrap();
        let n = 200_000u32;
        for i in 0..n {
            f.insert(&i.to_le_bytes());
        }
        assert_eq!(f.len(), n as usize);
        assert!(f.num_buckets() > (1 << 8), "filter should have expanded");
        // The defining invariant: every inserted item is still present after all the doublings.
        for i in 0..n {
            assert!(f.contains(&i.to_le_bytes()), "missing {i}");
        }
    }

    #[test]
    fn false_positive_rate_is_bounded() {
        let mut f = AlephFilter::new(8, 16).unwrap();
        let n = 100_000u32;
        for i in 0..n {
            f.insert(&i.to_le_bytes());
        }
        let trials = 200_000u32;
        let fps = (n..n + trials)
            .filter(|i| f.contains(&i.to_le_bytes()))
            .count();
        let fpr = fps as f64 / trials as f64;
        // With 16-bit fingerprints and ~9 expansions, FPR ≈ 2^-16·(log₂N+2) ≈ 3e-4; allow slack.
        assert!(fpr < 0.01, "FPR {fpr} too high ({fps}/{trials})");
    }

    #[test]
    fn void_entries_preserve_membership() {
        // Tiny fingerprints (F=3) force entries to erode to void after a few expansions, exercising
        // the void-duplication path. No false negatives must still hold.
        let mut f = AlephFilter::new(3, 3).unwrap();
        let n = 2_000u32;
        for i in 0..n {
            f.insert(&i.to_le_bytes());
        }
        // Should have expanded well past F=3 doublings, creating void entries.
        assert!(f.num_buckets() >= (1 << 9), "buckets: {}", f.num_buckets());
        for i in 0..n {
            assert!(
                f.contains(&i.to_le_bytes()),
                "missing {i} after void expansions"
            );
        }
    }
}
