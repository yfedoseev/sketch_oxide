//! k-ary sketch — sketch-based heavy-change detection.
//!
//! The k-ary sketch (Krishnamurthy, Sen, Zhang & Chen, "Sketch-based Change Detection: Methods,
//! Evaluation, and Applications", IMC 2003) is a linear sketch designed to find items whose value
//! *changed* the most between two points in time — the "deltoids". Like a Count-Min sketch it keeps
//! `depth` rows of `width` buckets and adds each item's signed value to one bucket per row, but it
//! reads back an **unbiased** per-key estimate `(bucket − total/width) / (1 − 1/width)` (median over
//! rows). Because the sketch is linear, the difference of two snapshots is itself a sketch — of the
//! *change vector* — so heavy changers are simply the keys whose estimate in the difference sketch
//! exceeds a threshold.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

/// A k-ary sketch over `depth × width` signed buckets.
///
/// # Example
/// ```
/// use sketch_oxide::statistics::KArySketch;
///
/// // Two epochs; one key surges, the rest are background.
/// let mut t1 = KArySketch::new(5, 2048).unwrap();
/// let mut t2 = KArySketch::new(5, 2048).unwrap();
/// for i in 0..1000u64 { t1.update(i, 10); t2.update(i, 10); } // unchanged
/// t2.update(42, 5000);                                        // a big surge on key 42
///
/// let diff = t2.difference(&t1).unwrap();
/// assert!(diff.estimate(42) > 4000.0, "change estimate {}", diff.estimate(42));
/// ```
#[derive(Debug, Clone)]
pub struct KArySketch {
    depth: usize,
    width: usize,
    /// `depth × width` signed buckets, row-major.
    buckets: Vec<i64>,
    /// Net total added (the L1/row sum, identical across rows).
    total: i64,
}

impl KArySketch {
    /// Creates a `depth × width` k-ary sketch.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `depth` or `width` is 0 (or `width` is 1, which makes the
    /// unbiased estimator undefined).
    pub fn new(depth: usize, width: usize) -> Result<Self> {
        if depth == 0 || width < 2 {
            return Err(SketchError::InvalidParameter {
                param: if depth == 0 { "depth" } else { "width" }.to_string(),
                value: format!("{depth}/{width}"),
                constraint: "depth > 0 and width >= 2".to_string(),
            });
        }
        Ok(Self {
            depth,
            width,
            buckets: vec![0i64; depth * width],
            total: 0,
        })
    }

    #[inline]
    fn column(&self, key: u64, r: usize) -> usize {
        (xxhash(&key.to_le_bytes(), r as u64) % self.width as u64) as usize
    }

    /// Adds signed `value` to `key` (negative values model decrements / deletions).
    pub fn update(&mut self, key: u64, value: i64) {
        self.total += value;
        for r in 0..self.depth {
            let idx = r * self.width + self.column(key, r);
            self.buckets[idx] += value;
        }
    }

    /// Unbiased estimate of `key`'s value: the median over rows of `(bucket − total/width) /
    /// (1 − 1/width)`.
    pub fn estimate(&self, key: u64) -> f64 {
        let w = self.width as f64;
        let mean = self.total as f64 / w;
        let denom = 1.0 - 1.0 / w;
        let mut est: Vec<f64> = (0..self.depth)
            .map(|r| {
                let bucket = self.buckets[r * self.width + self.column(key, r)] as f64;
                (bucket - mean) / denom
            })
            .collect();
        est.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = est.len();
        if n % 2 == 1 {
            est[n / 2]
        } else {
            0.5 * (est[n / 2 - 1] + est[n / 2])
        }
    }

