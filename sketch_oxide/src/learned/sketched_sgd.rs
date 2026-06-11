//! Sketched gradient compression (SketchedSGD / FetchSGD).
//!
//! In distributed SGD the per-step gradient is a huge, mostly-small vector that must be summed
//! across workers and largely consists of a few important ("heavy") coordinates. FetchSGD (Rothchild
//! et al., "FetchSGD: Communication-Efficient Federated Learning with Sketching", ICML 2020) and
//! the earlier SketchedSGD compress each worker's gradient with a **Count Sketch**: a small signed
//! linear sketch that (1) sums correctly when worker sketches are added — so aggregation is just
//! sketch addition — and (2) lets the server recover the **top-k** heaviest coordinates of the
//! summed gradient, which it applies and feeds back. Because the sketch is linear, momentum and
//! error-accumulation also live in sketch space.
//!
//! This is a Count Sketch specialized for real-valued coordinates: `depth` rows of `width` signed
//! buckets, with the unbiased median estimator and exact mergeability.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

/// A Count-Sketch gradient compressor whose sketches add across workers.
///
/// # Example
/// ```
/// use sketch_oxide::learned::GradientSketch;
///
/// // A gradient with three heavy coordinates among 10_000 small ones.
/// let dim = 10_000;
/// let mut grad = vec![0.0f64; dim];
/// grad[42] = 9.0; grad[1234] = -7.0; grad[9999] = 5.0;
///
/// let mut sk = GradientSketch::new(5, 4096).unwrap();
/// sk.accumulate(&grad);
///
/// // The server recovers the heaviest coordinates without ever seeing the full vector.
/// let top = sk.top_k(dim, 3);
/// let coords: Vec<usize> = top.iter().map(|&(i, _)| i).collect();
/// assert!(coords.contains(&42) && coords.contains(&1234) && coords.contains(&9999));
/// ```
#[derive(Debug, Clone)]
pub struct GradientSketch {
    depth: usize,
    width: usize,
    /// `depth × width` signed accumulators, row-major.
    table: Vec<f64>,
}

impl GradientSketch {
    /// Creates a sketch with `depth` hash rows of `width` buckets each.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `depth` or `width` is 0.
    pub fn new(depth: usize, width: usize) -> Result<Self> {
        if depth == 0 || width == 0 {
            return Err(SketchError::InvalidParameter {
                param: if depth == 0 { "depth" } else { "width" }.to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            depth,
            width,
            table: vec![0.0; depth * width],
        })
    }

    /// Bucket and sign for coordinate `coord` in row `r`.
    #[inline]
    fn loc(&self, coord: u64, r: usize) -> (usize, f64) {
        let h = xxhash(&coord.to_le_bytes(), r as u64);
        let bucket = (h % self.width as u64) as usize;
        // An independent bit chooses the sign, decorrelating it from the bucket choice.
        let sign = if (xxhash(&coord.to_le_bytes(), r as u64 + 0x1_0000) & 1) == 0 {
            1.0
        } else {
            -1.0
        };
        (r * self.width + bucket, sign)
    }

    /// Adds `value` to coordinate `coord`.
    pub fn add(&mut self, coord: u64, value: f64) {
        for r in 0..self.depth {
            let (idx, sign) = self.loc(coord, r);
            self.table[idx] += sign * value;
        }
    }

    /// Adds a whole dense gradient (coordinate `i` ↦ `gradient[i]`), skipping exact zeros.
    pub fn accumulate(&mut self, gradient: &[f64]) {
        for (i, &g) in gradient.iter().enumerate() {
            if g != 0.0 {
                self.add(i as u64, g);
            }
        }
    }

    /// Unbiased estimate of coordinate `coord` (median of the signed bucket readings).
    pub fn estimate(&self, coord: u64) -> f64 {
        let mut readings: Vec<f64> = (0..self.depth)
            .map(|r| {
                let (idx, sign) = self.loc(coord, r);
                sign * self.table[idx]
            })
            .collect();
        readings.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = readings.len();
        if n % 2 == 1 {
            readings[n / 2]
        } else {
            0.5 * (readings[n / 2 - 1] + readings[n / 2])
        }
    }

    /// Recovers the `k` coordinates of largest magnitude over the index range `0..dimension`,
    /// sorted by `|estimate|` descending.
    pub fn top_k(&self, dimension: usize, k: usize) -> Vec<(usize, f64)> {
        let mut all: Vec<(usize, f64)> = (0..dimension)
            .map(|i| (i, self.estimate(i as u64)))
            .collect();
        all.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap());
        all.truncate(k);
        all
    }

