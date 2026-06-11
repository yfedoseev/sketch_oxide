//! PeriodicSketch — finding top-K *periodic items* in data streams (Fan, Zhang, Yang et al., ICDE
//! 2022).
//!
//! An item is **periodic** if it recurs at a (roughly) fixed time interval — a beaconing host, a
//! money-laundering transaction pattern, a user's habitual clicks. PeriodicSketch is the first
//! one-pass `O(1)`-per-item structure for the top-K periodic items, and it pairs two sketches:
//!
//! * **Cover-Min** records, per item, the time interval since its previous arrival. It is a Count-Min
//!   relative: `d` rows of timestamp buckets; an arrival `(e, t)` reads the stored timestamps, reports
//!   `V = t − min(those timestamps)` (the *Min*, robust to collisions), then **covers** (overwrites)
//!   all `d` buckets with `t`. The reported `V` is the candidate period.
//! * **GSU** (Guaranteed Soft Uniform) keeps the top-K candidate elements `E = (item, interval)` keyed
//!   by frequency (the number of times that interval recurred). On a full bucket the least-frequent
//!   cell `L` (frequency `f_min`) is replaced by the incoming `E` only with probability
//!   `(t_fail + 1)/(2·f_min)`, where `t_fail` counts consecutive failed replacements in that bucket. On
//!   success the new cell's frequency is set to `f_min + ⌊t_fail/f_min⌋` and `t_fail` resets; on failure
//!   `t_fail` grows. This *soft-uniform* rule lets genuinely recurring elements accumulate while
//!   suppressing the over-counting that a plain min-replacement (Space-Saving) suffers on cold items.
//!
//! Intervals within `± delta_t` of each other are treated as the same period (binned), matching the
//! paper's tolerance parameter `ΔT`.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

const CM_SEED: u64 = 0x9213_0D1C_0000_0001;
const GSU_SEED: u64 = 0x6502_417E_0000_0001;

/// One GSU cell: a candidate periodic element and how often its interval has recurred.
#[derive(Debug, Clone, Copy)]
struct GsuCell {
    item: u64,
    bin: u64, // quantised interval
    freq: u64,
}

/// One GSU bucket: a few cells plus the consecutive-failure counter for its min cell.
#[derive(Debug, Clone)]
struct GsuBucket {
    cells: Vec<GsuCell>,
    t_fail: u64,
}

/// A PeriodicSketch finding top-K periodic items over `u64` item keys and `u64` timestamps.
///
/// # Example
/// ```
/// use sketch_oxide::streaming::PeriodicSketch;
///
/// let mut ps = PeriodicSketch::new(2048, 4, 1024, 4, 0, 7).unwrap();
/// // Item 1 arrives every 2 ticks (periodic); item 2 at irregular times (not periodic).
/// let mut t = 0u64;
/// for _ in 0..40 {
///     ps.insert(1, t);
///     ps.insert(2, t + (t % 7)); // jittered
///     t += 2;
/// }
/// // The most frequent periodic element is item 1 with interval 2.
/// let top = ps.top_k(1);
/// assert_eq!(top[0].0, 1);   // item
/// assert_eq!(top[0].1, 2);   // interval
/// ```
#[derive(Debug, Clone)]
pub struct PeriodicSketch {
    cm_width: usize,
    cm_depth: usize,
    cm: Vec<u64>, // timestamps, cm_depth × cm_width (0 = never seen)
    gsu_buckets: usize,
    cells_per_bucket: usize,
    gsu: Vec<GsuBucket>,
    granularity: u64, // 2·delta_t + 1
    delta_t: u64,
    rng: SmallRng,
}

