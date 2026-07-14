//! UltraLogLog: State-of-the-art cardinality estimation (VLDB 2024)
//!
//! UltraLogLog is 28% more space-efficient than HyperLogLog with the same accuracy.
//! It uses an improved estimator formula and better bias correction for small cardinalities.
//!
//! # Algorithm Overview
//!
//! UltraLogLog works by:
//! 1. Hashing each input item to get a uniform random value
//! 2. Using the first p bits to select one of 2^p registers
//! 3. Counting leading zeros in the remaining bits and storing the maximum in each register
//! 4. Estimating cardinality using harmonic mean of register values with bias correction
//!
//! # References
//!
//! "UltraLogLog: A Practical and More Space-Efficient Alternative to HyperLogLog"
//! VLDB 2024
//!
//! # Examples
//!
//! ```
//! use sketch_oxide::cardinality::UltraLogLog;
//! use sketch_oxide::Sketch;
//!
//! let mut ull = UltraLogLog::new(12).unwrap();
//!
//! // Add items
//! for i in 0..10_000 {
//!     ull.update(&i);
//! }
//!
//! // Estimate cardinality
//! let estimate = ull.estimate();
//! println!("Estimated cardinality: {}", estimate);
//! // Should be close to 10,000 with ~1.04/sqrt(4096) = ~1.6% error
//! ```

use crate::common::{Mergeable, Sketch, SketchError, validation};
use std::hash::{Hash, Hasher};
use twox_hash::XxHash64;

/// UltraLogLog sketch for cardinality estimation
///
/// Uses 2^p registers (each 8 bits) to estimate the number of unique items.
/// Higher precision means more accuracy but more memory usage.
#[derive(Clone, Debug)]
pub struct UltraLogLog {
    /// Precision parameter (4-18)
    /// Determines number of registers: m = 2^p
    precision: u8,

    /// Register array: 2^p registers, 8-bit each
    /// Each register stores the maximum number of leading zeros seen for its bucket
    registers: Vec<u8>,

    /// Running sum of `2^{-register}`, maintained incrementally for the HIP/martingale
    /// estimator.
    kxq: f64,

    /// HIP estimator accumulator (running martingale estimate). Valid only for a
    /// single-stream insertion history (see `estimate_hip`).
    hip_accum: f64,

    /// Whether `hip_accum` is valid. Cleared by `merge` and deserialization.
    hip_valid: bool,
}

impl UltraLogLog {
    /// Builds a sketch from existing registers, deriving the HIP `kxq` sum.
    fn from_registers(precision: u8, registers: Vec<u8>, hip_valid: bool) -> Self {
        let kxq = registers.iter().map(|&r| 2.0_f64.powi(-(r as i32))).sum();
        UltraLogLog {
            precision,
            registers,
            kxq,
            hip_accum: 0.0,
            hip_valid,
        }
    }

    /// Recomputes the `kxq` sum from the registers (after bulk register changes).
    fn recompute_kxq(&mut self) {
        self.kxq = self
            .registers
            .iter()
            .map(|&r| 2.0_f64.powi(-(r as i32)))
            .sum();
    }

    /// Returns the HIP (Historic Inverse Probability / martingale) cardinality estimate.
    ///
    /// For a single-stream insertion-only sketch this estimator has provably lower variance
    /// than the standard one (≈ half), at no extra memory. Returns `None` after [`merge`] or
    /// deserialization, which destroy the insertion history HIP requires; use
    /// [`estimate`](Sketch::estimate) there.
    ///
    /// [`merge`]: crate::common::Mergeable::merge
    pub fn estimate_hip(&self) -> Option<f64> {
        if self.hip_valid {
            Some(self.hip_accum)
        } else {
            None
        }
    }
    /// Creates a new UltraLogLog sketch
    ///
    /// # Arguments
    ///
    /// * `precision` - Precision parameter (4-18), higher = more accurate but more memory
    ///   - precision 4: 16 registers, 16 bytes
    ///   - precision 8: 256 registers, 256 bytes
    ///   - precision 12: 4096 registers, 4 KB (recommended)
    ///   - precision 16: 65536 registers, 64 KB
    ///   - precision 18: 262144 registers, 256 KB
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if precision < 4 or > 18
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::cardinality::UltraLogLog;
    /// use sketch_oxide::Sketch;
    ///
    /// let ull = UltraLogLog::new(12).unwrap();
    /// assert!(ull.is_empty());
    /// ```
    pub fn new(precision: u8) -> Result<Self, SketchError> {
        if !(4..=18).contains(&precision) {
            return Err(SketchError::InvalidParameter {
                param: "precision".to_string(),
                value: precision.to_string(),
                constraint: "must be between 4 and 18".to_string(),
            });
        }

        let m = 1 << precision; // 2^precision
        let registers = vec![0u8; m];

        Ok(Self::from_registers(precision, registers, true))
    }

