//! Robust Frequent Directions (RFD) — Frequent Directions with a regularizer that halves the error
//! (Luo, Chen, Zhang, Li & Zhang, "Robust Frequent Directions with Application in Online Learning",
//! JMLR 2019).
//!
//! [`FrequentDirections`](crate::matrix::FrequentDirections) sketches a row-stream `A` into a small
//! `B` with `‖AᵀA − BᵀB‖₂ ≤ ‖A − A_k‖²_F / (ℓ − k)`, but `BᵀB` is low-rank and *underestimates* `AᵀA`
//! — a problem for second-order online learning, which needs a non-singular, well-conditioned Hessian
//! approximation. RFD fixes this for free: it accumulates **half of each shrink** into a scalar
//! regularizer `δ`, and approximates the covariance by `BᵀB + δI`. Splitting the shrinkage between the
//! sketch and the regularizer **halves** the bound:
//!
//! `‖AᵀA − (BᵀB + δI)‖₂ ≤ ‖A − A_k‖²_F / (2(ℓ − k))`,
//!
//! while `BᵀB + δI` is full-rank and invertible. The update is otherwise identical to FD (the same
//! per-row cost), so RFD strictly improves FD's accuracy at no extra cost.

use crate::common::{Result, SketchError};

/// A Robust Frequent Directions sketch: an FD sketch `B` plus a regularizer `δ`.
///
/// # Example
/// ```
/// use sketch_oxide::matrix::RobustFrequentDirections;
///
/// let mut rfd = RobustFrequentDirections::new(4, 5).unwrap();
/// for i in 0..200u64 {
///     let mut row = vec![0.0; 5];
///     row[(i % 5) as usize] = 1.0;
///     rfd.append(&row);
/// }
/// // The covariance estimate BᵀB + δI is full-rank (every diagonal ≥ δ > 0).
/// let cov = rfd.covariance();
/// assert!(rfd.regularizer() >= 0.0);
/// assert_eq!(cov.len(), 5);
/// ```
#[derive(Debug, Clone)]
pub struct RobustFrequentDirections {
    d: usize,
    ell: usize,
    /// `2·ell` rows of length `d`; the first `count` are occupied.
    rows: Vec<Vec<f64>>,
    count: usize,
    /// Accumulated regularizer `δ` (half the total shrinkage).
    alpha: f64,
}

impl RobustFrequentDirections {
    /// Creates an RFD sketch keeping `ell` rows over `d`-dimensional input.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `ell == 0` or `d == 0`.
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
            alpha: 0.0,
        })
    }

    /// Number of sketch rows (`ℓ`).
    #[inline]
    pub fn ell(&self) -> usize {
        self.ell
    }

    /// Input dimension `d`.
    #[inline]
    pub fn dim(&self) -> usize {
        self.d
    }

    /// The accumulated regularizer `δ`.
    #[inline]
    pub fn regularizer(&self) -> f64 {
        self.alpha
    }

    /// Appends one row of length `d`.
    ///
    /// # Panics
    /// If `row.len() != d`.
    pub fn append(&mut self, row: &[f64]) {
        assert_eq!(row.len(), self.d, "row dimension mismatch");
        if self.count == self.rows.len() {
            self.condense();
        }
        self.rows[self.count].copy_from_slice(row);
        self.count += 1;
    }

    /// FD shrink, splitting the shrinkage between the sketch and the regularizer `δ`.
    #[allow(clippy::needless_range_loop)]
    fn condense(&mut self) {
        let m = self.count;
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
        let (eigvals, eigvecs) = jacobi_eigen(gram);
        let mut order: Vec<usize> = (0..m).collect();
        order.sort_by(|&a, &b| eigvals[b].partial_cmp(&eigvals[a]).unwrap());
        // Shrink by the (ℓ+1)-th largest squared singular value.
        let delta = eigvals[order[self.ell.min(m - 1)]].max(0.0);

        let mut new_rows = vec![vec![0.0; self.d]; self.rows.len()];
        let mut filled = 0;
        for &oi in &order {
            let lambda = eigvals[oi].max(0.0);
            let s = lambda.sqrt();
            let s_new = (lambda - delta).max(0.0).sqrt();
            if s <= 1e-12 || s_new <= 1e-12 {
                continue;
            }
            let coef = s_new / s;
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
        // Absorb half the shrinkage into the regularizer (the RFD refinement).
        self.alpha += delta / 2.0;
        self.rows = new_rows;
        self.count = filled;
    }

    /// The `d × d` covariance estimate `BᵀB + δI` — full-rank and invertible.
    #[allow(clippy::needless_range_loop)]
    pub fn covariance(&self) -> Vec<Vec<f64>> {
        let mut cov = vec![vec![0.0; self.d]; self.d];
        for row in self.rows.iter().take(self.count) {
            for i in 0..self.d {
                for j in 0..self.d {
                    cov[i][j] += row[i] * row[j];
                }
            }
        }
        for i in 0..self.d {
            cov[i][i] += self.alpha;
        }
        cov
    }
}

