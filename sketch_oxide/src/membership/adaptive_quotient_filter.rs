//! Adaptive Quotient Filter (AQF) — a filter that *fixes* false positives on feedback.
//!
//! An ordinary approximate-membership filter has a fixed false-positive rate, and the *same*
//! non-member keeps triggering the same false positive forever. The Adaptive Quotient Filter
//! (Bender, Farach-Colton, Kuszmaul, Pandey et al., "Bloom Filters, Adaptivity, and the Dictionary
//! Problem", FOCS 2018; the AQF of Pandey et al.) lets a caller **report** a false positive and
//! *adapt*: the colliding stored fingerprint is **extended** with more bits of its resident's hash
//! until it no longer matches the offending query — so that query (and queries like it) stops
//! false-positiving, while no membership is ever lost. Adapting on a stream of negative feedback
//! drives the *sustained* false-positive rate toward zero.
//!
//! This builds on the quotient-filter substrate of
//! [`CountingQuotientFilter`](crate::membership::CountingQuotientFilter): a key's hash splits into a
//! quotient (bucket), a remainder, and a run of *extension* bits revealed on demand.
//!
//! # Layout note
//!
//! Each slot here keeps the resident's full hash plus the current extension length — the clear,
//! verifiable form of the adaptive contract. The published AQF stores only the *minimal* extension
//! bits (a few bits per slot) in a rank-and-select layout; that is a space optimization over this
//! same contract and is a documented follow-up.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

/// One occupied slot: the resident's hash and how many extension bits are currently "revealed".
#[derive(Debug, Clone, Copy)]
struct Slot {
    hash: u64,
    ext_len: u32,
}

/// An Adaptive Quotient Filter over `2^q_bits` buckets with `r_bits` remainders.
///
/// # Example
/// ```
/// use sketch_oxide::membership::AdaptiveQuotientFilter;
///
/// let mut aqf = AdaptiveQuotientFilter::new(10, 8).unwrap();
/// aqf.insert(b"resident");
/// assert!(aqf.contains(b"resident"));        // no false negatives
///
/// // Suppose some key false-positives; reporting it adapts the filter so it stops.
/// if aqf.contains(b"intruder") {
///     aqf.adapt(b"intruder");
///     assert!(!aqf.contains(b"intruder"));   // fixed
/// }
/// assert!(aqf.contains(b"resident"));         // membership preserved
/// ```
#[derive(Debug, Clone)]
pub struct AdaptiveQuotientFilter {
    q_bits: u32,
    r_bits: u32,
    max_ext: u32,
    buckets: Vec<Vec<Slot>>,
    len: usize,
    adaptations: usize,
}

