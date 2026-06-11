//! Signed-update weighted sampling — a bounded-size weighted sample over a stream of **signed** weight
//! updates, supporting unbiased net subset-sum estimation (in the spirit of Cohen, Cormode & Duffield,
//! "Don't Let The Negatives Bring You Down: Sampling from Streams of Signed Updates", SIGMETRICS 2012).
//!
//! Classic weighted reservoir sampling assumes every update *adds* weight. Many real streams instead
//! carry **signed** updates — a key's weight can go up *and* down (corrections, refunds, expirations,
//! turnstile deletions). The goal is a small summary from which `Σ_{k ∈ Q} w(k)` (the net weight of any
//! query subset `Q`) can be estimated without bias, in space independent of the number of keys.
//!
//! # Reference construction (two-sided priority sampling)
//!
//! This is a **reference** implementation, *not* the verbatim CCD single-sample algorithm. It combines
//! two well-understood, provably-unbiased pieces:
//!
//! * **Priority sampling** (Duffield, Lund & Thorup, JACM 2007): each weighted item `i` gets a priority
//!   `qᵢ = wᵢ/uᵢ` with `uᵢ ∼ Uniform(0,1]`; the sample keeps the `k` highest-priority items and tracks
//!   `τ`, the largest *discarded* priority. The estimator `Σ_{kept, i∈Q} max(wᵢ, τ)` is **unbiased** for
//!   `Σ_{i∈Q} wᵢ`.
//! * **Two-sided decomposition**: positive updates feed a priority sample of the *positive* sub-stream,
//!   negative updates (by magnitude) feed a second sample of the *negative* sub-stream; by linearity the
//!   net weight of any subset is the positive estimate minus the negative estimate.
//!
//! The cost of the simplicity is variance: a key receiving both `+` and `−` updates is sampled on both
//! sides rather than by its (smaller) net weight, so the CCD single-sample scheme is strictly more
//! accurate.

use crate::common::{Result, SketchError};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

/// A bounded priority sample of a non-negative weighted stream with the Duffield–Lund–Thorup estimator.
#[derive(Debug, Clone)]
struct PrioritySample {
    k: usize,
    items: Vec<(u64, f64, f64)>, // (key, weight, priority q = w/u)
    tau: f64,                    // largest discarded priority (the threshold)
    rng: SmallRng,
}

impl PrioritySample {
    fn new(k: usize, rng: SmallRng) -> Self {
        Self {
            k,
            items: Vec::with_capacity(k + 1),
            tau: 0.0,
            rng,
        }
    }

    fn add(&mut self, key: u64, weight: f64) {
        // u ∈ (0, 1], avoiding 0 so q stays finite.
        let u = (self.rng.random::<f64>()).max(f64::MIN_POSITIVE);
        let q = weight / u;
        self.items.push((key, weight, q));
        if self.items.len() > self.k {
            // Evict the lowest-priority item; its priority is the new threshold candidate.
            let (idx, &(_, _, min_q)) = self
                .items
                .iter()
                .enumerate()
                .min_by(|a, b| a.1 .2.partial_cmp(&b.1 .2).unwrap())
                .unwrap();
            if min_q > self.tau {
                self.tau = min_q;
            }
            self.items.swap_remove(idx);
        }
    }

    fn subset_sum<F: Fn(u64) -> bool>(&self, predicate: &F) -> f64 {
        self.items
            .iter()
            .filter(|(key, _, _)| predicate(*key))
            .map(|(_, w, _)| w.max(self.tau))
            .sum()
    }
}

/// A signed-update weighted sampler over `u64` keys with `f64` weight deltas.
///
/// # Example
/// ```
/// use sketch_oxide::sampling::SignedUpdateSampler;
///
/// // Capacity large enough to retain every update ⇒ exact (no sampling loss).
/// let mut s = SignedUpdateSampler::with_seed(256, 7).unwrap();
/// s.update(1, 10.0);
/// s.update(1, -4.0); // key 1 nets +6
/// s.update(2, 8.0);
/// s.update(3, -3.0); // key 3 nets -3
///
/// assert!((s.estimate_subset_sum(|k| k == 1) - 6.0).abs() < 1e-9);
/// assert!((s.estimate_total() - 11.0).abs() < 1e-9); // 6 + 8 - 3
/// ```
#[derive(Debug, Clone)]
pub struct SignedUpdateSampler {
    positive: PrioritySample,
    negative: PrioritySample,
}

impl SignedUpdateSampler {
    /// Creates a sampler with capacity `k` per side (`2k` total), seeded from the OS.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `k == 0`.
    pub fn new(k: usize) -> Result<Self> {
        Self::build(k, SmallRng::from_os_rng(), SmallRng::from_os_rng())
    }

