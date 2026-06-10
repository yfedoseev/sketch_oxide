//! ECM-Sketch — Exponential Count-Min: per-key frequency over a sliding window.
//!
//! A Count-Min sketch where every counter is replaced by an Exponential Histogram, so each
//! cell tracks an approximate count *within the last `window` time units* instead of a
//! lifetime total (Papapetrou, Garofalakis & Deligiannakis, VLDB 2012). Querying a key
//! returns the minimum windowed count across its rows — the windowed analogue of Count-Min's
//! point query.
//!
//! It reuses the shared [`EhCore`](super) bucket engine, so the per-cell windowing inherits
//! the Datar et al. relative-error guarantee.
//!
//! # Errors come from two places
//!
//! - the Count-Min hashing collisions (overestimate, bounded by `ε · N` with
//!   `width = e/ε`), and
//! - the per-cell exponential histogram (bounded by `eh_epsilon` of the cell's true count).

use crate::common::{Result, SketchError};
use crate::streaming::eh_core::EhCore;

/// Exponential Count-Min sketch: approximate per-key counts over a sliding time window.
///
/// # Example
/// ```
/// use sketch_oxide::streaming::EcmSketch;
///
/// let mut ecm = EcmSketch::new(4, 512, 1000, 0.05).unwrap();
/// // "a" occurs 100 times at timestamps 0..100.
/// for t in 0..100u64 {
///     ecm.update(b"a", t);
/// }
/// // At time 100 the whole window [0,100] is in range: ~100.
/// let c = ecm.estimate(b"a", 100);
/// assert!(c >= 80 && c <= 120, "windowed count {c}");
/// // Far in the future the early events have aged out of the window.
/// assert!(ecm.estimate(b"a", 5000) < 20);
/// ```
#[derive(Debug, Clone)]
pub struct EcmSketch {
    depth: usize,
    width: usize,
    /// `depth * width` exponential-histogram cells, row-major.
    cells: Vec<EhCore>,
}

impl EcmSketch {
    /// Creates an ECM-Sketch.
    ///
    /// * `depth` — number of hash rows (Count-Min confidence; failure ≤ `e^-depth`).
    /// * `width` — counters per row (Count-Min accuracy; overestimate ≤ `e/width · N`).
    /// * `window` — sliding-window length, in the caller's timestamp units.
    /// * `eh_epsilon` — per-cell exponential-histogram relative error, in `(0, 1)`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `depth`/`width`/`window` is 0 or `eh_epsilon` is
    /// not in `(0, 1)`.
    pub fn new(depth: usize, width: usize, window: u64, eh_epsilon: f64) -> Result<Self> {
        if depth == 0 || width == 0 {
            return Err(SketchError::InvalidParameter {
                param: if depth == 0 { "depth" } else { "width" }.to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        // EhCore::new validates window > 0 and eh_epsilon in (0, 1).
        let cells = (0..depth * width)
            .map(|_| EhCore::new(window, eh_epsilon))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            depth,
            width,
            cells,
        })
    }

    /// Column for `item` in row `r`.
    #[inline]
    fn column(&self, item: &[u8], r: usize) -> usize {
        (crate::common::hash::xxhash(item, r as u64) % self.width as u64) as usize
    }

    /// Records one occurrence of `item` at `timestamp`.
    pub fn update(&mut self, item: &[u8], timestamp: u64) {
        for r in 0..self.depth {
            let col = self.column(item, r);
            self.cells[r * self.width + col].insert(timestamp, 1);
        }
    }

    /// Estimates the count of `item` within the window ending at `now`.
    ///
    /// Returns the minimum windowed count across rows (Count-Min point query). Each cell's
    /// windowed count is the EH estimate (full in-window buckets + half the straddling one).
    pub fn estimate(&self, item: &[u8], now: u64) -> u64 {
        (0..self.depth)
            .map(|r| {
                let col = self.column(item, r);
                self.cells[r * self.width + col].count_with_bounds(now).0
            })
            .min()
            .unwrap_or(0)
    }

    /// Drops buckets that have aged entirely out of the window for every cell. Optional —
    /// queries are already window-correct; this just reclaims memory.
    pub fn expire(&mut self, now: u64) {
        for cell in &mut self.cells {
            cell.expire(now);
        }
    }

    /// Number of hash rows.
    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Counters per row.
    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(EcmSketch::new(0, 10, 100, 0.1).is_err());
        assert!(EcmSketch::new(4, 0, 100, 0.1).is_err());
        assert!(EcmSketch::new(4, 10, 0, 0.1).is_err());
        assert!(EcmSketch::new(4, 10, 100, 0.0).is_err());
        assert!(EcmSketch::new(4, 10, 100, 1.0).is_err());
        assert!(EcmSketch::new(4, 10, 100, 0.1).is_ok());
    }

    #[test]
    fn counts_within_window() {
        let mut ecm = EcmSketch::new(5, 512, 1000, 0.02).unwrap();
        for t in 0..200u64 {
            ecm.update(b"a", t);
        }
        let c = ecm.estimate(b"a", 200);
        assert!((180..=220).contains(&c), "windowed count {c}");
    }

    #[test]
    fn old_events_age_out() {
        let mut ecm = EcmSketch::new(5, 512, 1000, 0.05).unwrap();
        for t in 0..100u64 {
            ecm.update(b"a", t);
        }
        // Window at t=5000 is [4000, 5000]; all events at [0,100] are gone.
        assert!(ecm.estimate(b"a", 5000) < 20);
    }

    #[test]
    fn never_underestimates_distinct_keys() {
        let mut ecm = EcmSketch::new(5, 1024, 10_000, 0.02).unwrap();
        // 50 occurrences of "hot", 1 each of many cold keys, all within window.
        for t in 0..50u64 {
            ecm.update(b"hot", t);
        }
        for i in 0..500u64 {
            ecm.update(&i.to_le_bytes(), 100 + i);
        }
        let hot = ecm.estimate(b"hot", 600);
        // Count-Min never underestimates; small overestimate possible.
        assert!(hot >= 50, "hot count {hot} should be >= 50");
        assert!(hot < 80, "hot count {hot} overestimated too much");
    }

    #[test]
    fn unseen_key_estimates_low() {
        let mut ecm = EcmSketch::new(5, 1024, 1000, 0.05).unwrap();
        for t in 0..100u64 {
            ecm.update(b"present", t);
        }
        // An unseen key may collide but should be small relative to the stream.
        assert!(ecm.estimate(b"absent", 100) < 100);
    }

    #[test]
    fn expire_reclaims_without_changing_query() {
        let mut ecm = EcmSketch::new(4, 256, 1000, 0.05).unwrap();
        for t in 0..200u64 {
            ecm.update(b"a", t);
        }
        let before = ecm.estimate(b"a", 200);
        ecm.expire(200);
        let after = ecm.estimate(b"a", 200);
        assert_eq!(before, after, "expire must not change in-window queries");
    }
}
