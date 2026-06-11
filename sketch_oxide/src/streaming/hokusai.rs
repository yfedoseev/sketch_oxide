//! Hokusai — time-adaptive frequency sketches (Matusevych, Smola & Ahmed, UAI 2012).
//!
//! A single Count-Min sketch answers "how often have I seen item `x`?" but forgets *when*. Hokusai
//! adds a **time axis** cheaply: it keeps a logarithmic ladder of Count-Min sketches so that, at any
//! moment, sketch `M_j` summarises item frequencies over the **most recent `2^j` time units**. Old
//! observations decay in resolution — recent windows are fine-grained, older ones exponentially
//! coarser — so `T` time steps fit in `O(log T)` space instead of `O(T)`.
//!
//! # Time aggregation (Algorithm 2)
//!
//! Events are accumulated into a per-unit aggregator. Each [`tick`](Hokusai::tick) completes one unit
//! interval and cascades it up the ladder with a **binary-carry swap-accumulate**: let `carry` be the
//! just-completed block and `ν = min(trailing_zeros(t), levels−1)`; for `j = 0..=ν`, swap `carry` with
//! `M_j` and fold the old `M_j` back into `carry`. This is exactly the carry propagation of a binary
//! counter, and (paper Theorem 4) leaves `M_j` holding the aggregate over the most recent `2^j` units.
//! Amortised cost is `O(1)` per tick: level `j` is touched only every `2^j` ticks.
//!
//! Folding past the top level discards data older than `2^(levels−1)` units — the intended
//! finite-memory truncation. Counts are Count-Min estimates, so every windowed count is an **upper
//! bound** on the truth (no underestimate, like all Count-Min sketches).
//!
//! # Note
//!
//! The paper also gives *item aggregation* (halving a sketch's width by folding bin `i` into `i+w/2`
//! to trade key resolution for time span) and *resolution extrapolation* (recovering per-instant
//! counts from the time/item marginals). This reference implements the headline time-aggregation
//! ladder, whose windowed frequencies are exact up to Count-Min collisions.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

const SEED: u64 = 0x484F_4B55_5341_4901; // "HOKUSAI" + tag

/// A behaviour-faithful Count-Min table with explicit row-major counters, supporting the
/// element-wise addition the time-aggregation cascade needs.
#[derive(Debug, Clone)]
struct CmTable {
    width: usize,
    depth: usize,
    counts: Vec<u64>, // depth × width, row-major
}

impl CmTable {
    fn new(width: usize, depth: usize) -> Self {
        Self {
            width,
            depth,
            counts: vec![0; width * depth],
        }
    }

    /// Column hit by `item` in row `r`.
    fn col(&self, item: &[u8], r: usize) -> usize {
        (xxhash(item, SEED ^ (r as u64)) % self.width as u64) as usize
    }

    /// Adds `count` to every row's counter for `item`.
    fn add(&mut self, item: &[u8], count: u64) {
        for r in 0..self.depth {
            let c = self.col(item, r);
            self.counts[r * self.width + c] += count;
        }
    }

    /// Count-Min point estimate: the minimum counter across rows (an upper bound on the truth).
    fn estimate(&self, item: &[u8]) -> u64 {
        (0..self.depth)
            .map(|r| {
                let c = self.col(item, r);
                self.counts[r * self.width + c]
            })
            .min()
            .unwrap_or(0)
    }

    /// Element-wise `self += other` (same dimensions).
    fn add_assign(&mut self, other: &CmTable) {
        for (a, b) in self.counts.iter_mut().zip(&other.counts) {
            *a += *b;
        }
    }
}

