//! CocoSketch — high-performance sketching for arbitrary partial-key queries (Zhang et al., SIGCOMM
//! 2021).
//!
//! Network measurement wants the size of flows defined on *arbitrary* key fields — by source IP, by
//! 5-tuple, by (src, dst) — all from one sketch. CocoSketch casts this to subset-sum estimation: it
//! keeps `d` arrays of `l` `(key, value)` buckets and updates them with **stochastic variance
//! minimization**. For an incoming packet `(e, w)`: if `e` already occupies one of its `d` buckets,
//! that bucket's value is incremented by `w`; otherwise the smallest-valued of the `d` buckets is
//! incremented by `w` and its key is *replaced* by `e` with probability `w / V_new`. This is Unbiased
//! Space-Saving restricted to `d` buckets per packet (`d ≪ l`), making each value an **unbiased**
//! estimate of its key's flow size while keeping the update cheap.
//!
//! A full-key estimate is the median bucket value holding that key; an arbitrary *partial*-key
//! estimate sums the estimates of all recorded full keys mapping to it (the subset-sum). This is the
//! basic CocoSketch (§4.1), transcribed faithfully from the paper.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use rand::Rng;
use std::collections::HashSet;

/// Base seed mixed with the array index to derive each array's hash function.
const ROW_SEED_BASE: u64 = 0xC0C0_5E70_0000_0001;

/// A CocoSketch over `d` arrays of `l` `(key, value)` buckets.
///
/// # Example
/// ```
/// use sketch_oxide::universal::CocoSketch;
/// use rand::{rngs::StdRng, SeedableRng};
///
/// let mut rng = StdRng::seed_from_u64(1);
/// let mut coco = CocoSketch::new(4, 1024).unwrap();
/// // Flow 0 is heavy (50_000 packets of size 1); the rest is a light tail.
/// for i in 0..100_000u64 {
///     let key = if i < 50_000 { 0 } else { 1 + i % 5000 };
///     coco.insert(key, 1.0, &mut rng);
/// }
/// let est = coco.estimate(0);
/// assert!((est - 50_000.0).abs() < 0.05 * 50_000.0, "estimate {est}");
/// ```
#[derive(Debug, Clone)]
pub struct CocoSketch {
    d: usize,
    l: usize,
    /// `d` arrays of `l` buckets, each `(key, value)`.
    arrays: Vec<Vec<(Option<u64>, f64)>>,
}

