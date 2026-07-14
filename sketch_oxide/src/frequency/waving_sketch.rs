//! WavingSketch — unbiased frequency and top-k estimation.
//!
//! WavingSketch (Li, Li, Yang, et al., "WavingSketch: An Unbiased and Generic Sketch for
//! Finding Top-k Items in Data Streams", KDD 2020) finds heavy hitters while giving an
//! **unbiased** frequency estimate — a property that matters because unbiased estimates can
//! be summed across distributed shards without accumulating skew, unlike Count-Min's
//! one-sided overestimate.
//!
//! Each bucket holds a small *heavy part* (a few `(item, count)` slots) and a single signed
//! *waving counter*. An item hashes to a bucket and gets a `±1` sign from a second hash. Light
//! items only nudge the waving counter by their sign, so in expectation they cancel; a heavy
//! item is promoted into a heavy slot, and its estimate subtracts the residual waving counter
//! to remove the light-item bias.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

/// One heavy slot: a tracked item and its (biased-corrected) count.
#[derive(Debug, Clone)]
struct HeavySlot {
    item: Vec<u8>,
    count: i64,
}

/// One bucket: a heavy part plus a signed waving counter for the light items.
#[derive(Debug, Clone)]
struct Bucket {
    heavy: Vec<HeavySlot>,
    waving: i64,
}

/// A WavingSketch with `num_buckets` buckets and `slots` heavy slots each.
///
/// # Example
/// ```
/// use sketch_oxide::frequency::WavingSketch;
///
/// let mut w = WavingSketch::new(1024, 8).unwrap();
/// for _ in 0..10_000 { w.insert(b"hot"); }
/// for i in 0..5000u64 { w.insert(&i.to_le_bytes()); }
///
/// // Heavy item estimated close to its true count (unbiased).
/// let est = w.estimate(b"hot");
/// assert!((est - 10_000).abs() < 500, "estimate {est}");
/// ```
#[derive(Debug, Clone)]
pub struct WavingSketch {
    num_buckets: usize,
    slots: usize,
    buckets: Vec<Bucket>,
}