/// A Hokusai time-adaptive frequency sketch over byte-string items.
///
/// `M_j` (`j` in `0..levels`) holds item frequencies over the most recent `2^j` completed time units;
/// [`tick`](Hokusai::tick) advances time by one unit. All windowed counts are Count-Min upper bounds.
///
/// # Example
/// ```
/// use sketch_oxide::streaming::Hokusai;
///
/// // 4 window levels (most recent 1, 2, 4, 8 units), 1024-wide, depth-4 Count-Min tables.
/// let mut h = Hokusai::new(4, 1024, 4).unwrap();
/// // See "x" once per time unit for 8 ticks.
/// for _ in 0..8 {
///     h.add(b"x");
///     h.tick();
/// }
/// // Most recent 1 / 2 / 4 / 8 units each contain that many "x".
/// assert_eq!(h.estimate_window(b"x", 0), 1);
/// assert_eq!(h.estimate_window(b"x", 1), 2);
/// assert_eq!(h.estimate_window(b"x", 2), 4);
/// assert_eq!(h.estimate_window(b"x", 3), 8);
/// ```
#[derive(Debug, Clone)]
pub struct Hokusai {
    levels: u32,
    width: usize,
    depth: usize,
    agg: CmTable,          // current (incomplete) unit-interval aggregator
    windows: Vec<CmTable>, // M_0..M_{levels-1}
    t: u64,                // number of completed time units
}

