//! Moments Sketch — mergeable quantiles from power moments via maximum entropy (Gan, Ding, Tang,
//! Sethi, Bailis, Zaharia, SIGMOD 2018).
//!
//! Most quantile sketches store sample-like summaries; the **Moments Sketch** instead keeps only a
//! handful of **power sums** `Σ xⁱ` (`i = 0..k`) plus the running `min`/`max`. These are trivially
//! **mergeable** — adding two sketches is adding their power sums — which makes the sketch tiny,
//! constant-size, and ideal as a pre-aggregated column statistic. Quantiles are recovered at query
//! time by fitting the **maximum-entropy distribution** whose moments match the stored ones, then
//! inverting its CDF.
//!
//! Concretely, the data domain `[min, max]` is mapped to `[−1, 1]`; the maximum-entropy density there
//! has the form `q(m) ∝ exp(Σᵢ λᵢ mⁱ)`. The Lagrange multipliers `λ` are found by Newton's method on
//! the convex moment-matching objective (gradient = moment residual, Hessian = moment covariance), with
//! all integrals evaluated on a fixed grid. The fitted CDF is then inverted by bisection.
//!
//! This is the query layer that the descriptive [`statistics::MomentsSketch`](crate::statistics) (which
//! accumulates the moments) documented as a follow-up.

use crate::common::{Result, SketchError};

const GRID: usize = 512; // quadrature points on [-1, 1]
const MAX_NEWTON: usize = 60;

/// A Moments Sketch keeping power sums `Σ xⁱ` for `i = 0..=k` and reconstructing quantiles by maximum
/// entropy.
///
/// # Example
/// ```
/// use sketch_oxide::quantiles::MomentsSketch;
///
/// // Uniform data on [0, 1000): the reconstructed quantiles are near-linear.
/// let mut s = MomentsSketch::new(5).unwrap();
/// for x in 0..1000u32 { s.add(x as f64); }
/// let med = s.quantile(0.5).unwrap();
/// assert!((med - 500.0).abs() < 40.0, "median {med}");
/// let p90 = s.quantile(0.9).unwrap();
/// assert!((p90 - 900.0).abs() < 50.0, "p90 {p90}");
/// ```
#[derive(Debug, Clone)]
pub struct MomentsSketch {
    k: usize,
    power_sums: Vec<f64>, // power_sums[i] = Σ xⁱ, length k+1
    n: u64,
    min: f64,
    max: f64,
}

impl MomentsSketch {
    /// Creates a sketch matching power moments up to order `k` (`2..=10`). Larger `k` is more
    /// expressive but numerically harder; `5`–`7` is a good default range.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `k` is not in `2..=10`.
    pub fn new(k: usize) -> Result<Self> {
        if !(2..=10).contains(&k) {
            return Err(SketchError::InvalidParameter {
                param: "k".to_string(),
                value: k.to_string(),
                constraint: "must be in 2..=10".to_string(),
            });
        }
        Ok(Self {
            k,
            power_sums: vec![0.0; k + 1],
            n: 0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        })
    }

    /// Adds a value `x` to the sketch.
    pub fn add(&mut self, x: f64) {
        self.n += 1;
        if x < self.min {
            self.min = x;
        }
        if x > self.max {
            self.max = x;
        }
        let mut p = 1.0;
        for ps in self.power_sums.iter_mut() {
            *ps += p;
            p *= x;
        }
    }

    /// Number of values added.
    #[inline]
    pub fn count(&self) -> u64 {
        self.n
    }

    /// Whether the sketch is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Minimum observed value, if any.
    pub fn min(&self) -> Option<f64> {
        (self.n > 0).then_some(self.min)
    }

    /// Maximum observed value, if any.
    pub fn max(&self) -> Option<f64> {
        (self.n > 0).then_some(self.max)
    }

    /// Merges another sketch into this one (adds the power sums). Both must share the same `k`.
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the two sketches differ in `k`.
    pub fn merge(&mut self, other: &Self) -> Result<()> {
        if self.k != other.k {
            return Err(SketchError::IncompatibleSketches {
                reason: "MomentsSketch instances must share the same k".to_string(),
            });
        }
        for (a, b) in self.power_sums.iter_mut().zip(&other.power_sums) {
            *a += *b;
        }
        self.n += other.n;
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
        Ok(())
    }

