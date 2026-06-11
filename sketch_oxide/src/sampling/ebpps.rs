//! EBPPS — exact probability-proportional-to-size (PPS) sampling with bounded sample size
//! (Lang, "Exact PPS Sampling with Bounded Sample Size", Information Processing Letters 2023).
//!
//! Given a stream of weighted items, EBPPS maintains a sample of at most `k` items such that, at all
//! times, each item appears in the sample with probability **exactly proportional to its weight**
//! (capped at 1) — the *exact PPS* property — using amortised constant time per item. It is the
//! bounded-size counterpart to VarOpt and the basis of Apache DataSketches' `ebpps` family.
//!
//! The state is a single number `c` (the *expected sample size*, including a fractional part), a list
//! of fully-included items, and one optional *partial item* carrying the fractional weight `c mod 1`.
//! On each update with weight `w` (paper / DataSketches reference):
//! 1. `ρ ← min(1/wₘₐₓ, k/W)` for the running max weight `wₘₐₓ` and cumulative weight `W`;
//! 2. **downsample** the existing sample by `ρ_new/ρ_old` (probabilistically shrinking `c` and
//!    evicting items so every item's inclusion probability stays `ρ·wᵢ`);
//! 3. **merge** in the new item as a one-element sample of mass `ρ·w` (≤ 1), reconciling the two
//!    fractional parts.
//!
//! A query draws a sample: every full item, plus the partial item with probability `c mod 1` — so the
//! realised sample size is at most `k` and each item's marginal inclusion probability is exactly
//! `ρ·wᵢ`.
//!
//! Transcribed faithfully from the algorithm's reference implementation (the IPL paper is terse on the
//! latent-sample `downsample`/`merge`/swap operations, which the author's Apache DataSketches code
//! makes precise).

use crate::common::SketchError;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

/// The latent weighted sample: full items plus an optional fractional "partial" item.
#[derive(Debug, Clone)]
struct Sample<T> {
    /// Expected sample size, including its fractional part.
    c: f64,
    partial: Option<T>,
    data: Vec<T>,
}

impl<T: Clone> Sample<T> {
    fn new() -> Self {
        Self {
            c: 0.0,
            partial: None,
            data: Vec::new(),
        }
    }

    /// Resets to a one-item sample of mass `theta ∈ [0, 1]`.
    fn replace_content(&mut self, item: T, theta: f64) {
        self.c = theta;
        if theta == 1.0 {
            self.data = vec![item];
            self.partial = None;
        } else {
            self.data = Vec::new();
            self.partial = Some(item);
        }
    }

    /// Moves a uniformly-random full item into the partial slot (removing it from `data`).
    fn move_one_to_partial(&mut self, rng: &mut SmallRng) {
        let idx = rng.random_range(0..self.data.len());
        self.partial = Some(self.data.swap_remove(idx));
    }

    /// Swaps the partial item with a uniformly-random full item (or fills it if empty).
    fn swap_with_partial(&mut self, rng: &mut SmallRng) {
        if self.partial.is_none() {
            self.move_one_to_partial(rng);
        } else {
            let idx = rng.random_range(0..self.data.len());
            let p = self.partial.take().unwrap();
            let d = std::mem::replace(&mut self.data[idx], p);
            self.partial = Some(d);
        }
    }

    /// Keeps a uniformly-random subset of `num` full items (partial Fisher–Yates).
    fn subsample(&mut self, num: usize, rng: &mut SmallRng) {
        let data_len = self.data.len();
        if num == data_len {
            return;
        }
        for i in 0..num {
            let j = i + rng.random_range(0..(data_len - i));
            self.data.swap(i, j);
        }
        self.data.truncate(num);
    }

    /// Shrinks the sample to mass `theta·c` while preserving each item's inclusion probability.
    fn downsample(&mut self, theta: f64, rng: &mut SmallRng) {
        if theta >= 1.0 {
            return;
        }
        let new_c = theta * self.c;
        let new_c_int = new_c.floor();
        let new_c_frac = new_c % 1.0;
        let c_int = self.c.floor();
        let c_frac = self.c % 1.0;

        if new_c_int == 0.0 {
            if rng.random::<f64>() > c_frac / self.c {
                self.swap_with_partial(rng);
            }
            self.data.clear();
        } else if new_c_int == c_int {
            if rng.random::<f64>() > (1.0 - theta * c_frac) / (1.0 - new_c_frac) {
                self.swap_with_partial(rng);
            }
        } else if rng.random::<f64>() < theta * c_frac {
            self.subsample(new_c_int as usize, rng);
            self.swap_with_partial(rng);
        } else {
            self.subsample(new_c_int as usize + 1, rng);
            self.move_one_to_partial(rng);
        }

        if new_c == new_c_int {
            self.partial = None;
        }
        self.c = new_c;
    }

