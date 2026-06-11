//! HyperLogLog joint estimators — set operations over two HLL sketches.
//!
//! A HyperLogLog summarizes *one* set's cardinality, but because its registers hold the maximum
//! observed rank per bucket, the register-wise **maximum** of two sketches is exactly the HLL of
//! their **union**. From the union and the two individual cardinalities, inclusion–exclusion gives
//! the intersection and the Jaccard similarity:
//!
//! ```text
//! |A ∪ B| = HLL(max(regA, regB))
//! |A ∩ B| = |A| + |B| − |A ∪ B|
//! J(A, B) = |A ∩ B| / |A ∪ B|
//! ```
//!
//! To keep inclusion–exclusion coherent, all four cardinalities are computed with the *same*
//! register-based HLL estimator (the classic bias-corrected harmonic-mean estimator with linear
//! counting for small ranges), rather than mixing in the sketches' own incremental estimates.
//!
//! Inclusion–exclusion amplifies relative error when the intersection is small relative to the
//! inputs, so use high-precision sketches (p ≥ 12) when the intersection matters.

use crate::cardinality::HyperLogLog;
use crate::common::{Result, SketchError};

/// Joint (two-set) estimator over a pair of HyperLogLog sketches of equal precision.
///
/// # Example
/// ```
/// use sketch_oxide::cardinality::HyperLogLog;
/// use sketch_oxide::statistics::HllJointEstimator;
///
/// let mut a = HyperLogLog::new(14).unwrap();
/// let mut b = HyperLogLog::new(14).unwrap();
/// for i in 0..10_000u64 { a.update(&i); }        // A = [0, 10000)
/// for i in 5_000..15_000u64 { b.update(&i); }    // B = [5000, 15000)
///
/// let j = HllJointEstimator::new(&a, &b).unwrap();
/// // |A ∪ B| ≈ 15000, |A ∩ B| ≈ 5000, J ≈ 1/3.
/// assert!((j.jaccard() - 0.3333).abs() < 0.05, "jaccard {}", j.jaccard());
/// ```
pub struct HllJointEstimator<'a> {
    a: &'a HyperLogLog,
    b: &'a HyperLogLog,
}

impl<'a> HllJointEstimator<'a> {
    /// Creates a joint estimator over two sketches.
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the two sketches have different precision.
    pub fn new(a: &'a HyperLogLog, b: &'a HyperLogLog) -> Result<Self> {
        if a.precision() != b.precision() {
            return Err(SketchError::IncompatibleSketches {
                reason: format!("precision mismatch: {} vs {}", a.precision(), b.precision()),
            });
        }
        Ok(Self { a, b })
    }

    /// The HLL bias constant `α_m`.
    fn alpha(m: usize) -> f64 {
        match m {
            16 => 0.673,
            32 => 0.697,
            64 => 0.709,
            _ => 0.7213 / (1.0 + 1.079 / m as f64),
        }
    }

    /// Classic register-based HLL cardinality estimate (harmonic mean + linear-counting small-range
    /// correction).
    fn estimate_registers(regs: &[u8]) -> f64 {
        let m = regs.len();
        let sum: f64 = regs.iter().map(|&r| 2.0_f64.powi(-(r as i32))).sum();
        let raw = Self::alpha(m) * (m * m) as f64 / sum;
        if raw <= 2.5 * m as f64 {
            let zeros = regs.iter().filter(|&&r| r == 0).count();
            if zeros > 0 {
                // Linear counting is more accurate when many registers are still empty.
                return m as f64 * (m as f64 / zeros as f64).ln();
            }
        }
        raw
    }

    /// Estimated cardinality of `A`.
    pub fn cardinality_a(&self) -> f64 {
        Self::estimate_registers(self.a.registers())
    }

    /// Estimated cardinality of `B`.
    pub fn cardinality_b(&self) -> f64 {
        Self::estimate_registers(self.b.registers())
    }

