//! DPSW-Sketch — a differentially private frequency sketch over sliding windows (Wang, Wang & Chen,
//! KDD 2024).
//!
//! DPSW-Sketch answers "how often did item `e` occur in the last `w` items?" while guaranteeing
//! **event-level `ρ`-zero-concentrated differential privacy** (`ρ`-zCDP): the released structure is
//! almost unchanged by any single event. It combines three ingredients:
//!
//! - **Private Count-Min (PCMS).** A Count-Min sketch whose released counters carry **Gaussian
//!   noise** of variance `σ² = a/ρ_local` (`a` = depth; the `ℓ₂`-sensitivity of a CM counter vector is
//!   `√(2a)`, so the Gaussian mechanism with this `σ` is `ρ_local`-zCDP). Queries on the noisy
//!   counters are post-processing and incur no further loss.
//! - **Disjoint substreams.** The stream is cut into substreams of `B = ⌈w^β⌉` items. The window `W_t`
//!   overlaps `O(w^{1−β})` of them. Because substreams are disjoint, their PCMSs compose under the
//!   **parallel** rule, so the whole structure inherits the per-substream budget.
//! - **Smooth histograms.** Within each substream we keep geometrically-spaced checkpoints, each a
//!   *forward* PCMS over a prefix and a *backward* PCMS over the matching suffix, so any window
//!   boundary inside the substream is approximated within a `(1±α)` factor. The per-checkpoint budgets
//!   are `ρ₁ = ρ(2α−α²)` for the whole-substream PCMS and `ρ_j = ρα^{j−2}(1−α)³/2` for the rest; they
//!   sum to **exactly `ρ` per substream** (`ρ₁ + 2·Σ_{j≥2} ρ_j = ρ`), which is what makes the whole
//!   sketch `ρ`-zCDP (paper Lemma 4.1).
//!
//! A query sums one PCMS per overlapping substream: the whole-substream PCMS for substreams fully
//! inside the window, a backward checkpoint for the oldest (partially-expired) substream, and a
//! forward checkpoint for the current (still-filling) one. Noise is added once, when a PCMS's range
//! completes, using a caller-supplied RNG — pass a CSPRNG (e.g. [`secure_rng`]) in production.
//!
//! [`secure_rng`]: crate::privacy::mechanisms::secure_rng

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use crate::privacy::mechanisms::discrete_gaussian;
use rand::Rng;
use std::collections::VecDeque;

const CMS_SEED: u64 = 0xD95A_0000_0000_0001;
const ROW_STRIDE: u64 = 0x9E37_79B9_7F4A_7C15;

/// A bare Count-Min counter grid. Noise is added in place once its range is finalized.
#[derive(Debug, Clone)]
struct Cms {
    counters: Vec<i64>,
}

impl Cms {
    fn new(n: usize) -> Self {
        Self {
            counters: vec![0; n],
        }
    }
}

fn cms_col(item: &[u8], r: usize, width: usize) -> usize {
    let seed = CMS_SEED.wrapping_add((r as u64).wrapping_mul(ROW_STRIDE));
    (xxhash(item, seed) % width as u64) as usize
}

fn cms_add(cms: &mut Cms, item: &[u8], depth: usize, width: usize) {
    for r in 0..depth {
        cms.counters[r * width + cms_col(item, r, width)] += 1;
    }
}

fn cms_query(cms: &Cms, item: &[u8], depth: usize, width: usize) -> i64 {
    (0..depth)
        .map(|r| cms.counters[r * width + cms_col(item, r, width)])
        .min()
        .unwrap_or(0)
}

fn cms_finalize<R: Rng + ?Sized>(cms: &mut Cms, sigma: f64, rng: &mut R) -> Result<()> {
    for c in cms.counters.iter_mut() {
        *c += discrete_gaussian(rng, sigma)?;
    }
    Ok(())
}

