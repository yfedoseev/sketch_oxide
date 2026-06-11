//! Cuckoo Heavy Keeper — high-precision top-k via cuckoo placement + HeavyKeeper decay.
//!
//! Cuckoo Heavy Keeper fuses two ideas to find heavy hitters (top-k flows) in tight memory:
//!
//! - **Cuckoo placement** (as in a cuckoo filter): each item has a fingerprint and *two* candidate
//!   buckets, `i1 = h(item)` and `i2 = i1 ⊕ h(fingerprint)`. A bucket holds a few `(fingerprint,
//!   count)` slots, so each flow gets its *own* exact counter — there is none of the
//!   Count-Min-style cross-flow count merging that [`HeavyKeeper`](crate::frequency::HeavyKeeper)
//!   inherits from its sketch array.
//! - **HeavyKeeper exponential decay**: when both candidate buckets are full and a new item wants
//!   in, it doesn't blindly overwrite. The *weakest* resident counter (smallest count `C`) is
//!   decremented only with probability `decay^(−C)`, and the newcomer takes the slot only if that
//!   knocks the resident to zero. Large counts are almost never decremented, so heavy hitters are
//!   protected while the long tail of mouse flows churns through the contested slots.
//!
//! The result keeps far more accurate per-flow counts than a plain sketch at the same size, which
//! is what makes its top-k precise.
//!
//! # Design note
//!
//! Buckets are an explicit array of `(fingerprint, count)` slots with partial-key cuckoo
//! addressing and on-contention decay — the clear, verifiable form of the algorithm. Cuckoo
//! *relocation* (kicking a resident fingerprint to its alternate bucket to make room before
//! resorting to decay, which raises load capacity) is a documented follow-up; it changes where a
//! fingerprint lives, not the counts a query returns.

use crate::common::SketchError;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;

/// One counter slot: a nonzero fingerprint and its count (`fingerprint == 0` means empty).
#[derive(Debug, Clone, Copy, Default)]
struct Slot {
    fingerprint: u16,
    count: u32,
}

/// A Cuckoo Heavy Keeper top-k heavy-hitter detector.
///
/// # Example
/// ```
/// use sketch_oxide::frequency::CuckooHeavyKeeper;
///
/// let mut chk = CuckooHeavyKeeper::new(8, 1024, 4).unwrap();
/// for _ in 0..10_000 { chk.update(b"elephant"); }
/// for i in 0..2_000u64 { chk.update(&i.to_le_bytes()); } // many mouse flows
///
/// // The elephant is found and counted close to its true frequency.
/// let est = chk.estimate(b"elephant");
/// assert!(est > 9_000, "elephant estimate {est}");
/// let top = chk.top_k();
/// assert_eq!(top.first().map(|(_, c)| *c), Some(est));
/// ```
#[derive(Debug, Clone)]
pub struct CuckooHeavyKeeper {
    k: usize,
    /// Number of buckets (a power of two, so `i1 ⊕ h(fp)` stays in range).
    num_buckets: usize,
    slots_per_bucket: usize,
    buckets: Vec<Slot>,
    decay_factor: f64,
    /// Tracked top-k items: `item_hash → best observed count`.
    top: HashMap<u64, u32>,
    total_updates: u64,
    rng: SmallRng,
}

impl CuckooHeavyKeeper {
    /// Creates a detector tracking the top `k` items in a table of `num_buckets × slots_per_bucket`
    /// counters. `num_buckets` is rounded up to a power of two. Uses the standard HeavyKeeper decay
    /// factor of 1.08; see [`with_decay`](CuckooHeavyKeeper::with_decay) to tune it.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `k`, `num_buckets`, or `slots_per_bucket` is 0.
    pub fn new(k: usize, num_buckets: usize, slots_per_bucket: usize) -> Result<Self, SketchError> {
        Self::with_decay(k, num_buckets, slots_per_bucket, 1.08)
    }

