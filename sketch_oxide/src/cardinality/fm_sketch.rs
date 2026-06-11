//! Flajolet–Martin (PCSA) — the original probabilistic distinct-counting sketch.
//!
//! The FM sketch with Probabilistic Counting and Stochastic Averaging (Flajolet & Martin,
//! "Probabilistic Counting Algorithms for Data Base Applications", JCSS 1985) is the ancestor
//! of LogLog / HyperLogLog. It keeps `m` bitmaps; each item lands in one bitmap (by part of
//! its hash) and sets the bit at position `trailing_zeros(rest of hash)`. After `n` distinct
//! items a bitmap is dense in its low bits and sparse above; the position of its lowest
//! **unset** bit estimates `log2(n/m)`. Averaging that fringe across the `m` bitmaps (the
//! stochastic-averaging trick) and applying the FM correction constant gives the cardinality.
//!
//! HyperLogLog later improved accuracy per bit by tracking the *maximum* rank instead of the
//! bitmap fringe; FM/PCSA is included for completeness and interoperability with legacy data.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

/// The Flajolet–Martin magic constant `φ ≈ 0.77351` correcting the fringe estimator.
const PHI: f64 = 0.775_351_22;

/// A Flajolet–Martin / PCSA cardinality sketch over `m` 64-bit bitmaps.
///
/// # Example
/// ```
/// use sketch_oxide::cardinality::FmSketch;
///
/// let mut fm = FmSketch::new(1024).unwrap();
/// for i in 0..50_000u64 { fm.add(&i.to_le_bytes()); }
/// let est = fm.estimate();
/// assert!((est - 50_000.0).abs() < 0.2 * 50_000.0, "estimate {est}");
/// ```
#[derive(Debug, Clone)]
pub struct FmSketch {
    m: usize,
    bitmaps: Vec<u64>,
}

impl FmSketch {
    /// Creates a sketch with `m` bitmaps (relative error `~0.78/√m`).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `m` is 0.
    pub fn new(m: usize) -> Result<Self> {
        if m == 0 {
            return Err(SketchError::InvalidParameter {
                param: "m".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            m,
            bitmaps: vec![0u64; m],
        })
    }

    /// Adds an item.
    pub fn add(&mut self, item: &[u8]) {
        let h = xxhash(item, 0);
        let idx = (h % self.m as u64) as usize;
        // Use an independent hash for the rank so it is uncorrelated with the bucket choice.
        let w = xxhash(item, 1);
        let rank = (w.trailing_zeros()).min(63);
        self.bitmaps[idx] |= 1u64 << rank;
    }

    /// Estimated number of distinct items.
    pub fn estimate(&self) -> f64 {
        // For each bitmap, R = position of the lowest zero bit = number of trailing ones.
        let sum_r: u64 = self
            .bitmaps
            .iter()
            .map(|&b| (!b).trailing_zeros() as u64)
            .sum();
        let mean_r = sum_r as f64 / self.m as f64;
        // FM/PCSA estimate: n ≈ (m / φ) · 2^mean_R.
        (self.m as f64 / PHI) * 2.0_f64.powf(mean_r)
    }

    /// Whether nothing has been added.
    pub fn is_empty(&self) -> bool {
        self.bitmaps.iter().all(|&b| b == 0)
    }

    /// Number of bitmaps.
    #[inline]
    pub fn num_bitmaps(&self) -> usize {
        self.m
    }

    /// Merges another sketch (bitwise OR of bitmaps); both must have the same `m`.
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the bitmap counts differ.
    pub fn merge(&mut self, other: &Self) -> Result<()> {
        if self.m != other.m {
            return Err(SketchError::IncompatibleSketches {
                reason: format!("m mismatch: {} vs {}", self.m, other.m),
            });
        }
        for (a, b) in self.bitmaps.iter_mut().zip(&other.bitmaps) {
            *a |= *b;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_m() {
        assert!(FmSketch::new(0).is_err());
        assert!(FmSketch::new(64).is_ok());
    }

    #[test]
    fn estimates_cardinality() {
        let mut fm = FmSketch::new(2048).unwrap();
        for i in 0..100_000u64 {
            fm.add(&i.to_le_bytes());
        }
        let est = fm.estimate();
        // FM has ~0.78/sqrt(m) error; allow generous slack.
        assert!((est - 100_000.0).abs() < 0.2 * 100_000.0, "estimate {est}");
    }

    #[test]
    fn duplicates_dont_count() {
        let mut fm = FmSketch::new(1024).unwrap();
        for _ in 0..10_000 {
            fm.add(b"same");
        }
        // One distinct item => estimate small relative to m.
        assert!(fm.estimate() < 4000.0);
    }

    #[test]
    fn merge_unions() {
        let mut a = FmSketch::new(2048).unwrap();
        let mut b = FmSketch::new(2048).unwrap();
        for i in 0..30_000u64 {
            a.add(&i.to_le_bytes());
        }
        for i in 15_000..45_000u64 {
            b.add(&i.to_le_bytes());
        }
        a.merge(&b).unwrap();
        // Union = 45000 distinct.
        assert!(
            (a.estimate() - 45_000.0).abs() < 0.25 * 45_000.0,
            "merged {}",
            a.estimate()
        );
    }

    #[test]
    fn merge_mismatch_errors() {
        let mut a = FmSketch::new(256).unwrap();
        let b = FmSketch::new(512).unwrap();
        assert!(a.merge(&b).is_err());
    }
}
