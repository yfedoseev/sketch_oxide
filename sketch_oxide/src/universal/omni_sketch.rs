//! OmniSketch — multi-dimensional frequency estimation with arbitrary predicates (Punter et al.,
//! VLDB 2024, Best Paper).
//!
//! A single OmniSketch answers `COUNT(*)` queries with equality predicates on *any subset* of
//! attributes, chosen at query time — e.g. "how many records have `srcIP = x` and `dstPort = y` and
//! `len > 40`?" — over a fast multi-attribute stream. It keeps one Count-Min-like `d × w` matrix per
//! searchable attribute, but each cell additionally stores a **bottom-`B` min-wise sample** of the
//! record-ids that hashed there (the `B` smallest values of a record-id hash `g`), plus the cell's
//! count.
//!
//! Inserting a record hashes each attribute value into its matrix (one cell per row) and offers the
//! record-id's hash to every touched cell's bottom-`B` sample. A query gathers the `p·d` cells for its
//! `p` predicates, **intersects** all their record-id samples (a record satisfying every predicate
//! hashes into all of them), takes the max cell count `n_max`, and estimates
//!
//! ```text
//! f̂(q) = (n_max / B) · |S∩|        (Eq. 4)
//! ```
//!
//! which corrects the sample's `B/n_max` sampling rate. Transcribed faithfully from Algorithm 1 and
//! the `S0∩`/`S1` estimator (Eq. 4) of the paper.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use std::collections::{BinaryHeap, HashSet};

/// Record-id hashing seed (the min-wise `g` function).
const RID_SEED: u64 = 0x0271_5E70_4944_0001;
/// Base seed for attribute/row hash functions.
const CELL_SEED_BASE: u64 = 0x0271_C311_0000_0001;

/// A single cell: a count plus a bottom-`B` min-wise sample of record-id hashes (a max-heap so the
/// largest — the one to evict — is at the top).
#[derive(Debug, Clone, Default)]
struct Cell {
    count: u64,
    sample: BinaryHeap<u64>,
}

impl Cell {
    /// Offers record-id hash `g` to the bottom-`b` sample.
    fn offer(&mut self, g: u64, b: usize) {
        if self.sample.len() < b {
            self.sample.push(g);
        } else if let Some(&max) = self.sample.peek() {
            if g < max {
                self.sample.pop();
                self.sample.push(g);
            }
        }
    }
}

/// A multi-dimensional sketch answering predicate `COUNT` queries over `num_attrs` attributes.
///
/// # Example
/// ```
/// use sketch_oxide::universal::OmniSketch;
///
/// // 2 attributes, 4 rows, 2048 columns, sample size 1024.
/// let mut omni = OmniSketch::new(2, 4, 2048, 1024).unwrap();
/// for i in 0..100_000u64 {
///     omni.insert(&[i % 10, i % 13], i); // attrs (i%10, i%13), record-id i
/// }
/// // Records with attr0 == 5 AND attr1 == 3.
/// let est = omni.query(&[(0, 5), (1, 3)]);
/// // True count is |{i : i%10==5 and i%13==3}| ≈ 769.
/// assert!((est - 769.0).abs() < 0.3 * 769.0, "estimate {est}");
/// ```
#[derive(Debug, Clone)]
pub struct OmniSketch {
    num_attrs: usize,
    d: usize,
    w: usize,
    b: usize,
    /// `sketches[attr][row][col]`.
    sketches: Vec<Vec<Vec<Cell>>>,
}

impl OmniSketch {
    /// Creates a sketch over `num_attrs` attributes, each a `d × w` matrix with a bottom-`b` record-id
    /// sample per cell.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if any of `num_attrs`, `d`, `w`, `b` is 0.
    pub fn new(num_attrs: usize, d: usize, w: usize, b: usize) -> Result<Self> {
        for (name, value) in [("num_attrs", num_attrs), ("d", d), ("w", w), ("b", b)] {
            if value == 0 {
                return Err(SketchError::InvalidParameter {
                    param: name.to_string(),
                    value: "0".to_string(),
                    constraint: "must be > 0".to_string(),
                });
            }
        }
        let sketches = vec![vec![vec![Cell::default(); w]; d]; num_attrs];
        Ok(Self {
            num_attrs,
            d,
            w,
            b,
            sketches,
        })
    }

    /// Column of attribute value `value` in attribute `attr`, row `row`.
    #[inline]
    fn col(&self, attr: usize, row: usize, value: u64) -> usize {
        let seed = CELL_SEED_BASE
            .wrapping_add(attr as u64 * self.d as u64)
            .wrapping_add(row as u64);
        (xxhash(&value.to_le_bytes(), seed) % self.w as u64) as usize
    }

    /// The min-wise hash of a record id.
    #[inline]
    fn rid_hash(rid: u64) -> u64 {
        xxhash(&rid.to_le_bytes(), RID_SEED)
    }

