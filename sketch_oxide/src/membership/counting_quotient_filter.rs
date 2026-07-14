//! Counting Quotient Filter (CQF) — a counting, deletable, mergeable filter.
//!
//! A Quotient Filter stores `p`-bit fingerprints by *quotienting*: the fingerprint splits into
//! a quotient `q` (which bucket) and a remainder `r` (what is stored), so only the `r_bits`
//! remainder is kept per item rather than the whole key. The **Counting** Quotient Filter
//! (Pandey, Bender, Johnson & Patro, "A General-Purpose Counting Filter: Making Every Bit
//! Count", SIGMOD 2017) attaches a count to each remainder, giving the one filter that is
//! simultaneously mergeable, **deletable**, and **counting** — the substrate that
//! feature-rich filters (Aleph, AQF, Memento) build on.
//!
//! # Layout note
//!
//! This implementation uses an explicit per-quotient bucket of `(remainder, count)` pairs. It
//! is functionally exact — the same false-positive semantics, counts, deletions, and merges as
//! the RSQF — and is the clear, verifiable reference layout. The rank-and-select *packed* slot
//! array of the RSQF (3 metadata bits/slot, run shifting) is a cache/space optimization over
//! this same contract and is left as a follow-up.
//!
//! # Guarantees
//!
//! - **No false negatives**: an inserted key always reports a count `≥` its true count.
//! - **Bounded false positives / over-count**: a query for an absent key returns non-zero only
//!   on a remainder collision, with probability `≈ load / 2^r_bits`.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

/// A Counting Quotient Filter over `2^q_bits` buckets with `r_bits` remainders.
///
/// # Example
/// ```
/// use sketch_oxide::membership::CountingQuotientFilter;
///
/// let mut cqf = CountingQuotientFilter::new(16, 12).unwrap();
/// cqf.insert(b"apple");
/// cqf.insert(b"apple");
/// cqf.insert(b"pear");
/// assert!(cqf.contains(b"apple"));
/// assert_eq!(cqf.count(b"apple"), 2);     // counting
/// cqf.remove(b"apple");
/// assert_eq!(cqf.count(b"apple"), 1);     // deletable
/// assert!(!cqf.contains(b"banana"));
/// ```
#[derive(Debug, Clone)]
pub struct CountingQuotientFilter {
    q_bits: u32,
    r_bits: u32,
    buckets: Vec<Vec<(u64, u64)>>,
    distinct: usize,
}

impl CountingQuotientFilter {
    /// Creates a CQF with `2^q_bits` buckets and `r_bits`-bit remainders.
    ///
    /// The false-positive rate is `≈ load / 2^r_bits`; pick `q_bits` for the expected item
    /// count and `r_bits` for the target FPR. `q_bits + r_bits` must not exceed 64.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `q_bits` or `r_bits` is 0, or their sum exceeds 64.
    pub fn new(q_bits: u32, r_bits: u32) -> Result<Self> {
        if q_bits == 0 || r_bits == 0 || q_bits + r_bits > 64 {
            return Err(SketchError::InvalidParameter {
                param: "q_bits/r_bits".to_string(),
                value: format!("{q_bits}/{r_bits}"),
                constraint: "each > 0 and q_bits + r_bits <= 64".to_string(),
            });
        }
        Ok(Self {
            q_bits,
            r_bits,
            buckets: vec![Vec::new(); 1usize << q_bits],
            distinct: 0,
        })
    }

    /// Splits a key into `(quotient, remainder)`.
    #[inline]
    fn fingerprint(&self, key: &[u8]) -> (usize, u64) {
        let h = xxhash(key, 0);
        let fp = h & ((1u128 << (self.q_bits + self.r_bits)) as u64).wrapping_sub(1);
        let q = (fp >> self.r_bits) as usize & ((1usize << self.q_bits) - 1);
        let r_mask = if self.r_bits == 64 {
            u64::MAX
        } else {
            (1u64 << self.r_bits) - 1
        };
        (q, fp & r_mask)
    }

    /// Inserts one occurrence of `key`.
    pub fn insert(&mut self, key: &[u8]) {
        self.insert_count(key, 1);
    }

    /// Inserts `count` occurrences of `key`.
    pub fn insert_count(&mut self, key: &[u8], count: u64) {
        if count == 0 {
            return;
        }
        let (q, r) = self.fingerprint(key);
        let bucket = &mut self.buckets[q];
        match bucket.iter_mut().find(|(rem, _)| *rem == r) {
            Some(entry) => entry.1 += count,
            None => {
                bucket.push((r, count));
                self.distinct += 1;
            }
        }
    }

    /// Estimated count of `key` (0 if absent). May over-count on a remainder collision.
    pub fn count(&self, key: &[u8]) -> u64 {
        let (q, r) = self.fingerprint(key);
        self.buckets[q]
            .iter()
            .find(|(rem, _)| *rem == r)
            .map_or(0, |(_, c)| *c)
    }

    /// Whether `key` is (probably) present.
    pub fn contains(&self, key: &[u8]) -> bool {
        self.count(key) > 0
    }