impl AdaptiveQuotientFilter {
    /// Creates a filter with `2^q_bits` buckets and `r_bits`-bit remainders.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `q_bits` or `r_bits` is 0, or `q_bits + r_bits > 64`.
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
            max_ext: 64 - q_bits - r_bits,
            buckets: vec![Vec::new(); 1usize << q_bits],
            len: 0,
            adaptations: 0,
        })
    }

    #[inline]
    fn bucket_of(&self, hash: u64) -> usize {
        (hash & ((1u64 << self.q_bits) - 1)) as usize
    }

    #[inline]
    fn remainder_of(&self, hash: u64) -> u64 {
        (hash >> self.q_bits) & ((1u64 << self.r_bits) - 1)
    }

    /// The `ext_len` extension bits just above the remainder.
    #[inline]
    fn ext_of(&self, hash: u64, ext_len: u32) -> u64 {
        if ext_len == 0 {
            return 0;
        }
        (hash >> (self.q_bits + self.r_bits)) & ((1u64 << ext_len) - 1)
    }

    /// Whether `a` and `b` agree on remainder and the first `ext_len` extension bits.
    #[inline]
    fn matches(&self, a: u64, b: u64, ext_len: u32) -> bool {
        self.remainder_of(a) == self.remainder_of(b)
            && self.ext_of(a, ext_len) == self.ext_of(b, ext_len)
    }

    /// Inserts a key.
    pub fn insert(&mut self, key: &[u8]) {
        let h = xxhash(key, 0);
        let b = self.bucket_of(h);
        // De-duplicate identical fingerprints (no benefit to storing twice).
        if self.buckets[b].iter().any(|s| s.hash == h) {
            return;
        }
        self.buckets[b].push(Slot {
            hash: h,
            ext_len: 0,
        });
        self.len += 1;
    }

    /// Whether `key` is (probably) present. No false negatives.
    pub fn contains(&self, key: &[u8]) -> bool {
        let h = xxhash(key, 0);
        let b = self.bucket_of(h);
        self.buckets[b]
            .iter()
            .any(|s| self.matches(s.hash, h, s.ext_len))
    }

    /// Reports `key` as a false positive and adapts: every slot currently matching `key` is extended
    /// until it no longer matches (if possible). Returns the number of slots adapted.
    ///
    /// Adapting a genuine member is a no-op for membership — its own hash always matches itself.
    pub fn adapt(&mut self, key: &[u8]) -> usize {
        let h = xxhash(key, 0);
        let b = self.bucket_of(h);
        let (q, r, max_ext) = (self.q_bits, self.r_bits, self.max_ext);
        let mut adapted = 0;
        for slot in &mut self.buckets[b] {
            if slot.hash == h {
                continue; // identical fingerprint (a real collision or the key itself): cannot adapt
            }
            // Currently matching? Extend until the resident's extension bits differ from key's.
            let matching = Self::matches_raw(slot.hash, h, slot.ext_len, q, r);
            if !matching {
                continue;
            }
            while slot.ext_len < max_ext && Self::matches_raw(slot.hash, h, slot.ext_len + 1, q, r)
            {
                slot.ext_len += 1;
            }
            if slot.ext_len < max_ext {
                slot.ext_len += 1; // the bit at which they now differ
            }
            adapted += 1;
        }
        if adapted > 0 {
            self.adaptations += 1;
        }
        adapted
    }

    /// Free-function form of `matches` (avoids borrowing `self` while iterating mutably).
    #[inline]
    fn matches_raw(a: u64, b: u64, ext_len: u32, q: u32, r: u32) -> bool {
        let rem_mask = (1u64 << r) - 1;
        if ((a >> q) & rem_mask) != ((b >> q) & rem_mask) {
            return false;
        }
        if ext_len == 0 {
            return true;
        }
        let ext_mask = (1u64 << ext_len) - 1;
        ((a >> (q + r)) & ext_mask) == ((b >> (q + r)) & ext_mask)
    }

    /// Number of keys inserted.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether nothing has been inserted.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Number of `adapt` calls that changed at least one slot.
    #[inline]
    pub fn adaptations(&self) -> usize {
        self.adaptations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(AdaptiveQuotientFilter::new(0, 8).is_err());
        assert!(AdaptiveQuotientFilter::new(8, 0).is_err());
        assert!(AdaptiveQuotientFilter::new(40, 40).is_err());
        assert!(AdaptiveQuotientFilter::new(10, 8).is_ok());
    }

    #[test]
    fn no_false_negatives() {
        let mut aqf = AdaptiveQuotientFilter::new(12, 8).unwrap();
        for i in 0..5000u64 {
            aqf.insert(&i.to_le_bytes());
        }
        for i in 0..5000u64 {
            assert!(aqf.contains(&i.to_le_bytes()), "false negative for {i}");
        }
    }

    #[test]
    fn adapt_fixes_a_false_positive() {
        // Build a filter and find a non-member that false-positives, then adapt it away.
        let mut aqf = AdaptiveQuotientFilter::new(8, 6).unwrap(); // small r ⇒ more FPs to find one
        for i in 0..2000u64 {
            aqf.insert(&i.to_le_bytes());
        }
        // Find a false positive among non-members.
        let mut fp_key = None;
        for i in 1_000_000u64..1_100_000 {
            if aqf.contains(&i.to_le_bytes()) {
                fp_key = Some(i);
                break;
            }
        }
        let fp = fp_key.expect("expected at least one false positive with a 6-bit remainder");
        assert!(aqf.contains(&fp.to_le_bytes()));
        let n = aqf.adapt(&fp.to_le_bytes());
        assert!(n > 0, "adapt should have touched a slot");
        assert!(
            !aqf.contains(&fp.to_le_bytes()),
            "false positive should be fixed"
        );
    }

    #[test]
    fn adapt_preserves_membership() {
        let mut aqf = AdaptiveQuotientFilter::new(8, 6).unwrap();
        let members: Vec<u64> = (0..3000).collect();
        for &m in &members {
            aqf.insert(&m.to_le_bytes());
        }
        // Adapt against many non-members.
        for i in 5_000_000u64..5_010_000 {
            if aqf.contains(&i.to_le_bytes()) {
                aqf.adapt(&i.to_le_bytes());
            }
        }
        // Every member must still be found.
        for &m in &members {
            assert!(aqf.contains(&m.to_le_bytes()), "lost member {m}");
        }
    }

    #[test]
    fn adapting_drives_down_sustained_fpr() {
        // Re-querying the same negatives after adapting on each should produce far fewer FPs.
        let mut aqf = AdaptiveQuotientFilter::new(10, 6).unwrap();
        for i in 0..4000u64 {
            aqf.insert(&i.to_le_bytes());
        }
        let negatives: Vec<u64> = (10_000_000u64..10_010_000).collect();
        let fp_before = negatives
            .iter()
            .filter(|&&q| aqf.contains(&q.to_le_bytes()))
            .count();
        for &q in &negatives {
            if aqf.contains(&q.to_le_bytes()) {
                aqf.adapt(&q.to_le_bytes());
            }
        }
        let fp_after = negatives
            .iter()
            .filter(|&&q| aqf.contains(&q.to_le_bytes()))
            .count();
        assert!(fp_before > 0, "expected some initial false positives");
        assert!(
            fp_after < fp_before / 4,
            "sustained FPR not reduced: {fp_before} -> {fp_after}"
        );
    }
}