/// One substream: a whole-substream PCMS plus forward (prefix) and backward (suffix) checkpoint PCMSs.
#[derive(Debug, Clone)]
struct Substream {
    /// Global 1-based index of the substream's first item.
    start: u64,
    /// Items added so far (`≤ B`).
    count: u64,
    whole: Cms,
    whole_final: bool,
    /// `forward[i]` covers positions `[0, lens[i]-1]` (a prefix of length `lens[i]`).
    forward: Vec<Cms>,
    forward_final: Vec<bool>,
    /// `backward[i]` covers positions `[B-lens[i], B-1]` (a suffix of length `lens[i]`).
    backward: Vec<Cms>,
    backward_final: Vec<bool>,
}

/// A DPSW-Sketch: `ρ`-zCDP frequency estimation over a sliding window of `w` items.
///
/// # Example
/// ```
/// use sketch_oxide::privacy::DpswSketch;
/// use sketch_oxide::privacy::mechanisms::secure_rng;
///
/// // window 50_000; ρ = 4 (zCDP); smooth-histogram α = 0.5; substream factor β = 0.5; CM 4×256.
/// let mut s = DpswSketch::new(50_000, 4.0, 0.5, 0.5, 4, 256).unwrap();
/// let mut rng = secure_rng();
/// for _ in 0..10_000 {
///     s.insert(b"heavy", &mut rng).unwrap();
/// }
/// // The private estimate stays close to the true window frequency.
/// let est = s.query(b"heavy");
/// assert!((est - 10_000).abs() < 1_000, "estimate {est}");
/// // The privacy budget actually spent per substream never exceeds ρ.
/// assert!(s.total_budget() <= 4.0 + 1e-9);
/// ```
#[derive(Debug, Clone)]
pub struct DpswSketch {
    w: u64,
    rho: f64,
    alpha: f64,
    b: u64,
    depth: usize,
    width: usize,
    /// Checkpoint lengths `lens[i]` (`< B`, geometrically decreasing), for the `j ≥ 2` checkpoints.
    lens: Vec<u64>,
    rho_whole: f64,
    rho_j: Vec<f64>,
    sigma_whole: f64,
    sigma_j: Vec<f64>,
    substreams: VecDeque<Substream>,
    t: u64,
}

impl DpswSketch {
    /// Creates a DPSW-Sketch.
    ///
    /// * `w` — sliding-window size in items (`≥ 1`).
    /// * `rho` — total `ρ`-zCDP budget (`> 0`); smaller is more private and noisier.
    /// * `alpha` — smooth-histogram factor in `(0, 1)`; smaller gives tighter window boundaries at
    ///   the cost of more checkpoints.
    /// * `beta` — substream-size factor in `(0, 1)`; the substream size is `B = ⌈w^β⌉`.
    /// * `depth`, `width` — Count-Min dimensions (`≥ 1`).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if any constraint above is violated.
    pub fn new(
        w: u64,
        rho: f64,
        alpha: f64,
        beta: f64,
        depth: usize,
        width: usize,
    ) -> Result<Self> {
        let err = |param: &str, value: String, constraint: &str| {
            Err(SketchError::InvalidParameter {
                param: param.to_string(),
                value,
                constraint: constraint.to_string(),
            })
        };
        if w < 1 {
            return err("w", w.to_string(), "must be >= 1");
        }
        if !(rho.is_finite() && rho > 0.0) {
            return err("rho", rho.to_string(), "must be a finite value > 0");
        }
        if !(alpha > 0.0 && alpha < 1.0) {
            return err("alpha", alpha.to_string(), "must be in (0, 1)");
        }
        if !(beta > 0.0 && beta < 1.0) {
            return err("beta", beta.to_string(), "must be in (0, 1)");
        }
        if depth < 1 {
            return err("depth", depth.to_string(), "must be >= 1");
        }
        if width < 1 {
            return err("width", width.to_string(), "must be >= 1");
        }

        let b = ((w as f64).powf(beta).ceil() as u64).max(1);

        // Geometric checkpoint lengths < B (smooth histogram, ratio 1-alpha).
        let mut lens = Vec::new();
        let mut cur = (b as f64 * (1.0 - alpha)).floor() as u64;
        while cur >= 1 {
            if lens.last() != Some(&cur) {
                lens.push(cur);
            }
            let next = (cur as f64 * (1.0 - alpha)).floor() as u64;
            if next >= cur {
                break;
            }
            cur = next;
        }

        // Budget split: ρ₁ for the whole substream, ρ_j = ρ·α^{j-2}·(1-α)³/2 for j ≥ 2 (i = j-2).
        let rho_whole = rho * (2.0 * alpha - alpha * alpha);
        let rho_j: Vec<f64> = (0..lens.len())
            .map(|i| rho * alpha.powi(i as i32) * (1.0 - alpha).powi(3) / 2.0)
            .collect();
        // σ = √(a/ρ_local), a = depth (ℓ₂-sensitivity √(2a) ⇒ σ² = 2a/(2ρ) = a/ρ).
        let sigma_whole = (depth as f64 / rho_whole).sqrt();
        let sigma_j: Vec<f64> = rho_j.iter().map(|&r| (depth as f64 / r).sqrt()).collect();

        Ok(Self {
            w,
            rho,
            alpha,
            b,
            depth,
            width,
            lens,
            rho_whole,
            rho_j,
            sigma_whole,
            sigma_j,
            substreams: VecDeque::new(),
            t: 0,
        })
    }