    /// The difference sketch `self − other` (a sketch of the change vector). Both must share
    /// dimensions.
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the dimensions differ.
    pub fn difference(&self, other: &Self) -> Result<Self> {
        if self.depth != other.depth || self.width != other.width {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "shape mismatch: {}x{} vs {}x{}",
                    self.depth, self.width, other.depth, other.width
                ),
            });
        }
        let buckets = self
            .buckets
            .iter()
            .zip(&other.buckets)
            .map(|(a, b)| a - b)
            .collect();
        Ok(Self {
            depth: self.depth,
            width: self.width,
            buckets,
            total: self.total - other.total,
        })
    }

    /// Among `candidates`, the keys whose absolute estimate exceeds `threshold`, paired with the
    /// estimate and sorted by magnitude descending. Run on a difference sketch to get heavy changers.
    pub fn heavy_changers(&self, candidates: &[u64], threshold: f64) -> Vec<(u64, f64)> {
        let mut out: Vec<(u64, f64)> = candidates
            .iter()
            .map(|&k| (k, self.estimate(k)))
            .filter(|&(_, e)| e.abs() >= threshold)
            .collect();
        out.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap());
        out
    }

    /// Number of hash rows.
    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Buckets per row.
    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }

    /// Net total added.
    #[inline]
    pub fn total(&self) -> i64 {
        self.total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(KArySketch::new(0, 16).is_err());
        assert!(KArySketch::new(4, 1).is_err());
        assert!(KArySketch::new(4, 16).is_ok());
    }

    #[test]
    fn estimates_a_lone_value() {
        let mut s = KArySketch::new(7, 4096).unwrap();
        s.update(123, 5000);
        for i in 0..2000u64 {
            s.update(1000 + i, 1); // light background
        }
        let est = s.estimate(123);
        assert!((est - 5000.0).abs() < 100.0, "estimate {est}");
    }

    #[test]
    fn difference_isolates_the_change() {
        let mut t1 = KArySketch::new(5, 4096).unwrap();
        let mut t2 = KArySketch::new(5, 4096).unwrap();
        for i in 0..5000u64 {
            t1.update(i, 10);
            t2.update(i, 10); // unchanged baseline
        }
        t2.update(42, 3000); // surge
        t2.update(7, -2000); // drop
        let diff = t2.difference(&t1).unwrap();
        assert!(
            (diff.estimate(42) - 3000.0).abs() < 150.0,
            "surge {}",
            diff.estimate(42)
        );
        assert!(
            (diff.estimate(7) + 2000.0).abs() < 150.0,
            "drop {}",
            diff.estimate(7)
        );
        // An unchanged key reads ~0.
        assert!(
            diff.estimate(100).abs() < 150.0,
            "unchanged {}",
            diff.estimate(100)
        );
    }

    #[test]
    fn heavy_changers_found() {
        let mut t1 = KArySketch::new(5, 4096).unwrap();
        let mut t2 = KArySketch::new(5, 4096).unwrap();
        for i in 0..3000u64 {
            t1.update(i, 5);
            t2.update(i, 5);
        }
        t2.update(11, 9000);
        t2.update(22, 8000);
        let diff = t2.difference(&t1).unwrap();
        let candidates: Vec<u64> = (0..3000).collect();
        let hc = diff.heavy_changers(&candidates, 4000.0);
        let keys: Vec<u64> = hc.iter().map(|&(k, _)| k).collect();
        assert!(
            keys.contains(&11) && keys.contains(&22),
            "heavy changers {keys:?}"
        );
        // Sorted by magnitude: 11 (9000) before 22 (8000).
        assert_eq!(hc[0].0, 11);
    }

    #[test]
    fn difference_mismatch_errors() {
        let a = KArySketch::new(5, 1024).unwrap();
        let b = KArySketch::new(5, 2048).unwrap();
        assert!(a.difference(&b).is_err());
    }

    #[test]
    fn linear_under_decrements() {
        let mut s = KArySketch::new(5, 4096).unwrap();
        s.update(9, 100);
        s.update(9, -40);
        for i in 0..1000u64 {
            s.update(2000 + i, 1);
        }
        assert!(
            (s.estimate(9) - 60.0).abs() < 50.0,
            "estimate {}",
            s.estimate(9)
        );
    }
}
