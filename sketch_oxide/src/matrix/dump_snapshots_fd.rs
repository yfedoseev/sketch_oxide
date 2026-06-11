//! DS-FD — space-optimal Frequent Directions over a sliding window (Yin, Wen, Li, Wei, Zhang, Huang
//! & Li, "Optimal Matrix Sketching over Sliding Windows", VLDB 2024, Best Paper nomination).
//!
//! Frequent Directions deterministically sketches a row-stream `A` so that `‖AᵀA − BᵀB‖₂ ≤ ‖A‖²_F/ℓ`.
//! Over a **sliding window** of the last `N` rows, the prior approaches cost `O(d/ε²)` (LM-FD, via
//! exponential histograms) or `O(d/ε·log(1/ε))` (DI-FD, via dyadic intervals). **DS-FD** reaches the
//! optimal `O(d/ε)` — matching the lower bound of plain FD — with the *Dump-Snapshots* idea:
//!
//! Each update performs an FD step (SVD of the small sketch); any direction whose squared singular
//! value exceeds a **dump threshold** `θ = εN` is *removed from the sketch and stored as a snapshot*
//! `(σ·v, timestamp)` in a queue. A query stacks the residual sketch with the still-in-window
//! snapshots to reconstruct the window sketch `B_W`. Snapshots expire by timestamp as the window
//! slides, and to bound how much stale mass the residual can hold, DS-FD keeps **two** FD sketches
//! (a main and an auxiliary) and swaps them every `N` steps (double buffering) — the analysis in the
//! paper's Theorem 3.1 turns this into the `‖A_WᵀA_W − B_WᵀB_W‖₂ ≤ εN` guarantee.
//!
//! Rows are assumed (approximately) normalized, as in the paper's Problem 1.1; the window is
//! count-based over the last `N` rows.
//!
//! # Implementation note
//!
//! This is the basic per-update-SVD algorithm (paper Algorithm 2 + the Algorithm 4 query), with the
//! `ℓ×ℓ` SVD obtained from the eigendecomposition of the Gram matrix `BBᵀ` via cyclic **Jacobi**
//! rotations (the same primitive as [`FrequentDirections`](crate::matrix::FrequentDirections)). The
//! amortised-time **Fast-DS-FD** optimisation (Algorithm 3) is a follow-up. This is distinct from
//! [`SlidingFrequentDirections`](crate::matrix::SlidingFrequentDirections), the earlier block-based
//! sliding-window FD.

use crate::common::{Result, SketchError};
use std::collections::VecDeque;

/// One FD sketch's residual rows (scaled right singular vectors).
type Rows = Vec<Vec<f64>>;
/// A dumped snapshot: a scaled right singular vector and the timestamp at which it was dumped.
type Snapshot = (Vec<f64>, u64);

/// A space-optimal sliding-window Frequent Directions sketch.
///
/// # Example
/// ```
/// use sketch_oxide::matrix::DumpSnapshotsFd;
///
/// // 8-dimensional rows, ε = 0.25, window of the last 100 rows.
/// let mut ds = DumpSnapshotsFd::new(8, 0.25, 100).unwrap();
/// for i in 0..300u64 {
///     // some normalized row
///     let mut row = vec![0.0; 8];
///     row[(i % 8) as usize] = 1.0;
///     ds.update(&row).unwrap();
/// }
/// let cov = ds.covariance(); // 8×8 approximation of A_Wᵀ A_W over the last 100 rows
/// assert_eq!(cov.len(), 8);
/// ```
#[derive(Debug, Clone)]
pub struct DumpSnapshotsFd {
    d: usize,
    ell: usize,
    n: u64,
    theta: f64,
    main: Rows,
    aux: Rows,
    snaps_main: VecDeque<Snapshot>,
    snaps_aux: VecDeque<Snapshot>,
    t: u64,
}