    /// Reconstructs an approximate dense gradient over `0..dimension`.
    pub fn unsketch(&self, dimension: usize) -> Vec<f64> {
        (0..dimension).map(|i| self.estimate(i as u64)).collect()
    }

    /// Number of hash rows.
    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Number of buckets per row.
    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }

    /// Adds another worker's sketch into this one (linear aggregation of gradients).
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the dimensions differ.
    pub fn merge(&mut self, other: &Self) -> Result<()> {
        if self.depth != other.depth || self.width != other.width {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "shape mismatch: {}x{} vs {}x{}",
                    self.depth, self.width, other.depth, other.width
                ),
            });
        }
        for (a, b) in self.table.iter_mut().zip(&other.table) {
            *a += *b;
        }
        Ok(())
    }

    /// Scales every bucket by `factor` (e.g. a learning rate or momentum decay in sketch space).
    pub fn scale(&mut self, factor: f64) {
        for v in &mut self.table {
            *v *= factor;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(GradientSketch::new(0, 16).is_err());
        assert!(GradientSketch::new(4, 0).is_err());
        assert!(GradientSketch::new(4, 16).is_ok());
    }

    #[test]
    fn recovers_top_k_heavy_coordinates() {
        let dim = 20_000;
        let mut grad = vec![0.0f64; dim];
        grad[100] = 12.0;
        grad[5000] = -9.0;
        grad[19_999] = 7.0;
        let mut sk = GradientSketch::new(5, 8192).unwrap();
        sk.accumulate(&grad);
        let top: Vec<usize> = sk.top_k(dim, 3).iter().map(|&(i, _)| i).collect();
        assert!(top.contains(&100));
        assert!(top.contains(&5000));
        assert!(top.contains(&19_999));
    }

    #[test]
    fn estimate_is_close_for_heavy_coords() {
        let mut sk = GradientSketch::new(7, 4096).unwrap();
        sk.add(77, 50.0);
        for i in 0..2000u64 {
            sk.add(1000 + i, 0.1); // small noise coords
        }
        let est = sk.estimate(77);
        assert!((est - 50.0).abs() < 5.0, "estimate {est}");
    }

    #[test]
    fn sketches_sum_across_workers() {
        // Two workers' gradients on the same coordinate must add after merge — the property that
        // makes FetchSGD aggregation just sketch addition.
        let mut w1 = GradientSketch::new(5, 2048).unwrap();
        let mut w2 = GradientSketch::new(5, 2048).unwrap();
        w1.add(500, 4.0);
        w2.add(500, 6.0);
        w1.merge(&w2).unwrap();
        let est = w1.estimate(500);
        assert!((est - 10.0).abs() < 1.0, "summed estimate {est}");
    }

    #[test]
    fn scale_applies_linearly() {
        let mut sk = GradientSketch::new(5, 1024).unwrap();
        sk.add(3, 8.0);
        sk.scale(0.5);
        assert!((sk.estimate(3) - 4.0).abs() < 1.0);
    }

    #[test]
    fn merge_mismatch_errors() {
        let mut a = GradientSketch::new(5, 1024).unwrap();
        let b = GradientSketch::new(5, 2048).unwrap();
        assert!(a.merge(&b).is_err());
    }
}