impl CocoSketch {
    /// Creates a sketch with `d` arrays of `l` buckets each.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `d` or `l` is 0.
    pub fn new(d: usize, l: usize) -> Result<Self> {
        if d == 0 {
            return Err(SketchError::InvalidParameter {
                param: "d".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        if l == 0 {
            return Err(SketchError::InvalidParameter {
                param: "l".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            d,
            l,
            arrays: vec![vec![(None, 0.0); l]; d],
        })
    }

    /// Bucket index of `key` in array `i`.
    #[inline]
    fn idx(&self, key: u64, i: usize) -> usize {
        let seed = ROW_SEED_BASE.wrapping_add(i as u64);
        (xxhash(&key.to_le_bytes(), seed) % self.l as u64) as usize
    }

    /// Records a packet for full key `e` with positive value (size) `w`, using stochastic variance
    /// minimization. Pass a CSPRNG/seeded RNG.
    pub fn insert<R: Rng + ?Sized>(&mut self, e: u64, w: f64, rng: &mut R) {
        if !(w.is_finite() && w > 0.0) {
            return;
        }
        // Case 1: e already occupies one of its d buckets.
        for i in 0..self.d {
            let j = self.idx(e, i);
            if self.arrays[i][j].0 == Some(e) {
                self.arrays[i][j].1 += w;
                return;
            }
        }
        // Case 2: increment the smallest-valued of the d buckets (random tie-break), then replace its
        // key with e with probability w / V_new.
        let mut min_val = f64::INFINITY;
        let mut choices: Vec<(usize, usize)> = Vec::with_capacity(self.d);
        for i in 0..self.d {
            let j = self.idx(e, i);
            let v = self.arrays[i][j].1;
            if v < min_val {
                min_val = v;
                choices.clear();
                choices.push((i, j));
            } else if v == min_val {
                choices.push((i, j));
            }
        }
        let (i, j) = choices[rng.random_range(0..choices.len())];
        self.arrays[i][j].1 += w;
        if rng.random::<f64>() < w / self.arrays[i][j].1 {
            self.arrays[i][j].0 = Some(e);
        }
    }

    /// Estimated size of full key `e`: the median value among the `d` buckets that currently hold `e`
    /// (0 if none do).
    pub fn estimate(&self, e: u64) -> f64 {
        let mut vals: Vec<f64> = Vec::with_capacity(self.d);
        for i in 0..self.d {
            let j = self.idx(e, i);
            if self.arrays[i][j].0 == Some(e) {
                vals.push(self.arrays[i][j].1);
            }
        }
        if vals.is_empty() {
            return 0.0;
        }
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mid = vals.len() / 2;
        if vals.len() % 2 == 1 {
            vals[mid]
        } else {
            (vals[mid - 1] + vals[mid]) / 2.0
        }
    }

    /// The distinct recorded full keys and their estimated sizes.
    pub fn recorded(&self) -> Vec<(u64, f64)> {
        let mut keys: HashSet<u64> = HashSet::new();
        for arr in &self.arrays {
            for &(k, _) in arr {
                if let Some(k) = k {
                    keys.insert(k);
                }
            }
        }
        keys.into_iter().map(|k| (k, self.estimate(k))).collect()
    }

    /// Estimated size of an arbitrary partial key: the subset sum of the estimates of all recorded
    /// full keys satisfying `pred`.
    pub fn estimate_partial<F: Fn(u64) -> bool>(&self, pred: F) -> f64 {
        self.recorded()
            .into_iter()
            .filter(|&(k, _)| pred(k))
            .map(|(_, v)| v)
            .sum()
    }

    /// Number of arrays `d`.
    #[inline]
    pub fn arrays(&self) -> usize {
        self.d
    }

    /// Buckets per array `l`.
    #[inline]
    pub fn buckets(&self) -> usize {
        self.l
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn rejects_bad_params() {
        assert!(CocoSketch::new(0, 1024).is_err());
        assert!(CocoSketch::new(4, 0).is_err());
        assert!(CocoSketch::new(4, 1024).is_ok());
    }

    #[test]
    fn estimates_heavy_flow() {
        let mut rng = StdRng::seed_from_u64(7);
        let mut coco = CocoSketch::new(4, 2048).unwrap();
        for i in 0..100_000u64 {
            let key = if i < 50_000 { 0 } else { 1 + i % 5000 };
            coco.insert(key, 1.0, &mut rng);
        }
        let est = coco.estimate(0);
        assert!((est - 50_000.0).abs() < 0.05 * 50_000.0, "estimate {est}");
    }

    #[test]
    fn estimates_weighted_sizes() {
        let mut rng = StdRng::seed_from_u64(13);
        let mut coco = CocoSketch::new(4, 2048).unwrap();
        // Three heavy flows with distinct sizes, plus a light tail.
        for _ in 0..30_000 {
            coco.insert(1, 1.0, &mut rng);
        }
        for _ in 0..20_000 {
            coco.insert(2, 1.0, &mut rng);
        }
        for _ in 0..10_000 {
            coco.insert(3, 1.0, &mut rng);
        }
        for i in 0..50_000u64 {
            coco.insert(1000 + i, 1.0, &mut rng);
        }
        assert!(
            (coco.estimate(1) - 30_000.0).abs() < 0.1 * 30_000.0,
            "f1 {}",
            coco.estimate(1)
        );
        assert!(
            (coco.estimate(2) - 20_000.0).abs() < 0.1 * 20_000.0,
            "f2 {}",
            coco.estimate(2)
        );
        assert!(
            (coco.estimate(3) - 10_000.0).abs() < 0.15 * 10_000.0,
            "f3 {}",
            coco.estimate(3)
        );
    }

    #[test]
    fn partial_key_subset_sum() {
        let mut rng = StdRng::seed_from_u64(21);
        let mut coco = CocoSketch::new(4, 2048).unwrap();
        // Heavy flows: even keys form one partial group, odd keys another.
        for _ in 0..25_000 {
            coco.insert(0, 1.0, &mut rng); // even
        }
        for _ in 0..15_000 {
            coco.insert(2, 1.0, &mut rng); // even
        }
        for _ in 0..20_000 {
            coco.insert(1, 1.0, &mut rng); // odd
        }
        for i in 0..40_000u64 {
            coco.insert(1000 + i, 1.0, &mut rng);
        }
        // Even-keyed heavy flows total ~40_000.
        let evens = coco.estimate_partial(|k| k < 1000 && k % 2 == 0);
        assert!((evens - 40_000.0).abs() < 0.12 * 40_000.0, "evens {evens}");
    }

    #[test]
    fn absent_key_is_zero() {
        let mut rng = StdRng::seed_from_u64(5);
        let mut coco = CocoSketch::new(4, 1024).unwrap();
        for i in 0..1000u64 {
            coco.insert(i, 1.0, &mut rng);
        }
        assert_eq!(coco.estimate(999_999), 0.0);
    }
}
