//! PGM-index — a learned index with provable worst-case bounds.
//!
//! The PGM-index (Ferragina & Vinciguerra, "The PGM-index: a fully-dynamic compressed
//! learned index with provable worst-case bounds", VLDB 2020) replaces a B-tree's routing
//! structure with a **piecewise-linear model** of the keys' positions. The sorted keys are
//! covered by the minimum number of line segments such that each segment predicts every
//! key's position to within a fixed error `ε`; a lookup evaluates the segment's line and then
//! binary-searches the tiny `2ε` window around the prediction. The result is range and rank
//! queries with worst-case guarantees in space far smaller than a B-tree.
//!
//! This is the static index: built once over a sorted key slice.

use crate::common::{Result, SketchError};

/// One linear segment: predicts position as `intercept + slope · (key − first_key)`.
#[derive(Debug, Clone)]
struct Segment {
    first_key: u64,
    intercept: f64,
    slope: f64,
}

/// A static PGM-index over a sorted set of `u64` keys.
///
/// # Example
/// ```
/// use sketch_oxide::range_filters::PgmIndex;
///
/// let keys: Vec<u64> = (0..10_000).map(|i| i * 3).collect(); // 0,3,6,...
/// let pgm = PgmIndex::new(&keys, 32).unwrap();
///
/// assert!(pgm.contains(2_997));   // = 999*3
/// assert!(!pgm.contains(2_998));
/// assert_eq!(pgm.rank(3_000), 1_000); // 1000 keys are < 3000
/// ```
#[derive(Debug, Clone)]
pub struct PgmIndex {
    keys: Vec<u64>,
    epsilon: usize,
    segments: Vec<Segment>,
}