    /// Like [`new`](CuckooHeavyKeeper::new) but with an explicit decay factor (`> 1.0`; larger
    /// decays the tail faster and protects heavy hitters more strongly).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if any size is 0 or `decay_factor <= 1.0`.
    pub fn with_decay(
        k: usize,
        num_buckets: usize,
        slots_per_bucket: usize,
        decay_factor: f64,
    ) -> Result<Self, SketchError> {
        if k == 0 || num_buckets == 0 || slots_per_bucket == 0 {
            return Err(SketchError::InvalidParameter {
                param: "k/num_buckets/slots_per_bucket".to_string(),
                value: format!("{k}/{num_buckets}/{slots_per_bucket}"),
                constraint: "all must be > 0".to_string(),
            });
        }
        if !decay_factor.is_finite() || decay_factor <= 1.0 {
            return Err(SketchError::InvalidParameter {
                param: "decay_factor".to_string(),
                value: decay_factor.to_string(),
                constraint: "must be > 1.0".to_string(),
            });
        }
        let num_buckets = num_buckets.next_power_of_two();
        Ok(Self {
            k,
            num_buckets,
            slots_per_bucket,
            buckets: vec![Slot::default(); num_buckets * slots_per_bucket],
            decay_factor,
            top: HashMap::new(),
            total_updates: 0,
            // Fixed seed: decay draws are reproducible run-to-run (deterministic tests).
            rng: SmallRng::seed_from_u64(0x9E37_79B9_7F4A_7C15),
        })
    }

    #[inline]
    fn item_hash(item: &[u8]) -> u64 {
        crate::common::hash::xxhash(item, 0)
    }

    /// Fingerprint (nonzero 16-bit) and the two candidate bucket indices for a hash.
    #[inline]
    fn locate(&self, hash: u64) -> (u16, usize, usize) {
        let fp = (((hash >> 32) as u16) | 1).max(1); // ensure nonzero
        let mask = self.num_buckets - 1;
        let i1 = (hash as usize) & mask;
        // Partial-key cuckoo: the alternate bucket is an involution of i1 via the fingerprint.
        let fp_hash = crate::common::hash::xxhash(&fp.to_le_bytes(), 1) as usize;
        let i2 = i1 ^ (fp_hash & mask);
        (fp, i1, i2)
    }

    #[inline]
    fn bucket_slots(&self, bucket: usize) -> std::ops::Range<usize> {
        let start = bucket * self.slots_per_bucket;
        start..start + self.slots_per_bucket
    }

    /// Records one occurrence of `item`.
    pub fn update(&mut self, item: &[u8]) {
        self.total_updates += 1;
        let hash = Self::item_hash(item);
        let (fp, i1, i2) = self.locate(hash);

        // 1. If the fingerprint already lives in a candidate bucket, bump its exact counter.
        for bucket in [i1, i2] {
            for idx in self.bucket_slots(bucket) {
                if self.buckets[idx].fingerprint == fp {
                    self.buckets[idx].count = self.buckets[idx].count.saturating_add(1);
                    let c = self.buckets[idx].count;
                    self.note_top(hash, c);
                    return;
                }
            }
        }

        // 2. Otherwise claim an empty slot if one exists.
        for bucket in [i1, i2] {
            for idx in self.bucket_slots(bucket) {
                if self.buckets[idx].fingerprint == 0 {
                    self.buckets[idx] = Slot {
                        fingerprint: fp,
                        count: 1,
                    };
                    self.note_top(hash, 1);
                    return;
                }
            }
        }

        // 3. Both candidate buckets are full: HeavyKeeper decay against the weakest resident.
        let mut min_idx = self.bucket_slots(i1).start;
        let mut min_count = u32::MAX;
        for bucket in [i1, i2] {
            for idx in self.bucket_slots(bucket) {
                if self.buckets[idx].count < min_count {
                    min_count = self.buckets[idx].count;
                    min_idx = idx;
                }
            }
        }
        let decay_prob = self.decay_factor.powi(-(min_count as i32));
        if self.rng.random::<f64>() < decay_prob {
            self.buckets[min_idx].count -= 1;
            if self.buckets[min_idx].count == 0 {
                // The weakest resident is knocked out; the newcomer takes the slot.
                self.buckets[min_idx] = Slot {
                    fingerprint: fp,
                    count: 1,
                };
                self.note_top(hash, 1);
            }
        }
        // If the coin did not land, the newcomer is dropped — it was not heavy enough to displace
        // an incumbent. That protective behaviour is exactly HeavyKeeper's accuracy mechanism.
    }

    /// Estimated count of `item` (0 if not currently held). May overestimate only on a fingerprint
    /// collision within a candidate bucket.
    pub fn estimate(&self, item: &[u8]) -> u32 {
        let (fp, i1, i2) = self.locate(Self::item_hash(item));
        let mut best = 0;
        for bucket in [i1, i2] {
            for idx in self.bucket_slots(bucket) {
                if self.buckets[idx].fingerprint == fp {
                    best = best.max(self.buckets[idx].count);
                }
            }
        }
        best
    }

