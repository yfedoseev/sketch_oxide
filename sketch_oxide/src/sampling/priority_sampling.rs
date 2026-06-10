//! Priority sampling — weighted sampling with unbiased subset-sum estimation.
//!
//! Priority sampling (Duffield, Lund & Thorup, "Priority sampling for estimation of arbitrary
//! subset sums", JACM 2007) keeps a size-`k` weighted sample from which **any** subset's total
//! weight can be estimated *without bias* and with small variance. Each item gets a priority
//! `weight / u` (`u ~ Uniform(0,1]`); the `k` highest-priority items are kept, and the
//! `(k+1)`-th highest priority becomes a threshold `τ`. Each kept item is then assigned an
//! adjusted weight `max(weight, τ)`; summing these over any subset of the sample is an
//! unbiased estimate of that subset's true weight. It is the substrate for time-decayed and
//! forward-decay-biased sampling.

use crate::common::{Result, SketchError};
use rand::Rng;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// A kept item: its priority, original weight, and value.
#[derive(Clone, Debug)]
struct Entry<T> {
    priority: f64,
    weight: f64,
    item: T,
}

impl<T> PartialEq for Entry<T> {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}
impl<T> Eq for Entry<T> {}
impl<T> PartialOrd for Entry<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> Ord for Entry<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority.total_cmp(&other.priority)
    }
}

/// A priority sample of up to `k` items supporting unbiased subset-sum estimation.
///
/// # Example
/// ```
/// use sketch_oxide::sampling::PrioritySampling;
///
/// let mut s = PrioritySampling::with_seed(50, 1).unwrap();
/// for i in 0..1000u64 { s.update(i, 1.0); } // total weight 1000
/// // The estimated total is unbiased (here within sampling noise of 1000).
/// let est = s.estimated_total();
/// assert!(est > 700.0 && est < 1300.0, "estimated total {est}");
/// ```
#[derive(Debug, Clone)]
pub struct PrioritySampling<T: Clone> {
    k: usize,
    /// Min-priority heap of capacity `k + 1`; the smallest priority is the threshold item.
    heap: BinaryHeap<std::cmp::Reverse<Entry<T>>>,
    rng: rand::rngs::SmallRng,
}

impl<T: Clone> PrioritySampling<T> {
    /// Creates a priority sample of size `k`, seeded from the OS.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `k == 0`.
    pub fn new(k: usize) -> Result<Self> {
        use rand::SeedableRng;
        Self::from_rng(k, rand::rngs::SmallRng::from_os_rng())
    }

    /// Creates a priority sample with a fixed RNG seed.
    pub fn with_seed(k: usize, seed: u64) -> Result<Self> {
        use rand::SeedableRng;
        Self::from_rng(k, rand::rngs::SmallRng::seed_from_u64(seed))
    }