    /// Merges `other` (consumed) into `self`, reconciling the two fractional parts.
    fn merge(&mut self, other: Sample<T>, rng: &mut SmallRng) {
        let c_frac = self.c % 1.0;
        let other_c_frac = other.c % 1.0;
        self.c += other.c;
        self.data.extend(other.data);

        if c_frac == 0.0 && other_c_frac == 0.0 {
            self.partial = None;
        } else if c_frac + other_c_frac == 1.0 || self.c == self.c.floor() {
            if rng.random::<f64>() <= c_frac {
                if let Some(p) = self.partial.take() {
                    self.data.push(p);
                }
            } else if let Some(p) = other.partial {
                self.data.push(p);
            }
            self.partial = None;
        } else if c_frac + other_c_frac < 1.0 {
            if rng.random::<f64>() > c_frac / (c_frac + other_c_frac) {
                self.partial = other.partial;
            }
        } else if rng.random::<f64>() <= (1.0 - c_frac) / ((1.0 - c_frac) + (1.0 - other_c_frac)) {
            if let Some(p) = other.partial {
                self.data.push(p);
            }
        } else {
            if let Some(p) = self.partial.take() {
                self.data.push(p);
            }
            self.partial = other.partial;
        }
    }

    /// Draws a realised sample: all full items, plus the partial item with probability `c mod 1`.
    fn draw(&self, rng: &mut SmallRng) -> Option<Vec<T>> {
        let c_frac = self.c % 1.0;
        let include_partial = self.partial.is_some() && rng.random::<f64>() < c_frac;
        if self.data.is_empty() && !include_partial {
            return None;
        }
        let mut result = self.data.clone();
        if include_partial {
            result.push(self.partial.clone().unwrap());
        }
        Some(result)
    }
}

/// An EBPPS sketch: a bounded-size exact-PPS sample over a weighted stream.
///
/// # Example
/// ```
/// use sketch_oxide::sampling::EbppsSketch;
///
/// // Keep at most 20 items. One very heavy item (u32::MAX) dominates the stream.
/// let mut s = EbppsSketch::with_seed(20, 42).unwrap();
/// s.update(u32::MAX, 1000.0).unwrap();
/// for i in 0..500u32 {
///     s.update(i, 1.0).unwrap();
/// }
/// let sample = s.sample().unwrap();
/// assert!(sample.len() <= 20);            // bounded sample size
/// assert!(sample.contains(&u32::MAX));    // the dominant item is (almost) always present
/// ```
#[derive(Debug, Clone)]
pub struct EbppsSketch<T> {
    k: f64,
    cumulative_wt: f64,
    wt_max: f64,
    rho: f64,
    n: u64,
    sample: Sample<T>,
    rng: SmallRng,
}

impl<T: Clone> EbppsSketch<T> {
    /// Creates an EBPPS sketch keeping at most `k` items (`k ≥ 1`), seeded from the OS.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `k == 0`.
    pub fn new(k: usize) -> Result<Self, SketchError> {
        Self::build(k, SmallRng::from_os_rng())
    }

    /// Like [`new`](Self::new) but with a fixed RNG seed for reproducibility.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `k == 0`.
    pub fn with_seed(k: usize, seed: u64) -> Result<Self, SketchError> {
        Self::build(k, SmallRng::seed_from_u64(seed))
    }

