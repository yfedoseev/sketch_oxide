//! RHHH — Randomized Hierarchical Heavy Hitters with constant-time updates.
//!
//! A *hierarchical* heavy hitter is a heavy hitter at some level of a key hierarchy — for example
//! source-IP prefixes at `/8`, `/16`, `/24`, `/32`. The exact approach updates a heavy-hitter
//! structure at *every* level on each packet, costing `O(H)` per update for `H` levels. RHHH
//! (Ben-Basat, Einziger, Friedman & Kassner, "Constant Time Updates in Hierarchical Heavy
//! Hitters", SIGCOMM 2017) makes updates `O(1)` by sampling: each item updates the heavy-hitter
//! structure of a *single* randomly chosen level. Since a level is picked with probability `1/H`,
//! a level's raw count is scaled by `H` to recover an unbiased frequency estimate.
//!
//! Here the hierarchy is the byte-/bit-prefix lattice of a `u64` key: level `l` keeps the top
//! `(l+1)·bits_per_level` significant bits of the key (so the coarsest level is `0`, the finest is
//! `num_levels − 1`). Each level owns a Misra–Gries [`FrequentItems`](crate::frequency::FrequentItems)
//! counter.
//!
//! # Note
//!
//! This delivers the RHHH *counting* core (sampled level, per-level Misra–Gries, `H`-scaling) and
//! per-level heavy-hitter queries. The full HHH output — conditioning a prefix's count on the
//! counts already explained by its heavy-hitter descendants — is a documented follow-up on top of
//! these per-level estimates; it changes which prefixes are *reported*, not how they are counted.

use crate::common::{Result, SketchError};
use crate::frequency::FrequentItems;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

/// A Randomized Hierarchical Heavy Hitters sketch over the bit-prefix hierarchy of `u64` keys.
///
/// # Example
/// ```
/// use sketch_oxide::frequency::Rhhh;
///
/// // 4-level byte hierarchy over 32-bit keys (think IPv4 /8 /16 /24 /32).
/// let mut r = Rhhh::with_seed(4, 8, 256, 1).unwrap();
/// for _ in 0..100_000 { r.update(0x0A_0B_0C_0D); }   // one very heavy address
/// for i in 0..50_000u64 { r.update(i); }              // background noise
///
/// // Its /32 (finest) prefix is estimated near its true frequency.
/// let est = r.estimate(0x0A_0B_0C_0D, 3);
/// assert!(est > 70_000.0, "heavy-hitter estimate {est}");
/// ```
#[derive(Debug, Clone)]
pub struct Rhhh {
    num_levels: usize,
    bits_per_level: u32,
    levels: Vec<FrequentItems<u64>>,
    rng: SmallRng,
    total: u64,
}

impl Rhhh {
    /// Creates an RHHH sketch with `num_levels` hierarchy levels of `bits_per_level` bits each and
    /// a per-level Misra–Gries capacity of `capacity`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if any argument is 0 or `num_levels · bits_per_level > 64`.
    pub fn new(num_levels: usize, bits_per_level: u32, capacity: usize) -> Result<Self> {
        Self::with_seed(num_levels, bits_per_level, capacity, 0x_4848_4805_9E37_79B9)
    }