    /// Inserts a record: `attrs` are its per-attribute values (length `num_attrs`), `rid` its unique
    /// record id. Records with the wrong number of attributes are ignored.
    pub fn insert(&mut self, attrs: &[u64], rid: u64) {
        if attrs.len() != self.num_attrs {
            return;
        }
        let g = Self::rid_hash(rid);
        for (i, &val) in attrs.iter().enumerate() {
            for j in 0..self.d {
                let k = self.col(i, j, val);
                let cell = &mut self.sketches[i][j][k];
                cell.count += 1;
                cell.offer(g, self.b);
            }
        }
    }

    /// Estimates the number of records satisfying all the equality predicates `(attr, value)`:
    /// `f̂(q) = (n_max / B) · |S∩|`. An empty predicate list or any out-of-range attribute yields 0.
    pub fn query(&self, predicates: &[(usize, u64)]) -> f64 {
        if predicates.is_empty() || predicates.iter().any(|&(a, _)| a >= self.num_attrs) {
            return 0.0;
        }
        // Gather the p·d accessed cells.
        let mut cells: Vec<&Cell> = Vec::with_capacity(predicates.len() * self.d);
        let mut n_max = 0u64;
        for &(attr, val) in predicates {
            for j in 0..self.d {
                let k = self.col(attr, j, val);
                let cell = &self.sketches[attr][j][k];
                n_max = n_max.max(cell.count);
                cells.push(cell);
            }
        }
        // Intersect all sample sets, starting from the smallest for efficiency.
        let smallest = cells.iter().min_by_key(|c| c.sample.len()).unwrap();
        let mut inter: HashSet<u64> = smallest.sample.iter().copied().collect();
        for cell in &cells {
            if std::ptr::eq(*cell, *smallest) {
                continue;
            }
            let s: HashSet<u64> = cell.sample.iter().copied().collect();
            inter.retain(|g| s.contains(g));
            if inter.is_empty() {
                break;
            }
        }
        (n_max as f64 / self.b as f64) * inter.len() as f64
    }

    /// Number of searchable attributes.
    #[inline]
    pub fn num_attrs(&self) -> usize {
        self.num_attrs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exact count of records `i in 0..n` satisfying all `(attr_fn, value)` predicates, where the
    /// record's attribute values are `[i%10, i%13, i%7]`.
    fn exact(n: u64, preds: &[(usize, u64)]) -> u64 {
        let mods = [10u64, 13, 7];
        (0..n)
            .filter(|&i| preds.iter().all(|&(a, v)| i % mods[a] == v))
            .count() as u64
    }

    fn build(n: u64) -> OmniSketch {
        let mut omni = OmniSketch::new(3, 4, 4096, 2048).unwrap();
        for i in 0..n {
            omni.insert(&[i % 10, i % 13, i % 7], i);
        }
        omni
    }

    #[test]
    fn rejects_bad_params() {
        assert!(OmniSketch::new(0, 4, 1024, 256).is_err());
        assert!(OmniSketch::new(2, 0, 1024, 256).is_err());
        assert!(OmniSketch::new(2, 4, 0, 256).is_err());
        assert!(OmniSketch::new(2, 4, 1024, 0).is_err());
        assert!(OmniSketch::new(2, 4, 1024, 256).is_ok());
    }

    #[test]
    fn single_predicate() {
        let omni = build(100_000);
        let truth = exact(100_000, &[(0, 5)]) as f64; // ~10_000
        let est = omni.query(&[(0, 5)]);
        assert!(
            (est - truth).abs() < 0.2 * truth,
            "est {est}, truth {truth}"
        );
    }

    #[test]
    fn two_predicates() {
        let omni = build(100_000);
        let truth = exact(100_000, &[(0, 5), (1, 3)]) as f64; // ~769
        let est = omni.query(&[(0, 5), (1, 3)]);
        assert!(
            (est - truth).abs() < 0.3 * truth,
            "est {est}, truth {truth}"
        );
    }

    #[test]
    fn three_predicates() {
        let omni = build(200_000);
        let truth = exact(200_000, &[(0, 5), (1, 3), (2, 2)]) as f64;
        let est = omni.query(&[(0, 5), (1, 3), (2, 2)]);
        // Fewer matches ⇒ higher relative variance; allow generous slack.
        assert!(
            (est - truth).abs() < 0.4 * truth + 30.0,
            "est {est}, truth {truth}"
        );
    }

    #[test]
    fn impossible_combination_is_small() {
        let omni = build(100_000);
        // attr1 (i%13) is never 99, so no record satisfies this; the estimate is just collisions.
        let est = omni.query(&[(0, 5), (1, 99)]);
        assert!(est < 0.02 * 100_000.0, "spurious estimate {est}");
    }

    #[test]
    fn empty_and_out_of_range() {
        let omni = build(1000);
        assert_eq!(omni.query(&[]), 0.0);
        assert_eq!(omni.query(&[(9, 1)]), 0.0); // attr 9 does not exist
    }
}