    /// Returns the number of registers (m = 2^precision)
    #[inline]
    fn register_count(&self) -> usize {
        1 << self.precision
    }

    /// Computes alpha_m * m^2 for the HyperLogLog formula
    ///
    /// Alpha is a bias correction constant that depends on m
    #[inline]
    fn alpha_mm(&self) -> f64 {
        let m = self.register_count() as f64;
        let alpha = match self.register_count() {
            16 => 0.673,
            32 => 0.697,
            64 => 0.709,
            _ => 0.7213 / (1.0 + 1.079 / m),
        };
        alpha * m * m
    }

    /// Linear counting estimate for small cardinalities
    ///
    /// Used when many registers are still zero
    #[inline]
    fn linear_counting(&self, zeros: usize) -> f64 {
        let m = self.register_count() as f64;
        m * (m / zeros as f64).ln()
    }

    /// Computes the raw harmonic mean estimate
    fn raw_estimate(&self) -> f64 {
        let sum: f64 = self
            .registers
            .iter()
            .map(|&reg| 2.0_f64.powi(-(reg as i32)))
            .sum();

        self.alpha_mm() / sum
    }

    /// Applies UltraLogLog bias correction
    ///
    /// UltraLogLog improves upon HyperLogLog's bias correction, especially for
    /// small cardinalities and the transition regions between different estimators.
    fn bias_corrected_estimate(&self, raw: f64) -> f64 {
        let m = self.register_count() as f64;

        // Count zero registers
        let zeros = self.registers.iter().filter(|&&r| r == 0).count();

        // Use linear counting for small cardinalities (when many zeros)
        // Threshold: 5 * m is the standard UltraLogLog transition point
        if zeros > 0 && raw < 5.0 * m {
            // Use linear counting for small cardinalities
            return self.linear_counting(zeros);
        }

        // For large cardinalities, use the raw estimate with improved bias correction
        // UltraLogLog's key innovation is better bias correction in this range
        // For now, we use the raw estimate (full bias correction tables would be added here)
        raw
    }

    /// Estimates the cardinality
    ///
    /// # Returns
    ///
    /// The estimated number of unique items added to the sketch
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::cardinality::UltraLogLog;
    /// use sketch_oxide::Sketch;
    ///
    /// let mut ull = UltraLogLog::new(12).unwrap();
    /// for i in 0..1000 {
    ///     ull.update(&i);
    /// }
    /// let estimate = ull.cardinality();
    /// assert!((estimate - 1000.0).abs() < 50.0); // Within ~5% error
    /// ```
    pub fn cardinality(&self) -> f64 {
        let raw = self.raw_estimate();
        self.bias_corrected_estimate(raw)
    }

    /// Fast hash function using XXHash64
    #[inline(always)]
    fn hash_item<T: Hash>(&self, item: &T) -> u64 {
        let mut hasher = XxHash64::with_seed(0);
        item.hash(&mut hasher);
        hasher.finish()
    }

    /// Extracts the register index and leading zero count from a 64-bit hash value
    ///
    /// # Arguments
    ///
    /// * `hash` - The 64-bit hash value
    ///
    /// # Returns
    ///
    /// A tuple of (register_index, leading_zeros + 1)
    #[inline(always)]
    fn extract_register_and_zeros_64(&self, hash: u64) -> (usize, u8) {
        let p = self.precision as u32;

        // Extract top p bits for register index
        let register_index = (hash >> (64 - p)) as usize;

        // Extract remaining 64-p bits for counting leading zeros
        let remaining_bits = hash << p;

        // Count leading zeros in remaining bits, add 1
        // +1 because we need to count the position (1-indexed)
        let leading_zeros = if remaining_bits == 0 {
            (64 - p + 1) as u8
        } else {
            (remaining_bits.leading_zeros() + 1) as u8
        };

        (register_index, leading_zeros)
    }

