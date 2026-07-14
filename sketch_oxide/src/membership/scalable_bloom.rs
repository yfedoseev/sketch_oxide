//! Scalable Bloom Filter — a Bloom filter that grows to hold unbounded insertions.
//!
//! A plain Bloom filter must be sized for its capacity up front; exceed it and the
//! false-positive rate blows up. A Scalable Bloom Filter (Almeida, Baquero, Preguiça & Hutchison,
//! "Scalable Bloom Filters", IPL 2007) instead chains sub-filters: when the active one fills,
//! a new, larger sub-filter with a *tighter* target FPR is appended. The geometric tightening
//! (ratio `r`) keeps the **compounded** false-positive rate bounded by the configured target,
//! so the structure absorbs an unknown number of items while honouring its FPR guarantee.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

/// One sub-filter with a fill budget.
#[derive(Debug, Clone)]
struct Slice {
    bits: Vec<u64>,
    m: usize,
    k: u32,
    seed: u64,
    count: usize,
    capacity: usize,
}

impl Slice {
    fn new(capacity: usize, fp: f64, seed: u64) -> Self {
        let n = capacity.max(1);
        let ln2 = std::f64::consts::LN_2;
        let m = (-(n as f64) * fp.ln() / (ln2 * ln2)).ceil().max(64.0) as usize;
        let k = ((m as f64 / n as f64) * ln2).round().clamp(1.0, 30.0) as u32;
        Self {
            bits: vec![0u64; m.div_ceil(64)],
            m,
            k,
            seed,
            count: 0,
            capacity,
        }
    }

    fn positions(&self, key: &[u8]) -> impl Iterator<Item = usize> + '_ + use<'_> {
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
        self.count += 1;
    }

    fn contains(&self, key: &[u8]) -> bool {
        self.positions(key)
            .all(|p| self.bits[p / 64] & (1 << (p % 64)) != 0)
    }

    fn is_full(&self) -> bool {
        self.count >= self.capacity
    }
}

/// A Scalable Bloom Filter with a bounded compounded false-positive rate.
///
/// # Example
/// ```
/// use sketch_oxide::membership::ScalableBloomFilter;
///
/// // initial capacity 1000, target FPR 1%.
/// let mut f = ScalableBloomFilter::new(1000, 0.01).unwrap();
/// for i in 0..50_000u64 { f.insert(&i.to_le_bytes()); } // far past initial capacity
/// for i in 0..50_000u64 { assert!(f.contains(&i.to_le_bytes())); } // no false negatives
/// assert!(f.num_slices() > 1, "should have grown");
/// ```
#[derive(Debug, Clone)]
pub struct ScalableBloomFilter {
    slices: Vec<Slice>,
    target_fpr: f64,
    /// Tightening ratio `r` (each new slice's FPR is `r×` the previous).
    ratio: f64,
    /// Capacity growth factor per slice.
    growth: usize,
    next_seed: u64,
}

