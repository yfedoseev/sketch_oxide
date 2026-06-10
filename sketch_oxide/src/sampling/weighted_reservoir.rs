//! Weighted reservoir sampling without replacement (Efraimidis–Spirakis A-Res).
//!
//! Maintains a size-`k` sample of a stream where each item carries a positive weight, such
//! that an item's chance of being in the sample grows with its weight — the weighted
//! analogue of [`ReservoirSampling`](super::ReservoirSampling), in one pass and bounded
//! memory.
//!
//! # Algorithm
//!
//! Each arriving item with weight `w` is assigned a random key `key = u^(1/w)` with
//! `u ~ Uniform(0, 1)`. The reservoir keeps the `k` items with the **largest** keys (a
//! min-key heap evicts the smallest when full). Larger weights push keys toward 1, so
//! heavier items are retained more often. This is the "A-Res" scheme of Efraimidis &
//! Spirakis, "Weighted random sampling with a reservoir" (IPL 2006); the keys are comparable
//! across independently built reservoirs, so two samples [`merge`](Self::merge) by simply
//! keeping the top-`k` keys of their union.
//!
//! # Randomness
//!
//! Uses a fast PRNG seeded from the OS by default. For reproducible samples use
//! [`with_seed`](Self::with_seed). (This is statistical sampling, not differential privacy —
//! no CSPRNG is required.)

use crate::common::{Result, SketchError};
use rand::Rng;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// One reservoir entry: a sampling key and its item. Ordered by key only.
#[derive(Clone, Debug)]
struct Keyed<T> {
    key: f64,
    item: T,
}

impl<T> PartialEq for Keyed<T> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}
impl<T> Eq for Keyed<T> {}
impl<T> PartialOrd for Keyed<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> Ord for Keyed<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Keys are in (0, 1], never NaN; total_cmp gives a total order.
        self.key.total_cmp(&other.key)
    }
}

/// Weighted reservoir of up to `k` items (Efraimidis–Spirakis A-Res).
///
/// # Example
/// ```
/// use sketch_oxide::sampling::WeightedReservoirSampling;
///
/// let mut res = WeightedReservoirSampling::with_seed(2, 42).unwrap();
/// res.update("rare", 1.0);
/// res.update("common", 100.0);
/// res.update("common2", 100.0);
/// assert_eq!(res.len(), 2);
/// ```
#[derive(Debug, Clone)]
pub struct WeightedReservoirSampling<T: Clone> {
    /// Target sample size.
    k: usize,
    /// Min-key heap (the smallest key sits at the top via `Reverse`), so the weakest
    /// retained item is evicted first.
    heap: BinaryHeap<std::cmp::Reverse<Keyed<T>>>,
    /// Total number of items seen (including evicted/skipped).
    count: u64,
    rng: rand::rngs::SmallRng,
}

impl<T: Clone> WeightedReservoirSampling<T> {
    /// Creates a reservoir holding up to `k` items, seeded from the OS.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `k == 0`.
    pub fn new(k: usize) -> Result<Self> {
        use rand::SeedableRng;
        Self::from_rng(k, rand::rngs::SmallRng::from_os_rng())
    }

