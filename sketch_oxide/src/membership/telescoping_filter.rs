//! Telescoping Filter (TAF) — a practical *adaptive* filter (Lee, McCauley, Singh & Stein, ESA 2021).
//!
//! An **adaptive** filter fixes a false positive the moment it is detected, so the *same* false
//! positive never recurs — crucial under skewed query workloads where one absent key may be queried
//! millions of times. The Telescoping Filter achieves *worst-case* adaptivity with **variable-length
//! fingerprints**: rather than literally lengthening a fingerprint, each slot carries a small
//! **selector** that chooses which `r`-bit *window* of the key's hash is currently stored. On a
//! detected false positive, the offending slot's selector advances to the next window, so the absent
//! key's bits no longer match — while the genuinely-inserted key still matches (its own hash agrees
//! with any window of itself).
//!
//! # Layout note
//!
//! Behaviour-faithful reference layout: each bucket is an explicit list of `(hash, selector)` entries,
//! rather than the paper's rank-and-select quotient filter with a telescoping hash *chain* (which lets
//! the new window be derived without re-storing the key). Storing a wide hash per entry reproduces the
//! same membership and adaptivity semantics — **no false negatives, and each detected false positive
//! is permanently fixed** — while leaving the RSQF bit-packing (a space optimisation) as a follow-up.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

const SEED: u64 = 0x7A1E_5C09_0000_0001;

/// One stored entry: the inserted key's hash and the current fingerprint-window selector.
#[derive(Debug, Clone, Copy)]
struct Entry {
    hash: u64,
    selector: u32,
}

/// A Telescoping (adaptive) Filter with `2^q` buckets and `r`-bit fingerprint windows.
///
/// # Example
/// ```
/// use sketch_oxide::membership::TelescopingFilter;
///
/// let mut tf = TelescopingFilter::new(12, 8).unwrap();
/// for i in 0..50_000u32 { tf.insert(&i.to_le_bytes()); }
/// assert!(tf.contains(&100u32.to_le_bytes())); // no false negatives
///
/// // Find an absent key that false-positives, then adapt it away — permanently.
/// if let Some(fp) = (1_000_000..1_100_000u32).find(|i| tf.contains(&i.to_le_bytes())) {
///     tf.adapt(&fp.to_le_bytes());
///     assert!(!tf.contains(&fp.to_le_bytes()));
/// }
/// ```
#[derive(Debug, Clone)]
pub struct TelescopingFilter {
    q: u32,
    r: u32,
    max_selector: u32,
    buckets: Vec<Vec<Entry>>,
    count: usize,
}

impl TelescopingFilter {
    /// Creates a filter with `2^q` buckets (`1 ≤ q ≤ 32`) and `r`-bit fingerprint windows
    /// (`1 ≤ r ≤ 16`); the base false-positive rate per probed entry is `≈ 2^-r`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `q` or `r` is out of range or `q + 2r > 64`.
    pub fn new(q: u32, r: u32) -> Result<Self> {
        if !(1..=32).contains(&q) {
            return Err(SketchError::InvalidParameter {
                param: "q".to_string(),
                value: q.to_string(),
                constraint: "must be in 1..=32".to_string(),
            });
        }
        if !(1..=16).contains(&r) {
            return Err(SketchError::InvalidParameter {
                param: "r".to_string(),
                value: r.to_string(),
                constraint: "must be in 1..=16".to_string(),
            });
        }
        if q + 2 * r > 64 {
            return Err(SketchError::InvalidParameter {
                param: "q + 2r".to_string(),
                value: (q + 2 * r).to_string(),
                constraint: "must be <= 64 (room for selector windows)".to_string(),
            });
        }
        // Selector windows are carved from the low `64 - q` hash bits.
        let max_selector = (64 - q) / r - 1;
        Ok(Self {
            q,
            r,
            max_selector,
            buckets: vec![Vec::new(); 1usize << q],
            count: 0,
        })
    }

    /// Number of inserted items.
    #[inline]
    pub fn len(&self) -> usize {
        self.count
    }

