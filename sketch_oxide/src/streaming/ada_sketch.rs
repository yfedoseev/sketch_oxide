//! Ada-Sketch — a time-adaptive Count-Min sketch (pre-emphasis / de-emphasis).
//!
//! Ada-Sketches (Shrivastava, Konig & Bilenko, "Time Adaptive Sketches (Ada-Sketches) for
//! Summarizing Data Streams", SIGMOD 2016) make a Count-Min sketch *time-aware* so that recent
//! occurrences count for more than old ones, while keeping O(1) updates. The trick is **pre-emphasis**
//! at update time and **de-emphasis** at query time: an update at logical time `t` adds a growing
//! weight `e^{α t}` to the counters instead of 1, and a query at the current time `T` divides the
//! Count-Min estimate by `e^{α T}`. An item seen Δ steps ago therefore contributes `e^{-αΔ}` to its
//! estimate — an exponential time decay — with no per-item bookkeeping.
//!
//! Left unmanaged, `e^{α t}` overflows. Ada-Sketch handles this with a **global rescale**: whenever
//! the live weight grows past a threshold, every counter is divided down and the time origin is
//! advanced, which is exact because the decay is relative.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

/// Rescale once the live emphasis weight exceeds this, to keep `f64` counters well-conditioned.
const RESCALE_THRESHOLD: f64 = 1e150;

/// A time-adaptive Count-Min sketch.
///
/// # Example
/// ```
/// use sketch_oxide::streaming::AdaSketch;
///
/// let mut s = AdaSketch::new(4, 2048, 0.001).unwrap();
/// // "old" arrives early, "recent" arrives late — equal raw counts.
/// for _ in 0..1000 { s.update(b"old"); }
/// for _ in 0..100_000 { s.update(b"filler"); } // advance time
/// for _ in 0..1000 { s.update(b"recent"); }
///
/// // Time-decayed estimate weights the recent occurrences far higher.
/// assert!(s.estimate(b"recent") > s.estimate(b"old"));
/// ```
#[derive(Debug, Clone)]
pub struct AdaSketch {
    depth: usize,
    width: usize,
    alpha: f64,
    counters: Vec<f64>,
    /// Logical clock (increments per update).
    t: f64,
    /// Time origin for the current scale (the emphasis weight is `e^{α (t - base)}`).
    base: f64,
}