    /// Creates a reservoir with a fixed RNG seed for reproducible samples.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `k == 0`.
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
            heap: BinaryHeap::with_capacity(k + 1),
            count: 0,
            rng,
        })
    }

    /// Offers `item` with the given positive `weight`. Items with non-positive or non-finite
    /// weight are ignored.
    pub fn update(&mut self, item: T, weight: f64) {
        if !weight.is_finite() || weight <= 0.0 {
            return;
        }
        self.count += 1;

        // key = u^(1/w); guard u away from 0 to keep the log well-defined.
        let u: f64 = self.rng.random::<f64>().max(f64::MIN_POSITIVE);
        let key = u.powf(1.0 / weight);
        self.offer(Keyed { key, item });
    }

    /// Inserts an already-keyed entry, evicting the smallest key if at capacity.
    fn offer(&mut self, entry: Keyed<T>) {
        if self.heap.len() < self.k {
            self.heap.push(std::cmp::Reverse(entry));
        } else if let Some(std::cmp::Reverse(smallest)) = self.heap.peek() {
            if entry.key > smallest.key {
                self.heap.pop();
                self.heap.push(std::cmp::Reverse(entry));
            }
        }
    }

    /// Returns the currently sampled items (in unspecified order).
    pub fn sample(&self) -> Vec<&T> {
        self.heap.iter().map(|r| &r.0.item).collect()
    }

    /// Consumes the reservoir and returns the sampled items.
    pub fn into_sample(self) -> Vec<T> {
        self.heap.into_iter().map(|r| r.0.item).collect()
    }

    /// Number of items currently in the sample.
    pub fn len(&self) -> usize {
        self.heap.len()
    }

    /// Whether the sample is currently empty.
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    /// Target sample size `k`.
    pub fn capacity(&self) -> usize {
        self.k
    }

    /// Total number of (positively weighted) items offered.
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Empties the sample (keeps `k` and the RNG state).
    pub fn clear(&mut self) {
        self.heap.clear();
        self.count = 0;
    }

    /// Merges another reservoir into this one, keeping the top-`k` items of the union by
    /// sampling key. Both must target the same `k`.
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the two reservoirs have different `k`.
    pub fn merge(&mut self, other: &Self) -> Result<()> {
        if self.k != other.k {
            return Err(SketchError::IncompatibleSketches {
                reason: format!("k mismatch: {} vs {}", self.k, other.k),
            });
        }
        self.count += other.count;
        for r in &other.heap {
            self.offer(r.0.clone());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_k() {
        assert!(WeightedReservoirSampling::<u32>::new(0).is_err());
    }

    #[test]
    fn fills_up_to_k() {
        let mut res = WeightedReservoirSampling::with_seed(3, 1).unwrap();
        for i in 0..100u32 {
            res.update(i, 1.0);
        }
        assert_eq!(res.len(), 3);
        assert_eq!(res.count(), 100);
    }

    #[test]
    fn non_positive_weight_ignored() {
        let mut res = WeightedReservoirSampling::with_seed(5, 1).unwrap();
        res.update(1u32, 0.0);
        res.update(2u32, -1.0);
        res.update(3u32, f64::NAN);
        assert!(res.is_empty());
        assert_eq!(res.count(), 0);
    }

    #[test]
    fn heavier_items_selected_more_often() {
        // k=1: a weight-9 item should win ~90% of the time vs a weight-1 item.
        let trials = 4000;
        let mut wins_heavy = 0;
        for seed in 0..trials {
            let mut res = WeightedReservoirSampling::with_seed(1, seed).unwrap();
            res.update("light", 1.0);
            res.update("heavy", 9.0);
            if res.into_sample()[0] == "heavy" {
                wins_heavy += 1;
            }
        }
        let frac = wins_heavy as f64 / trials as f64;
        assert!(
            frac > 0.85 && frac < 0.95,
            "heavy selected {frac} of the time"
        );
    }

    #[test]
    fn merge_keeps_top_k() {
        let mut a = WeightedReservoirSampling::with_seed(4, 1).unwrap();
        let mut b = WeightedReservoirSampling::with_seed(4, 2).unwrap();
        for i in 0..50u32 {
            a.update(i, 1.0);
            b.update(i + 50, 1.0);
        }
        a.merge(&b).unwrap();
        assert_eq!(a.len(), 4);
        assert_eq!(a.count(), 100);
    }

    #[test]
    fn merge_requires_same_k() {
        let mut a = WeightedReservoirSampling::<u32>::with_seed(2, 1).unwrap();
        let b = WeightedReservoirSampling::<u32>::with_seed(3, 1).unwrap();
        assert!(a.merge(&b).is_err());
    }
}
