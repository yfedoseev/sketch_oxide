//! APBF — Age-Partitioned Bloom Filter for sliding-window membership.
//!
//! A Bloom filter answers "have I seen this?" but never forgets, so it is useless for "have
//! I seen this *recently*?". APBF (Shtul, Baquero & Almeida, "Age-Partitioned Bloom Filters",
//! 2021) gives windowed membership with a false-positive guarantee over the last `n`
//! insertions: it keeps `k + l` Bloom slices, inserts each element into the front `k` slices,
//! and periodically **shifts** — prepending a fresh empty slice and dropping the oldest. An
//! element stays reportable until its block of `k` slices has aged past the `l` grace slices,
//! i.e. for one full window; then it falls out automatically. A query reports membership if
//! the element matches in `k` consecutive slices.
//!
//! Unlike a Stable/Counting Bloom filter, the window is a precise count of recent insertions,
//! and the FPR is bounded over exactly that window.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use std::collections::VecDeque;

/// One Bloom slice with its own hash seed (so a slice's identity is independent of its
/// shifting position).
#[derive(Debug, Clone)]
struct Slice {
    bits: Vec<u64>,
    seed: u64,
}

impl Slice {
    fn new(slice_bits: usize, seed: u64) -> Self {
        Self {
            bits: vec![0u64; slice_bits.div_ceil(64)],
            seed,
        }
    }

    fn bit_index(&self, item: &[u8], slice_bits: usize) -> usize {
        (xxhash(item, self.seed) % slice_bits as u64) as usize
    }

    fn set(&mut self, item: &[u8], slice_bits: usize) {
        let i = self.bit_index(item, slice_bits);
        self.bits[i / 64] |= 1 << (i % 64);
    }

    fn get(&self, item: &[u8], slice_bits: usize) -> bool {
        let i = self.bit_index(item, slice_bits);
        self.bits[i / 64] & (1 << (i % 64)) != 0
    }
}

/// Age-Partitioned Bloom Filter over a sliding window of insertions.
///
/// # Example
/// ```
/// use sketch_oxide::streaming::Apbf;
///
/// // Window ~1000 recent insertions, ~1% FPR.
/// let mut apbf = Apbf::new(1000, 0.01).unwrap();
/// apbf.insert(b"recent");
/// assert!(apbf.contains(b"recent"));
///
/// // Push far more than a window of other items: "recent" ages out.
/// for i in 0..5000u64 { apbf.insert(&i.to_le_bytes()); }
/// assert!(!apbf.contains(b"recent"));
/// ```
#[derive(Debug, Clone)]
pub struct Apbf {
    /// Slices, front (index 0) = newest.
    slices: VecDeque<Slice>,
    k: usize,
    slice_bits: usize,
    batch_size: u64,
    inserts_in_batch: u64,
    next_seed: u64,
}