    /// Removes one occurrence of `key`. Returns the remaining count. Removing below zero is
    /// clamped at zero and drops the remainder.
    pub fn remove(&mut self, key: &[u8]) -> u64 {
        let (q, r) = self.fingerprint(key);
        let bucket = &mut self.buckets[q];
        if let Some(pos) = bucket.iter().position(|(rem, _)| *rem == r) {
            if bucket[pos].1 > 1 {
                bucket[pos].1 -= 1;
                bucket[pos].1
            } else {
                bucket.swap_remove(pos);
                self.distinct -= 1;
                0
            }
        } else {
            0
        }
    }

    /// Number of distinct retained remainders (an upper bound on distinct keys).
    #[inline]
    pub fn len(&self) -> usize {
        self.distinct
    }

    /// Whether nothing is stored.
    pub fn is_empty(&self) -> bool {
        self.distinct == 0
    }

    /// Quotient bits.
    #[inline]
    pub fn q_bits(&self) -> u32 {
        self.q_bits
    }

    /// Remainder bits.
    #[inline]
    pub fn r_bits(&self) -> u32 {
        self.r_bits
    }

    /// Merges another CQF (counts add). Both must have the same `(q_bits, r_bits)`.
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the configurations differ.
    pub fn merge(&mut self, other: &Self) -> Result<()> {
        if self.q_bits != other.q_bits || self.r_bits != other.r_bits {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "config mismatch: {}/{} vs {}/{}",
                    self.q_bits, self.r_bits, other.q_bits, other.r_bits
                ),
            });
        }
        for (q, obucket) in other.buckets.iter().enumerate() {
            for &(r, c) in obucket {
                let bucket = &mut self.buckets[q];
                match bucket.iter_mut().find(|(rem, _)| *rem == r) {
                    Some(entry) => entry.1 += c,
                    None => {
                        bucket.push((r, c));
                        self.distinct += 1;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(CountingQuotientFilter::new(0, 8).is_err());
        assert!(CountingQuotientFilter::new(8, 0).is_err());
        assert!(CountingQuotientFilter::new(40, 40).is_err()); // sum > 64
        assert!(CountingQuotientFilter::new(16, 12).is_ok());
    }

    #[test]
    fn no_false_negatives() {
        let mut cqf = CountingQuotientFilter::new(16, 12).unwrap();
        for i in 0..30_000u64 {
            cqf.insert(&i.to_le_bytes());
        }
        for i in 0..30_000u64 {
            assert!(cqf.contains(&i.to_le_bytes()), "false negative for {i}");
        }
    }

    #[test]
    fn counts_occurrences() {
        let mut cqf = CountingQuotientFilter::new(16, 16).unwrap();
        for _ in 0..1000 {
            cqf.insert(b"hot");
        }
        assert_eq!(cqf.count(b"hot"), 1000);
    }

    #[test]
    fn deletes() {
        let mut cqf = CountingQuotientFilter::new(16, 16).unwrap();
        cqf.insert_count(b"x", 3);
        assert_eq!(cqf.remove(b"x"), 2);
        assert_eq!(cqf.remove(b"x"), 1);
        assert_eq!(cqf.remove(b"x"), 0);
        assert!(!cqf.contains(b"x"));
        assert_eq!(cqf.remove(b"x"), 0); // removing absent is harmless
    }

    #[test]
    fn false_positive_rate_bounded() {
        let mut cqf = CountingQuotientFilter::new(14, 12).unwrap(); // ~16k buckets, 12-bit rem
        for i in 0..8000u64 {
            cqf.insert(&i.to_le_bytes());
        }
        let mut fp = 0;
        for i in 1_000_000..1_010_000u64 {
            if cqf.contains(&i.to_le_bytes()) {
                fp += 1;
            }
        }
        // FPR ~ load / 2^12; well under 5%.
        assert!(fp < 300, "false positives {fp}/10000");
    }

    #[test]
    fn merge_adds_counts() {
        let mut a = CountingQuotientFilter::new(16, 16).unwrap();
        let mut b = CountingQuotientFilter::new(16, 16).unwrap();
        a.insert_count(b"shared", 2);
        a.insert(b"a_only");
        b.insert_count(b"shared", 5);
        b.insert(b"b_only");
        a.merge(&b).unwrap();
        assert_eq!(a.count(b"shared"), 7);
        assert!(a.contains(b"a_only") && a.contains(b"b_only"));
    }

    #[test]
    fn merge_mismatch_errors() {
        let mut a = CountingQuotientFilter::new(16, 12).unwrap();
        let b = CountingQuotientFilter::new(16, 16).unwrap();
        assert!(a.merge(&b).is_err());
    }
}

// ---------------------------------------------------------------------------
// Capability-trait adoption (fable5 doc 01 F3): express the inherent API via
// the orthogonal capability traits, delegating to the inherent methods.
// ---------------------------------------------------------------------------
use crate::common::capabilities::*;

impl Update<[u8]> for CountingQuotientFilter {
    fn update(&mut self, item: &[u8]) {
        self.insert(item);
    }
}

impl Filter<[u8]> for CountingQuotientFilter {
    fn contains(&self, item: &[u8]) -> bool {
        CountingQuotientFilter::contains(self, item)
    }
}
