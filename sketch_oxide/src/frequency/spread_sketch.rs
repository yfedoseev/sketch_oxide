//! SpreadSketch — per-key distinct-count (spread) estimation and superspreader detection.
//!
//! Many network-telemetry questions are about *spread*, not volume: how many distinct
//! destinations does a source contact (port scans, DDoS bots, super-spreaders)? A plain
//! Count-Min counts packets, not distinct peers. SpreadSketch (Tang, Huang & Lee, "SpreadSketch:
//! Toward Invertible and Network-Wide Detection of Superspreaders", INFOCOM 2020) replaces
//! each Count-Min counter with a *distinct-count* estimator. Here each cell is a small
//! HyperLogLog, so a source's spread is the minimum over its rows' HLL estimates — the
//! distinct-count analogue of Count-Min's point query.

use crate::cardinality::HyperLogLog;
use crate::common::hash::xxhash;
use crate::common::{Result, Sketch, SketchError};

/// A SpreadSketch over `depth × width` HyperLogLog cells.
///
/// # Example
/// ```
/// use sketch_oxide::frequency::SpreadSketch;
///
/// let mut s = SpreadSketch::new(4, 256, 12).unwrap();
/// // Source "scanner" contacts 2000 distinct destinations.
/// for d in 0..2000u64 { s.update(b"scanner", &d.to_le_bytes()); }
/// // Source "normal" contacts 5.
/// for d in 0..5u64 { s.update(b"normal", &d.to_le_bytes()); }
///
/// assert!((s.spread(b"scanner") - 2000.0).abs() < 200.0);
/// assert!(s.spread(b"normal") < 10.0);
/// ```
#[derive(Debug, Clone)]
pub struct SpreadSketch {
    depth: usize,
    width: usize,
    cells: Vec<HyperLogLog>,
}

impl SpreadSketch {
    /// Creates a sketch with `depth` rows, `width` cells per row, and HLL `precision`
    /// (4–18) for each cell.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `depth` or `width` is 0; propagates HLL precision
    /// validation.
    pub fn new(depth: usize, width: usize, precision: u8) -> Result<Self> {
        if depth == 0 || width == 0 {
            return Err(SketchError::InvalidParameter {
                param: if depth == 0 { "depth" } else { "width" }.to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        let cells = (0..depth * width)
            .map(|_| HyperLogLog::new(precision))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            depth,
            width,
            cells,
        })
    }

    #[inline]
    fn column(&self, src: &[u8], r: usize) -> usize {
        (xxhash(src, r as u64) % self.width as u64) as usize
    }

    /// Records that source `src` contacted destination `dst`.
    pub fn update(&mut self, src: &[u8], dst: &[u8]) {
        for r in 0..self.depth {
            let c = self.column(src, r);
            self.cells[r * self.width + c].update(&dst);
        }
    }

    /// Estimates the spread (number of distinct destinations) of `src`: the minimum over rows
    /// of the cell HLL estimates (distinct-count analogue of a Count-Min point query).
    pub fn spread(&self, src: &[u8]) -> f64 {
        (0..self.depth)
            .map(|r| self.cells[r * self.width + self.column(src, r)].estimate())
            .fold(f64::INFINITY, f64::min)
    }

    /// Whether `src`'s estimated spread is at least `threshold` (a superspreader).
    pub fn is_superspreader(&self, src: &[u8], threshold: f64) -> bool {
        self.spread(src) >= threshold
    }

    /// Number of hash rows.
    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Cells per row.
    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_dims() {
        assert!(SpreadSketch::new(0, 10, 12).is_err());
        assert!(SpreadSketch::new(4, 0, 12).is_err());
        assert!(SpreadSketch::new(4, 10, 12).is_ok());
    }

    #[test]
    fn estimates_spread() {
        let mut s = SpreadSketch::new(4, 256, 12).unwrap();
        for d in 0..1000u64 {
            s.update(b"src", &d.to_le_bytes());
        }
        let sp = s.spread(b"src");
        assert!((sp - 1000.0).abs() < 0.1 * 1000.0, "spread {sp}");
    }

    #[test]
    fn repeated_destinations_count_once() {
        let mut s = SpreadSketch::new(4, 256, 12).unwrap();
        for _ in 0..1000 {
            s.update(b"src", b"same_dst"); // 1000 packets, 1 distinct destination
        }
        assert!(s.spread(b"src") < 2.0, "spread {}", s.spread(b"src"));
    }

    #[test]
    fn distinguishes_superspreaders() {
        let mut s = SpreadSketch::new(5, 512, 12).unwrap();
        for d in 0..5000u64 {
            s.update(b"scanner", &d.to_le_bytes());
        }
        for d in 0..20u64 {
            s.update(b"normal", &d.to_le_bytes());
        }
        assert!(s.is_superspreader(b"scanner", 1000.0));
        assert!(!s.is_superspreader(b"normal", 1000.0));
    }

    #[test]
    fn unseen_source_has_zero_spread() {
        let s = SpreadSketch::new(4, 256, 12).unwrap();
        assert_eq!(s.spread(b"never"), 0.0);
    }

    #[test]
    fn many_sources_independent() {
        let mut s = SpreadSketch::new(5, 1024, 12).unwrap();
        // 100 sources, source i contacts i*10 distinct destinations.
        for src in 1..=20u64 {
            for d in 0..(src * 10) {
                s.update(&src.to_le_bytes(), &d.to_le_bytes());
            }
        }
        // High-spread source estimated well above a low-spread one.
        assert!(s.spread(&20u64.to_le_bytes()) > s.spread(&2u64.to_le_bytes()));
        assert!((s.spread(&20u64.to_le_bytes()) - 200.0).abs() < 60.0);
    }
}