impl Apbf {
    /// Creates an APBF for a window of about `window` recent insertions at false-positive
    /// rate `fpr`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `window` is 0 or `fpr` is not in `(0, 1)`.
    pub fn new(window: u64, fpr: f64) -> Result<Self> {
        if window == 0 {
            return Err(SketchError::InvalidParameter {
                param: "window".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        if !(fpr > 0.0 && fpr < 1.0) {
            return Err(SketchError::InvalidParameter {
                param: "fpr".to_string(),
                value: fpr.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }

        // k consecutive slices must all false-positive for a query FP => k ~ log2(1/fpr).
        let k = (-(fpr.log2())).ceil().max(1.0) as usize;
        let l = k; // grace slices; window = l batches.
        let total = k + l;
        let batch_size = (window / l as u64).max(1);
        // Elements a slice accumulates over its k-shift front life.
        let n_per_slice = k as u64 * batch_size;
        // Per-slice target FPR so that p^k ~ fpr.
        let p = fpr.powf(1.0 / k as f64);
        let slice_bits = ((n_per_slice as f64 / p).ceil() as usize).max(64);

        let slices: VecDeque<Slice> = (0..total as u64)
            .map(|s| Slice::new(slice_bits, s))
            .collect();

        Ok(Self {
            slices,
            k,
            slice_bits,
            batch_size,
            inserts_in_batch: 0,
            next_seed: total as u64,
        })
    }

    /// Number of slices (`k + l`).
    #[inline]
    pub fn num_slices(&self) -> usize {
        self.slices.len()
    }

    /// Slices an element is inserted into (`k`).
    #[inline]
    pub fn k(&self) -> usize {
        self.k
    }

    /// Inserts `item` into the front `k` slices, shifting the window when a batch fills.
    pub fn insert(&mut self, item: &[u8]) {
        let bits = self.slice_bits;
        for j in 0..self.k {
            self.slices[j].set(item, bits);
        }
        self.inserts_in_batch += 1;
        if self.inserts_in_batch >= self.batch_size {
            self.shift();
        }
    }

    /// Advances the window: drop the oldest slice, prepend a fresh empty one.
    fn shift(&mut self) {
        self.slices.pop_back();
        self.slices
            .push_front(Slice::new(self.slice_bits, self.next_seed));
        self.next_seed += 1;
        self.inserts_in_batch = 0;
    }

    /// Tests whether `item` was inserted within the current window. May return a false
    /// positive (bounded by `fpr`); never a false negative for an in-window element.
    pub fn contains(&self, item: &[u8]) -> bool {
        let n = self.slices.len();
        if n < self.k {
            return false;
        }
        // Look for k consecutive slices that all contain the item.
        let bits = self.slice_bits;
        'windows: for start in 0..=(n - self.k) {
            for j in start..start + self.k {
                if !self.slices[j].get(item, bits) {
                    continue 'windows;
                }
            }
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(Apbf::new(0, 0.01).is_err());
        assert!(Apbf::new(1000, 0.0).is_err());
        assert!(Apbf::new(1000, 1.0).is_err());
        assert!(Apbf::new(1000, 0.01).is_ok());
    }

    #[test]
    fn recent_items_are_found() {
        let mut apbf = Apbf::new(1000, 0.01).unwrap();
        for i in 0..100u64 {
            apbf.insert(&i.to_le_bytes());
        }
        for i in 0..100u64 {
            assert!(apbf.contains(&i.to_le_bytes()), "missing recent item {i}");
        }
    }

    #[test]
    fn old_items_age_out() {
        let mut apbf = Apbf::new(500, 0.01).unwrap();
        apbf.insert(b"old");
        assert!(apbf.contains(b"old"));
        // Insert well over a window of other items => "old" shifts out.
        for i in 0..3000u64 {
            apbf.insert(&i.to_le_bytes());
        }
        assert!(!apbf.contains(b"old"), "old item should have aged out");
    }

    #[test]
    fn no_false_negative_within_window() {
        let mut apbf = Apbf::new(2000, 0.01).unwrap();
        // Insert 1000 items (well within the 2000 window) => all must be found.
        let keys: Vec<u64> = (0..1000).collect();
        for &k in &keys {
            apbf.insert(&k.to_le_bytes());
        }
        for &k in &keys {
            assert!(apbf.contains(&k.to_le_bytes()), "false negative for {k}");
        }
    }

    #[test]
    fn false_positive_rate_is_bounded() {
        let mut apbf = Apbf::new(1000, 0.01).unwrap();
        for i in 0..1000u64 {
            apbf.insert(&i.to_le_bytes());
        }
        // Query 5000 never-inserted keys; false positives should be well under 5%.
        let mut fp = 0;
        for i in 1_000_000..1_005_000u64 {
            if apbf.contains(&i.to_le_bytes()) {
                fp += 1;
            }
        }
        assert!(fp < 250, "false positives {fp}/5000 too high");
    }

    #[test]
    fn slice_structure() {
        let apbf = Apbf::new(1000, 0.01).unwrap();
        // fpr 0.01 => k = ceil(log2(100)) = 7; total = 2k = 14.
        assert_eq!(apbf.k(), 7);
        assert_eq!(apbf.num_slices(), 14);
    }
}