/// Eigendecomposition of a symmetric matrix by cyclic Jacobi rotations. Column `k` of the returned
/// eigenvectors is the eigenvector for `eigenvalues[k]`.
#[allow(clippy::needless_range_loop)]
fn jacobi_eigen(mut a: Vec<Vec<f64>>) -> (Vec<f64>, Vec<Vec<f64>>) {
    let n = a.len();
    let mut v = vec![vec![0.0; n]; n];
    for (i, row) in v.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    if n <= 1 {
        let eigvals = (0..n).map(|i| a[i][i]).collect();
        return (eigvals, v);
    }
    for _sweep in 0..100 {
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
                let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
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
    use crate::matrix::FrequentDirections;

    struct Lcg(u64);
    impl Lcg {
        fn next_f64(&mut self) -> f64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((self.0 >> 11) as f64) / ((1u64 << 53) as f64) - 0.5
        }
    }

    fn spectral_norm(m: &[Vec<f64>]) -> f64 {
        let (eig, _) = jacobi_eigen(m.to_vec());
        eig.into_iter().fold(0.0, |acc, x| acc.max(x.abs()))
    }

    fn diff(a: &[Vec<f64>], b: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let d = a.len();
        (0..d)
            .map(|i| (0..d).map(|j| a[i][j] - b[i][j]).collect())
            .collect()
    }

    #[test]
    fn rejects_bad_params() {
        assert!(RobustFrequentDirections::new(0, 5).is_err());
        assert!(RobustFrequentDirections::new(4, 0).is_err());
        assert!(RobustFrequentDirections::new(4, 5).is_ok());
    }

    fn make_data(rng: &mut Lcg, n: usize, d: usize) -> Vec<Vec<f64>> {
        (0..n)
            .map(|_| (0..d).map(|_| rng.next_f64()).collect())
            .collect()
    }

    fn true_cov(data: &[Vec<f64>], d: usize) -> Vec<Vec<f64>> {
        let mut cov = vec![vec![0.0; d]; d];
        for row in data {
            for i in 0..d {
                for j in 0..d {
                    cov[i][j] += row[i] * row[j];
                }
            }
        }
        cov
    }

    #[test]
    fn error_bound_is_halved_vs_fd() {
        let d = 8;
        let ell = 4;
        let mut rng = Lcg(0x9E37_79B9_7F4A_7C15);
        let data = make_data(&mut rng, 500, d);
        let truth = true_cov(&data, d);
        let frob_sq: f64 = data.iter().flat_map(|r| r.iter()).map(|x| x * x).sum();

        let mut rfd = RobustFrequentDirections::new(ell, d).unwrap();
        let mut fd = FrequentDirections::new(ell, d).unwrap();
        for row in &data {
            rfd.append(row);
            fd.append(row);
        }
        let rfd_err = spectral_norm(&diff(&truth, &rfd.covariance()));
        let fd_err = spectral_norm(&diff(&truth, &fd.covariance()));

        // RFD's guaranteed bound is ‖A‖²_F / (2ℓ); FD's is ‖A‖²_F / ℓ.
        assert!(
            rfd_err <= frob_sq / (2.0 * ell as f64) + 1e-6,
            "RFD error {rfd_err} exceeds halved bound {}",
            frob_sq / (2.0 * ell as f64)
        );
        // And RFD is at least as accurate as plain FD.
        assert!(
            rfd_err <= fd_err + 1e-9,
            "RFD error {rfd_err} should not exceed FD error {fd_err}"
        );
    }

    #[test]
    fn covariance_is_full_rank_after_shrinks() {
        let d = 6;
        let ell = 3;
        let mut rng = Lcg(0xABCD_1234);
        let data = make_data(&mut rng, 300, d);
        let mut rfd = RobustFrequentDirections::new(ell, d).unwrap();
        for row in &data {
            rfd.append(row);
        }
        // Shrinks happened ⇒ δ > 0, so BᵀB + δI has every eigenvalue ≥ δ > 0 (invertible).
        assert!(rfd.regularizer() > 0.0, "regularizer should be positive");
        let cov = rfd.covariance();
        let (eig, _) = jacobi_eigen(cov);
        let min_eig = eig.into_iter().fold(f64::INFINITY, f64::min);
        assert!(
            min_eig >= rfd.regularizer() - 1e-9,
            "min eigenvalue {min_eig} below δ {}",
            rfd.regularizer()
        );
    }
}