impl ScalableBloomFilter {
    /// Creates a scalable filter with `initial_capacity` and bounded compounded `target_fpr`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `initial_capacity` is 0 or `target_fpr` is not in
    /// `(0, 1)`.
    pub fn new(initial_capacity: usize, target_fpr: f64) -> Result<Self> {
        if initial_capacity == 0 {
            return Err(SketchError::InvalidParameter {
                param: "initial_capacity".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        if !(target_fpr > 0.0 && target_fpr < 1.0) {
            return Err(SketchError::InvalidParameter {
                param: "target_fpr".to_string(),
                value: target_fpr.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }
        let ratio = 0.5;
        // Slice 0's FPR so the geometric sum Σ fpr0·r^i = fpr0/(1-r) ≤ target.
        let slice0_fpr = target_fpr * (1.0 - ratio);
        Ok(Self {
            slices: vec![Slice::new(initial_capacity, slice0_fpr, 0)],
            target_fpr,
            ratio,
            growth: 2,
            next_seed: 1,
        })
    }

    /// Inserts `key`, growing with a new tighter sub-filter if the active one is full.
    pub fn insert(&mut self, key: &[u8]) {
        if self.contains(key) {
            return; // already present: avoid wasting fill budget
        }
        if self.slices.last().is_some_and(Slice::is_full) {
            self.grow();
        }
        self.slices.last_mut().unwrap().insert(key);
    }

    fn grow(&mut self) {
        let last = self.slices.last().unwrap();
        let new_capacity = last.capacity * self.growth;
        // Tighten: this slice's FPR is ratio^(slice index) of slice 0's.
        let slice_fpr =
            self.target_fpr * (1.0 - self.ratio) * self.ratio.powi(self.slices.len() as i32);
        self.slices
            .push(Slice::new(new_capacity, slice_fpr, self.next_seed));
        self.next_seed += 1;
    }

    /// Tests membership (any sub-filter). Never a false negative for an inserted key.
    pub fn contains(&self, key: &[u8]) -> bool {
        self.slices.iter().any(|s| s.contains(key))
    }

    /// Number of sub-filters (grows over time).
    #[inline]
    pub fn num_slices(&self) -> usize {
        self.slices.len()
    }

    /// Total number of distinct items inserted.
    pub fn len(&self) -> usize {
        self.slices.iter().map(|s| s.count).sum()
    }

    /// Whether nothing has been inserted.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Total size in bits across all sub-filters.
    pub fn size_bits(&self) -> usize {
        self.slices.iter().map(|s| s.m).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(ScalableBloomFilter::new(0, 0.01).is_err());
        assert!(ScalableBloomFilter::new(100, 0.0).is_err());
        assert!(ScalableBloomFilter::new(100, 1.0).is_err());
        assert!(ScalableBloomFilter::new(100, 0.01).is_ok());
    }

    #[test]
    fn grows_past_initial_capacity() {
        let mut f = ScalableBloomFilter::new(1000, 0.01).unwrap();
        for i in 0..20_000u64 {
            f.insert(&i.to_le_bytes());
        }
        assert!(f.num_slices() > 1, "should have added sub-filters");
    }

    #[test]
    fn no_false_negatives() {
        let mut f = ScalableBloomFilter::new(500, 0.01).unwrap();
        for i in 0..50_000u64 {
            f.insert(&i.to_le_bytes());
        }
        for i in 0..50_000u64 {
            assert!(f.contains(&i.to_le_bytes()), "false negative for {i}");
        }
    }

    #[test]
    fn false_positive_rate_bounded() {
        let mut f = ScalableBloomFilter::new(1000, 0.01).unwrap();
        for i in 0..20_000u64 {
            f.insert(&i.to_le_bytes());
        }
        let mut fp = 0;
        for i in 1_000_000..1_010_000u64 {
            if f.contains(&i.to_le_bytes()) {
                fp += 1;
            }
        }
        // Compounded FPR bounded by ~1%; allow slack.
        assert!(fp < 300, "false positives {fp}/10000 exceed bound");
    }

    #[test]
    fn duplicate_inserts_dont_grow() {
        let mut f = ScalableBloomFilter::new(100, 0.01).unwrap();
        for _ in 0..10_000 {
            f.insert(b"same");
        }
        assert_eq!(f.len(), 1);
        assert_eq!(f.num_slices(), 1);
    }
}

// ---------------------------------------------------------------------------
// Capability-trait adoption (fable5 doc 01 F3): express the inherent API via
// the orthogonal capability traits, delegating to the inherent methods.
// ---------------------------------------------------------------------------
use crate::common::capabilities::*;

impl Update<[u8]> for ScalableBloomFilter {
    fn update(&mut self, item: &[u8]) {
        self.insert(item);
    }
}

impl Filter<[u8]> for ScalableBloomFilter {
    fn contains(&self, item: &[u8]) -> bool {
        ScalableBloomFilter::contains(self, item)
    }
}
