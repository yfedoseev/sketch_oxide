//! Count-Min-Log sketch — Count-Min with *approximate logarithmic* counters (Pitel & Fouquier, 2015).
//!
//! A [Count-Min sketch](crate::frequency::CountMinSketch) spends a full machine word per counter, yet
//! its accuracy is dominated by hash collisions, not counter width. **Count-Min-Log** replaces each
//! linear counter with a small **Morris-style approximate counter**: a counter holding value `c`
//! represents an estimated count `value(c)` that grows geometrically, and an increment is *applied
//! probabilistically* with probability `1/(value(c+1) − value(c))` so the represented count stays
//! unbiased. Counting up to billions then needs only ~16 bits per cell instead of 32–64, so the same
//! memory budget buys a wider/deeper table and a smaller relative error — especially for the
//! low-frequency items where plain Count-Min is worst.
//!
//! * **value(c)** `= c` for `c ≤ limit` (a small exact region), then `limit + (base^{c−limit} − 1) /
//!   (base − 1)`.
//! * **increment probability** from a counter at value `c` is `1` while `c < limit`, then
//!   `base^{−(c−limit)}` — e.g. for `base = 2`, `1 → 2` happens with probability ½, `2 → 3` with ¼, …
//! * **update** is *conservative*: only the counters equal to the current minimum across the rows are
//!   (probabilistically) incremented, which curbs over-counting.
//! * **estimate(key)** `= min over rows of value(counter)`.
//!
//! A `base` close to `1` makes the approximate counters nearly exact (tiny relative error) at the cost
//! of more counter levels; the default targets billions of counts in 16-bit cells.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

const DEFAULT_BASE: f64 = 1.000_25;
const DEFAULT_LIMIT: u16 = 128;
const HASH_SEED: u64 = 0xC11A_06C0_DE10_0001;

/// A Count-Min-Log frequency sketch over byte-string keys.
///
/// # Example
/// ```
/// use sketch_oxide::frequency::CountMinLog;
///
/// let mut cml = CountMinLog::new(2048, 8, 7).unwrap();
/// for _ in 0..10_000 { cml.add(b"hot"); }
/// for _ in 0..50 { cml.add(b"warm"); }
/// cml.add(b"cold");
///
/// // Small counts (within the exact region) are recovered exactly.
/// assert_eq!(cml.estimate(b"cold"), 1.0);
/// assert_eq!(cml.estimate(b"warm"), 50.0);
/// // Large counts are approximate but close, and ordering is preserved.
/// let hot = cml.estimate(b"hot");
/// assert!((hot - 10_000.0).abs() / 10_000.0 < 0.15, "hot estimate {hot}");
/// assert!(hot > cml.estimate(b"warm"));
/// ```
#[derive(Debug, Clone)]
pub struct CountMinLog {
    width: usize,
    depth: usize,
    base: f64,
    limit: u16,
    counters: Vec<u16>, // depth × width
    rng: SmallRng,
    total: u64,
}

impl CountMinLog {
    /// Creates a sketch with `width` counters per row and `depth` rows, default base and exact region.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `width == 0` or `depth` is not in `1..=32`.
    pub fn new(width: usize, depth: usize, seed: u64) -> Result<Self> {
        Self::with_params(width, depth, DEFAULT_BASE, DEFAULT_LIMIT, seed)
    }