impl Hokusai {
    /// Creates a Hokusai sketch with `levels` window levels (the most recent `1, 2, …, 2^(levels−1)`
    /// units), each a `width`-wide, `depth`-deep Count-Min table.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `levels` is not in `1..=32`, `width == 0`, or `depth` is
    /// not in `1..=16`.
    pub fn new(levels: u32, width: usize, depth: usize) -> Result<Self> {
        if !(1..=32).contains(&levels) {
            return Err(SketchError::InvalidParameter {
                param: "levels".to_string(),
                value: levels.to_string(),
                constraint: "must be in 1..=32".to_string(),
            });
        }
        if width == 0 {
            return Err(SketchError::InvalidParameter {
                param: "width".to_string(),
                value: "0".to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        if !(1..=16).contains(&depth) {
            return Err(SketchError::InvalidParameter {
                param: "depth".to_string(),
                value: depth.to_string(),
                constraint: "must be in 1..=16".to_string(),
            });
        }
        let windows = (0..levels).map(|_| CmTable::new(width, depth)).collect();
        Ok(Self {
            levels,
            width,
            depth,
            agg: CmTable::new(width, depth),
            windows,
            t: 0,
        })
    }

    /// Records one occurrence of `item` in the current (in-progress) time unit.
    pub fn add(&mut self, item: &[u8]) {
        self.agg.add(item, 1);
    }

    /// Records `count` occurrences of `item` in the current time unit.
    pub fn add_count(&mut self, item: &[u8], count: u64) {
        self.agg.add(item, count);
    }

    /// Completes the current time unit and cascades it up the window ladder (Algorithm 2).
    pub fn tick(&mut self) {
        self.t += 1;
        // `carry` starts as the just-completed unit block; the aggregator is reset for the next unit.
        let mut carry = std::mem::replace(&mut self.agg, CmTable::new(self.width, self.depth));
        let nu = self.t.trailing_zeros().min(self.levels - 1);
        for j in 0..=nu as usize {
            // T = M_j; M_j = carry; carry = carry + T  (binary-carry swap-accumulate).
            std::mem::swap(&mut self.windows[j], &mut carry);
            carry.add_assign(&self.windows[j]);
        }
        // Any residual `carry` past the top level is data older than 2^(levels−1) units — dropped.
    }

    /// Estimated count of `item` over the most recent `2^level` completed time units (an upper bound).
    ///
    /// # Panics
    /// If `level >= levels`.
    pub fn estimate_window(&self, item: &[u8], level: u32) -> u64 {
        assert!(level < self.levels, "level out of range");
        self.windows[level as usize].estimate(item)
    }

    /// Estimated count of `item` in the current in-progress (not-yet-ticked) time unit.
    pub fn estimate_current(&self, item: &[u8]) -> u64 {
        self.agg.estimate(item)
    }

    /// The span (number of time units) covered by window `level`, i.e. `2^level`.
    pub fn window_span(&self, level: u32) -> u64 {
        1u64 << level
    }

    /// Number of completed time units (ticks) so far.
    pub fn time(&self) -> u64 {
        self.t
    }

    /// Number of window levels.
    pub fn levels(&self) -> u32 {
        self.levels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(Hokusai::new(0, 256, 4).is_err());
        assert!(Hokusai::new(33, 256, 4).is_err());
        assert!(Hokusai::new(4, 0, 4).is_err());
        assert!(Hokusai::new(4, 256, 0).is_err());
        assert!(Hokusai::new(4, 256, 17).is_err());
        assert!(Hokusai::new(4, 256, 4).is_ok());
    }

    #[test]
    fn windows_hold_recent_exponential_spans() {
        // One "x" per unit for 8 units: window 2^j holds exactly 2^j of them.
        let mut h = Hokusai::new(4, 4096, 4).unwrap();
        for _ in 0..8 {
            h.add(b"x");
            h.tick();
        }
        assert_eq!(h.time(), 8);
        assert_eq!(h.estimate_window(b"x", 0), 1);
        assert_eq!(h.estimate_window(b"x", 1), 2);
        assert_eq!(h.estimate_window(b"x", 2), 4);
        assert_eq!(h.estimate_window(b"x", 3), 8);
        assert_eq!(h.window_span(3), 8);
    }

    #[test]
    fn old_observations_decay_out_of_recent_windows() {
        // "old" appears only in the first unit, then nothing.
        let mut h = Hokusai::new(3, 4096, 4).unwrap(); // windows of span 1, 2, 4
        h.add(b"old");
        h.tick(); // t = 1, "old" is in unit block [0,1]
        for _ in 0..3 {
            h.tick(); // t = 2, 3, 4 — empty units
        }
        // At t = 4 the top window (span 4) still covers [0,4], so "old" is visible there.
        assert_eq!(h.estimate_window(b"old", 2), 1);
        // The most-recent-1 window already forgot it.
        assert_eq!(h.estimate_window(b"old", 0), 0);
        // Advance to t = 8: the top window refreshes to [4,8] and "old" is gone everywhere.
        for _ in 0..4 {
            h.tick();
        }
        assert_eq!(h.estimate_window(b"old", 0), 0);
        assert_eq!(h.estimate_window(b"old", 1), 0);
        assert_eq!(h.estimate_window(b"old", 2), 0);
    }

    #[test]
    fn windowed_counts_are_upper_bounds() {
        // With many distinct keys and a narrow sketch, estimates may collide upward but never below
        // the true windowed count — the Count-Min one-sided guarantee, preserved per window.
        let mut h = Hokusai::new(5, 64, 4).unwrap();
        let mut truth_recent16 = 0u64; // true count of key 7 in the most recent 16 units
        for t in 0..32u64 {
            // key 7 appears every unit; lots of noise keys too
            h.add_count(b"\x07", 3);
            truth_recent16 += 3;
            for n in 0..50u64 {
                h.add(&n.to_le_bytes());
            }
            h.tick();
            if t >= 16 {
                truth_recent16 -= 3; // it left the trailing edge of the 16-window
            }
        }
        // Window level 4 spans 2^4 = 16 units.
        let est = h.estimate_window(b"\x07", 4);
        assert!(
            est >= truth_recent16,
            "estimate {est} below true windowed count {truth_recent16}"
        );
    }

    #[test]
    fn current_unit_is_separate_from_windows() {
        let mut h = Hokusai::new(3, 1024, 4).unwrap();
        h.add(b"y");
        h.add(b"y");
        // Before ticking, "y" is only in the current aggregator, not in any completed window.
        assert_eq!(h.estimate_current(b"y"), 2);
        assert_eq!(h.estimate_window(b"y", 0), 0);
        h.tick();
        // After ticking, it has moved into the most-recent-1 window.
        assert_eq!(h.estimate_current(b"y"), 0);
        assert_eq!(h.estimate_window(b"y", 0), 2);
    }
}