    /// Estimated cardinality of `A ∪ B` (register-wise max).
    pub fn union(&self) -> f64 {
        let union: Vec<u8> = self
            .a
            .registers()
            .iter()
            .zip(self.b.registers())
            .map(|(&x, &y)| x.max(y))
            .collect();
        Self::estimate_registers(&union)
    }

    /// Estimated cardinality of `A ∩ B` via inclusion–exclusion (clamped at 0).
    pub fn intersection(&self) -> f64 {
        (self.cardinality_a() + self.cardinality_b() - self.union()).max(0.0)
    }

    /// Estimated Jaccard similarity `|A ∩ B| / |A ∪ B|`.
    pub fn jaccard(&self) -> f64 {
        let u = self.union();
        if u <= 0.0 {
            0.0
        } else {
            (self.intersection() / u).clamp(0.0, 1.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hll_of(range: std::ops::Range<u64>, p: u8) -> HyperLogLog {
        let mut h = HyperLogLog::new(p).unwrap();
        for i in range {
            h.update(&i);
        }
        h
    }

    #[test]
    fn precision_mismatch_errors() {
        let a = HyperLogLog::new(12).unwrap();
        let b = HyperLogLog::new(14).unwrap();
        assert!(HllJointEstimator::new(&a, &b).is_err());
    }

    #[test]
    fn half_overlap() {
        let a = hll_of(0..10_000, 14);
        let b = hll_of(5_000..15_000, 14);
        let j = HllJointEstimator::new(&a, &b).unwrap();
        assert!(
            (j.union() - 15_000.0).abs() < 0.04 * 15_000.0,
            "union {}",
            j.union()
        );
        assert!(
            (j.intersection() - 5_000.0).abs() < 0.2 * 5_000.0,
            "inter {}",
            j.intersection()
        );
        assert!(
            (j.jaccard() - 0.3333).abs() < 0.05,
            "jaccard {}",
            j.jaccard()
        );
    }

    #[test]
    fn disjoint_sets() {
        let a = hll_of(0..10_000, 14);
        let b = hll_of(1_000_000..1_010_000, 14);
        let j = HllJointEstimator::new(&a, &b).unwrap();
        assert!(
            (j.union() - 20_000.0).abs() < 0.04 * 20_000.0,
            "union {}",
            j.union()
        );
        // Intersection of disjoint sets is ~0 (small relative to inputs).
        assert!(
            j.intersection() < 0.1 * 10_000.0,
            "inter {}",
            j.intersection()
        );
        assert!(j.jaccard() < 0.1, "jaccard {}", j.jaccard());
    }

    #[test]
    fn identical_sets() {
        let a = hll_of(0..10_000, 14);
        let b = hll_of(0..10_000, 14);
        let j = HllJointEstimator::new(&a, &b).unwrap();
        assert!(j.jaccard() > 0.9, "jaccard {}", j.jaccard());
        assert!(
            (j.intersection() - 10_000.0).abs() < 0.1 * 10_000.0,
            "inter {}",
            j.intersection()
        );
    }

    #[test]
    fn individual_cardinalities_reasonable() {
        let a = hll_of(0..50_000, 14);
        let b = hll_of(0..3_000, 14);
        let j = HllJointEstimator::new(&a, &b).unwrap();
        assert!((j.cardinality_a() - 50_000.0).abs() < 0.05 * 50_000.0);
        assert!((j.cardinality_b() - 3_000.0).abs() < 0.05 * 3_000.0);
    }

    #[test]
    fn subset_relationship() {
        // B ⊂ A: intersection ≈ |B|, union ≈ |A|.
        let a = hll_of(0..20_000, 14);
        let b = hll_of(0..4_000, 14);
        let j = HllJointEstimator::new(&a, &b).unwrap();
        assert!(
            (j.union() - 20_000.0).abs() < 0.05 * 20_000.0,
            "union {}",
            j.union()
        );
        assert!(
            (j.intersection() - 4_000.0).abs() < 0.3 * 4_000.0,
            "inter {}",
            j.intersection()
        );
    }
}