    fn from_rng(k: usize, rng: rand::rngs::SmallRng) -> Result<Self> {
        if k == 0 {
            return Err(SketchError::InvalidParameter {
                param: "k".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            k,
            heap: BinaryHeap::with_capacity(k + 2),
            rng,
        })
    }

    /// Offers `item` with positive `weight`. Non-positive/non-finite weights are ignored.
    pub fn update(&mut self, item: T, weight: f64) {
        if !weight.is_finite() || weight <= 0.0 {
            return;
        }
        let u = self.rng.random::<f64>().max(f64::MIN_POSITIVE);
        let priority = weight / u;
        let entry = Entry {
            priority,
            weight,
            item,
        };
        // Keep the top k+1 priorities; the smallest acts as the threshold.
        if self.heap.len() <= self.k {
            self.heap.push(std::cmp::Reverse(entry));
        } else if let Some(std::cmp::Reverse(smallest)) = self.heap.peek() {
            if entry.priority > smallest.priority {
                self.heap.pop();
                self.heap.push(std::cmp::Reverse(entry));
            }
        }
    }

    /// The threshold `τ`: the `(k+1)`-th highest priority seen, or 0 if fewer than `k+1` items
    /// have been offered.
    fn threshold(&self) -> f64 {
        if self.heap.len() > self.k {
            self.heap.peek().map_or(0.0, |r| r.0.priority)
        } else {
            0.0
        }
    }

    /// The sampled items with their unbiased adjusted weights `max(weight, τ)`. The threshold
    /// item itself is excluded once the sample is full.
    pub fn sample(&self) -> Vec<(T, f64)> {
        let tau = self.threshold();
        let full = self.heap.len() > self.k;
        let min_priority = if full {
            self.heap.peek().map_or(f64::NEG_INFINITY, |r| r.0.priority)
        } else {
            f64::NEG_INFINITY
        };
        self.heap
            .iter()
            .filter(|r| !full || r.0.priority > min_priority)
            .map(|r| (r.0.item.clone(), r.0.weight.max(tau)))
            .collect()
    }

    /// Unbiased estimate of the total weight of all items offered.
    pub fn estimated_total(&self) -> f64 {
        self.sample().iter().map(|(_, w)| *w).sum()
    }

    /// Unbiased estimate of the total weight of the sampled items satisfying `pred`.
    pub fn estimated_subset_sum<F: Fn(&T) -> bool>(&self, pred: F) -> f64 {
        self.sample()
            .iter()
            .filter(|(item, _)| pred(item))
            .map(|(_, w)| *w)
            .sum()
    }

    /// Number of items currently in the sample (excludes the threshold item once full).
    pub fn len(&self) -> usize {
        self.heap.len().min(self.k)
    }

    /// Whether the sample is empty.
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    /// Target sample size `k`.
    pub fn capacity(&self) -> usize {
        self.k
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_k() {
        assert!(PrioritySampling::<u64>::new(0).is_err());
        assert!(PrioritySampling::<u64>::new(10).is_ok());
    }

    #[test]
    fn fills_to_k() {
        let mut s = PrioritySampling::with_seed(20, 1).unwrap();
        for i in 0..1000u64 {
            s.update(i, 1.0);
        }
        assert_eq!(s.len(), 20);
    }

    #[test]
    fn estimated_total_is_unbiased_on_average() {
        // Average the estimate over many seeds; should converge to the true total (1000).
        let trials = 200;
        let mut sum = 0.0;
        for seed in 0..trials {
            let mut s = PrioritySampling::with_seed(30, seed).unwrap();
            for i in 0..1000u64 {
                s.update(i, 1.0);
            }
            sum += s.estimated_total();
        }
        let mean = sum / trials as f64;
        assert!(
            (mean - 1000.0).abs() < 60.0,
            "mean estimate {mean} vs true 1000"
        );
    }

    #[test]
    fn weighted_total_unbiased() {
        // Items with weight = their value; true total = sum 1..=100 = 5050.
        let trials = 300;
        let mut sum = 0.0;
        for seed in 0..trials {
            let mut s = PrioritySampling::with_seed(40, seed).unwrap();
            for i in 1..=100u64 {
                s.update(i, i as f64);
            }
            sum += s.estimated_total();
        }
        let mean = sum / trials as f64;
        assert!((mean - 5050.0).abs() < 400.0, "mean {mean} vs true 5050");
    }

    #[test]
    fn subset_sum_estimates_partial() {
        // Estimate the weight of even keys (true 500 of the 1000 unit-weight items).
        let trials = 200;
        let mut sum = 0.0;
        for seed in 0..trials {
            let mut s = PrioritySampling::with_seed(50, seed + 7).unwrap();
            for i in 0..1000u64 {
                s.update(i, 1.0);
            }
            sum += s.estimated_subset_sum(|&k| k % 2 == 0);
        }
        let mean = sum / trials as f64;
        assert!(
            (mean - 500.0).abs() < 60.0,
            "even-subset mean {mean} vs 500"
        );
    }

    #[test]
    fn ignores_bad_weights() {
        let mut s = PrioritySampling::with_seed(10, 1).unwrap();
        s.update(1u64, 0.0);
        s.update(2u64, -1.0);
        s.update(3u64, f64::NAN);
        assert!(s.is_empty());
    }
}
