//! DS-FD — Frequent Directions over a sliding window of rows.
//!
//! [`FrequentDirections`](crate::matrix::FrequentDirections) summarizes *all* rows ever seen.
//! Streaming applications often want the covariance of only the **last `W` rows** — a sliding
//! window. DS-FD (the sliding-window matrix-sketching line, e.g. Wei et al., "Matrix Sketching Over
//! Sliding Windows", SIGMOD 2016) achieves this by partitioning the window into `num_blocks`
//! consecutive blocks, keeping one Frequent Directions sketch per block in a ring buffer. Because the
//! covariance `AᵀA` is *additive* over a partition of the rows, the window covariance is recovered by
//! **summing the per-block sketch covariances** — and the FD error of each block adds up to the same
//! `‖A_window‖²_F / ℓ` bound. As the window slides, the oldest block is reset, dropping its rows.
//!
//! The window is count-based: it holds `num_blocks · rows_per_block` rows (the current partially-filled
//! block plus the full recent blocks).

use crate::common::{Result, SketchError};
use crate::matrix::FrequentDirections;

/// A sliding-window Frequent Directions sketch over `d`-dimensional rows.
///
/// # Example
/// ```
/// use sketch_oxide::matrix::SlidingFrequentDirections;
///
/// // Window of 5 blocks × 200 rows = 1000 rows, retaining 4 directions.
/// let mut sw = SlidingFrequentDirections::new(4, 3, 200, 5).unwrap();
/// // Older rows along (1,0,0); the most recent window along (0,1,0).
/// for _ in 0..2000 { sw.append(&[1.0, 0.0, 0.0]); }
/// for _ in 0..1000 { sw.append(&[0.0, 1.0, 0.0]); }
///
/// // The window now sees only the recent (0,1,0) rows.
/// let cov = sw.windowed_covariance();
/// assert!(cov[1][1] > 0.0 && cov[0][0] < 1e-6 * cov[1][1], "stale direction lingered");
/// ```
#[derive(Debug, Clone)]
pub struct SlidingFrequentDirections {
    d: usize,
    rows_per_block: usize,
    blocks: Vec<FrequentDirections>,
    current: usize,
    rows_in_current: usize,
}

