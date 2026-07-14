//! ExaLogLog — space-efficient approximate distinct counting up to the exa-scale (Ertl, EDBT 2025).
//!
//! ExaLogLog (ELL) is the latest in the HyperLogLog lineage and the successor to
//! [`UltraLogLog`](crate::cardinality::UltraLogLog): it needs up to 43% less space than HyperLogLog
//! for the same estimation error. It generalises HLL/EHLL/ULL/PCSA with two structural parameters,
//! `t` (which replaces the geometric update-value distribution by `ρ_update(k) = 2^{-φ(k)}`, easy to
//! derive from a 64-bit hash) and `d` (extra per-register bits that *memorise* the occurrence of
//! recent update values, sharpening the estimate). Each of the `m = 2^p` registers packs a `(6+t)`-bit
//! maximum update value `u` and `d` tracking bits.
//!
//! Insertion follows the paper's Algorithm 2: a 64-bit hash yields a register index and an update
//! value `k = nlz(a)·2^t + ⟨t low bits⟩ + 1`; if `k` exceeds the register's max the tracking bits are
//! shifted, and if `k` is within `d` of the max the corresponding tracking bit is set. Cardinality
//! uses the **martingale (HIP) estimator** (Algorithm 4), which is simple, unbiased, and optimal for
//! the non-distributed case: it accumulates `1/μ` on every register change, where `μ = Σ_r h(r)` is
//! the probability that the next distinct element alters some register. The state-change probability
//! `h(r)` is computed exactly from `ρ_update` and the register's tracking bits. Transcribed faithfully
//! from the paper; the alternative maximum-likelihood estimator is a documented follow-up.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use std::hash::Hash;

/// Hashing seed.
const ELL_SEED: u64 = 0xE7A1_0610_6000_0001;

/// An ExaLogLog distinct-count sketch with precision `p`, update parameter `t`, and `d` tracking bits.
///
/// # Example
/// ```
/// use sketch_oxide::cardinality::ExaLogLog;
///
/// // p = 12 ⇒ 4096 registers; the paper's recommended t = 2, d = 20.
/// let mut ell = ExaLogLog::new(12, 2, 20).unwrap();
/// for i in 0..100_000u64 {
///     ell.add(&i);
/// }
/// let est = ell.estimate();
/// assert!((est - 100_000.0).abs() < 0.05 * 100_000.0, "estimate {est}");
/// ```
#[derive(Debug, Clone)]
pub struct ExaLogLog {
    p: u32,
    t: u32,
    d: u32,
    m: usize,
    /// Packed registers: upper `(6+t)` bits = max update value `u`, lower `d` bits = tracking bits.
    registers: Vec<u64>,
    /// `ρ_update(k)` cumulative tail sums: `omega_tail[u] = Σ_{k>u} ρ_update(k)`.
    omega_tail: Vec<f64>,
    /// `ρ_update(k)` for `k` in `1..=k_max`.
    rho: Vec<f64>,
    k_max: u32,
    /// Martingale estimate and running state-change probability.
    estimate: f64,
    mu: f64,
}

impl ExaLogLog {
    /// Creates a sketch with precision `p` (`4 ≤ p ≤ 18`), update parameter `t` (`0 ≤ t ≤ 3`), and
    /// `d` tracking bits (`0 ≤ d ≤ 30`). The paper recommends `t = 2`, `d = 20`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if the parameters are out of range.
    pub fn new(p: u32, t: u32, d: u32) -> Result<Self> {
        if !(4..=18).contains(&p) {
            return Err(SketchError::InvalidParameter {
                param: "p".to_string(),
                value: p.to_string(),
                constraint: "must be in 4..=18".to_string(),
            });
        }
        if t > 3 {
            return Err(SketchError::InvalidParameter {
                param: "t".to_string(),
                value: t.to_string(),
                constraint: "must be in 0..=3".to_string(),
            });
        }
        if d > 30 {
            return Err(SketchError::InvalidParameter {
                param: "d".to_string(),
                value: d.to_string(),
                constraint: "must be in 0..=30".to_string(),
            });
        }
        let m = 1usize << p;
        // Update values range over 1..=(65-p-t)·2^t.
        let k_max = (65 - p - t) * (1 << t);
        // ρ_update(k) = 2^{-φ(k)},  φ(k) = min(t+1+⌊(k-1)/2^t⌋, 64-p).
        let mut rho = vec![0.0f64; (k_max + 1) as usize];
        for k in 1..=k_max {
            let phi = (t + 1 + (k - 1) / (1 << t)).min(64 - p);
            rho[k as usize] = (-(phi as f64) * std::f64::consts::LN_2).exp();
        }
        // omega_tail[u] = Σ_{k=u+1}^{k_max} ρ_update(k).
        let mut omega_tail = vec![0.0f64; (k_max + 1) as usize];
        let mut acc = 0.0;
        for u in (0..k_max).rev() {
            acc += rho[(u + 1) as usize];
            omega_tail[u as usize] = acc;
        }
        Ok(Self {
            p,
            t,
            d,
            m,
            registers: vec![0u64; m],
            omega_tail,
            rho,
            k_max,
            estimate: 0.0,
            mu: 1.0,
        })
    }

    /// Probability `h(r)` that the next distinct element changes register value `r`.
    fn h(&self, r: u64) -> f64 {
        let u = (r >> self.d) as u32; // max update value
        // ω(u): probability of an update value strictly greater than u.
        let mut sum = self.omega_tail[u as usize];
        // Plus updates in [u-d, u-1] whose tracking bit is not yet set.
        if u >= 1 {
            let lo = if u > self.d { u - self.d } else { 1 };
            for k in lo..u {
                let j = u - k; // 1..=d
                let bit = (r >> (self.d - j)) & 1;
                if bit == 0 {
                    sum += self.rho[k as usize];
                }
            }
        }
        sum / self.m as f64
    }

