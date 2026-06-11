//! MV-Sketch — an invertible sketch for heavy hitters and heavy changers.
//!
//! MV-Sketch (Tang, Liu & Lee, "MV-Sketch: A Fast and Compact Invertible Sketch for Heavy
//! Flow Detection", INFOCOM 2019) makes heavy-hitter detection **invertible**: it can
//! enumerate the heavy keys directly from the sketch, with no separately stored key list.
//! Each cell runs a Boyer–Moore majority vote — tracking a candidate key, its vote count, and
//! the cell total — so the dominant flow in a cell is recoverable. Querying min-combines the
//! rows; inverting scans every cell's candidate.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use std::collections::HashSet;

/// One cell: total count, the majority-vote candidate, and its vote balance.
#[derive(Debug, Clone, Default)]
struct Cell {
    total: i64,
    votes: i64,
    candidate: Option<Vec<u8>>,
}

/// An MV-Sketch over `depth × width` majority-vote cells.
///
/// # Example
/// ```
/// use sketch_oxide::frequency::MvSketch;
///
/// let mut mv = MvSketch::new(4, 1024).unwrap();
/// for _ in 0..10_000 { mv.update(b"heavy"); }
/// for i in 0..5000u64 { mv.update(&i.to_le_bytes()); }
///
/// assert!(mv.estimate(b"heavy") >= 9_000);
/// // Invertible: recover the heavy key without a stored key list.
/// let hh = mv.heavy_hitters(5_000);
/// assert!(hh.iter().any(|(k, _)| k == b"heavy"));
/// ```
#[derive(Debug, Clone)]
pub struct MvSketch {
    depth: usize,
    width: usize,
    cells: Vec<Cell>,
}

impl MvSketch {
    /// Creates a `depth × width` MV-Sketch.
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
            cells: vec![Cell::default(); depth * width],
        })
    }

    #[inline]
    fn column(&self, key: &[u8], r: usize) -> usize {
        (xxhash(key, r as u64) % self.width as u64) as usize
    }

    /// Records one occurrence of `key`.
    pub fn update(&mut self, key: &[u8]) {
        for r in 0..self.depth {
            let idx = r * self.width + self.column(key, r);
            let cell = &mut self.cells[idx];
            cell.total += 1;
            // Boyer–Moore majority vote.
            if cell.votes == 0 {
                cell.candidate = Some(key.to_vec());
                cell.votes = 1;
            } else if cell.candidate.as_deref() == Some(key) {
                cell.votes += 1;
            } else {
                cell.votes -= 1;
            }
        }
    }

    /// Per-row estimate for `key`: `(total + votes)/2` if `key` is the cell's candidate, else
    /// the non-candidate upper bound `(total − votes)/2`.
    fn row_estimate(&self, key: &[u8], r: usize) -> i64 {
        let cell = &self.cells[r * self.width + self.column(key, r)];
        if cell.candidate.as_deref() == Some(key) {
            (cell.total + cell.votes) / 2
        } else {
            (cell.total - cell.votes) / 2
        }
    }

    /// Estimates the count of `key` (min over rows).
    pub fn estimate(&self, key: &[u8]) -> i64 {
        (0..self.depth)
            .map(|r| self.row_estimate(key, r))
            .min()
            .unwrap_or(0)
            .max(0)
    }

    /// Recovers the heavy hitters with estimated count at least `threshold`, as
    /// `(key, count)` sorted by count descending — **without** a stored key list (the
    /// invertibility property). Candidates are every cell's majority-vote candidate.
    pub fn heavy_hitters(&self, threshold: i64) -> Vec<(Vec<u8>, i64)> {
        let mut seen: HashSet<Vec<u8>> = HashSet::new();
        let mut out = Vec::new();
        for cell in &self.cells {
            if let Some(k) = &cell.candidate {
                if seen.insert(k.clone()) {
                    let est = self.estimate(k);
                    if est >= threshold {
                        out.push((k.clone(), est));
                    }
                }
            }
        }
        out.sort_by_key(|e| std::cmp::Reverse(e.1));
        out
    }

    /// Number of hash rows.
    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_dims() {
        assert!(MvSketch::new(0, 16).is_err());
        assert!(MvSketch::new(4, 0).is_err());
        assert!(MvSketch::new(4, 16).is_ok());
    }

    #[test]
    fn heavy_key_estimated() {
        let mut mv = MvSketch::new(4, 1024).unwrap();
        for _ in 0..10_000 {
            mv.update(b"heavy");
        }
        for i in 0..5000u64 {
            mv.update(&i.to_le_bytes());
        }
        let e = mv.estimate(b"heavy");
        assert!(e >= 9_000 && e <= 11_000, "estimate {e}");
    }

    #[test]
    fn invertible_recovery() {
        // The defining property: recover heavy keys without storing them.
        let mut mv = MvSketch::new(5, 2048).unwrap();
        for _ in 0..8000 {
            mv.update(b"big");
        }
        for _ in 0..4000 {
            mv.update(b"mid");
        }
        for i in 0..3000u64 {
            mv.update(&i.to_le_bytes());
        }
        let hh = mv.heavy_hitters(3000);
        assert!(hh.iter().any(|(k, _)| k == b"big"), "big not recovered");
        assert!(hh.iter().any(|(k, _)| k == b"mid"), "mid not recovered");
        assert_eq!(hh.first().map(|(k, _)| k.clone()), Some(b"big".to_vec()));
    }

    #[test]
    fn light_key_estimates_low() {
        let mut mv = MvSketch::new(5, 2048).unwrap();
        for _ in 0..50_000 {
            mv.update(b"dominant");
        }
        assert!(
            mv.estimate(b"absent") < 500,
            "absent {}",
            mv.estimate(b"absent")
        );
    }

    #[test]
    fn single_key_exact() {
        let mut mv = MvSketch::new(3, 1024).unwrap();
        for _ in 0..200 {
            mv.update(b"only");
        }
        // No contention => the candidate's count equals the total.
        assert_eq!(mv.estimate(b"only"), 200);
    }
}