    /// Mapped raw moments `μ_p = (1/n) Σ mⱼᵖ` for `p = 0..=k`, where `m = a·x + b` maps `[min,max]` to
    /// `[−1, 1]`.
    #[allow(clippy::needless_range_loop)] // `q` indexes the binomial table and power sums together
    fn mapped_moments(&self) -> Vec<f64> {
        let a = 2.0 / (self.max - self.min);
        let b = -(self.max + self.min) / (self.max - self.min);
        // Binomial coefficients up to k (Pascal's triangle).
        let k = self.k;
        let mut binom = vec![vec![0.0; k + 1]; k + 1];
        binom[0][0] = 1.0;
        for p in 1..=k {
            binom[p][0] = 1.0;
            for q in 1..=p {
                binom[p][q] = binom[p - 1][q - 1] + binom[p - 1][q];
            }
        }
        let mut mu = vec![0.0; k + 1];
        let inv_n = 1.0 / self.n as f64;
        for (p, mu_p) in mu.iter_mut().enumerate() {
            // m^p = Σ_q C(p,q) a^q b^{p-q} x^q  ⇒  Σ_j m_j^p = Σ_q C(p,q) a^q b^{p-q} (Σ x^q)
            let mut acc = 0.0;
            for q in 0..=p {
                acc += binom[p][q] * a.powi(q as i32) * b.powi((p - q) as i32) * self.power_sums[q];
            }
            *mu_p = acc * inv_n;
        }
        mu
    }

    /// Estimates the `φ`-quantile (`φ` in `[0, 1]`) by maximum-entropy reconstruction. Returns `None`
    /// if the sketch is empty.
    pub fn quantile(&self, phi: f64) -> Option<f64> {
        if self.n == 0 {
            return None;
        }
        let phi = phi.clamp(0.0, 1.0);
        if self.max <= self.min {
            return Some(self.min);
        }
        let mu = self.mapped_moments(); // mu[0..=k]; mu[0] == 1

        // Fit q(m) ∝ exp(Σ_{i=1}^k λ_i m^i) on [-1,1] matching moments mu[1..=k].
        let lambda = self.fit_lambda(&mu);

        // Build CDF on the grid, then invert by bisection.
        let (grid_m, cdf) = self.grid_cdf(&lambda);
        let target = phi;
        // Find m* with cdf(m*) = target via linear interpolation over the grid.
        let m_star = invert_cdf(&grid_m, &cdf, target);
        // Map back to data domain.
        let a = 2.0 / (self.max - self.min);
        let b = -(self.max + self.min) / (self.max - self.min);
        Some((m_star - b) / a)
    }

    /// Newton solve for the Lagrange multipliers `λ_1..λ_k` of the maximum-entropy density.
    fn fit_lambda(&self, mu: &[f64]) -> Vec<f64> {
        let k = self.k;
        let mut lambda = vec![0.0; k]; // lambda[i] is the multiplier for m^{i+1}
        let nodes = grid_nodes();

        let mut best = lambda.clone();
        let mut best_resid = f64::INFINITY;

        for _ in 0..MAX_NEWTON {
            // Moments of the current density via grid quadrature: e_p = E_q[m^p] for p = 0..=2k.
            let e = density_moments(&lambda, &nodes, 2 * k);
            // Gradient_i = E_q[m^{i+1}] - mu[i+1]   (i = 0..k-1)
            let mut grad = vec![0.0; k];
            let mut resid = 0.0;
            for i in 0..k {
                grad[i] = e[i + 1] - mu[i + 1];
                resid += grad[i] * grad[i];
            }
            let resid = resid.sqrt();
            if resid < best_resid {
                best_resid = resid;
                best.copy_from_slice(&lambda);
            }
            if resid < 1e-9 {
                break;
            }
            // Hessian_ij = E_q[m^{i+j+2}] - E_q[m^{i+1}] E_q[m^{j+1}]
            let mut hess = vec![vec![0.0; k]; k];
            for i in 0..k {
                for j in 0..k {
                    hess[i][j] = e[i + j + 2] - e[i + 1] * e[j + 1];
                }
            }
            let step = match solve_linear(&hess, &grad) {
                Some(s) => s,
                None => break, // singular Hessian; keep best so far
            };
            // Damped Newton: try full step, halving until the residual improves.
            let mut t = 1.0;
            let mut applied = false;
            for _ in 0..20 {
                let mut trial = lambda.clone();
                for i in 0..k {
                    trial[i] -= t * step[i];
                }
                let e_trial = density_moments(&trial, &nodes, k);
                let mut r = 0.0;
                for i in 0..k {
                    let d = e_trial[i + 1] - mu[i + 1];
                    r += d * d;
                }
                if r.sqrt() < resid {
                    lambda = trial;
                    applied = true;
                    break;
                }
                t *= 0.5;
            }
            if !applied {
                break; // no improving step; stop
            }
        }
        best
    }