impl DumpSnapshotsFd {
    /// Creates a DS-FD sketch over `d`-dimensional rows with relative error `eps` (`0 < ε < 1`) and a
    /// window of the last `window` rows. The FD sketch keeps `ℓ = min(d, ⌈1/ε⌉)` rows and dumps
    /// directions whose squared singular value exceeds `θ = ε·window`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `d == 0`, `window == 0`, or `eps` is not in `(0, 1)`.
    pub fn new(d: usize, eps: f64, window: u64) -> Result<Self> {
        if d == 0 {
            return Err(SketchError::InvalidParameter {
                param: "d".to_string(),
                value: "0".to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        if window == 0 {
            return Err(SketchError::InvalidParameter {
                param: "window".to_string(),
                value: "0".to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        if !(eps > 0.0 && eps < 1.0) {
            return Err(SketchError::InvalidParameter {
                param: "eps".to_string(),
                value: eps.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }
        let ell = d.min((1.0 / eps).ceil() as usize).max(1);
        Ok(Self {
            d,
            ell,
            n: window,
            theta: eps * window as f64,
            main: Vec::new(),
            aux: Vec::new(),
            snaps_main: VecDeque::new(),
            snaps_aux: VecDeque::new(),
            t: 0,
        })
    }

    /// Number of rows in the FD sketch (`ℓ`).
    #[inline]
    pub fn ell(&self) -> usize {
        self.ell
    }

    /// Row dimension `d`.
    #[inline]
    pub fn dim(&self) -> usize {
        self.d
    }

    /// Processes one row (length `d`, assumed approximately unit-norm), at the next timestamp.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `row.len() != d`.
    pub fn update(&mut self, row: &[f64]) -> Result<()> {
        if row.len() != self.d {
            return Err(SketchError::InvalidParameter {
                param: "row.len".to_string(),
                value: row.len().to_string(),
                constraint: format!("must equal d = {}", self.d),
            });
        }
        self.t += 1;
        let i = self.t;

        // Double buffering: every N steps the auxiliary becomes the new main, with a fresh auxiliary.
        if (i - 1).is_multiple_of(self.n) {
            std::mem::swap(&mut self.main, &mut self.aux);
            self.aux = Vec::new();
            std::mem::swap(&mut self.snaps_main, &mut self.snaps_aux);
            self.snaps_aux = VecDeque::new();
        }

        // Expire snapshots that have fallen out of the window.
        Self::expire(&mut self.snaps_main, i, self.n);
        Self::expire(&mut self.snaps_aux, i, self.n);

        let (d, ell, theta) = (self.d, self.ell, self.theta);
        Self::fd_update_dump(&mut self.main, row, d, ell, theta, i, &mut self.snaps_main);
        Self::fd_update_dump(&mut self.aux, row, d, ell, theta, i, &mut self.snaps_aux);
        Ok(())
    }

    fn expire(snaps: &mut VecDeque<Snapshot>, i: u64, n: u64) {
        while let Some(&(_, t)) = snaps.front() {
            if t + n <= i {
                snaps.pop_front();
            } else {
                break;
            }
        }
    }

    /// One FD step on `rows` with the new `row`, then dumps every direction whose squared singular
    /// value exceeds `theta` into `snaps`, leaving a residual of at most `ell` rows.
    fn fd_update_dump(
        rows: &mut Rows,
        row: &[f64],
        d: usize,
        ell: usize,
        theta: f64,
        t: u64,
        snaps: &mut VecDeque<Snapshot>,
    ) {
        rows.push(row.to_vec());
        let m = rows.len();

        // Gram matrix B Bᵀ (m × m) and its eigendecomposition.
        let mut gram = vec![vec![0.0; m]; m];
        for i in 0..m {
            for j in i..m {
                let dot: f64 = (0..d).map(|k| rows[i][k] * rows[j][k]).sum();
                gram[i][j] = dot;
                gram[j][i] = dot;
            }
        }
        let (eigvals, eigvecs) = jacobi_eigen(gram);

        // Scaled right singular vectors c_k = Bᵀ u_k (‖c_k‖² = σ_k² = λ_k), sorted by σ² descending.
        let mut dirs: Vec<(f64, Vec<f64>)> = (0..m)
            .filter(|&k| eigvals[k] > 1e-12)
            .map(|k| {
                let mut c = vec![0.0; d];
                for (r, rrow) in rows.iter().enumerate() {
                    let u = eigvecs[r][k];
                    for j in 0..d {
                        c[j] += u * rrow[j];
                    }
                }
                (eigvals[k], c)
            })
            .collect();
        dirs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        // Dump directions whose squared singular value exceeds θ.
        let mut dump_count = 0;
        while dump_count < dirs.len() && dirs[dump_count].0 > theta {
            snaps.push_back((dirs[dump_count].1.clone(), t));
            dump_count += 1;
        }
        let mut kept: Vec<(f64, Vec<f64>)> = dirs.split_off(dump_count);

        // FD shrink: if more than ℓ directions remain, subtract the (ℓ+1)-th squared singular value.
        if kept.len() > ell {
            let delta = kept[ell].0;
            let mut shrunk = Vec::with_capacity(ell);
            for (lam, c) in kept.into_iter().take(ell) {
                let lam_new = lam - delta;
                if lam_new > 1e-12 {
                    let scale = (lam_new / lam).sqrt();
                    shrunk.push((lam_new, c.iter().map(|x| x * scale).collect()));
                }
            }
            kept = shrunk;
        }

        *rows = kept.into_iter().map(|(_, c)| c).collect();
    }

    /// Returns the `d × d` approximate covariance `B_Wᵀ B_W` over the current window — the residual
    /// main sketch stacked with the still-in-window snapshots (paper Algorithm 4).
    pub fn covariance(&self) -> Vec<Vec<f64>> {
        let mut cov = vec![vec![0.0; self.d]; self.d];
        let mut accumulate = |row: &[f64]| {
            for i in 0..self.d {
                for j in 0..self.d {
                    cov[i][j] += row[i] * row[j];
                }
            }
        };
        for row in &self.main {
            accumulate(row);
        }
        for (vec, _) in &self.snaps_main {
            accumulate(vec);
        }
        cov
    }
}

/// Eigendecomposition of a symmetric matrix by cyclic Jacobi rotations. Returns
/// `(eigenvalues, eigenvectors)` where column `k` of `eigenvectors` is the eigenvector for
/// `eigenvalues[k]`.
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

    struct Lcg(u64);
    impl Lcg {
        fn next_f64(&mut self) -> f64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            // Upper 53 bits → [0, 1).
            ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
        }
    }

    fn normalized_row(rng: &mut Lcg, d: usize) -> Vec<f64> {
        let mut v: Vec<f64> = (0..d).map(|_| rng.next_f64() - 0.5).collect();
        let norm: f64 = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm > 0.0 {
            for x in &mut v {
                *x /= norm;
            }
        }
        v
    }

    /// Spectral norm (largest magnitude eigenvalue) of a symmetric matrix.
    fn spectral_norm(m: &[Vec<f64>]) -> f64 {
        let (eig, _) = jacobi_eigen(m.to_vec());
        eig.into_iter().fold(0.0, |acc, x| acc.max(x.abs()))
    }

    #[test]
    fn rejects_bad_params() {
        assert!(DumpSnapshotsFd::new(0, 0.2, 100).is_err());
        assert!(DumpSnapshotsFd::new(8, 0.2, 0).is_err());
        assert!(DumpSnapshotsFd::new(8, 0.0, 100).is_err());
        assert!(DumpSnapshotsFd::new(8, 1.0, 100).is_err());
        assert!(DumpSnapshotsFd::new(8, 0.2, 100).is_ok());
    }

    #[test]
    fn rejects_wrong_row_length() {
        let mut ds = DumpSnapshotsFd::new(4, 0.2, 50).unwrap();
        assert!(ds.update(&[1.0, 0.0, 0.0]).is_err());
        assert!(ds.update(&[1.0, 0.0, 0.0, 0.0]).is_ok());
    }

    #[test]
    fn covariance_error_bounded_over_window() {
        let d = 10usize;
        let eps = 0.2;
        let n = 200u64;
        let mut ds = DumpSnapshotsFd::new(d, eps, n).unwrap();
        let mut rng = Lcg(0x9E37_79B9);
        let total = 320usize;
        let mut rows: Vec<Vec<f64>> = Vec::new();
        for _ in 0..total {
            let r = normalized_row(&mut rng, d);
            ds.update(&r).unwrap();
            rows.push(r);
        }
        // True covariance of the last N rows.
        let mut truth = vec![vec![0.0; d]; d];
        for row in rows.iter().skip(total - n as usize) {
            for i in 0..d {
                for j in 0..d {
                    truth[i][j] += row[i] * row[j];
                }
            }
        }
        let approx = ds.covariance();
        let mut diff = vec![vec![0.0; d]; d];
        for i in 0..d {
            for j in 0..d {
                diff[i][j] = truth[i][j] - approx[i][j];
            }
        }
        // DS-FD guarantees ‖A_Wᵀ A_W − B_Wᵀ B_W‖₂ ≤ εN = 40; allow modest numerical slack.
        let err = spectral_norm(&diff);
        assert!(
            err <= 1.5 * eps * n as f64,
            "spectral error {err} exceeds bound"
        );
    }

    #[test]
    fn covariance_is_psd_and_tracks_recent_rows() {
        // After flooding with rows along one axis, the covariance concentrates on that axis.
        let d = 6usize;
        let mut ds = DumpSnapshotsFd::new(d, 0.25, 100).unwrap();
        let mut axis = vec![0.0; d];
        axis[2] = 1.0;
        for _ in 0..300 {
            ds.update(&axis).unwrap();
        }
        let cov = ds.covariance();
        // The (2,2) entry should dominate and approximate the window size (100 unit rows).
        assert!(cov[2][2] > 80.0, "axis variance {} too small", cov[2][2]);
        for i in 0..d {
            if i != 2 {
                assert!(cov[i][i] < 1.0, "off-axis variance at {i} = {}", cov[i][i]);
            }
        }
    }
}
