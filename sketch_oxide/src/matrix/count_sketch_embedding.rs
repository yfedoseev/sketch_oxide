//! CountSketch sparse subspace embedding (sketch-and-solve).
//!
//! The CountSketch transform (Clarkson & Woodruff, "Low Rank Approximation and Regression in Input
//! Sparsity Time", STOC 2013) is an `s × n` matrix `S` in which every column has a single `±1` in a
//! random row. Applying it to an `n × d` matrix `A` — adding each row of `A`, with a random sign, to
//! one row of the sketch — costs only `O(nnz(A))` and produces a tiny `s × d` sketch `SA` that is a
//! **subspace embedding**: `‖SAx‖ ≈ ‖Ax‖` for every `x` once `s = O(d²/ε²)`. That makes it a drop-in
//! accelerator for **sketch-and-solve** least squares — solve `min_x ‖(SA)x − (Sb)‖` over the small
//! sketched system instead of the full one.
//!
//! Concretely the embedding preserves the norm of any fixed vector in expectation
//! (`E[‖Sy‖²] = ‖y‖²`), with variance shrinking as `s` grows.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

/// A CountSketch subspace embedding mapping `n`-row inputs to `s`-row sketches.
///
/// # Example
/// ```
/// use sketch_oxide::matrix::CountSketchEmbedding;
///
/// // Overdetermined least squares: A x ≈ b with a known solution.
/// let x_true = [2.0, -1.0, 0.5];
/// let a: Vec<Vec<f64>> = (0..400)
///     .map(|i| vec![1.0, (i as f64) * 0.01, ((i * 7 % 13) as f64)])
///     .collect();
/// let b: Vec<f64> = a.iter().map(|row| row.iter().zip(&x_true).map(|(c, x)| c * x).sum()).collect();
///
/// let cs = CountSketchEmbedding::new(80, 42).unwrap();
/// let x_hat = cs.sketched_least_squares(&a, &b).unwrap();
/// for (h, t) in x_hat.iter().zip(&x_true) {
///     assert!((h - t).abs() < 0.1, "x_hat {h} vs {t}");
/// }
/// ```
#[derive(Debug, Clone)]
pub struct CountSketchEmbedding {
    rows: usize,
    seed: u64,
}