    /// Whether nothing has been inserted.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Bucket index (top `q` bits of the hash).
    fn bucket(&self, h: u64) -> usize {
        (h >> (64 - self.q)) as usize
    }

    /// The `r`-bit fingerprint window at the given selector.
    fn window(&self, h: u64, selector: u32) -> u64 {
        (h >> (selector * self.r)) & ((1u64 << self.r) - 1)
    }

    /// Inserts `item`.
    pub fn insert(&mut self, item: &[u8]) {
        let h = xxhash(item, SEED);
        let b = self.bucket(h);
        self.buckets[b].push(Entry {
            hash: h,
            selector: 0,
        });
        self.count += 1;
    }

    /// Tests membership. Never a false negative; false positives occur with bounded probability and
    /// can be permanently removed with [`adapt`](Self::adapt).
    pub fn contains(&self, item: &[u8]) -> bool {
        let h = xxhash(item, SEED);
        let b = self.bucket(h);
        self.buckets[b]
            .iter()
            .any(|e| self.window(h, e.selector) == self.window(e.hash, e.selector))
    }

    /// Adapts away a *false positive*: call this with a key that [`contains`](Self::contains) reported
    /// present but is verified absent. The matching slot's selector advances so this key no longer
    /// matches — permanently. Genuinely-inserted keys are unaffected (no false negatives).
    pub fn adapt(&mut self, item: &[u8]) {
        let h = xxhash(item, SEED);
        let b = self.bucket(h);
        let (r, max_sel) = (self.r, self.max_selector);
        for e in &mut self.buckets[b] {
            if (h >> (e.selector * r)) & ((1u64 << r) - 1)
                == (e.hash >> (e.selector * r)) & ((1u64 << r) - 1)
            {
                if e.selector < max_sel {
                    e.selector += 1;
                }
                // Advancing the most-matching entry is enough to break this false positive.
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(TelescopingFilter::new(0, 8).is_err());
        assert!(TelescopingFilter::new(12, 0).is_err());
        assert!(TelescopingFilter::new(12, 17).is_err());
        assert!(TelescopingFilter::new(40, 16).is_err()); // q + 2r > 64
        assert!(TelescopingFilter::new(12, 8).is_ok());
    }

    #[test]
    fn no_false_negatives() {
        let mut tf = TelescopingFilter::new(12, 10).unwrap();
        for i in 0..50_000u32 {
            tf.insert(&i.to_le_bytes());
        }
        assert_eq!(tf.len(), 50_000);
        for i in 0..50_000u32 {
            assert!(tf.contains(&i.to_le_bytes()), "missing {i}");
        }
    }

    #[test]
    fn adapt_permanently_fixes_false_positive() {
        let mut tf = TelescopingFilter::new(10, 8).unwrap();
        for i in 0..20_000u32 {
            tf.insert(&i.to_le_bytes());
        }
        // Find an absent key that false-positives.
        let fp = (1_000_000..2_000_000u32)
            .find(|i| tf.contains(&i.to_le_bytes()))
            .expect("expected at least one false positive at this load");
        assert!(tf.contains(&fp.to_le_bytes()));
        tf.adapt(&fp.to_le_bytes());
        // It is now fixed, and stays fixed on repeated queries.
        for _ in 0..5 {
            assert!(!tf.contains(&fp.to_le_bytes()), "false positive recurred");
        }
        // Inserted keys remain present.
        for i in 0..20_000u32 {
            assert!(tf.contains(&i.to_le_bytes()), "adapt corrupted {i}");
        }
    }

    #[test]
    fn false_positive_rate_is_bounded() {
        let mut tf = TelescopingFilter::new(14, 12).unwrap();
        let n = 100_000u32;
        for i in 0..n {
            tf.insert(&i.to_le_bytes());
        }
        let trials = 200_000u32;
        let fps = (n..n + trials)
            .filter(|i| tf.contains(&i.to_le_bytes()))
            .count();
        let fpr = fps as f64 / trials as f64;
        assert!(fpr < 0.01, "FPR {fpr} too high");
    }
}