    /// Total privacy budget actually allocated within any one substream:
    /// `ρ₁ + 2·Σ_j ρ_j ≤ ρ`. (Equals `ρ` in the limit of infinitely many checkpoints.)
    pub fn total_budget(&self) -> f64 {
        self.rho_whole + 2.0 * self.rho_j.iter().sum::<f64>()
    }

    /// Substream size `B = ⌈w^β⌉`.
    #[inline]
    pub fn substream_size(&self) -> u64 {
        self.b
    }

    /// The total `ρ`-zCDP budget this sketch targets.
    #[inline]
    pub fn rho(&self) -> f64 {
        self.rho
    }

    /// The smooth-histogram factor `α`.
    #[inline]
    pub fn alpha(&self) -> f64 {
        self.alpha
    }

    /// Number of items processed.
    #[inline]
    pub fn len(&self) -> u64 {
        self.t
    }

    /// Whether nothing has been inserted.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.t == 0
    }

    /// Global index (1-based) of the first item still inside the window at the current time.
    fn window_left(&self) -> u64 {
        self.t.saturating_sub(self.w) + 1
    }

    /// Inserts one occurrence of `item`, drawing finalization noise from `rng` (pass a CSPRNG in
    /// production).
    ///
    /// # Errors
    /// Propagates a noise-sampling error from the Gaussian mechanism (only on invalid `σ`).
    pub fn insert<R: Rng + ?Sized>(&mut self, item: &[u8], rng: &mut R) -> Result<()> {
        self.t += 1;
        let (depth, width, b) = (self.depth, self.width, self.b);
        let lens = self.lens.clone();
        let sigma_j = self.sigma_j.clone();
        let sigma_whole = self.sigma_whole;
        let cells = depth * width;

        // Start a fresh substream if needed.
        if self.substreams.back().is_none_or(|s| s.count == b) {
            let nl = lens.len();
            self.substreams.push_back(Substream {
                start: self.t,
                count: 0,
                whole: Cms::new(cells),
                whole_final: false,
                forward: (0..nl).map(|_| Cms::new(cells)).collect(),
                forward_final: vec![false; nl],
                backward: (0..nl).map(|_| Cms::new(cells)).collect(),
                backward_final: vec![false; nl],
            });
        }

        let s = self.substreams.back_mut().unwrap();
        let p = s.count; // 0-based position of this item within the substream
        cms_add(&mut s.whole, item, depth, width);
        for (i, &len) in lens.iter().enumerate() {
            if p < len {
                cms_add(&mut s.forward[i], item, depth, width);
            }
            if p >= b - len {
                cms_add(&mut s.backward[i], item, depth, width);
            }
        }
        s.count += 1;

        // Finalize each forward checkpoint the moment its prefix is complete.
        for (i, &len) in lens.iter().enumerate() {
            if !s.forward_final[i] && s.count == len {
                cms_finalize(&mut s.forward[i], sigma_j[i], rng)?;
                s.forward_final[i] = true;
            }
        }
        // When the substream fills, finalize its whole and backward PCMSs.
        if s.count == b {
            cms_finalize(&mut s.whole, sigma_whole, rng)?;
            s.whole_final = true;
            for (i, &sig) in sigma_j.iter().enumerate() {
                cms_finalize(&mut s.backward[i], sig, rng)?;
                s.backward_final[i] = true;
            }
        }

        // Drop substreams whose every item has expired out of the window.
        let wl = self.window_left();
        while let Some(front) = self.substreams.front() {
            if front.count == b && front.start + b - 1 < wl {
                self.substreams.pop_front();
            } else {
                break;
            }
        }
        Ok(())
    }

    /// A backward (suffix) checkpoint covering at least `suffix_len` items, as tight as possible.
    fn query_suffix(&self, s: &Substream, item: &[u8], suffix_len: u64) -> i64 {
        let mut best_len = self.b;
        let mut chosen = if s.whole_final { Some(&s.whole) } else { None };
        for i in 0..self.lens.len() {
            if self.lens[i] >= suffix_len && self.lens[i] <= best_len && s.backward_final[i] {
                best_len = self.lens[i];
                chosen = Some(&s.backward[i]);
            }
        }
        chosen.map_or(0, |c| cms_query(c, item, self.depth, self.width))
    }

    /// A forward (prefix) checkpoint covering at most `prefix_len` items, as tight as possible.
    fn query_prefix(&self, s: &Substream, item: &[u8], prefix_len: u64) -> i64 {
        if s.count == self.b && s.whole_final {
            return cms_query(&s.whole, item, self.depth, self.width);
        }
        let mut best_len = 0;
        let mut chosen: Option<&Cms> = None;
        for i in 0..self.lens.len() {
            if self.lens[i] <= prefix_len && self.lens[i] >= best_len && s.forward_final[i] {
                best_len = self.lens[i];
                chosen = Some(&s.forward[i]);
            }
        }
        chosen.map_or(0, |c| cms_query(c, item, self.depth, self.width))
    }

    /// Estimated frequency of `item` within the current window (paper Algorithm 3). Post-processing
    /// of the noisy counters, so it preserves `ρ`-zCDP. May be slightly off from the smooth-histogram
    /// boundary approximation and the Gaussian noise.
    pub fn query(&self, item: &[u8]) -> i64 {
        if self.substreams.is_empty() {
            return 0;
        }
        let wl = self.window_left();
        let mut total: i64 = 0;
        for s in &self.substreams {
            let s_end = s.start + s.count - 1;
            if s_end < wl {
                continue; // fully expired
            }
            let est = if s.start < wl {
                // Oldest, partially-expired substream: suffix [wl, s_end].
                self.query_suffix(s, item, s_end - wl + 1)
            } else if s.count == self.b && s.whole_final {
                // Fully inside the window: the whole-substream PCMS.
                cms_query(&s.whole, item, self.depth, self.width)
            } else {
                // Current, still-filling substream: prefix [s.start, t].
                self.query_prefix(s, item, s.count)
            };
            total += est.max(0);
        }
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn rng() -> StdRng {
        StdRng::seed_from_u64(0xD957_1234_5678_9ABC)
    }

    #[test]
    fn rejects_bad_params() {
        assert!(DpswSketch::new(0, 1.0, 0.5, 0.5, 4, 256).is_err());
        assert!(DpswSketch::new(100, 0.0, 0.5, 0.5, 4, 256).is_err());
        assert!(DpswSketch::new(100, 1.0, 0.0, 0.5, 4, 256).is_err());
        assert!(DpswSketch::new(100, 1.0, 0.5, 1.0, 4, 256).is_err());
        assert!(DpswSketch::new(100, 1.0, 0.5, 0.5, 0, 256).is_err());
        assert!(DpswSketch::new(100, 1.0, 0.5, 0.5, 4, 0).is_err());
        assert!(DpswSketch::new(100, 1.0, 0.5, 0.5, 4, 256).is_ok());
    }

    #[test]
    fn budget_never_exceeds_rho() {
        // The core privacy invariant: the per-substream budget split sums to at most ρ.
        for &rho in &[0.5f64, 1.0, 4.0, 10.0] {
            for &alpha in &[0.1f64, 0.3, 0.5, 0.7, 0.9] {
                let s = DpswSketch::new(100_000, rho, alpha, 0.5, 4, 256).unwrap();
                assert!(
                    s.total_budget() <= rho + 1e-9,
                    "budget {} exceeds rho {rho} (alpha {alpha})",
                    s.total_budget()
                );
                // ...and approaches ρ as checkpoints accumulate (within a checkpoint-truncation gap).
                assert!(
                    s.total_budget() > 0.5 * rho,
                    "budget collapsed: {}",
                    s.total_budget()
                );
            }
        }
    }

    #[test]
    fn empty_is_zero() {
        let s = DpswSketch::new(1000, 4.0, 0.5, 0.5, 4, 256).unwrap();
        assert!(s.is_empty());
        assert_eq!(s.query(b"x"), 0);
    }

    #[test]
    fn estimates_window_frequency() {
        // A single heavy item, window larger than the stream ⇒ everything is in-window.
        let mut s = DpswSketch::new(50_000, 8.0, 0.5, 0.5, 4, 256).unwrap();
        let mut r = rng();
        let n = 10_000;
        for _ in 0..n {
            s.insert(b"heavy", &mut r).unwrap();
        }
        let est = s.query(b"heavy");
        // Error is the smooth-histogram boundary (≤ ~α·B for the current substream) plus Gaussian
        // noise — both small relative to n.
        assert!(
            (est - n).abs() < (n as f64 * 0.05) as i64,
            "estimate {est} vs {n}"
        );
    }

    #[test]
    fn old_items_expire_from_window() {
        let mut s = DpswSketch::new(5_000, 8.0, 0.5, 0.5, 4, 256).unwrap();
        let mut r = rng();
        for _ in 0..5_000 {
            s.insert(b"old", &mut r).unwrap();
        }
        for _ in 0..5_000 {
            s.insert(b"new", &mut r).unwrap();
        }
        let old = s.query(b"old");
        let new = s.query(b"new");
        // The window now holds the recent ~5000 items: "new" dominates, "old" has largely expired.
        assert!(new > 4_000, "new {new} should fill the window");
        assert!(old < 1_500, "old {old} should have mostly expired");
    }

    #[test]
    fn distinct_items_do_not_bleed() {
        let mut s = DpswSketch::new(50_000, 8.0, 0.5, 0.5, 4, 512).unwrap();
        let mut r = rng();
        for _ in 0..8_000 {
            s.insert(b"a", &mut r).unwrap();
        }
        for _ in 0..2_000 {
            s.insert(b"b", &mut r).unwrap();
        }
        let a = s.query(b"a");
        let b = s.query(b"b");
        assert!((a - 8_000).abs() < 800, "a estimate {a}");
        assert!((b - 2_000).abs() < 800, "b estimate {b}");
        assert!(a > b, "heavier item should estimate higher");
    }
}