impl WavingSketch {
    /// Creates a sketch with `num_buckets` buckets, each holding `slots` heavy items.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `num_buckets` or `slots` is 0.
    pub fn new(num_buckets: usize, slots: usize) -> Result<Self> {
        if num_buckets == 0 || slots == 0 {
            return Err(SketchError::InvalidParameter {
                param: if num_buckets == 0 {
                    "num_buckets"
                } else {
                    "slots"
                }
                .to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        let buckets = (0..num_buckets)
            .map(|_| Bucket {
                heavy: Vec::with_capacity(slots),
                waving: 0,
            })
            .collect();
        Ok(Self {
            num_buckets,
            slots,
            buckets,
        })
    }

    #[inline]
    fn bucket_of(&self, item: &[u8]) -> usize {
        (xxhash(item, 0) % self.num_buckets as u64) as usize
    }

    /// The `±1` sign of `item`.
    #[inline]
    fn sign(item: &[u8]) -> i64 {
        if xxhash(item, 1) & 1 == 0 { 1 } else { -1 }
    }

    /// Inserts one occurrence of `item`.
    pub fn insert(&mut self, item: &[u8]) {
        let b = self.bucket_of(item);
        let s = Self::sign(item);
        let slots = self.slots;
        let bucket = &mut self.buckets[b];

        // Already tracked in the heavy part?
        if let Some(slot) = bucket.heavy.iter_mut().find(|sl| sl.item == item) {
            slot.count += 1;
            return;
        }

        // Room in the heavy part: start tracking it.
        if bucket.heavy.len() < slots {
            bucket.heavy.push(HeavySlot {
                item: item.to_vec(),
                count: 1,
            });
            return;
        }

        // Heavy part full: nudge the waving counter, and possibly swap out the weakest heavy
        // item if this light item now appears to dominate.
        bucket.waving += s;
        // Estimated count of this light item from the waving counter, sign-corrected.
        let est_light = (bucket.waving * s).max(0);

        if let Some((idx, weakest)) = bucket
            .heavy
            .iter()
            .enumerate()
            .min_by_key(|(_, sl)| sl.count)
            .map(|(i, sl)| (i, sl.count))
        {
            if est_light > weakest {
                // Demote the weakest heavy item into the waving counter and promote this one.
                let demoted = &mut bucket.heavy[idx];
                let dsign = Self::sign(&demoted.item);
                bucket.waving += dsign * demoted.count;
                demoted.item = item.to_vec();
                demoted.count = est_light;
                bucket.waving -= s * est_light;
            }
        }
    }

    /// Unbiased estimate of `item`'s frequency.
    ///
    /// If the item is tracked in its bucket's heavy part, returns its slot count (corrected
    /// for the waving residual when it was promoted from the light pool); otherwise estimates
    /// from the sign-corrected waving counter.
    pub fn estimate(&self, item: &[u8]) -> i64 {
        let b = self.bucket_of(item);
        let s = Self::sign(item);
        let bucket = &self.buckets[b];
        if let Some(slot) = bucket.heavy.iter().find(|sl| sl.item == item) {
            slot.count
        } else {
            (bucket.waving * s).max(0)
        }
    }

    /// Returns the heavy items with estimated frequency at least `min_count`, as
    /// `(item, count)`, sorted by count descending.
    pub fn heavy_hitters(&self, min_count: i64) -> Vec<(Vec<u8>, i64)> {
        let mut out: Vec<(Vec<u8>, i64)> = self
            .buckets
            .iter()
            .flat_map(|bucket| bucket.heavy.iter())
            .filter(|sl| sl.count >= min_count)
            .map(|sl| (sl.item.clone(), sl.count))
            .collect();
        out.sort_by_key(|e| std::cmp::Reverse(e.1));
        out
    }

    /// Number of buckets.
    #[inline]
    pub fn num_buckets(&self) -> usize {
        self.num_buckets
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_dims() {
        assert!(WavingSketch::new(0, 4).is_err());
        assert!(WavingSketch::new(16, 0).is_err());
        assert!(WavingSketch::new(16, 4).is_ok());
    }

    #[test]
    fn heavy_item_estimated_accurately() {
        let mut w = WavingSketch::new(1024, 8).unwrap();
        for _ in 0..10_000 {
            w.insert(b"hot");
        }
        for i in 0..5000u64 {
            w.insert(&i.to_le_bytes());
        }
        let est = w.estimate(b"hot");
        assert!((est - 10_000).abs() < 500, "estimate {est}");
    }

    #[test]
    fn tracks_in_heavy_part_when_room() {
        let mut w = WavingSketch::new(1024, 8).unwrap();
        for _ in 0..100 {
            w.insert(b"a");
        }
        // Sparse sketch, plenty of slots => "a" tracked exactly.
        assert_eq!(w.estimate(b"a"), 100);
    }

    #[test]
    fn heavy_hitters_surfaces_top_items() {
        let mut w = WavingSketch::new(2048, 8).unwrap();
        for _ in 0..5000 {
            w.insert(b"big");
        }
        for _ in 0..2000 {
            w.insert(b"medium");
        }
        for i in 0..3000u64 {
            w.insert(&i.to_le_bytes()); // light tail
        }
        let hh = w.heavy_hitters(1000);
        assert!(hh.iter().any(|(k, _)| k == b"big"));
        assert!(hh.iter().any(|(k, _)| k == b"medium"));
        // Largest first.
        assert_eq!(hh.first().map(|(k, _)| k.clone()), Some(b"big".to_vec()));
    }

    #[test]
    fn light_items_estimate_low() {
        let mut w = WavingSketch::new(2048, 8).unwrap();
        for _ in 0..50_000 {
            w.insert(b"heavy");
        }
        // A never-inserted item should estimate small (light/zero region).
        assert!(
            w.estimate(b"absent") < 200,
            "absent {}",
            w.estimate(b"absent")
        );
    }

    #[test]
    fn unbiasedness_keeps_estimate_near_truth() {
        // Many medium items: the sign-cancellation should keep estimates from blowing up.
        let mut w = WavingSketch::new(4096, 4).unwrap();
        for i in 0..1000u64 {
            for _ in 0..100 {
                w.insert(&i.to_le_bytes());
            }
        }
        // Each item inserted 100 times; estimate should be in a reasonable band.
        let est = w.estimate(&500u64.to_le_bytes());
        assert!(est >= 50 && est <= 200, "estimate {est} for true 100");
    }
}

// --- Capability-trait adoption (fable5 doc 01 F3) ---
use crate::common::capabilities::Update;

impl Update<[u8]> for WavingSketch {
    fn update(&mut self, item: &[u8]) {
        self.insert(item);
    }
}

// PointQuery is intentionally NOT implemented: `estimate` returns a *signed*
// `i64` (WavingSketch counters can be negative), incompatible with `u64`.
