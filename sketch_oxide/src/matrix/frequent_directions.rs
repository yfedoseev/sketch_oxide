//! Frequent Directions — deterministic low-rank matrix sketching.
//!
//! Frequent Directions (Liberty, "Simple and Deterministic Matrix Sketching", KDD 2013; Ghashami,
//! Liberty, Phillips & Woodruff, SICOMP 2016) is the matrix analogue of the Misra–Gries frequent-
//! items sketch. It summarizes a stream of `d`-dimensional rows in a tiny `ℓ × d` sketch `B` whose
//! covariance approximates the data's: `‖AᵀA − BᵀB‖₂ ≤ ‖A‖²_F / ℓ`, **deterministically** (no
//! randomness, no failure probability). When the sketch fills, it takes the SVD of `B`, subtracts
//! the squared smallest retained singular value from every squared singular value (the "shrink"),
//! and zeroes the rows that hit zero — freeing space while throwing away only the least-significant
//! directions.
//!
//! The shrink needs the singular values/vectors of the small sketch, obtained here from the
//! eigendecomposition of `B Bᵀ` (an `ℓ′ × ℓ′` symmetric matrix) by the classic cyclic **Jacobi**
//! method — no external linear-algebra dependency.

use crate::common::{Result, SketchError};

/// A Frequent Directions sketch over `d`-dimensional rows, retaining `ell` directions.
///
/// # Example
/// ```
/// use sketch_oxide::matrix::FrequentDirections;
///
/// // Rows lie mostly along one direction; the sketch should capture it.
/// let mut fd = FrequentDirections::new(8, 3).unwrap();
/// for i in 0..1000 {
///     let s = (i % 7) as f64 + 1.0;
///     fd.append(&[s, 2.0 * s, 0.0]); // all along (1, 2, 0)
/// }
/// let cov = fd.covariance();
/// // The (0,0)/(1,1)/(0,1) block dominates; the third coordinate carries ~no energy.
/// assert!(cov[2][2] < 1e-6 * cov[0][0], "leak into unused dim");
/// ```
#[derive(Debug, Clone)]
pub struct FrequentDirections {
    d: usize,
    ell: usize,
    /// `2·ell` rows of length `d`; the first `count` are occupied.
    rows: Vec<Vec<f64>>,
    count: usize,
}

