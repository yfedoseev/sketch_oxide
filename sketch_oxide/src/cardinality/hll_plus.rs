//! HyperLogLog++ — engineered HyperLogLog with 64-bit hashing and empirical bias correction
//! (Heule, Nunkesser & Hall, "HyperLogLog in Practice", EDBT 2013).
//!
//! HyperLogLog++ is Google's production refinement of HyperLogLog. It keeps the same `m = 2^p`
//! register structure but improves accuracy across the whole cardinality range:
//!
//! - **64-bit hashing** removes HLL's large-range correction entirely (a 32-bit hash space saturates
//!   near `2^32`; 64 bits does not for realistic cardinalities).
//! - **Empirical bias correction.** HLL's raw estimate is biased for cardinalities up to a few `m`.
//!   HLL++ subtracts an *empirically measured* bias, looked up by interpolating (`k = 6` nearest
//!   neighbours) over per-precision sample tables (`RAW_ESTIMATE`/`BIAS`, calibrated by the authors).
//! - **LinearCounting with a tuned threshold.** Below a per-precision threshold the estimate switches
//!   to LinearCounting (`m·ln(m/V)`, `V` = empty registers), which is more accurate for small sets.
//!
//! The estimation rule (paper §4): compute the raw estimate `E`; if `E ≤ 5m`, correct it to
//! `E' = E − bias(E)`; let `H = LinearCounting(m, V)` when there are empty registers, else `H = E'`;
//! return `H` if `H ≤ threshold(p)`, otherwise `E'`.
//!
//! # Implementation note
//!
//! This implements the **dense** representation. HLL++'s *sparse* representation (a variable-length
//! encoding that saves memory and enables higher-precision LinearCounting at small cardinalities) is a
//! memory optimisation over the same accuracy contract and is left as a follow-up. The bias/threshold
//! tables in [`hll_plus_data`](super::hll_plus_data) are transcribed verbatim from the canonical
//! reference data.

use super::hll_plus_data::{BIAS, RAW_ESTIMATE, THRESHOLD};
use crate::common::SketchError;
use std::hash::{Hash, Hasher};
use twox_hash::XxHash64;

/// A HyperLogLog++ distinct-count sketch with precision `p` (`4 ≤ p ≤ 18`).
///
/// # Example
/// ```
/// use sketch_oxide::cardinality::HyperLogLogPlus;
///
/// let mut hll = HyperLogLogPlus::new(14).unwrap();
/// for i in 0..100_000u64 {
///     hll.add(&i);
/// }
/// let est = hll.estimate();
/// assert!((est - 100_000.0).abs() < 0.02 * 100_000.0, "estimate {est}");
/// ```
#[derive(Debug, Clone)]
pub struct HyperLogLogPlus {
    precision: u8,
    registers: Vec<u8>,
}

impl HyperLogLogPlus {
    /// Creates a HyperLogLog++ with precision `precision` (`4..=18`), giving `2^precision` registers
    /// and a relative standard error of about `1.04 / √(2^precision)`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `precision` is outside `4..=18` (the range covered by the
    /// bias-correction tables).
    pub fn new(precision: u8) -> Result<Self, SketchError> {
        if !(4..=18).contains(&precision) {
            return Err(SketchError::InvalidParameter {
                param: "precision".to_string(),
                value: precision.to_string(),
                constraint: "must be in 4..=18".to_string(),
            });
        }
        Ok(Self {
            precision,
            registers: vec![0u8; 1usize << precision],
        })
    }

    /// Number of registers (`2^precision`).
    #[inline]
    pub fn num_registers(&self) -> usize {
        self.registers.len()
    }

    /// Adds one item.
    pub fn add<T: Hash>(&mut self, item: &T) {
        let mut hasher = XxHash64::with_seed(0);
        item.hash(&mut hasher);
        self.update_hash(hasher.finish());
    }

    /// Updates the sketch with a pre-computed 64-bit hash.
    pub fn update_hash(&mut self, hash: u64) {
        let p = self.precision;
        let idx = (hash >> (64 - p)) as usize;
        // Rank of the leftmost set bit in the remaining 64-p bits, +1. Setting bit p-1 of the shifted
        // value bounds the rank at 64-p+1 (the maximum) when the suffix is all zeros.
        let w = (hash << p) | (1u64 << (p - 1));
        let rho = (w.leading_zeros() + 1) as u8;
        if rho > self.registers[idx] {
            self.registers[idx] = rho;
        }
    }

    /// The bias-correction constant `α_m`.
    fn alpha(&self) -> f64 {
        let m = self.registers.len();
        match m {
            16 => 0.673,
            32 => 0.697,
            64 => 0.709,
            _ => 0.7213 / (1.0 + 1.079 / m as f64),
        }
    }

