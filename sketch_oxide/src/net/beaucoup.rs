//! BeauCoup — per-key distinct counting and super-spreader detection via coupon collectors.
//!
//! BeauCoup (Chen, Liu, Zhao, Braverman & Rexford, "BeauCoup: Answering Many Network Traffic
//! Queries, One Memory Update at a Time", SIGCOMM 2020) detects keys that contact many *distinct*
//! values — e.g. source IPs that touch many destinations (super-spreaders / port scanners). It uses
//! the **coupon-collector** principle: each key tries to collect `m` coupons; a `(key, value)`
//! observation activates one coupon (chosen by hashing the pair) only with a small probability `q`,
//! so a key fills its coupons only after it has seen *many distinct* values. The fraction of coupons
//! collected inverts to a per-key distinct-count estimate, and keys that fill (nearly) all coupons
//! are flagged as super-spreaders.
//!
//! Because activation is keyed on the `(key, value)` *pair*, repeats of the same pair are
//! idempotent — only distinct values move the estimate. With `q ≪ 1` only a small fraction of
//! observations ever touch memory, which is BeauCoup's headline property.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use std::collections::HashMap;

/// A BeauCoup per-key distinct-value estimator over `m ≤ 64` coupons.
///
/// # Example
/// ```
/// use sketch_oxide::net::BeauCoup;
///
/// let mut bc = BeauCoup::new(64, 0.5).unwrap();
/// // Source 1 contacts 300 distinct destinations; source 2 contacts 5.
/// for d in 0..300u64 { bc.record(1, d); }
/// for d in 0..5u64 { bc.record(2, d); }
///
/// assert!(bc.estimate_distinct(1) > 100.0); // a super-spreader
/// assert!(bc.estimate_distinct(2) < 30.0);  // not
/// ```
#[derive(Debug, Clone)]
pub struct BeauCoup {
    m: u32,
    q: f64,
    /// Per-key coupon bitmap (bit `c` set ⇒ coupon `c` collected).
    coupons: HashMap<u64, u64>,
}

impl BeauCoup {
    /// Creates a BeauCoup with `num_coupons` coupons per key (`2..=64`) and per-observation coupon
    /// activation probability `activation_prob` (`(0, 1]`). Smaller `activation_prob` raises the
    /// distinct-count range the sketch can resolve and reduces memory writes.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `num_coupons` is outside `2..=64` or `activation_prob`
    /// is not in `(0, 1]`.
    pub fn new(num_coupons: u32, activation_prob: f64) -> Result<Self> {
        if !(2..=64).contains(&num_coupons) {
            return Err(SketchError::InvalidParameter {
                param: "num_coupons".to_string(),
                value: num_coupons.to_string(),
                constraint: "must be in 2..=64".to_string(),
            });
        }
        if !(activation_prob > 0.0 && activation_prob <= 1.0) {
            return Err(SketchError::InvalidParameter {
                param: "activation_prob".to_string(),
                value: activation_prob.to_string(),
                constraint: "must be in (0, 1]".to_string(),
            });
        }
        Ok(Self {
            m: num_coupons,
            q: activation_prob,
            coupons: HashMap::new(),
        })
    }

    /// 16 bytes encoding the `(key, value)` pair, for hashing.
    #[inline]
    fn pair_bytes(key: u64, value: u64) -> [u8; 16] {
        let mut b = [0u8; 16];
        b[..8].copy_from_slice(&key.to_le_bytes());
        b[8..].copy_from_slice(&value.to_le_bytes());
        b
    }

    /// Records that `key` was observed with `value` (e.g. a source contacting a destination).
    pub fn record(&mut self, key: u64, value: u64) {
        let bytes = Self::pair_bytes(key, value);
        // Activate only with probability q (deterministic in the pair, so repeats are idempotent).
        let activate = (xxhash(&bytes, 1) as f64) / (u64::MAX as f64) < self.q;
        if !activate {
            return;
        }
        let coupon = (xxhash(&bytes, 2) % self.m as u64) as u32;
        *self.coupons.entry(key).or_insert(0) |= 1u64 << coupon;
    }