impl FrequentDirections {
    /// Creates a sketch keeping `ell` directions over `d`-dimensional rows.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `ell` or `d` is 0.
    pub fn new(ell: usize, d: usize) -> Result<Self> {
        if ell == 0 || d == 0 {
            return Err(SketchError::InvalidParameter {
                param: if ell == 0 { "ell" } else { "d" }.to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            d,
            ell,
            rows: vec![vec![0.0; d]; 2 * ell],
            count: 0,
        })
    }

    /// Appends a `d`-dimensional row to the stream.
    ///
    /// # Panics
    /// Panics if `row.len() != d`.
    pub fn append(&mut self, row: &[f64]) {
        assert_eq!(row.len(), self.d, "row dimension mismatch");
        if self.count == self.rows.len() {
            self.condense();
        }
        self.rows[self.count].copy_from_slice(row);
        self.count += 1;
    }

    /// SVD-shrink: drop the least-significant directions to free half the rows.
    #[allow(clippy::needless_range_loop)] // index-based matrix algebra reads clearest here
    fn condense(&mut self) {
        let m = self.count;
        // Gram matrix M = B Bᵀ (m × m, symmetric).
        let mut gram = vec![vec![0.0; m]; m];
        for i in 0..m {
            for j in i..m {
                let dot: f64 = self.rows[i]
                    .iter()
                    .zip(&self.rows[j])
                    .map(|(a, b)| a * b)
                    .sum();
                gram[i][j] = dot;
                gram[j][i] = dot;
            }
        }
        let (mut eigvals, eigvecs) = jacobi_eigen(gram);
        // Sort eigenpairs by eigenvalue descending.
        let mut order: Vec<usize> = (0..m).collect();
        order.sort_by(|&a, &b| eigvals[b].partial_cmp(&eigvals[a]).unwrap());
        // Shrink by the ℓ-th largest squared singular value (the median of 2ℓ).
        let delta = eigvals[order[self.ell.min(m - 1)]].max(0.0);

        let mut new_rows = vec![vec![0.0; self.d]; self.rows.len()];
        let mut filled = 0;
        for &oi in &order {
            let lambda = eigvals[oi].max(0.0);
            let s = lambda.sqrt();
            let s_new = (lambda - delta).max(0.0).sqrt();
            if s <= 1e-12 || s_new <= 1e-12 {
                continue; // this direction is dropped (row stays zero)
            }
            let coef = s_new / s;
            // new_row = (s_new/s) · Σ_k u[k] · B_old[k] = s_new · (k-th right singular vector).
            let dst = &mut new_rows[filled];
            for k in 0..m {
                let w = coef * eigvecs[k][oi];
                if w == 0.0 {
                    continue;
                }
                for (dd, &bv) in self.rows[k].iter().enumerate() {
                    dst[dd] += w * bv;
                }
            }
            filled += 1;
        }
        // Reset zeroed eigenvalues so a future read is clean (not strictly needed).
        eigvals.clear();
        self.rows = new_rows;
        self.count = filled;
    }

    /// The current `ℓ × d` sketch rows (only the occupied ones).
    pub fn sketch(&self) -> &[Vec<f64>] {
        &self.rows[..self.count]
    }

    /// The approximate covariance `BᵀB` (a `d × d` matrix) — close to the true `AᵀA`.
    #[allow(clippy::needless_range_loop)] // index-based matrix algebra reads clearest here
    pub fn covariance(&self) -> Vec<Vec<f64>> {
        let mut cov = vec![vec![0.0; self.d]; self.d];
        for row in &self.rows[..self.count] {
            for i in 0..self.d {
                for j in 0..self.d {
                    cov[i][j] += row[i] * row[j];
                }
            }
        }
        cov
    }

    /// Number of retained directions `ℓ`.
    #[inline]
    pub fn ell(&self) -> usize {
        self.ell
    }

    /// Row dimension `d`.
    #[inline]
    pub fn dim(&self) -> usize {
        self.d
    }
}

/// Eigendecomposition of a symmetric matrix by the cyclic Jacobi method. Returns `(eigenvalues,
/// eigenvectors)` where `eigenvectors[k][i]` is component `k` of eigenvector `i`.
#[allow(clippy::needless_range_loop)] // index-based Givens rotations read clearest here
fn jacobi_eigen(mut a: Vec<Vec<f64>>) -> (Vec<f64>, Vec<Vec<f64>>) {
    let n = a.len();
    // V starts as the identity; its columns accumulate the eigenvectors.
    let mut v = vec![vec![0.0; n]; n];
    for (i, row) in v.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    if n == 1 {
        return (vec![a[0][0]], v);
    }
    for _sweep in 0..100 {
        // Sum of squared off-diagonal entries; stop when negligible.
        let mut off = 0.0;
        for p in 0..n {
            for q in (p + 1)..n {
                off += a[p][q] * a[p][q];
            }
        }
        if off < 1e-24 {
            break;
        }
        for p in 0..n {
            for q in (p + 1)..n {
                if a[p][q].abs() < 1e-300 {
                    continue;
                }
                // Jacobi rotation angle zeroing a[p][q].
                let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                // Rotate rows/columns p and q of A.
                for k in 0..n {
                    let akp = a[k][p];
                    let akq = a[k][q];
                    a[k][p] = c * akp - s * akq;
                    a[k][q] = s * akp + c * akq;
                }
                for k in 0..n {
                    let apk = a[p][k];
                    let aqk = a[q][k];
                    a[p][k] = c * apk - s * aqk;
                    a[q][k] = s * apk + c * aqk;
                }
                // Accumulate the rotation into the eigenvector matrix.
                for row in v.iter_mut() {
                    let vp = row[p];
                    let vq = row[q];
                    row[p] = c * vp - s * vq;
                    row[q] = s * vp + c * vq;
                }
            }
        }
    }
    let eigvals = (0..n).map(|i| a[i][i]).collect();
    (eigvals, v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spectral_norm_sym(m: &[Vec<f64>]) -> f64 {
        let (eig, _) = jacobi_eigen(m.to_vec());
        eig.iter().fold(0.0, |acc, &e| acc.max(e.abs()))
    }

    #[test]
    fn rejects_bad_params() {
        assert!(FrequentDirections::new(0, 4).is_err());
        assert!(FrequentDirections::new(4, 0).is_err());
        assert!(FrequentDirections::new(4, 4).is_ok());
    }

    #[test]
    fn jacobi_diagonalizes_known_matrix() {
        // [[2,1],[1,2]] has eigenvalues 3 and 1.
        let (mut eig, _) = jacobi_eigen(vec![vec![2.0, 1.0], vec![1.0, 2.0]]);
        eig.sort_by(|a, b| b.partial_cmp(a).unwrap());
        assert!((eig[0] - 3.0).abs() < 1e-9);
        assert!((eig[1] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn covariance_error_within_bound() {
        // Random-ish low-rank-ish data; FD covariance must satisfy ‖AᵀA − BᵀB‖₂ ≤ 2‖A‖²_F / ℓ.
        let d = 12;
        let ell = 6;
        let mut fd = FrequentDirections::new(ell, d).unwrap();
        let mut ata = vec![vec![0.0; d]; d];
        let mut fro2 = 0.0;
        for i in 0..3000u64 {
            // A row dominated by two directions plus a little spread.
            let a = ((i * 2_654_435_761) % 97) as f64 / 97.0;
            let b = ((i * 40_503) % 89) as f64 / 89.0;
            let mut row = vec![0.0; d];
            for (j, r) in row.iter_mut().enumerate() {
                *r = a * ((j == 0) as i32 as f64)
                    + b * ((j == 1) as i32 as f64)
                    + 0.05 * (((i + j as u64) % 11) as f64 / 11.0);
            }
            for x in 0..d {
                fro2 += row[x] * row[x];
                for y in 0..d {
                    ata[x][y] += row[x] * row[y];
                }
            }
            fd.append(&row);
        }
        let btb = fd.covariance();
        let mut diff = vec![vec![0.0; d]; d];
        for x in 0..d {
            for y in 0..d {
                diff[x][y] = ata[x][y] - btb[x][y];
            }
        }
        let err = spectral_norm_sym(&diff);
        let bound = 2.0 * fro2 / ell as f64;
        assert!(err <= bound, "error {err} exceeds bound {bound}");
    }

    #[test]
    fn captures_dominant_direction() {
        // All rows along (1, 2, 0): the unused third coordinate should carry ~no covariance.
        let mut fd = FrequentDirections::new(4, 3).unwrap();
        for i in 0..2000 {
            let s = (i % 13) as f64 + 1.0;
            fd.append(&[s, 2.0 * s, 0.0]);
        }
        let cov = fd.covariance();
        assert!(cov[0][0] > 0.0);
        assert!(
            cov[2][2] < 1e-6 * cov[0][0],
            "leak {} into unused dim",
            cov[2][2]
        );
        // The (1,2,0) direction: cov[1][1] ≈ 4·cov[0][0], cov[0][1] ≈ 2·cov[0][0].
        assert!((cov[1][1] - 4.0 * cov[0][0]).abs() < 1e-6 * cov[0][0]);
        assert!((cov[0][1] - 2.0 * cov[0][0]).abs() < 1e-6 * cov[0][0]);
    }

    #[test]
    fn sketch_stays_small() {
        let ell = 5;
        let mut fd = FrequentDirections::new(ell, 8).unwrap();
        for i in 0..10_000u64 {
            let row: Vec<f64> = (0..8).map(|j| ((i + j) % 17) as f64).collect();
            fd.append(&row);
        }
        assert!(
            fd.sketch().len() <= 2 * ell,
            "sketch rows {}",
            fd.sketch().len()
        );
    }
}