    /// Like [`new`](Self::new) with an explicit seed for reproducible level sampling.
    ///
    /// # Errors
    /// As [`new`](Self::new).
    pub fn with_seed(
        num_levels: usize,
        bits_per_level: u32,
        capacity: usize,
        seed: u64,
    ) -> Result<Self> {
        if num_levels == 0 || bits_per_level == 0 || capacity == 0 {
            return Err(SketchError::InvalidParameter {
                param: "num_levels/bits_per_level/capacity".to_string(),
                value: format!("{num_levels}/{bits_per_level}/{capacity}"),
                constraint: "all must be > 0".to_string(),
            });
        }
        if num_levels as u32 * bits_per_level > 64 {
            return Err(SketchError::InvalidParameter {
                param: "num_levels * bits_per_level".to_string(),
                value: (num_levels as u32 * bits_per_level).to_string(),
                constraint: "must be <= 64".to_string(),
            });
        }
        let levels = (0..num_levels)
            .map(|_| FrequentItems::new(capacity))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            num_levels,
            bits_per_level,
            levels,
            rng: SmallRng::seed_from_u64(seed),
            total: 0,
        })
    }

    /// The level-`level` generalization of `key`: its top `(level+1)·bits_per_level` significant
    /// bits, as an integer prefix.
    #[inline]
    pub fn prefix(&self, key: u64, level: usize) -> u64 {
        let total_bits = self.num_levels as u32 * self.bits_per_level;
        let kept = (level as u32 + 1) * self.bits_per_level;
        key >> (total_bits - kept)
    }

    /// Records one occurrence of `key`, updating exactly one randomly chosen level (O(1)).
    pub fn update(&mut self, key: u64) {
        self.total += 1;
        let level = self.rng.random_range(0..self.num_levels);
        let p = self.prefix(key, level);
        self.levels[level].update(p);
    }

    /// Unbiased frequency estimate of `key`'s prefix at `level` (the level count scaled by the
    /// number of levels). Returns 0 if the prefix is not currently tracked.
    pub fn estimate(&self, key: u64, level: usize) -> f64 {
        if level >= self.num_levels {
            return 0.0;
        }
        let p = self.prefix(key, level);
        let raw = self.levels[level]
            .get_estimate(&p)
            .map_or(0, |(count, _)| count);
        raw as f64 * self.num_levels as f64
    }

    /// Per-level heavy hitters whose scaled estimate is at least `threshold`, as
    /// `(level, prefix, scaled_count)`.
    pub fn heavy_hitters(&self, threshold: f64) -> Vec<(usize, u64, f64)> {
        let scale = self.num_levels as f64;
        let mut out = Vec::new();
        for (level, mg) in self.levels.iter().enumerate() {
            for (prefix, count, _) in
                mg.frequent_items(crate::frequency::ErrorType::NoFalseNegatives)
            {
                let scaled = count as f64 * scale;
                if scaled >= threshold {
                    out.push((level, prefix, scaled));
                }
            }
        }
        out.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
        out
    }

    /// Number of hierarchy levels.
    #[inline]
    pub fn num_levels(&self) -> usize {
        self.num_levels
    }

    /// Total number of updates processed.
    #[inline]
    pub fn total_updates(&self) -> u64 {
        self.total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(Rhhh::new(0, 8, 64).is_err());
        assert!(Rhhh::new(4, 0, 64).is_err());
        assert!(Rhhh::new(4, 8, 0).is_err());
        assert!(Rhhh::new(9, 8, 64).is_err()); // 72 > 64
        assert!(Rhhh::new(4, 8, 64).is_ok());
    }

    #[test]
    fn prefix_generalizes_correctly() {
        let r = Rhhh::new(4, 8, 64).unwrap();
        let key = 0x0A_0B_0C_0D; // 10.11.12.13
        assert_eq!(r.prefix(key, 0), 0x0A); // /8
        assert_eq!(r.prefix(key, 1), 0x0A0B); // /16
        assert_eq!(r.prefix(key, 2), 0x0A0B0C); // /24
        assert_eq!(r.prefix(key, 3), 0x0A0B0C0D); // /32
    }

    #[test]
    fn heavy_hitter_estimated_at_finest_level() {
        let mut r = Rhhh::with_seed(4, 8, 256, 7).unwrap();
        let heavy = 0x0A_0B_0C_0Du64;
        for _ in 0..100_000 {
            r.update(heavy);
        }
        for i in 0..50_000u64 {
            r.update(i); // background
        }
        let est = r.estimate(heavy, 3);
        // Scaled estimate should land within ~25% of the true 100k despite sampling.
        assert!((est - 100_000.0).abs() < 0.25 * 100_000.0, "estimate {est}");
    }

    #[test]
    fn coarse_prefixes_aggregate_descendants() {
        // Many addresses share the /8 prefix 0x0A; that prefix's count should be large.
        let mut r = Rhhh::with_seed(4, 8, 512, 3).unwrap();
        for i in 0..200_000u64 {
            let key = 0x0A00_0000u64 | (i & 0x00FF_FFFF); // all in 10.0.0.0/8
            r.update(key);
        }
        let est = r.estimate(0x0A00_0000, 0); // /8 prefix 0x0A
        assert!(est > 100_000.0, "/8 aggregate estimate {est}");
    }

    #[test]
    fn heavy_hitters_lists_the_elephant() {
        let mut r = Rhhh::with_seed(4, 8, 256, 11).unwrap();
        let heavy = 0x0102_0304u64;
        for _ in 0..80_000 {
            r.update(heavy);
        }
        for i in 0..40_000u64 {
            r.update(0xF000_0000 | i);
        }
        let hh = r.heavy_hitters(50_000.0);
        // The elephant's /32 must be among the reported heavy hitters.
        assert!(hh.iter().any(|&(lvl, p, _)| lvl == 3 && p == heavy));
    }

    #[test]
    fn deterministic_with_seed() {
        let run = || {
            let mut r = Rhhh::with_seed(4, 8, 128, 99).unwrap();
            for i in 0..20_000u64 {
                r.update(i % 1000);
            }
            r.estimate(7, 3)
        };
        assert_eq!(run(), run());
    }
}

// --- Capability-trait adoption (fable5 doc 01 F3) ---
use crate::common::capabilities::Update;

impl Update<u64> for Rhhh {
    fn update(&mut self, item: &u64) {
        Rhhh::update(self, *item);
    }
}

// PointQuery is intentionally NOT implemented: `estimate(key, level)` needs a
// hierarchy `level` argument and returns `f64`, matching neither PointQuery's
// single-item shape nor its `u64` return.