    /// Like [`new`](Self::new) but with a fixed seed for reproducibility (the two sides use independent
    /// derived seeds).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `k == 0`.
    pub fn with_seed(k: usize, seed: u64) -> Result<Self> {
        Self::build(
            k,
            SmallRng::seed_from_u64(seed),
            SmallRng::seed_from_u64(seed ^ 0x9E37_79B9_7F4A_7C15),
        )
    }

    fn build(k: usize, pos_rng: SmallRng, neg_rng: SmallRng) -> Result<Self> {
        if k == 0 {
            return Err(SketchError::InvalidParameter {
                param: "k".to_string(),
                value: "0".to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        Ok(Self {
            positive: PrioritySample::new(k, pos_rng),
            negative: PrioritySample::new(k, neg_rng),
        })
    }

    /// Applies a signed weight update `delta` to `key`. Positive deltas feed the positive sample,
    /// negative deltas (by magnitude) the negative sample; a zero delta is a no-op.
    pub fn update(&mut self, key: u64, delta: f64) {
        if delta > 0.0 {
            self.positive.add(key, delta);
        } else if delta < 0.0 {
            self.negative.add(key, -delta);
        }
    }

    /// Unbiased estimate of the net weight `Σ_{k : predicate(k)} w(k)` over all keys matching
    /// `predicate` (positive-sample estimate minus negative-sample estimate).
    pub fn estimate_subset_sum<F: Fn(u64) -> bool>(&self, predicate: F) -> f64 {
        self.positive.subset_sum(&predicate) - self.negative.subset_sum(&predicate)
    }

    /// Unbiased estimate of the total net weight across all keys.
    pub fn estimate_total(&self) -> f64 {
        self.estimate_subset_sum(|_| true)
    }

    /// Per-side sample capacity `k`.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.positive.k
    }

    /// Total number of sampled items currently retained (both sides).
    #[inline]
    pub fn sample_size(&self) -> usize {
        self.positive.items.len() + self.negative.items.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(SignedUpdateSampler::new(0).is_err());
        assert!(SignedUpdateSampler::new(16).is_ok());
    }

    #[test]
    fn exact_net_weight_under_capacity() {
        // Capacity covers every update ⇒ no eviction ⇒ τ = 0 ⇒ exact net sums.
        let mut s = SignedUpdateSampler::with_seed(256, 1).unwrap();
        s.update(1, 10.0);
        s.update(1, -4.0); // net +6
        s.update(2, 8.0);
        s.update(3, -3.0); // net -3
        s.update(2, 1.0); // key 2 net +9
        assert!((s.estimate_subset_sum(|k| k == 1) - 6.0).abs() < 1e-9);
        assert!((s.estimate_subset_sum(|k| k == 2) - 9.0).abs() < 1e-9);
        assert!((s.estimate_subset_sum(|k| k == 3) - (-3.0)).abs() < 1e-9);
        assert!((s.estimate_total() - 12.0).abs() < 1e-9); // 6 + 9 - 3
    }

    #[test]
    fn subset_predicate_sums_groups() {
        let mut s = SignedUpdateSampler::with_seed(512, 9).unwrap();
        for k in 0..100u64 {
            s.update(k, 2.0);
            if k % 2 == 0 {
                s.update(k, -1.0); // even keys net +1, odd keys net +2
            }
        }
        // Even keys: 50 × net 1 = 50; odd keys: 50 × net 2 = 100 (under capacity ⇒ exact).
        assert!((s.estimate_subset_sum(|k| k % 2 == 0) - 50.0).abs() < 1e-6);
        assert!((s.estimate_subset_sum(|k| k % 2 == 1) - 100.0).abs() < 1e-6);
    }

    /// Net total of the deterministic test stream used below.
    fn deterministic_truth() -> f64 {
        let mut truth = 0.0;
        for k in 0..2000u64 {
            truth += 1.0 + (k % 5) as f64; // weights 1..=5
            if k % 3 == 0 {
                truth -= 0.5; // every third key gets a small negative correction
            }
        }
        truth
    }

    #[test]
    fn unbiased_total_under_sampling() {
        // Far more keys than capacity ⇒ sampling kicks in; the estimator is unbiased, so the mean over
        // many seeds converges to the true net total.
        let runs = 120u64;
        let mut sum = 0.0;
        for seed in 0..runs {
            let mut s = SignedUpdateSampler::with_seed(64, seed.wrapping_mul(2654435761)).unwrap();
            for k in 0..2000u64 {
                s.update(k, 1.0 + (k % 5) as f64);
                if k % 3 == 0 {
                    s.update(k, -0.5);
                }
            }
            sum += s.estimate_total();
        }
        let truth = deterministic_truth();
        let mean = sum / runs as f64;
        assert!(
            (mean - truth).abs() < 0.1 * truth,
            "mean {mean} far from truth {truth}"
        );
    }
}