impl SlidingFrequentDirections {
    /// Creates a sliding-window sketch retaining `ell` directions over `d`-dimensional rows, with a
    /// window of `num_blocks` blocks of `rows_per_block` rows each.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `ell`, `d`, `rows_per_block`, or `num_blocks` is 0.
    pub fn new(ell: usize, d: usize, rows_per_block: usize, num_blocks: usize) -> Result<Self> {
        if rows_per_block == 0 || num_blocks == 0 {
            return Err(SketchError::InvalidParameter {
                param: "rows_per_block/num_blocks".to_string(),
                value: format!("{rows_per_block}/{num_blocks}"),
                constraint: "both must be > 0".to_string(),
            });
        }
        let blocks = (0..num_blocks)
            .map(|_| FrequentDirections::new(ell, d))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            d,
            rows_per_block,
            blocks,
            current: 0,
            rows_in_current: 0,
        })
    }

    /// Appends a row to the window's newest block, advancing (and recycling the oldest block) when
    /// the current block fills.
    ///
    /// # Panics
    /// Panics if `row.len() != d`.
    pub fn append(&mut self, row: &[f64]) {
        assert_eq!(row.len(), self.d, "row dimension mismatch");
        self.blocks[self.current].append(row);
        self.rows_in_current += 1;
        if self.rows_in_current == self.rows_per_block {
            // Advance to the next ring slot (the oldest block) and reset it for the new block.
            self.current = (self.current + 1) % self.blocks.len();
            self.blocks[self.current] =
                FrequentDirections::new(self.ell(), self.d).expect("ell and d already validated");
            self.rows_in_current = 0;
        }
    }

    /// The approximate covariance `BᵀB` of the rows currently in the window (sum of the per-block
    /// sketch covariances), a `d × d` matrix.
    pub fn windowed_covariance(&self) -> Vec<Vec<f64>> {
        let mut cov = vec![vec![0.0; self.d]; self.d];
        for block in &self.blocks {
            let bc = block.covariance();
            for i in 0..self.d {
                for (cov_ij, bc_ij) in cov[i].iter_mut().zip(&bc[i]) {
                    *cov_ij += *bc_ij;
                }
            }
        }
        cov
    }

    /// Retained directions `ℓ`.
    #[inline]
    pub fn ell(&self) -> usize {
        self.blocks[0].ell()
    }

    /// Row dimension `d`.
    #[inline]
    pub fn dim(&self) -> usize {
        self.d
    }

    /// Window size in rows (`num_blocks · rows_per_block`).
    #[inline]
    pub fn window(&self) -> usize {
        self.blocks.len() * self.rows_per_block
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spectral_norm_sym(m: &[Vec<f64>]) -> f64 {
        // Reuse FrequentDirections' Jacobi via a 1-row trick is awkward; do a small power iteration.
        let n = m.len();
        let mut v = vec![1.0 / (n as f64).sqrt(); n];
        let mut lambda = 0.0;
        for _ in 0..200 {
            let mut mv = vec![0.0; n];
            for i in 0..n {
                for j in 0..n {
                    mv[i] += m[i][j] * v[j];
                }
            }
            let norm = mv.iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm < 1e-300 {
                return 0.0;
            }
            for (vi, mvi) in v.iter_mut().zip(&mv) {
                *vi = mvi / norm;
            }
            lambda = norm;
        }
        lambda
    }

    #[test]
    fn rejects_bad_params() {
        assert!(SlidingFrequentDirections::new(0, 4, 100, 5).is_err());
        assert!(SlidingFrequentDirections::new(4, 0, 100, 5).is_err());
        assert!(SlidingFrequentDirections::new(4, 4, 0, 5).is_err());
        assert!(SlidingFrequentDirections::new(4, 4, 100, 0).is_err());
        assert!(SlidingFrequentDirections::new(4, 4, 100, 5).is_ok());
    }

    #[test]
    fn forgets_stale_directions() {
        let mut sw = SlidingFrequentDirections::new(4, 3, 200, 5).unwrap();
        for _ in 0..2000 {
            sw.append(&[1.0, 0.0, 0.0]); // old direction
        }
        for _ in 0..1000 {
            sw.append(&[0.0, 1.0, 0.0]); // recent direction (exactly one window)
        }
        let cov = sw.windowed_covariance();
        assert!(cov[1][1] > 0.0);
        assert!(
            cov[0][0] < 1e-6 * cov[1][1],
            "stale x-energy {} lingered",
            cov[0][0]
        );
    }

    #[test]
    fn windowed_covariance_tracks_recent_rows() {
        // Feed exactly one window of rows along (1,2,0); covariance should reflect that structure.
        let ell = 4;
        let mut sw = SlidingFrequentDirections::new(ell, 3, 250, 4).unwrap();
        for i in 0..1000 {
            let s = (i % 11) as f64 + 1.0;
            sw.append(&[s, 2.0 * s, 0.0]);
        }
        let cov = sw.windowed_covariance();
        assert!(cov[0][0] > 0.0);
        assert!(cov[2][2] < 1e-6 * cov[0][0], "unused dim leaked");
        assert!((cov[1][1] - 4.0 * cov[0][0]).abs() < 1e-6 * cov[0][0]);
        assert!((cov[0][1] - 2.0 * cov[0][0]).abs() < 1e-6 * cov[0][0]);
    }

    #[test]
    fn covariance_error_within_window_bound() {
        // Compare the windowed covariance against the exact covariance of the last `window` rows.
        let ell = 8;
        let d = 10;
        let rows_per_block = 500;
        let num_blocks = 4;
        let window = rows_per_block * num_blocks;
        let mut sw = SlidingFrequentDirections::new(ell, d, rows_per_block, num_blocks).unwrap();
        let total = window + 1500;
        let row_of = |i: u64| -> Vec<f64> {
            let a = ((i * 2_654_435_761) % 97) as f64 / 97.0;
            let b = ((i * 40_503) % 89) as f64 / 89.0;
            (0..d)
                .map(|j| {
                    a * ((j == 0) as i32 as f64)
                        + b * ((j == 1) as i32 as f64)
                        + 0.05 * (((i + j as u64) % 11) as f64 / 11.0)
                })
                .collect()
        };
        for i in 0..total as u64 {
            sw.append(&row_of(i));
        }
        // Exact covariance of the last `window` rows.
        let mut ata = vec![vec![0.0; d]; d];
        let mut fro2 = 0.0;
        for i in (total - window) as u64..total as u64 {
            let r = row_of(i);
            for x in 0..d {
                fro2 += r[x] * r[x];
                for y in 0..d {
                    ata[x][y] += r[x] * r[y];
                }
            }
        }
        let cov = sw.windowed_covariance();
        let mut diff = vec![vec![0.0; d]; d];
        for x in 0..d {
            for y in 0..d {
                diff[x][y] = ata[x][y] - cov[x][y];
            }
        }
        let err = spectral_norm_sym(&diff);
        // Each block contributes ‖block‖²_F/ℓ; summed ≤ ‖window‖²_F/ℓ (×2 slack for the fast FD).
        let bound = 2.0 * fro2 / ell as f64;
        assert!(err <= bound, "error {err} exceeds window bound {bound}");
    }
}