impl CountSketchEmbedding {
    /// Creates a CountSketch with `rows` sketch rows and a fixed `seed` (so the embedding is
    /// deterministic and reusable).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `rows` is 0.
    pub fn new(rows: usize, seed: u64) -> Result<Self> {
        if rows == 0 {
            return Err(SketchError::InvalidParameter {
                param: "rows".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self { rows, seed })
    }

    /// The sketch row and `±1` sign that input row `i` maps to.
    #[inline]
    fn place(&self, i: usize) -> (usize, f64) {
        let bytes = (i as u64).to_le_bytes();
        let row = (xxhash(&bytes, self.seed) % self.rows as u64) as usize;
        let sign = if xxhash(&bytes, self.seed ^ 0x5BD1_E995) & 1 == 0 {
            1.0
        } else {
            -1.0
        };
        (row, sign)
    }

    /// Sketches an `n`-vector to an `rows`-vector: `Sy`.
    pub fn apply_vec(&self, y: &[f64]) -> Vec<f64> {
        let mut out = vec![0.0; self.rows];
        for (i, &v) in y.iter().enumerate() {
            let (r, sign) = self.place(i);
            out[r] += sign * v;
        }
        out
    }

    /// Sketches an `n × d` matrix (row-major) to an `rows × d` matrix: `SA`.
    pub fn apply(&self, a: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let d = a.first().map_or(0, |row| row.len());
        let mut out = vec![vec![0.0; d]; self.rows];
        for (i, row) in a.iter().enumerate() {
            let (r, sign) = self.place(i);
            for (o, &v) in out[r].iter_mut().zip(row) {
                *o += sign * v;
            }
        }
        out
    }

    /// Sketch row count `s`.
    #[inline]
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Solves `min_x ‖A x − b‖²` approximately by sketching to `min_x ‖(SA)x − (Sb)‖²` and solving
    /// the small `d × d` normal equations.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `a` is empty or row/length shapes are inconsistent, or
    /// [`SketchError::ReconciliationError`] if the sketched normal equations are singular.
    pub fn sketched_least_squares(&self, a: &[Vec<f64>], b: &[f64]) -> Result<Vec<f64>> {
        if a.is_empty() || a.len() != b.len() {
            return Err(SketchError::InvalidParameter {
                param: "a/b".to_string(),
                value: format!(
                    "{}x{} vs {}",
                    a.len(),
                    a.first().map_or(0, |r| r.len()),
                    b.len()
                ),
                constraint: "a non-empty with one b entry per row".to_string(),
            });
        }
        let d = a[0].len();
        let sa = self.apply(a);
        let sb = self.apply_vec(b);

        // Normal equations: (SAᵀ SA) x = SAᵀ Sb, a d×d system.
        let mut ata = vec![vec![0.0; d]; d];
        let mut atb = vec![0.0; d];
        for (row, &rhs) in sa.iter().zip(&sb) {
            for j in 0..d {
                atb[j] += row[j] * rhs;
                for k in 0..d {
                    ata[j][k] += row[j] * row[k];
                }
            }
        }
        Self::solve(ata, atb)
    }

    /// Gaussian elimination with partial pivoting for a small dense system `M x = y`.
    fn solve(mut m: Vec<Vec<f64>>, mut y: Vec<f64>) -> Result<Vec<f64>> {
        let n = y.len();
        for col in 0..n {
            // Partial pivot.
            let mut piv = col;
            for r in (col + 1)..n {
                if m[r][col].abs() > m[piv][col].abs() {
                    piv = r;
                }
            }
            if m[piv][col].abs() < 1e-12 {
                return Err(SketchError::ReconciliationError {
                    reason: "sketched normal equations are singular".to_string(),
                });
            }
            m.swap(col, piv);
            y.swap(col, piv);
            // Eliminate below.
            for r in (col + 1)..n {
                let f = m[r][col] / m[col][col];
                let (pivot_row, lower) = m.split_at_mut(r);
                let dst = &mut lower[0];
                for (d, &s) in dst.iter_mut().zip(&pivot_row[col]).skip(col) {
                    *d -= f * s;
                }
                y[r] -= f * y[col];
            }
        }
        // Back-substitution.
        let mut x = vec![0.0; n];
        for i in (0..n).rev() {
            let mut s = y[i];
            for j in (i + 1)..n {
                s -= m[i][j] * x[j];
            }
            x[i] = s / m[i][i];
        }
        Ok(x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn norm_sq(v: &[f64]) -> f64 {
        v.iter().map(|x| x * x).sum()
    }

    #[test]
    fn rejects_zero_rows() {
        assert!(CountSketchEmbedding::new(0, 1).is_err());
        assert!(CountSketchEmbedding::new(100, 1).is_ok());
    }

    #[test]
    fn preserves_vector_norm_in_expectation() {
        // E[‖Sy‖²] = ‖y‖²; averaged over seeds the ratio is close to 1.
        let n = 4000;
        let y: Vec<f64> = (0..n).map(|i| ((i * 31 % 97) as f64) - 48.0).collect();
        let target = norm_sq(&y);
        let mut ratios = Vec::new();
        for seed in 0..16u64 {
            let cs = CountSketchEmbedding::new(600, seed).unwrap();
            ratios.push(norm_sq(&cs.apply_vec(&y)) / target);
        }
        let avg = ratios.iter().sum::<f64>() / ratios.len() as f64;
        assert!((avg - 1.0).abs() < 0.1, "mean norm ratio {avg}");
    }

    #[test]
    fn sketched_least_squares_recovers_solution() {
        let x_true = [3.0, -2.0, 1.5, 0.25];
        let a: Vec<Vec<f64>> = (0..600)
            .map(|i| {
                let f = i as f64;
                vec![1.0, f * 0.01, (i % 17) as f64, ((i * 5) % 11) as f64 - 5.0]
            })
            .collect();
        let b: Vec<f64> = a
            .iter()
            .map(|row| row.iter().zip(&x_true).map(|(c, x)| c * x).sum())
            .collect();
        let cs = CountSketchEmbedding::new(120, 7).unwrap();
        let x_hat = cs.sketched_least_squares(&a, &b).unwrap();
        for (h, t) in x_hat.iter().zip(&x_true) {
            assert!((h - t).abs() < 0.15, "x_hat {h} vs true {t}");
        }
    }

    #[test]
    fn apply_shapes() {
        let cs = CountSketchEmbedding::new(10, 1).unwrap();
        let a = vec![vec![1.0, 2.0, 3.0]; 100];
        let sa = cs.apply(&a);
        assert_eq!(sa.len(), 10);
        assert_eq!(sa[0].len(), 3);
    }

    #[test]
    fn rejects_bad_least_squares_input() {
        let cs = CountSketchEmbedding::new(10, 1).unwrap();
        assert!(cs.sketched_least_squares(&[], &[]).is_err());
        let a = vec![vec![1.0, 2.0]; 5];
        assert!(cs.sketched_least_squares(&a, &[1.0, 2.0]).is_err()); // mismatched b length
    }

    #[test]
    fn preserves_exact_when_no_collisions() {
        // With more rows than inputs and distinct buckets, ‖Sy‖² should be very close to ‖y‖².
        let y: Vec<f64> = (0..50).map(|i| (i as f64) - 25.0).collect();
        let cs = CountSketchEmbedding::new(5000, 3).unwrap();
        let ratio = norm_sq(&cs.apply_vec(&y)) / norm_sq(&y);
        assert!((ratio - 1.0).abs() < 0.05, "ratio {ratio}");
    }
}
