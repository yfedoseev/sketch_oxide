//! Sliding Sketch framework — turn a point-query sketch into a sliding-window one.
//!
//! The Sliding Sketch framework (Gou, Yang, et al., "Sliding Sketches: A Framework using Time
//! Zones for Data Stream Processing in Sliding Windows", KDD 2020) is a generic adapter:
//! given any sketch whose state is an array of counters (Count-Min, CU, Count-Sketch,
//! HeavyKeeper, Bloom…), it answers point queries over the **last `W` time units** instead of
//! the whole stream, at a small constant memory factor.
//!
//! # Time zones
//!
//! The window `W` is split into `z` equal time zones. Each counter slot is replicated `z`
//! times — one per zone — in a ring. As time advances, the "current" zone rotates and the
//! zone that falls out of the window is cleared. An update writes the current zone; a query
//! sums the slot across all live zones. Old data is forgotten zone-by-zone, giving a smooth
//! sliding window with error `O(1/z)` of the window edge.
//!
//! Here it wraps a generic counter array for windowed frequency (a sliding Count-Min point
//! query). It uses the [`time`](crate::common::time) convention: drive it with `advance(now)`.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError, Temporal, Timestamp};

/// A sliding-window frequency sketch built by the time-zone framework.
///
/// # Example
/// ```
/// use sketch_oxide::streaming::SlidingSketch;
/// use sketch_oxide::common::Temporal;
///
/// // window 1000 units, 10 zones, Count-Min 4x1024.
/// let mut s = SlidingSketch::new(1000, 10, 4, 1024).unwrap();
/// s.advance(0);
/// for _ in 0..500 { s.update(b"a"); }
/// assert!(s.estimate(b"a") >= 500);
///
/// // Advance a full window past the inserts: they age out.
/// s.advance(2000);
/// assert!(s.estimate(b"a") < 100);
/// ```
#[derive(Debug, Clone)]
pub struct SlidingSketch {
    window: u64,
    zones: usize,
    depth: usize,
    width: usize,
    /// `depth * width * zones` counters, indexed `[(r*width + c) * zones + zone]`.
    counters: Vec<u64>,
    /// Watermark / current time.
    now: Timestamp,
    /// The zone index currently being written.
    current_zone: usize,
}

impl SlidingSketch {
    /// Creates a sliding sketch over a `window`, split into `zones` time zones, wrapping a
    /// `depth × width` counter array.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if any of `window`, `zones`, `depth`, `width` is 0.
    pub fn new(window: u64, zones: usize, depth: usize, width: usize) -> Result<Self> {
        let bad = if window == 0 {
            Some("window")
        } else if zones == 0 {
            Some("zones")
        } else if depth == 0 {
            Some("depth")
        } else if width == 0 {
            Some("width")
        } else {
            None
        };
        if let Some(p) = bad {
            return Err(SketchError::InvalidParameter {
                param: p.to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            window,
            zones,
            depth,
            width,
            counters: vec![0u64; depth * width * zones],
            now: 0,
            current_zone: 0,
        })
    }

    #[inline]
    fn slot(&self, item: &[u8], r: usize) -> usize {
        (xxhash(item, r as u64) % self.width as u64) as usize
    }

    #[inline]
    fn cell(&self, r: usize, c: usize, zone: usize) -> usize {
        (r * self.width + c) * self.zones + zone
    }

    /// Records one occurrence of `item` in the current time zone.
    pub fn update(&mut self, item: &[u8]) {
        for r in 0..self.depth {
            let c = self.slot(item, r);
            let idx = self.cell(r, c, self.current_zone);
            self.counters[idx] += 1;
        }
    }

    /// Estimates the count of `item` over the live window (sum over zones, min over rows).
    pub fn estimate(&self, item: &[u8]) -> u64 {
        (0..self.depth)
            .map(|r| {
                let c = self.slot(item, r);
                (0..self.zones)
                    .map(|z| self.counters[self.cell(r, c, z)])
                    .sum::<u64>()
            })
            .min()
            .unwrap_or(0)
    }

    /// Clears a zone's counters (called when a zone rotates out of the window).
    fn clear_zone(&mut self, zone: usize) {
        for r in 0..self.depth {
            for c in 0..self.width {
                let idx = self.cell(r, c, zone);
                self.counters[idx] = 0;
            }
        }
    }

    /// Width of one time zone in time units.
    #[inline]
    fn zone_span(&self) -> u64 {
        (self.window / self.zones as u64).max(1)
    }

    /// Number of time zones.
    #[inline]
    pub fn num_zones(&self) -> usize {
        self.zones
    }
}

impl Temporal for SlidingSketch {
    /// Advances to `now`, rotating the current zone forward and clearing zones that have aged
    /// out of the window. Monotonic.
    fn advance(&mut self, now: Timestamp) {
        if now <= self.now {
            return;
        }
        let span = self.zone_span();
        let elapsed_zones = ((now - self.now) / span) as usize;
        if elapsed_zones == 0 {
            self.now = now;
            return;
        }
        // Rotate forward, clearing each newly-entered zone (which holds the oldest data).
        let steps = elapsed_zones.min(self.zones);
        for _ in 0..steps {
            self.current_zone = (self.current_zone + 1) % self.zones;
            self.clear_zone(self.current_zone);
        }
        self.now = now;
    }

    fn watermark(&self) -> Timestamp {
        self.now
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_params() {
        assert!(SlidingSketch::new(0, 10, 4, 64).is_err());
        assert!(SlidingSketch::new(100, 0, 4, 64).is_err());
        assert!(SlidingSketch::new(100, 10, 0, 64).is_err());
        assert!(SlidingSketch::new(100, 10, 4, 0).is_err());
        assert!(SlidingSketch::new(100, 10, 4, 64).is_ok());
    }

    #[test]
    fn counts_within_window() {
        let mut s = SlidingSketch::new(1000, 10, 4, 1024).unwrap();
        s.advance(0);
        for _ in 0..500 {
            s.update(b"a");
        }
        assert!(s.estimate(b"a") >= 500, "estimate {}", s.estimate(b"a"));
    }

    #[test]
    fn old_data_ages_out() {
        let mut s = SlidingSketch::new(1000, 10, 4, 1024).unwrap();
        s.advance(0);
        for _ in 0..500 {
            s.update(b"a");
        }
        // Advance two full windows: every zone that held "a" is cleared.
        s.advance(2500);
        assert!(s.estimate(b"a") < 50, "estimate {}", s.estimate(b"a"));
    }

    #[test]
    fn partial_window_retains_recent() {
        let mut s = SlidingSketch::new(1000, 10, 4, 1024).unwrap();
        s.advance(0);
        for _ in 0..300 {
            s.update(b"a");
        }
        s.advance(500); // half a window later
        for _ in 0..300 {
            s.update(b"a");
        }
        // Both batches are within the 1000-window at t=500.
        assert!(s.estimate(b"a") >= 600, "estimate {}", s.estimate(b"a"));
    }

    #[test]
    fn advance_is_monotonic() {
        let mut s = SlidingSketch::new(1000, 10, 4, 64).unwrap();
        s.advance(500);
        s.advance(200); // ignored
        assert_eq!(s.watermark(), 500);
    }

    #[test]
    fn never_underestimates_recent() {
        let mut s = SlidingSketch::new(10_000, 20, 5, 2048).unwrap();
        s.advance(0);
        for _ in 0..50 {
            s.update(b"target");
        }
        for i in 0..5000u64 {
            s.update(&i.to_le_bytes());
        }
        assert!(s.estimate(b"target") >= 50, "no underestimate");
    }
}