impl AdaSketch {
    /// Creates a `depth × width` Ada-Sketch with decay rate `alpha` (larger ⇒ faster forgetting).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `depth` or `width` is 0, or `alpha` is not positive and
    /// finite.
    pub fn new(depth: usize, width: usize, alpha: f64) -> Result<Self> {
        if depth == 0 || width == 0 {
            return Err(SketchError::InvalidParameter {
                param: if depth == 0 { "depth" } else { "width" }.to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        if !alpha.is_finite() || alpha <= 0.0 {
            return Err(SketchError::InvalidParameter {
                param: "alpha".to_string(),
                value: alpha.to_string(),
                constraint: "must be a positive finite number".to_string(),
            });
        }
        Ok(Self {
            depth,
            width,
            alpha,
            counters: vec![0.0; depth * width],
            t: 0.0,
            base: 0.0,
        })
    }

    #[inline]
    fn column(&self, item: &[u8], r: usize) -> usize {
        (xxhash(item, r as u64) % self.width as u64) as usize
    }

    /// The current emphasis weight `e^{α (t - base)}`.
    #[inline]
    fn weight(&self) -> f64 {
        (self.alpha * (self.t - self.base)).exp()
    }

    /// Records one occurrence of `item` at the current logical time, advancing the clock.
    pub fn update(&mut self, item: &[u8]) {
        self.update_weighted(item, 1.0);
    }

    /// Records `count` occurrences of `item` at the current logical time.
    pub fn update_weighted(&mut self, item: &[u8], count: f64) {
        self.t += 1.0;
        let w = self.weight();
        if w > RESCALE_THRESHOLD {
            self.rescale();
        }
        let w = self.weight();
        for r in 0..self.depth {
            let idx = r * self.width + self.column(item, r);
            self.counters[idx] += count * w;
        }
    }

    /// Divides every counter down and advances the time origin so the live weight returns near 1.
    /// Exact because all emphasis is relative to `base`.
    fn rescale(&mut self) {
        let shift = self.t - self.base;
        let factor = (-self.alpha * shift).exp();
        for c in &mut self.counters {
            *c *= factor;
        }
        self.base = self.t;
    }

    /// Time-decayed frequency estimate of `item`: occurrences are weighted by `e^{-α·age}`.
    pub fn estimate(&self, item: &[u8]) -> f64 {
        let raw = (0..self.depth)
            .map(|r| self.counters[r * self.width + self.column(item, r)])
            .fold(f64::INFINITY, f64::min);
        // De-emphasis: divide by the current weight to express the estimate in "now" units.
        raw / self.weight()
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

    /// Logical time (number of updates processed).
    #[inline]
    pub fn time(&self) -> f64 {
        self.t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(AdaSketch::new(0, 16, 0.01).is_err());
        assert!(AdaSketch::new(4, 0, 0.01).is_err());
        assert!(AdaSketch::new(4, 16, 0.0).is_err());
        assert!(AdaSketch::new(4, 16, -1.0).is_err());
        assert!(AdaSketch::new(4, 16, 0.01).is_ok());
    }

    #[test]
    fn recent_outweighs_old() {
        let mut s = AdaSketch::new(5, 4096, 0.001).unwrap();
        for _ in 0..1000 {
            s.update(b"old");
        }
        for i in 0..50_000u64 {
            s.update(&i.to_le_bytes()); // advance time with unrelated items
        }
        for _ in 0..1000 {
            s.update(b"recent");
        }
        let recent = s.estimate(b"recent");
        let old = s.estimate(b"old");
        assert!(recent > old, "recent {recent} should exceed old {old}");
    }

    #[test]
    fn equal_recent_items_are_close() {
        let mut s = AdaSketch::new(5, 4096, 0.0005).unwrap();
        for _ in 0..500 {
            s.update(b"a");
            s.update(b"b"); // interleaved, same recency
        }
        let a = s.estimate(b"a");
        let b = s.estimate(b"b");
        assert!(
            (a - b).abs() < 0.2 * a.max(b),
            "a {a} b {b} should be close"
        );
    }

    #[test]
    fn survives_long_streams_via_rescale() {
        // Many updates would overflow e^{α t} without rescaling; estimates must stay finite.
        let mut s = AdaSketch::new(4, 1024, 0.5).unwrap();
        for i in 0..1_000_000u64 {
            s.update(&(i % 64).to_le_bytes());
        }
        let est = s.estimate(&0u64.to_le_bytes());
        assert!(
            est.is_finite() && est > 0.0,
            "estimate {est} not finite/positive"
        );
    }

    #[test]
    fn recent_burst_estimate_matches_decay_sum() {
        // A burst of N consecutive updates spans time, so even within it the earlier occurrences
        // decay relative to the last: the decayed estimate is Σ_{j=0}^{N-1} e^{-α j}, slightly below
        // the raw count N (≈181 for N=200, α=0.001) and never above it with a wide sketch.
        let alpha = 0.001;
        let n = 200usize;
        let mut s = AdaSketch::new(5, 8192, alpha).unwrap();
        for _ in 0..n {
            s.update(b"burst");
        }
        let expected: f64 = (0..n).map(|j| (-alpha * j as f64).exp()).sum();
        let est = s.estimate(b"burst");
        assert!(
            (est - expected).abs() < 0.05 * expected,
            "estimate {est} should be ≈ decay sum {expected}"
        );
        assert!(
            est < n as f64,
            "decayed estimate {est} must be below raw count {n}"
        );
    }
}