    /// Estimated number of distinct values seen for `key`, by inverting the coupon-collector model
    /// (0 if unseen; a conservative lower bound once all coupons are collected/saturated).
    pub fn estimate_distinct(&self, key: u64) -> f64 {
        let collected = self.coupons.get(&key).map_or(0, |b| b.count_ones());
        if collected == 0 {
            return 0.0;
        }
        let m = self.m as f64;
        // Cap below m so ln(1 − j/m) stays finite when saturated (returns a lower bound).
        let j = (collected as f64).min(m - 0.5);
        // After d distinct values a coupon stays empty w.p. (1 − q/m)^d, so j/m = 1 − (1 − q/m)^d.
        (1.0 - j / m).ln() / (1.0 - self.q / m).ln()
    }

    /// Keys whose estimated distinct count is at least `threshold`, as `(key, estimate)` sorted by
    /// estimate descending.
    pub fn super_spreaders(&self, threshold: f64) -> Vec<(u64, f64)> {
        let mut out: Vec<(u64, f64)> = self
            .coupons
            .keys()
            .map(|&k| (k, self.estimate_distinct(k)))
            .filter(|&(_, e)| e >= threshold)
            .collect();
        out.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        out
    }

    /// Number of keys currently tracked.
    #[inline]
    pub fn num_keys(&self) -> usize {
        self.coupons.len()
    }

    /// Number of coupons per key.
    #[inline]
    pub fn num_coupons(&self) -> u32 {
        self.m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(BeauCoup::new(1, 0.5).is_err());
        assert!(BeauCoup::new(65, 0.5).is_err());
        assert!(BeauCoup::new(64, 0.0).is_err());
        assert!(BeauCoup::new(64, 1.5).is_err());
        assert!(BeauCoup::new(64, 0.5).is_ok());
    }

    #[test]
    fn unseen_key_is_zero() {
        let bc = BeauCoup::new(64, 0.5).unwrap();
        assert_eq!(bc.estimate_distinct(99), 0.0);
    }

    #[test]
    fn estimates_distinct_count() {
        let mut bc = BeauCoup::new(64, 0.5).unwrap();
        for d in 0..200u64 {
            bc.record(7, d);
        }
        let est = bc.estimate_distinct(7);
        assert!((est - 200.0).abs() < 0.35 * 200.0, "estimate {est}");
    }

    #[test]
    fn repeats_do_not_inflate() {
        let mut bc = BeauCoup::new(64, 0.5).unwrap();
        for _ in 0..10_000 {
            bc.record(3, 42); // the same single pair, many times
        }
        // Only one distinct value → estimate stays tiny.
        assert!(
            bc.estimate_distinct(3) < 5.0,
            "estimate {}",
            bc.estimate_distinct(3)
        );
    }

    #[test]
    fn detects_super_spreaders() {
        let mut bc = BeauCoup::new(64, 0.5).unwrap();
        // Heavy spreaders.
        for d in 0..400u64 {
            bc.record(1, d);
            bc.record(2, d + 1_000_000);
        }
        // Light keys.
        for d in 0..10u64 {
            bc.record(10, d);
            bc.record(11, d);
        }
        let ss = bc.super_spreaders(100.0);
        let keys: Vec<u64> = ss.iter().map(|&(k, _)| k).collect();
        assert!(
            keys.contains(&1) && keys.contains(&2),
            "super spreaders {keys:?}"
        );
        assert!(
            !keys.contains(&10) && !keys.contains(&11),
            "light keys flagged"
        );
    }

    #[test]
    fn lower_activation_resolves_larger_counts() {
        // With small q the same coupon budget resolves bigger distinct counts without saturating.
        let mut bc = BeauCoup::new(64, 0.1).unwrap();
        for d in 0..1000u64 {
            bc.record(5, d);
        }
        let est = bc.estimate_distinct(5);
        assert!((est - 1000.0).abs() < 0.4 * 1000.0, "estimate {est}");
    }
}
