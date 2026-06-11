//! On-Off Sketch — persistence estimation over periodic streams.
//!
//! Persistence asks a different question than frequency: not "how often does this item
//! appear?" but "in how many distinct time periods does it appear *at all*?". A scanner that
//! sends one probe every period is *persistent* but not *frequent*; a flash crowd is frequent
//! but not persistent. The On-Off Sketch (Zhang, Wang, Tong, et al., "On-Off Sketch: A Fast
//! and Dense Sketch for Finding Persistent Items in Data Streams", VLDB 2020) measures
//! persistence in small space: each cell keeps a persistence counter plus a single "on" flag
//! that is set the first time an item touches the cell within a period and reset at period
//! boundaries — so each item is counted at most once per period, no matter how often it
//! appears.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

/// One cell: a persistence count and a per-period "on" flag.
#[derive(Debug, Clone, Default)]
struct Cell {
    persistence: u64,
    on: bool,
}

/// An On-Off Sketch over `depth × width` cells.
///
/// # Example
/// ```
/// use sketch_oxide::frequency::OnOffSketch;
///
/// let mut s = OnOffSketch::new(4, 1024).unwrap();
/// // "scanner" appears once in each of 100 periods; "burst" appears 1000x but in 1 period.
/// for _ in 0..100 {
///     s.update(b"scanner");
///     s.new_period();
/// }
/// for _ in 0..1000 { s.update(b"burst"); }
///
/// assert!(s.persistence(b"scanner") >= 90);  // highly persistent
/// assert!(s.persistence(b"burst") <= 2);     // not persistent despite high frequency
/// ```
#[derive(Debug, Clone)]
pub struct OnOffSketch {
    depth: usize,
    width: usize,
    cells: Vec<Cell>,
}

impl OnOffSketch {
    /// Creates a `depth × width` On-Off Sketch.
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
    fn column(&self, item: &[u8], r: usize) -> usize {
        (xxhash(item, r as u64) % self.width as u64) as usize
    }

    /// Records that `item` appeared in the current period. Repeated calls within the same
    /// period do not increase its persistence (the "on" flag absorbs them).
    pub fn update(&mut self, item: &[u8]) {
        for r in 0..self.depth {
            let idx = r * self.width + self.column(item, r);
            let cell = &mut self.cells[idx];
            if !cell.on {
                cell.persistence += 1;
                cell.on = true;
            }
        }
    }

    /// Ends the current period: clears every cell's "on" flag so the next period counts fresh.
    pub fn new_period(&mut self) {
        for cell in &mut self.cells {
            cell.on = false;
        }
    }

    /// Estimates the persistence of `item` — the number of distinct periods it appeared in
    /// (min over rows, Count-Min style; one-sided overestimate from collisions).
    pub fn persistence(&self, item: &[u8]) -> u64 {
        (0..self.depth)
            .map(|r| self.cells[r * self.width + self.column(item, r)].persistence)
            .min()
            .unwrap_or(0)
    }

    /// Returns items (from a candidate set) whose persistence is at least `threshold`.
    pub fn persistent_items<'a>(
        &self,
        candidates: &[&'a [u8]],
        threshold: u64,
    ) -> Vec<(&'a [u8], u64)> {
        let mut out: Vec<(&[u8], u64)> = candidates
            .iter()
            .map(|&c| (c, self.persistence(c)))
            .filter(|(_, p)| *p >= threshold)
            .collect();
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
        assert!(OnOffSketch::new(0, 16).is_err());
        assert!(OnOffSketch::new(4, 0).is_err());
        assert!(OnOffSketch::new(4, 16).is_ok());
    }

    #[test]
    fn persistence_counts_distinct_periods() {
        let mut s = OnOffSketch::new(4, 1024).unwrap();
        for _ in 0..50 {
            s.update(b"x");
            s.new_period();
        }
        assert_eq!(s.persistence(b"x"), 50);
    }

    #[test]
    fn repeats_within_period_count_once() {
        let mut s = OnOffSketch::new(4, 1024).unwrap();
        for _ in 0..10 {
            for _ in 0..100 {
                s.update(b"x"); // 100 appearances in this period
            }
            s.new_period();
        }
        // 10 periods, many appearances each => persistence 10, not 1000.
        assert_eq!(s.persistence(b"x"), 10);
    }

    #[test]
    fn distinguishes_persistent_from_frequent() {
        // "scanner": once per period across 200 periods => highly persistent.
        // "burst": 5000 appearances all in one period => frequent but not persistent.
        let mut t = OnOffSketch::new(5, 2048).unwrap();
        for _ in 0..200 {
            t.update(b"scanner");
            t.new_period();
        }
        for _ in 0..5000 {
            t.update(b"burst"); // all in the current (final) period
        }
        assert!(
            t.persistence(b"scanner") >= 180,
            "scanner {}",
            t.persistence(b"scanner")
        );
        assert!(
            t.persistence(b"burst") <= 3,
            "burst {}",
            t.persistence(b"burst")
        );
    }

    #[test]
    fn unseen_item_zero() {
        let s = OnOffSketch::new(4, 1024).unwrap();
        assert_eq!(s.persistence(b"never"), 0);
    }

    #[test]
    fn persistent_items_filters_and_sorts() {
        let mut s = OnOffSketch::new(5, 2048).unwrap();
        for p in 0..100u64 {
            s.update(b"always");
            if p % 2 == 0 {
                s.update(b"half");
            }
            s.new_period();
        }
        let cands: Vec<&[u8]> = vec![b"always", b"half", b"never"];
        let hits = s.persistent_items(&cands, 40);
        assert_eq!(hits.first().map(|(k, _)| *k), Some(b"always".as_slice()));
        assert!(hits.iter().any(|(k, _)| *k == b"half"));
        assert!(!hits.iter().any(|(k, _)| *k == b"never"));
    }
}