impl PgmIndex {
    /// Builds a PGM-index over `keys` (which must be sorted ascending) with position error
    /// bound `epsilon`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `epsilon == 0` or `keys` is not sorted ascending.
    pub fn new(keys: &[u64], epsilon: usize) -> Result<Self> {
        if epsilon == 0 {
            return Err(SketchError::InvalidParameter {
                param: "epsilon".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        if keys.windows(2).any(|w| w[0] > w[1]) {
            return Err(SketchError::InvalidParameter {
                param: "keys".to_string(),
                value: "unsorted".to_string(),
                constraint: "must be sorted ascending".to_string(),
            });
        }

        let segments = Self::build_segments(keys, epsilon);
        Ok(Self {
            keys: keys.to_vec(),
            epsilon,
            segments,
        })
    }

    /// Greedy cone-shrinking segmentation: each segment is the longest run whose positions a
    /// single line can predict within `±epsilon`.
    fn build_segments(keys: &[u64], epsilon: usize) -> Vec<Segment> {
        let mut segments = Vec::new();
        let eps = epsilon as f64;
        let mut i = 0;
        while i < keys.len() {
            let x0 = keys[i] as f64;
            let y0 = i as f64;
            // Feasible slope interval, shrunk by each subsequent point.
            let mut slope_lo = f64::NEG_INFINITY;
            let mut slope_hi = f64::INFINITY;
            let mut j = i + 1;
            while j < keys.len() {
                let dx = keys[j] as f64 - x0;
                let dy = j as f64 - y0;
                if dx == 0.0 {
                    // Duplicate key: positions must already be within eps; keep extending.
                    j += 1;
                    continue;
                }
                let lo = (dy - eps) / dx;
                let hi = (dy + eps) / dx;
                let new_lo = slope_lo.max(lo);
                let new_hi = slope_hi.min(hi);
                if new_lo > new_hi {
                    break; // cone collapsed: close the segment before j
                }
                slope_lo = new_lo;
                slope_hi = new_hi;
                j += 1;
            }
            let slope = if slope_lo.is_finite() && slope_hi.is_finite() {
                0.5 * (slope_lo + slope_hi)
            } else if slope_hi.is_finite() {
                slope_hi
            } else if slope_lo.is_finite() {
                slope_lo
            } else {
                0.0 // single-point segment
            };
            segments.push(Segment {
                first_key: keys[i],
                intercept: y0,
                slope,
            });
            i = j;
        }
        segments
    }

    /// Finds the segment governing `key` (the last one whose `first_key <= key`).
    fn segment_for(&self, key: u64) -> &Segment {
        // Segments are ordered by first_key; binary search for the last <= key.
        let idx = match self.segments.binary_search_by(|s| s.first_key.cmp(&key)) {
            Ok(i) => i,
            Err(0) => 0,
            Err(i) => i - 1,
        };
        &self.segments[idx]
    }

    /// Predicts a key's position, then returns the exact `[lo, hi)` search window in `keys`.
    fn search_window(&self, key: u64) -> (usize, usize) {
        let seg = self.segment_for(key);
        let pred = seg.intercept + seg.slope * (key as f64 - seg.first_key as f64);
        let pred = pred.round();
        let lo = (pred as i64 - self.epsilon as i64).max(0) as usize;
        let hi = ((pred as i64 + self.epsilon as i64 + 1).max(0) as usize).min(self.keys.len());
        (lo.min(self.keys.len()), hi)
    }

    /// Returns the number of keys strictly less than `key` (its rank / lower-bound position).
    pub fn rank(&self, key: u64) -> usize {
        if self.keys.is_empty() {
            return 0;
        }
        let (lo, hi) = self.search_window(key);
        // partition_point over the window: first index in [lo,hi) whose key >= key, offset by lo.
        let window = &self.keys[lo..hi];
        let in_window = window.partition_point(|&k| k < key);
        // Keys before lo are all < key (since the window is centered on the prediction and
        // bounded by epsilon, lo keys are guaranteed < key).
        lo + in_window
    }

    /// Whether `key` is present.
    pub fn contains(&self, key: u64) -> bool {
        let r = self.rank(key);
        r < self.keys.len() && self.keys[r] == key
    }

    /// Number of keys indexed.
    #[inline]
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Whether the index is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Number of linear segments (the index's size proxy).
    #[inline]
    pub fn num_segments(&self) -> usize {
        self.segments.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(PgmIndex::new(&[1, 2, 3], 0).is_err());
        assert!(PgmIndex::new(&[3, 2, 1], 4).is_err()); // unsorted
        assert!(PgmIndex::new(&[1, 2, 3], 4).is_ok());
    }

    #[test]
    fn contains_dense_keys() {
        let keys: Vec<u64> = (0..10_000).collect();
        let pgm = PgmIndex::new(&keys, 16).unwrap();
        for &k in &[0u64, 1, 500, 9_999] {
            assert!(pgm.contains(k), "missing {k}");
        }
        assert!(!pgm.contains(10_000));
    }

    #[test]
    fn contains_sparse_keys() {
        let keys: Vec<u64> = (0..10_000).map(|i| i * 7 + 3).collect();
        let pgm = PgmIndex::new(&keys, 32).unwrap();
        assert!(pgm.contains(3)); // first
        assert!(pgm.contains(7 * 5000 + 3));
        assert!(!pgm.contains(4)); // not a multiple
        assert!(!pgm.contains(7 * 5000 + 4));
    }

    #[test]
    fn rank_is_correct() {
        let keys: Vec<u64> = (0..1000).map(|i| i * 10).collect(); // 0,10,...,9990
        let pgm = PgmIndex::new(&keys, 8).unwrap();
        assert_eq!(pgm.rank(0), 0);
        assert_eq!(pgm.rank(10), 1);
        assert_eq!(pgm.rank(15), 2); // 0 and 10 are < 15
        assert_eq!(pgm.rank(9990), 999);
        assert_eq!(pgm.rank(100_000), 1000);
    }

    #[test]
    fn fewer_segments_than_keys() {
        // Linear data => a handful of segments for many keys (the whole point).
        let keys: Vec<u64> = (0..100_000).collect();
        let pgm = PgmIndex::new(&keys, 32).unwrap();
        assert!(
            pgm.num_segments() < 100,
            "expected few segments, got {}",
            pgm.num_segments()
        );
    }

    #[test]
    fn handles_irregular_distribution() {
        // Mix of dense and sparse regions.
        let mut keys: Vec<u64> = (0..5000).collect();
        keys.extend((0..5000).map(|i| 1_000_000 + i * 1000));
        keys.sort_unstable();
        let pgm = PgmIndex::new(&keys, 32).unwrap();
        for &k in &keys {
            assert!(pgm.contains(k), "missing {k}");
        }
        assert!(!pgm.contains(7_777_777));
    }

    #[test]
    fn empty_index() {
        let pgm = PgmIndex::new(&[], 4).unwrap();
        assert!(pgm.is_empty());
        assert_eq!(pgm.rank(5), 0);
        assert!(!pgm.contains(5));
    }
}