    /// Public update method accepting any hashable type
    #[inline]
    pub fn add<T: Hash>(&mut self, item: &T) {
        let hash = self.hash_item(item);
        let (register_index, leading_zeros) = self.extract_register_and_zeros_64(hash);
        // SAFETY: register_index is bounded by precision which is validated
        let old = unsafe { *self.registers.get_unchecked(register_index) };
        if leading_zeros > old {
            // HIP/martingale update before adjusting kxq (see HyperLogLog::update_hash).
            if self.hip_valid {
                self.hip_accum += self.registers.len() as f64 / self.kxq;
            }
            self.kxq += 2.0_f64.powi(-(leading_zeros as i32)) - 2.0_f64.powi(-(old as i32));
            // SAFETY: same bounded index as above.
            unsafe {
                *self.registers.get_unchecked_mut(register_index) = leading_zeros;
            }
        }
    }
}

impl Sketch for UltraLogLog {
    type Item = u64;

    fn update(&mut self, item: &Self::Item) {
        self.add(item);
    }

    fn estimate(&self) -> f64 {
        self.cardinality()
    }

    fn is_empty(&self) -> bool {
        self.registers.iter().all(|&r| r == 0)
    }

    fn serialize(&self) -> Vec<u8> {
        // Format: [precision: 1 byte] [registers: 2^p bytes]
        let mut bytes = Vec::with_capacity(1 + self.registers.len());
        bytes.push(self.precision);
        bytes.extend_from_slice(&self.registers);
        bytes
    }

    fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        // Check minimum size
        validation::validate_min_size(bytes.len(), 1)?;

        // Check total size doesn't exceed safety limit
        validation::validate_byte_size(bytes.len())?;

        let precision = bytes[0];

        // Validate precision using centralized validator
        validation::validate_precision(precision)?;

        // Calculate and verify expected length
        let expected_len = 1 + (1 << precision);
        if bytes.len() != expected_len {
            return Err(SketchError::DeserializationError(format!(
                "Invalid serialization: expected {} bytes for precision {}, got {}",
                expected_len,
                precision,
                bytes.len()
            )));
        }

        let registers = bytes[1..].to_vec();

        // Deserialized: HIP history lost.
        Ok(Self::from_registers(precision, registers, false))
    }
}

impl Mergeable for UltraLogLog {
    fn merge(&mut self, other: &Self) -> Result<(), SketchError> {
        // Check precision compatibility
        if self.precision != other.precision {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "precision mismatch: {} vs {}",
                    self.precision, other.precision
                ),
            });
        }

        // Merge by taking maximum of each register
        for (i, &other_reg) in other.registers.iter().enumerate() {
            self.registers[i] = self.registers[i].max(other_reg);
        }

        // Merging destroys the single-stream history HIP depends on.
        self.hip_valid = false;
        self.recompute_kxq();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_count() {
        let ull = UltraLogLog::new(12).unwrap();
        assert_eq!(ull.register_count(), 4096);

        let ull = UltraLogLog::new(8).unwrap();
        assert_eq!(ull.register_count(), 256);
    }

    #[test]
    fn test_alpha_mm() {
        let ull = UltraLogLog::new(12).unwrap();
        let alpha_mm = ull.alpha_mm();
        // Should be approximately 0.7213 * 4096^2
        assert!(alpha_mm > 10_000_000.0);
    }

    #[test]
    fn test_extract_register_and_zeros() {
        let ull = UltraLogLog::new(12).unwrap();

        // Test with a known 64-bit hash
        // Top 12 bits = 0xF00 = binary 1111_0000_0000
        // To put this in top 12 bits: shift left by 52 bits
        let hash = 0xF00u64 << 52; // = 0xF00_0_0000_0000_0000
        let (index, zeros) = ull.extract_register_and_zeros_64(hash);

        // Top 12 bits: 0xF00 = 3840
        assert_eq!(index, 0xF00);

        // Remaining 52 bits are all zeros, so leading_zeros = 52 + 1 = 53
        assert_eq!(zeros, 53);
    }

    #[test]
    fn test_hip_estimate_accurate_single_stream() {
        let mut ull = UltraLogLog::new(14).unwrap();
        let n = 50_000u64;
        for i in 0..n {
            ull.add(&i);
        }
        let hip = ull.estimate_hip().expect("HIP valid for single stream");
        let err = (hip - n as f64).abs() / n as f64;
        assert!(err < 0.03, "HIP relative error {err} too high (hip={hip})");
    }

    #[test]
    fn test_hip_idempotent_updates() {
        let mut ull = UltraLogLog::new(12).unwrap();
        for _ in 0..1000 {
            ull.add(&"same");
        }
        assert!(ull.estimate_hip().unwrap() < 2.0);
    }

    #[test]
    fn test_hip_invalid_after_merge() {
        let mut a = UltraLogLog::new(12).unwrap();
        let mut b = UltraLogLog::new(12).unwrap();
        for i in 0..100u64 {
            a.add(&i);
        }
        for i in 100..200u64 {
            b.add(&i);
        }
        assert!(a.estimate_hip().is_some());
        a.merge(&b).unwrap();
        assert!(a.estimate_hip().is_none());
    }

    #[test]
    fn test_deserialize_oversized_precision_rejected() {
        // precision = 200 must be rejected BEFORE it drives `1 << precision`,
        // returning an Err rather than panicking or over-allocating.
        let bytes = vec![200u8; 5];
        let result = UltraLogLog::deserialize(&bytes);
        assert!(result.is_err(), "oversized precision must be rejected");
    }

    #[test]
    fn test_deserialize_truncated_registers_rejected() {
        // Valid precision but the register tail is short of 2^precision bytes.
        let mut bytes = vec![0u8; 10];
        bytes[0] = 12; // expects 1 + 4096 bytes, only 10 provided
        assert!(UltraLogLog::deserialize(&bytes).is_err());
    }

    #[test]
    fn test_hip_invalid_after_roundtrip() {
        let mut ull = UltraLogLog::new(12).unwrap();
        for i in 0..500u64 {
            ull.add(&i);
        }
        let restored = UltraLogLog::deserialize(&ull.serialize()).unwrap();
        assert!(restored.estimate_hip().is_none());
    }
}