    /// Creates a sketch with an explicit logarithm `base` (`> 1`) and exact-region `limit` (counts up
    /// to `limit` are stored exactly; a base nearer `1` is more accurate but reaches smaller maxima).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `width == 0`, `depth` is not in `1..=32`, `base <= 1.0`, or
    /// `limit == 0`.
    pub fn with_params(
        width: usize,
        depth: usize,
        base: f64,
        limit: u16,
        seed: u64,
    ) -> Result<Self> {
        if width == 0 {
            return Err(SketchError::InvalidParameter {
                param: "width".to_string(),
                value: "0".to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        if !(1..=32).contains(&depth) {
            return Err(SketchError::InvalidParameter {
                param: "depth".to_string(),
                value: depth.to_string(),
                constraint: "must be in 1..=32".to_string(),
            });
        }
        if base <= 1.0 || !base.is_finite() {
            return Err(SketchError::InvalidParameter {
                param: "base".to_string(),
                value: base.to_string(),
                constraint: "must be > 1.0".to_string(),
            });
        }
        if limit == 0 {
            return Err(SketchError::InvalidParameter {
                param: "limit".to_string(),
                value: "0".to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        Ok(Self {
            width,
            depth,
            base,
            limit,
            counters: vec![0; width * depth],
            rng: SmallRng::seed_from_u64(seed),
            total: 0,
        })
    }

    /// The estimated count represented by a raw counter value `c`.
    fn value(&self, c: u16) -> f64 {
        if c <= self.limit {
            c as f64
        } else {
            let k = (c - self.limit) as i32;
            self.limit as f64 + (self.base.powi(k) - 1.0) / (self.base - 1.0)
        }
    }

    /// Probability of incrementing a counter currently at value `c`.
    fn increment_prob(&self, c: u16) -> f64 {
        if c < self.limit {
            1.0
        } else {
            self.base.powi(-((c - self.limit) as i32))
        }
    }

    /// Column hit by `key` in row `r`.
    fn col(&self, key: &[u8], r: usize) -> usize {
        let h = xxhash(
            key,
            HASH_SEED ^ (r as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15),
        );
        (h % self.width as u64) as usize
    }

    /// Records one occurrence of `key` (conservative, probabilistic increment).
    #[allow(clippy::needless_range_loop)] // `r` indexes the flat counter table and `cols`
    pub fn add(&mut self, key: &[u8]) {
        self.total += 1;
        // Current counter values for this key, and their minimum.
        let mut cmin = u16::MAX;
        let mut cols = [0usize; 32];
        for r in 0..self.depth {
            let c = self.col(key, r);
            cols[r] = c;
            cmin = cmin.min(self.counters[r * self.width + c]);
        }
        if cmin == u16::MAX {
            return; // counter saturated; cannot grow further
        }
        // One probabilistic decision based on the minimum counter, applied to all minima (conservative).
        if self.rng.random::<f64>() < self.increment_prob(cmin) {
            for r in 0..self.depth {
                let idx = r * self.width + cols[r];
                if self.counters[idx] == cmin {
                    self.counters[idx] = cmin + 1;
                }
            }
        }
    }

    /// Estimated frequency of `key` (the minimum represented count across rows).
    #[allow(clippy::needless_range_loop)] // `r` indexes the flat counter table
    pub fn estimate(&self, key: &[u8]) -> f64 {
        let mut best = f64::INFINITY;
        for r in 0..self.depth {
            let c = self.counters[r * self.width + self.col(key, r)];
            best = best.min(self.value(c));
        }
        best
    }

    /// Total number of `add` calls processed.
    #[inline]
    pub fn total(&self) -> u64 {
        self.total
    }

    /// Counters per row.
    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }

    /// Number of rows.
    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(CountMinLog::with_params(0, 4, 2.0, 16, 1).is_err());
        assert!(CountMinLog::with_params(64, 0, 2.0, 16, 1).is_err());
        assert!(CountMinLog::with_params(64, 33, 2.0, 16, 1).is_err());
        assert!(CountMinLog::with_params(64, 4, 1.0, 16, 1).is_err());
        assert!(CountMinLog::with_params(64, 4, 2.0, 0, 1).is_err());
        assert!(CountMinLog::with_params(64, 4, 2.0, 16, 1).is_ok());
    }

    #[test]
    fn small_counts_are_exact() {
        // Counts within the exact region (<= limit) are recovered exactly, with no collisions.
        let mut cml = CountMinLog::new(4096, 8, 3).unwrap();
        for _ in 0..100 {
            cml.add(b"a");
        }
        for _ in 0..7 {
            cml.add(b"b");
        }
        cml.add(b"c");
        assert_eq!(cml.estimate(b"a"), 100.0);
        assert_eq!(cml.estimate(b"b"), 7.0);
        assert_eq!(cml.estimate(b"c"), 1.0);
        assert_eq!(cml.estimate(b"absent"), 0.0);
        assert_eq!(cml.total(), 108);
    }

    #[test]
    fn base_two_counter_quantises_geometrically() {
        // With base 2 and limit 1, value(c) doubles each level past the limit: 1, 2, 4, 8, ...
        // (the gap value(c+1)−value(c) = 2^(c-1) is exactly the reciprocal of the increment prob).
        let cml = CountMinLog::with_params(16, 1, 2.0, 1, 1).unwrap();
        assert_eq!(cml.value(0), 0.0);
        assert_eq!(cml.value(1), 1.0);
        assert_eq!(cml.value(2), 2.0);
        assert_eq!(cml.value(3), 4.0);
        assert_eq!(cml.value(4), 8.0);
        // Increment probabilities halve each level past the limit.
        assert!((cml.increment_prob(1) - 1.0).abs() < 1e-9);
        assert!((cml.increment_prob(2) - 0.5).abs() < 1e-9);
        assert!((cml.increment_prob(3) - 0.25).abs() < 1e-9);
    }

    #[test]
    fn large_counts_are_approximately_correct() {
        let mut cml = CountMinLog::new(4096, 8, 9).unwrap();
        for _ in 0..100_000 {
            cml.add(b"heavy");
        }
        let est = cml.estimate(b"heavy");
        assert!(
            (est - 100_000.0).abs() / 100_000.0 < 0.10,
            "estimate {est} off by more than 10%"
        );
    }

    #[test]
    fn preserves_ordering_of_frequencies() {
        let mut cml = CountMinLog::new(4096, 8, 5).unwrap();
        for _ in 0..50_000 {
            cml.add(b"x");
        }
        for _ in 0..5_000 {
            cml.add(b"y");
        }
        for _ in 0..500 {
            cml.add(b"z");
        }
        assert!(cml.estimate(b"x") > cml.estimate(b"y"));
        assert!(cml.estimate(b"y") > cml.estimate(b"z"));
    }

    #[test]
    fn never_underestimates_beyond_counter_noise() {
        // Conservative update keeps estimates near or above the truth for an isolated heavy key.
        let mut cml = CountMinLog::new(8192, 10, 13).unwrap();
        for _ in 0..20_000 {
            cml.add(b"k");
        }
        let est = cml.estimate(b"k");
        // Within the Morris counter's relative error band.
        assert!(est > 18_000.0 && est < 22_000.0, "estimate {est}");
    }
}

// --- Capability-trait adoption (fable5 doc 01 F3) ---
use crate::common::capabilities::Update;

impl Update<[u8]> for CountMinLog {
    fn update(&mut self, item: &[u8]) {
        self.add(item);
    }
}

// PointQuery is intentionally NOT implemented: `CountMinLog::estimate` returns
// `f64` (a Morris-counter approximation), not the `u64` count PointQuery requires.