    /// The current top-k heavy hitters as `(item_hash, count)`, sorted by count descending.
    pub fn top_k(&self) -> Vec<(u64, u32)> {
        let mut entries: Vec<(u64, u32)> = self.top.iter().map(|(&h, &c)| (h, c)).collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        entries
    }

    /// Total number of updates processed.
    #[inline]
    pub fn total_updates(&self) -> u64 {
        self.total_updates
    }

    /// Number of buckets (rounded up to a power of two).
    #[inline]
    pub fn num_buckets(&self) -> usize {
        self.num_buckets
    }

    /// Records `item_hash`'s current count in the top-k set, keeping at most `k` strongest.
    fn note_top(&mut self, item_hash: u64, count: u32) {
        if let Some(c) = self.top.get_mut(&item_hash) {
            *c = (*c).max(count);
            return;
        }
        if self.top.len() < self.k {
            self.top.insert(item_hash, count);
            return;
        }
        // Replace the current weakest if this item is stronger.
        if let Some((&min_h, &min_c)) = self.top.iter().min_by_key(|(_, &c)| c) {
            if count > min_c {
                self.top.remove(&min_h);
                self.top.insert(item_hash, count);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(CuckooHeavyKeeper::new(0, 64, 4).is_err());
        assert!(CuckooHeavyKeeper::new(8, 0, 4).is_err());
        assert!(CuckooHeavyKeeper::new(8, 64, 0).is_err());
        assert!(CuckooHeavyKeeper::with_decay(8, 64, 4, 1.0).is_err());
        assert!(CuckooHeavyKeeper::new(8, 64, 4).is_ok());
    }

    #[test]
    fn buckets_rounded_to_power_of_two() {
        let chk = CuckooHeavyKeeper::new(8, 1000, 4).unwrap();
        assert_eq!(chk.num_buckets(), 1024);
    }

    #[test]
    fn counts_a_lone_heavy_hitter_exactly() {
        let mut chk = CuckooHeavyKeeper::new(8, 1024, 4).unwrap();
        for _ in 0..5000 {
            chk.update(b"whale");
        }
        // No contention for a single flow → exact count.
        assert_eq!(chk.estimate(b"whale"), 5000);
    }

    #[test]
    fn heavy_hitter_survives_a_sea_of_mice() {
        let mut chk = CuckooHeavyKeeper::new(8, 256, 4).unwrap();
        // One elephant...
        for _ in 0..20_000 {
            chk.update(b"elephant");
        }
        // ...amid tens of thousands of distinct one-shot mouse flows.
        for i in 0..40_000u64 {
            chk.update(&i.to_le_bytes());
        }
        let est = chk.estimate(b"elephant");
        // HeavyKeeper decay protects the elephant: estimate stays close to the truth.
        assert!(est > 18_000, "elephant estimate {est} eroded too far");
        // And it is the #1 reported heavy hitter.
        let top = chk.top_k();
        assert_eq!(top.first().map(|(_, c)| *c), Some(est));
    }

    #[test]
    fn top_k_capacity_respected() {
        let mut chk = CuckooHeavyKeeper::new(5, 1024, 4).unwrap();
        for f in 0..50u64 {
            for _ in 0..(f + 1) * 10 {
                chk.update(&f.to_le_bytes());
            }
        }
        let top = chk.top_k();
        assert!(top.len() <= 5);
        // Sorted by count descending.
        for w in top.windows(2) {
            assert!(w[0].1 >= w[1].1);
        }
    }

    #[test]
    fn absent_item_estimates_zero_or_low() {
        let mut chk = CuckooHeavyKeeper::new(8, 1024, 4).unwrap();
        for _ in 0..1000 {
            chk.update(b"present");
        }
        // An item never inserted should read 0 unless it collides on a fingerprint+bucket.
        assert_eq!(chk.estimate(b"definitely-never-seen-xyz"), 0);
    }

    #[test]
    fn deterministic_across_instances() {
        // The seeded RNG makes runs reproducible: two identical streams give identical estimates.
        let build = || {
            let mut chk = CuckooHeavyKeeper::new(8, 128, 4).unwrap();
            for i in 0..5000u64 {
                chk.update(&(i % 50).to_le_bytes());
            }
            chk
        };
        let a = build();
        let b = build();
        for f in 0..50u64 {
            assert_eq!(a.estimate(&f.to_le_bytes()), b.estimate(&f.to_le_bytes()));
        }
    }
}