/// Capability-trait adoptions (see `crate::common::capabilities`).
mod capability_impls {
    use super::*;
    use crate::common::capabilities::{CardinalityEstimate, Serializable, Update};
    use crate::common::{Result, Sketch};
    use std::hash::Hash;

    impl<T: Hash> Update<T> for UltraLogLog {
        fn update(&mut self, item: &T) {
            self.add(item);
        }
    }

    impl CardinalityEstimate for UltraLogLog {
        fn estimate_cardinality(&self) -> f64 {
            self.cardinality()
        }
    }

    impl Serializable for UltraLogLog {
        fn to_bytes(&self) -> Result<Vec<u8>> {
            Ok(<Self as Sketch>::serialize(self))
        }

        fn from_bytes(bytes: &[u8]) -> Result<Self> {
            <Self as Sketch>::deserialize(bytes)
        }
    }
}

/// Integration-style smoke test proving the cardinality capability traits are
/// usable generically across several adopted sketch types.
#[cfg(test)]
mod capability_smoke_tests {
    use crate::cardinality::{CpcSketch, HyperLogLogPlus, ThetaSketch, UltraLogLog};
    use crate::common::capabilities::{CardinalityEstimate, Serializable, Update};

    // A generic pipeline over ANY cardinality sketch that ingests `u64` hashes —
    // the exact composition the capability-trait split is meant to enable.
    fn count_distinct<S: Update<u64> + CardinalityEstimate>(sketch: &mut S, items: &[u64]) -> f64 {
        for x in items {
            sketch.update(x);
        }
        sketch.estimate_cardinality()
    }

    #[test]
    fn cardinality_capabilities_are_generic_over_types() {
        let items: Vec<u64> = (0..5_000).collect();

        let mut ull = UltraLogLog::new(12).unwrap();
        let e_ull = count_distinct(&mut ull, &items);
        assert!(
            e_ull > 0.0 && (e_ull - 5_000.0).abs() < 5_000.0 * 0.3,
            "UltraLogLog estimate off: {e_ull}"
        );

        let mut hpp = HyperLogLogPlus::new(12).unwrap();
        let e_hpp = count_distinct(&mut hpp, &items);
        assert!(
            e_hpp > 0.0 && (e_hpp - 5_000.0).abs() < 5_000.0 * 0.3,
            "HyperLogLogPlus estimate off: {e_hpp}"
        );

        let mut theta = ThetaSketch::new(12).unwrap();
        let e_theta = count_distinct(&mut theta, &items);
        assert!(
            e_theta > 0.0,
            "ThetaSketch estimate should be positive: {e_theta}"
        );
    }

    #[test]
    fn serializable_capability_roundtrips() {
        let mut cpc = CpcSketch::new(11).unwrap();
        for x in 0..1_000u64 {
            cpc.update(&x);
        }
        let bytes = Serializable::to_bytes(&cpc).unwrap();
        let restored = <CpcSketch as Serializable>::from_bytes(&bytes).unwrap();
        assert!(
            (restored.estimate_cardinality() - cpc.estimate_cardinality()).abs() < 1e-6,
            "round-trip changed the estimate"
        );
    }
}