impl PeriodicSketch {
    /// Creates a PeriodicSketch.
    ///
    /// * `cm_width`/`cm_depth` size the Cover-Min interval recorder.
    /// * `gsu_buckets`/`cells_per_bucket` size the GSU top-K candidate store.
    /// * `delta_t` is the interval tolerance `ΔT`: intervals within `±delta_t` share a period.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if any size is `0` or `cm_depth` exceeds `16`.
    pub fn new(
        cm_width: usize,
        cm_depth: usize,
        gsu_buckets: usize,
        cells_per_bucket: usize,
        delta_t: u64,
        seed: u64,
    ) -> Result<Self> {
        let check = |ok: bool, param: &str, val: String, c: &str| {
            if ok {
                Ok(())
            } else {
                Err(SketchError::InvalidParameter {
                    param: param.to_string(),
                    value: val,
                    constraint: c.to_string(),
                })
            }
        };
        check(
            cm_width > 0,
            "cm_width",
            cm_width.to_string(),
            "must be >= 1",
        )?;
        check(
            (1..=16).contains(&cm_depth),
            "cm_depth",
            cm_depth.to_string(),
            "must be in 1..=16",
        )?;
        check(
            gsu_buckets > 0,
            "gsu_buckets",
            gsu_buckets.to_string(),
            "must be >= 1",
        )?;
        check(
            cells_per_bucket > 0,
            "cells_per_bucket",
            cells_per_bucket.to_string(),
            "must be >= 1",
        )?;
        Ok(Self {
            cm_width,
            cm_depth,
            cm: vec![0; cm_width * cm_depth],
            gsu_buckets,
            cells_per_bucket,
            gsu: vec![
                GsuBucket {
                    cells: Vec::new(),
                    t_fail: 0,
                };
                gsu_buckets
            ],
            granularity: 2 * delta_t + 1,
            delta_t,
            rng: SmallRng::seed_from_u64(seed),
        })
    }

    /// Cover-Min column for `item` in row `r`.
    fn cm_col(&self, item: u64, r: usize) -> usize {
        let h = xxhash(
            &item.to_le_bytes(),
            CM_SEED ^ (r as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15),
        );
        (h % self.cm_width as u64) as usize
    }

    /// Records arrival of `item` at `timestamp`. Timestamps should be non-decreasing.
    pub fn insert(&mut self, item: u64, timestamp: u64) {
        // Cover-Min: read the previous timestamps, then overwrite all rows with `timestamp`.
        let mut min_ts = u64::MAX;
        for r in 0..self.cm_depth {
            let idx = r * self.cm_width + self.cm_col(item, r);
            let prev = self.cm[idx];
            if prev != 0 && prev < min_ts {
                min_ts = prev;
            }
            self.cm[idx] = timestamp;
        }
        // No prior arrival, or a non-positive interval: no period to record.
        if min_ts == u64::MAX || timestamp <= min_ts {
            return;
        }
        let interval = timestamp - min_ts;
        let bin = interval / self.granularity;
        self.gsu_insert(item, bin);
    }

    /// Inserts the element `(item, bin)` into the GSU sketch with the soft-uniform replacement rule.
    fn gsu_insert(&mut self, item: u64, bin: u64) {
        let j = (gsu_hash(item, bin) % self.gsu_buckets as u64) as usize;
        let cap = self.cells_per_bucket;
        let bucket = &mut self.gsu[j];

        // Case 2: element already present → increment its frequency.
        if let Some(cell) = bucket
            .cells
            .iter_mut()
            .find(|c| c.item == item && c.bin == bin)
        {
            cell.freq += 1;
            return;
        }
        // Case 1a: room available → insert with frequency 1.
        if bucket.cells.len() < cap {
            bucket.cells.push(GsuCell { item, bin, freq: 1 });
            return;
        }
        // Case 1b: full → Guaranteed Soft Uniform replacement of the least-frequent cell.
        let (min_idx, f_min) = bucket
            .cells
            .iter()
            .enumerate()
            .map(|(i, c)| (i, c.freq))
            .min_by_key(|&(_, f)| f)
            .unwrap();
        let p_replace = (bucket.t_fail + 1) as f64 / (2.0 * f_min as f64);
        if self.rng.random::<f64>() < p_replace {
            let new_freq = f_min + bucket.t_fail / f_min;
            bucket.cells[min_idx] = GsuCell {
                item,
                bin,
                freq: new_freq,
            };
            bucket.t_fail = 0;
        } else {
            bucket.t_fail += 1;
        }
    }