    /// Empirically-measured bias for a raw estimate `e` at precision `p`, by averaging the `BIAS`
    /// values of the `k = 6` nearest `RAW_ESTIMATE` sample points.
    fn estimate_bias(e: f64, p: u8) -> f64 {
        let raw = RAW_ESTIMATE[(p - 4) as usize];
        let bias = BIAS[(p - 4) as usize];
        if e <= raw[0] {
            return bias[0];
        }
        if e >= raw[raw.len() - 1] {
            return bias[raw.len() - 1];
        }
        let idx = raw.partition_point(|&x| x < e);
        let lo = idx.saturating_sub(6);
        let hi = (idx + 6).min(raw.len());
        let mut cand: Vec<(f64, f64)> = (lo..hi).map(|i| ((raw[i] - e).abs(), bias[i])).collect();
        cand.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let k = 6.min(cand.len());
        cand[..k].iter().map(|&(_, b)| b).sum::<f64>() / k as f64
    }

    /// Estimated number of distinct items (paper §4: raw estimate, bias correction, and
    /// LinearCounting below the per-precision threshold).
    pub fn estimate(&self) -> f64 {
        let m = self.registers.len() as f64;
        let sum: f64 = self.registers.iter().map(|&r| 2f64.powi(-(r as i32))).sum();
        let e_raw = self.alpha() * m * m / sum;
        let e = if e_raw <= 5.0 * m {
            e_raw - Self::estimate_bias(e_raw, self.precision)
        } else {
            e_raw
        };

        let v = self.registers.iter().filter(|&&r| r == 0).count();
        let h = if v != 0 {
            m * (m / v as f64).ln() // LinearCounting
        } else {
            e
        };

        if h <= THRESHOLD[(self.precision - 4) as usize] {
            h
        } else {
            e
        }
    }

    /// Merges `other` into `self` (register-wise maximum). Both must share the same precision.
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the precisions differ.
    pub fn merge(&mut self, other: &Self) -> Result<(), SketchError> {
        if self.precision != other.precision {
            return Err(SketchError::IncompatibleSketches {
                reason: "HyperLogLogPlus sketches must share the same precision".to_string(),
            });
        }
        for (a, &b) in self.registers.iter_mut().zip(other.registers.iter()) {
            if b > *a {
                *a = b;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_precision() {
        assert!(HyperLogLogPlus::new(3).is_err());
        assert!(HyperLogLogPlus::new(19).is_err());
        assert!(HyperLogLogPlus::new(4).is_ok());
        assert!(HyperLogLogPlus::new(18).is_ok());
    }

    #[test]
    fn empty_is_zero() {
        let hll = HyperLogLogPlus::new(14).unwrap();
        assert_eq!(hll.estimate(), 0.0);
    }

    fn check(precision: u8, n: u64, rel: f64) {
        let mut hll = HyperLogLogPlus::new(precision).unwrap();
        for i in 0..n {
            hll.add(&i);
        }
        let est = hll.estimate();
        assert!(
            (est - n as f64).abs() <= rel * n as f64,
            "p={precision} n={n}: estimate {est} (rel err {:.4})",
            (est - n as f64).abs() / n as f64
        );
    }

    #[test]
    fn accurate_across_cardinalities() {
        // Relative standard error ≈ 1.04/√m; allow a few sigma.
        check(14, 1_000, 0.03);
        check(14, 100_000, 0.02);
        check(14, 1_000_000, 0.02);
    }

    #[test]
    fn small_cardinality_uses_linear_counting() {
        // Below the threshold, LinearCounting gives tight estimates for small sets.
        check(14, 50, 0.05);
        check(14, 200, 0.05);
        check(16, 100, 0.05);
    }

    #[test]
    fn bias_correction_range() {
        // Cardinalities of a few m are exactly where HLL is biased and HLL++ corrects it.
        check(12, 4_000, 0.03);
        check(12, 8_000, 0.03);
    }

    #[test]
    fn merge_unions_cardinality() {
        let mut a = HyperLogLogPlus::new(14).unwrap();
        let mut b = HyperLogLogPlus::new(14).unwrap();
        for i in 0..60_000u64 {
            a.add(&i);
        }
        for i in 40_000..100_000u64 {
            b.add(&i);
        }
        a.merge(&b).unwrap();
        // Union is 0..100_000 distinct.
        let est = a.estimate();
        assert!(
            (est - 100_000.0).abs() < 0.02 * 100_000.0,
            "merged estimate {est}"
        );
    }

    #[test]
    fn merge_rejects_mismatched_precision() {
        let mut a = HyperLogLogPlus::new(12).unwrap();
        let b = HyperLogLogPlus::new(14).unwrap();
        assert!(a.merge(&b).is_err());
    }
}