    /// CDF of the fitted density sampled on the grid: returns `(m_nodes, cdf)` with `cdf` normalised to
    /// `[0, 1]`.
    fn grid_cdf(&self, lambda: &[f64]) -> (Vec<f64>, Vec<f64>) {
        let nodes = grid_nodes();
        let dm = 2.0 / GRID as f64;
        let mut dens = vec![0.0; GRID];
        for (idx, &m) in nodes.iter().enumerate() {
            dens[idx] = exp_poly(lambda, m);
        }
        let mut cdf = vec![0.0; GRID];
        let mut acc = 0.0;
        for idx in 0..GRID {
            acc += dens[idx] * dm;
            cdf[idx] = acc;
        }
        let total = acc.max(1e-300);
        for c in cdf.iter_mut() {
            *c /= total;
        }
        (nodes, cdf)
    }
}

/// Midpoints of `GRID` equal cells on `[-1, 1]`.
fn grid_nodes() -> Vec<f64> {
    let dm = 2.0 / GRID as f64;
    (0..GRID).map(|i| -1.0 + (i as f64 + 0.5) * dm).collect()
}

/// `exp(Σ_i λ_i m^{i+1})`.
fn exp_poly(lambda: &[f64], m: f64) -> f64 {
    let mut s = 0.0;
    let mut p = m;
    for &l in lambda {
        s += l * p;
        p *= m;
    }
    s.exp()
}

/// Normalised moments `E_q[m^p]` for `p = 0..=max_p` of `q(m) ∝ exp(Σ λ_i m^{i+1})` on the grid.
fn density_moments(lambda: &[f64], nodes: &[f64], max_p: usize) -> Vec<f64> {
    let dm = 2.0 / GRID as f64;
    let mut raw = vec![0.0; max_p + 1];
    let mut z = 0.0;
    for &m in nodes {
        let w = exp_poly(lambda, m) * dm;
        z += w;
        let mut mp = 1.0;
        for r in raw.iter_mut() {
            *r += w * mp;
            mp *= m;
        }
    }
    let z = z.max(1e-300);
    for r in raw.iter_mut() {
        *r /= z;
    }
    raw
}

/// Solves `H x = g` for symmetric `H` by Gaussian elimination with partial pivoting. `None` if
/// singular.
#[allow(clippy::needless_range_loop)] // `j` indexes the augmented matrix rows in the elimination
fn solve_linear(h: &[Vec<f64>], g: &[f64]) -> Option<Vec<f64>> {
    let n = g.len();
    let mut a = vec![vec![0.0; n + 1]; n];
    for i in 0..n {
        for j in 0..n {
            a[i][j] = h[i][j];
        }
        a[i][n] = g[i];
    }
    for col in 0..n {
        // Partial pivot.
        let mut piv = col;
        for r in (col + 1)..n {
            if a[r][col].abs() > a[piv][col].abs() {
                piv = r;
            }
        }
        if a[piv][col].abs() < 1e-12 {
            return None;
        }
        a.swap(col, piv);
        let d = a[col][col];
        for j in col..=n {
            a[col][j] /= d;
        }
        for r in 0..n {
            if r != col {
                let f = a[r][col];
                for j in col..=n {
                    a[r][j] -= f * a[col][j];
                }
            }
        }
    }
    Some((0..n).map(|i| a[i][n]).collect())
}