    /// Adds one element.
    pub fn add<T: Hash>(&mut self, item: &T) {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        use std::hash::Hasher;
        item.hash(&mut hasher);
        let hash = xxhash(&hasher.finish().to_le_bytes(), ELL_SEED);

        // Register index: bits [p+t-1 .. t].
        let i = ((hash >> self.t) & ((1u64 << self.p) - 1)) as usize;
        // a = (remaining high bits) followed by (p+t) ones, as a 64-bit value.
        let high = hash >> (self.p + self.t); // top 64-p-t bits
        let ones = (1u64 << (self.p + self.t)) - 1;
        let a = (high << (self.p + self.t)) | ones;
        let nlz = a.leading_zeros();
        // k = nlz·2^t + ⟨t low bits⟩ + 1.
        let low_t = if self.t == 0 {
            0
        } else {
            hash & ((1u64 << self.t) - 1)
        };
        let k = (nlz << self.t) as u64 + low_t + 1;
        let k = k.min(self.k_max as u64);

        let r_old = self.registers[i];
        let u = r_old >> self.d;
        let r_new = if k > u {
            // New maximum: shift the tracking bits and fold the old max in. When delta exceeds the
            // d tracking bits, everything shifts out, so the folded part is 0 (and we avoid an
            // out-of-range shift).
            let delta = k - u;
            let track = r_old & ((1u64 << self.d) - 1);
            let folded = if delta > self.d as u64 {
                0
            } else {
                ((1u64 << self.d) + track) >> delta
            };
            (k << self.d) | folded
        } else if k < u && (self.d as i64 + (k as i64 - u as i64)) >= 0 {
            // k within d of the max: set the corresponding tracking bit.
            let pos = self.d - (u - k) as u32;
            r_old | (1u64 << pos)
        } else {
            r_old
        };

        if r_new != r_old {
            // Martingale (HIP) update: accumulate 1/μ, then lower μ by the register's reduced
            // change probability.
            self.estimate += 1.0 / self.mu;
            self.mu -= self.h(r_old) - self.h(r_new);
            self.registers[i] = r_new;
        }
    }

    /// Unbiased martingale estimate of the number of distinct elements.
    #[inline]
    pub fn estimate(&self) -> f64 {
        self.estimate
    }

    /// Number of registers `m = 2^p`.
    #[inline]
    pub fn registers(&self) -> usize {
        self.m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(ExaLogLog::new(3, 2, 20).is_err());
        assert!(ExaLogLog::new(19, 2, 20).is_err());
        assert!(ExaLogLog::new(12, 4, 20).is_err());
        assert!(ExaLogLog::new(12, 2, 31).is_err());
        assert!(ExaLogLog::new(12, 2, 20).is_ok());
    }

    #[test]
    fn empty_is_zero() {
        let ell = ExaLogLog::new(12, 2, 20).unwrap();
        assert_eq!(ell.estimate(), 0.0);
    }

    fn check_accuracy(p: u32, t: u32, d: u32, n: u64, rel: f64) {
        let mut ell = ExaLogLog::new(p, t, d).unwrap();
        for i in 0..n {
            ell.add(&i);
        }
        let est = ell.estimate();
        assert!(
            (est - n as f64).abs() < rel * n as f64,
            "p={p} t={t} d={d} n={n}: estimate {est}"
        );
    }

    #[test]
    fn accurate_recommended_config() {
        // p=12 ⇒ 4096 registers; martingale RSD ≈ 2.6%. The estimate is deterministic per dataset,
        // so use a generous bound that any *gross* error would still violate.
        check_accuracy(12, 2, 20, 1000, 0.07);
        check_accuracy(12, 2, 20, 100_000, 0.07);
        check_accuracy(12, 2, 20, 1_000_000, 0.07);
    }

    #[test]
    fn accurate_hll_equivalent_config() {
        // t=0, d=0 ⇒ ELL reduces to plain HyperLogLog (with martingale estimation).
        check_accuracy(12, 0, 0, 50_000, 0.07);
        check_accuracy(14, 0, 0, 200_000, 0.04);
    }

    #[test]
    fn unbiased_over_many_datasets() {
        // Averaging over independent datasets drives the variance down, exposing any *systematic*
        // bias (e.g. a tracking-bit error) to within ~1%.
        let n = 50_000u64;
        let runs = 40u64;
        let mut sum = 0.0;
        for s in 0..runs {
            let mut ell = ExaLogLog::new(12, 2, 20).unwrap();
            let offset = s.wrapping_mul(1_000_000_007);
            for i in 0..n {
                ell.add(&offset.wrapping_add(i));
            }
            sum += ell.estimate();
        }
        let mean = sum / runs as f64;
        assert!((mean - n as f64).abs() < 0.015 * n as f64, "mean {mean}");
    }

    #[test]
    fn duplicates_do_not_inflate() {
        let mut ell = ExaLogLog::new(12, 2, 20).unwrap();
        for i in 0..20_000u64 {
            for _ in 0..5 {
                ell.add(&i);
            }
        }
        let est = ell.estimate();
        assert!((est - 20_000.0).abs() < 0.05 * 20_000.0, "estimate {est}");
    }
}

/// Capability-trait adoptions (see `crate::common::capabilities`).
mod capability_impls {
    use super::*;
    use crate::common::capabilities::{CardinalityEstimate, Update};
    use std::hash::Hash;

    impl<T: Hash> Update<T> for ExaLogLog {
        fn update(&mut self, item: &T) {
            self.add(item);
        }
    }

    impl CardinalityEstimate for ExaLogLog {
        fn estimate_cardinality(&self) -> f64 {
            self.estimate()
        }
    }
}