    fn build(k: usize, rng: SmallRng) -> Result<Self, SketchError> {
        if k == 0 {
            return Err(SketchError::InvalidParameter {
                param: "k".to_string(),
                value: "0".to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        Ok(Self {
            k: k as f64,
            cumulative_wt: 0.0,
            wt_max: 0.0,
            rho: 1.0,
            n: 0,
            sample: Sample::new(),
            rng,
        })
    }

    /// Processes one item with weight `weight` (`weight ≥ 0`, finite). Zero-weight items are ignored.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `weight` is negative or not finite.
    pub fn update(&mut self, item: T, weight: f64) -> Result<(), SketchError> {
        if weight < 0.0 || !weight.is_finite() {
            return Err(SketchError::InvalidParameter {
                param: "weight".to_string(),
                value: weight.to_string(),
                constraint: "must be nonnegative and finite".to_string(),
            });
        }
        if weight == 0.0 {
            return Ok(());
        }
        let new_cum = self.cumulative_wt + weight;
        let new_wt_max = self.wt_max.max(weight);
        let new_rho = (1.0 / new_wt_max).min(self.k / new_cum);

        if self.cumulative_wt > 0.0 {
            self.sample.downsample(new_rho / self.rho, &mut self.rng);
        }
        let mut tmp = Sample::new();
        tmp.replace_content(item, new_rho * weight);
        self.sample.merge(tmp, &mut self.rng);

        self.cumulative_wt = new_cum;
        self.wt_max = new_wt_max;
        self.rho = new_rho;
        self.n += 1;
        Ok(())
    }

    /// Draws the current sample (`≤ k` items). Each call re-resolves the partial item, so successive
    /// draws may differ. Returns `None` if nothing has been sampled.
    pub fn sample(&mut self) -> Option<Vec<T>> {
        // Split the borrow so the sample can use the sketch's RNG.
        let Self { sample, rng, .. } = self;
        sample.draw(rng)
    }

    /// Number of items processed.
    #[inline]
    pub fn n(&self) -> u64 {
        self.n
    }

    /// Total weight seen.
    #[inline]
    pub fn cumulative_weight(&self) -> f64 {
        self.cumulative_wt
    }

    /// The current expected sample size `c` (`≤ k`).
    #[inline]
    pub fn c(&self) -> f64 {
        self.sample.c
    }

    /// The current PPS scaling factor `ρ`; an item of weight `w` is sampled with probability `ρ·w`.
    #[inline]
    pub fn rho(&self) -> f64 {
        self.rho
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_k() {
        assert!(EbppsSketch::<u32>::new(0).is_err());
        assert!(EbppsSketch::<u32>::with_seed(10, 1).is_ok());
    }

    #[test]
    fn empty_sample_is_none() {
        let mut s = EbppsSketch::<u32>::with_seed(10, 1).unwrap();
        assert!(s.sample().is_none());
        assert_eq!(s.n(), 0);
    }

    #[test]
    fn rejects_bad_weight() {
        let mut s = EbppsSketch::<u32>::with_seed(10, 1).unwrap();
        assert!(s.update(1, -1.0).is_err());
        assert!(s.update(1, f64::INFINITY).is_err());
        assert!(s.update(1, f64::NAN).is_err());
        assert!(s.update(1, 0.0).is_ok()); // ignored, not an error
        assert_eq!(s.n(), 0);
    }

    #[test]
    fn sample_size_is_bounded_by_k() {
        let k = 16usize;
        let mut s = EbppsSketch::<u32>::with_seed(k, 7).unwrap();
        for i in 0..10_000u32 {
            s.update(i, 1.0).unwrap();
        }
        // c approaches k for a long unit-weight stream, and the realised sample never exceeds k.
        assert!(s.c() <= k as f64 + 1e-9, "c {} exceeds k", s.c());
        assert!(s.c() > k as f64 - 1.0, "c {} should approach k", s.c());
        for _ in 0..200 {
            let sample = s.sample().unwrap();
            assert!(sample.len() <= k, "sample size {} exceeds k", sample.len());
        }
    }

    #[test]
    fn dominant_item_is_always_present() {
        let mut s = EbppsSketch::<&str>::with_seed(20, 99).unwrap();
        s.update("whale", 1_000_000.0).unwrap();
        for i in 0..1_000u32 {
            s.update(Box::leak(format!("krill-{i}").into_boxed_str()), 1.0)
                .unwrap();
        }
        // ρ·w_whale ≈ 1, so the whale is in every realised sample.
        for _ in 0..500 {
            assert!(s.sample().unwrap().contains(&"whale"));
        }
    }

    #[test]
    fn exact_pps_inclusion_probabilities() {
        // Items A/B/C plus 100 unit-weight items. After the stream,
        //   ρ = min(1/w_max, k/W),  inclusion prob π_i = ρ·w_i.
        let k = 20usize;
        let weights: Vec<(u32, f64)> = {
            let mut v = vec![(0u32, 100.0), (1, 50.0), (2, 10.0)];
            for i in 0..100u32 {
                v.push((100 + i, 1.0));
            }
            v
        };
        let w_total: f64 = weights.iter().map(|&(_, w)| w).sum();
        let w_max: f64 = 100.0;
        let rho: f64 = (1.0 / w_max).min(k as f64 / w_total);
        let pi = |w: f64| (rho * w).min(1.0);

        let runs = 3_000u64;
        let mut count_a = 0u32;
        let mut count_b = 0u32;
        let mut count_c = 0u32;
        for seed in 0..runs {
            let mut s = EbppsSketch::<u32>::with_seed(k, seed).unwrap();
            for &(item, w) in &weights {
                s.update(item, w).unwrap();
            }
            let sample = s.sample().unwrap();
            if sample.contains(&0) {
                count_a += 1;
            }
            if sample.contains(&1) {
                count_b += 1;
            }
            if sample.contains(&2) {
                count_c += 1;
            }
        }
        let emp_a = count_a as f64 / runs as f64;
        let emp_b = count_b as f64 / runs as f64;
        let emp_c = count_c as f64 / runs as f64;
        assert!(
            (emp_a - pi(100.0)).abs() < 0.04,
            "A: {emp_a} vs {}",
            pi(100.0)
        );
        assert!(
            (emp_b - pi(50.0)).abs() < 0.04,
            "B: {emp_b} vs {}",
            pi(50.0)
        );
        assert!(
            (emp_c - pi(10.0)).abs() < 0.04,
            "C: {emp_c} vs {}",
            pi(10.0)
        );
    }
}