/// Inverts a monotone grid CDF for `target` by linear interpolation; returns the corresponding `m`.
fn invert_cdf(nodes: &[f64], cdf: &[f64], target: f64) -> f64 {
    if target <= cdf[0] {
        return nodes[0];
    }
    if target >= *cdf.last().unwrap() {
        return *nodes.last().unwrap();
    }
    // Binary search for the first index whose cdf >= target.
    let mut lo = 0usize;
    let mut hi = cdf.len() - 1;
    while lo < hi {
        let mid = (lo + hi) / 2;
        if cdf[mid] < target {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if lo == 0 {
        return nodes[0];
    }
    let (c0, c1) = (cdf[lo - 1], cdf[lo]);
    let (m0, m1) = (nodes[lo - 1], nodes[lo]);
    if (c1 - c0).abs() < 1e-300 {
        m1
    } else {
        m0 + (m1 - m0) * (target - c0) / (c1 - c0)
    }
}

// Capability-trait adoptions (fable5 doc 01 F3): delegate to inherent methods.
mod capability_impls {
    use super::*;
    use crate::common::capabilities::{QuantileQuery, Update};

    impl Update<f64> for MomentsSketch {
        fn update(&mut self, item: &f64) {
            self.add(*item);
        }
    }

    // `quantile(&self, ..) -> Option<f64>` is immutable, so `QuantileQuery` fits.
    impl QuantileQuery for MomentsSketch {
        fn quantile(&self, rank: f64) -> Option<f64> {
            MomentsSketch::quantile(self, rank)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(MomentsSketch::new(1).is_err());
        assert!(MomentsSketch::new(11).is_err());
        assert!(MomentsSketch::new(5).is_ok());
    }

    #[test]
    fn empty_sketch_has_no_quantiles() {
        let s = MomentsSketch::new(4).unwrap();
        assert!(s.is_empty());
        assert!(s.quantile(0.5).is_none());
        assert!(s.min().is_none());
    }

    #[test]
    fn uniform_quantiles_are_linear() {
        let mut s = MomentsSketch::new(5).unwrap();
        for x in 0..2000u32 {
            s.add(x as f64);
        }
        assert_eq!(s.count(), 2000);
        for &(phi, want) in &[(0.1, 200.0), (0.25, 500.0), (0.5, 1000.0), (0.9, 1800.0)] {
            let q = s.quantile(phi).unwrap();
            assert!(
                (q - want).abs() < 80.0,
                "quantile({phi}) = {q}, want ~{want}"
            );
        }
    }

    #[test]
    fn bell_shaped_median_near_center() {
        // Triangular-ish distribution via sum of two uniforms: symmetric, peaked at the center.
        let mut s = MomentsSketch::new(6).unwrap();
        let mut state = 12345u64;
        let mut rng = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 11) as f64 / (1u64 << 53) as f64
        };
        for _ in 0..20_000 {
            let x = (rng() + rng()) * 500.0; // in [0, 1000], peaked at 500
            s.add(x);
        }
        let med = s.quantile(0.5).unwrap();
        assert!((med - 500.0).abs() < 40.0, "median {med}");
    }

    #[test]
    fn merge_is_additive() {
        let mut a = MomentsSketch::new(5).unwrap();
        let mut b = MomentsSketch::new(5).unwrap();
        for x in 0..1000u32 {
            a.add(x as f64);
        }
        for x in 1000..2000u32 {
            b.add(x as f64);
        }
        a.merge(&b).unwrap();
        assert_eq!(a.count(), 2000);
        assert_eq!(a.min(), Some(0.0));
        assert_eq!(a.max(), Some(1999.0));
        let med = a.quantile(0.5).unwrap();
        assert!((med - 1000.0).abs() < 80.0, "merged median {med}");
    }

    #[test]
    fn merge_rejects_mismatched_k() {
        let mut a = MomentsSketch::new(4).unwrap();
        let b = MomentsSketch::new(5).unwrap();
        assert!(a.merge(&b).is_err());
    }

    #[test]
    fn constant_stream_returns_the_constant() {
        let mut s = MomentsSketch::new(4).unwrap();
        for _ in 0..100 {
            s.add(42.0);
        }
        assert_eq!(s.quantile(0.5), Some(42.0));
        assert_eq!(s.quantile(0.01), Some(42.0));
    }
}