    /// Returns up to `k` periodic items as `(item, interval, frequency)`, most frequent first. The
    /// interval is the representative period of its tolerance bin.
    pub fn top_k(&self, k: usize) -> Vec<(u64, u64, u64)> {
        let mut all: Vec<(u64, u64, u64)> = self
            .gsu
            .iter()
            .flat_map(|b| b.cells.iter())
            .map(|c| (c.item, c.bin * self.granularity + self.delta_t, c.freq))
            .collect();
        all.sort_unstable_by_key(|c| std::cmp::Reverse(c.2));
        all.truncate(k);
        all
    }

    /// The interval tolerance `ΔT`.
    #[inline]
    pub fn delta_t(&self) -> u64 {
        self.delta_t
    }
}

/// Hash of a GSU element `(item, bin)` to a bucket-distributing value.
fn gsu_hash(item: u64, bin: u64) -> u64 {
    let h = xxhash(&item.to_le_bytes(), GSU_SEED);
    h ^ bin.wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(PeriodicSketch::new(0, 4, 64, 4, 0, 1).is_err());
        assert!(PeriodicSketch::new(64, 0, 64, 4, 0, 1).is_err());
        assert!(PeriodicSketch::new(64, 17, 64, 4, 0, 1).is_err());
        assert!(PeriodicSketch::new(64, 4, 0, 4, 0, 1).is_err());
        assert!(PeriodicSketch::new(64, 4, 64, 0, 0, 1).is_err());
        assert!(PeriodicSketch::new(64, 4, 64, 4, 0, 1).is_ok());
    }

    #[test]
    fn detects_a_periodic_item() {
        let mut ps = PeriodicSketch::new(4096, 4, 2048, 4, 0, 7).unwrap();
        // Item 1: strictly periodic with interval 2.
        let mut t = 0u64;
        for _ in 0..50 {
            ps.insert(1, t);
            t += 2;
        }
        let top = ps.top_k(1);
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].0, 1, "item");
        assert_eq!(top[0].1, 2, "interval");
        assert!(top[0].2 >= 40, "frequency {} too low", top[0].2);
    }

    #[test]
    fn distinguishes_two_periods() {
        let mut ps = PeriodicSketch::new(8192, 4, 4096, 8, 0, 11).unwrap();
        // Item 10 has period 2, item 20 has period 3 — interleave them on a shared clock.
        for t in 0..300u64 {
            if t % 2 == 0 {
                ps.insert(10, t);
            }
            if t % 3 == 0 {
                ps.insert(20, t);
            }
        }
        let top = ps.top_k(10);
        // Both periodic elements should be present with their correct intervals.
        assert!(
            top.iter().any(|&(i, v, f)| i == 10 && v == 2 && f >= 100),
            "missing (10, period 2): {top:?}"
        );
        assert!(
            top.iter().any(|&(i, v, f)| i == 20 && v == 3 && f >= 60),
            "missing (20, period 3): {top:?}"
        );
    }

    #[test]
    fn first_occurrence_records_no_interval() {
        let mut ps = PeriodicSketch::new(256, 4, 64, 4, 0, 3).unwrap();
        ps.insert(42, 100); // only one arrival ⇒ no interval ⇒ nothing in GSU
        assert!(ps.top_k(5).is_empty());
    }

    #[test]
    fn tolerance_bins_nearby_intervals() {
        // With delta_t = 1, intervals 4 and 5 fall in the same period bin and accumulate together.
        let mut ps = PeriodicSketch::new(4096, 4, 1024, 4, 1, 5).unwrap();
        let mut t = 0u64;
        for i in 0..40u64 {
            ps.insert(1, t);
            t += if i % 2 == 0 { 4 } else { 5 }; // alternating 4 / 5 ≈ same period within ±1
        }
        let top = ps.top_k(1);
        assert_eq!(top[0].0, 1);
        // Both intervals share one bin, so the frequency reflects nearly all arrivals.
        assert!(top[0].2 >= 30, "binned frequency {} too low", top[0].2);
    }
}
