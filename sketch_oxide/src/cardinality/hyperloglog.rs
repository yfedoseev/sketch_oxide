//! HyperLogLog: Classic cardinality estimation for ecosystem compatibility
//!
//! HyperLogLog (Flajolet et al. 2007) is the industry standard for cardinality estimation,
//! used by Redis, PostgreSQL, Druid, Spark, ClickHouse, and many other systems.
//!
//! While UltraLogLog is 28% more efficient, HyperLogLog is ubiquitous in production
//! systems. This implementation enables:
//! - Import sketches from Redis/other databases
//! - Export to HLL format for legacy systems
//! - Gradual migration path to UltraLogLog
//!
//! # Algorithm Overview
//!
//! HyperLogLog works by:
//! 1. Hashing each input item to get a uniform random 64-bit value
//! 2. Using the first p bits to select one of 2^p registers
//! 3. Counting leading zeros in the remaining bits + 1, storing max in each register
//! 4. Estimating cardinality using harmonic mean with bias correction
//!
//! # Time Complexity
//!
//! - Update: O(1)
//! - Estimate: O(m) where m = 2^precision
//! - Merge: O(m)
//!
//! # Space Complexity
//!
//! O(2^p) bytes where p is precision (typically 4KB for p=12)
//!
//! # References
//!
//! - Flajolet et al. "HyperLogLog: the analysis of a near-optimal cardinality estimation algorithm" (2007)
//! - Google's HyperLogLog++ improvements (2013)
//!
//! # Examples
//!
//! ```
//! use sketch_oxide::cardinality::HyperLogLog;
//! use sketch_oxide::Sketch;
//!
//! let mut hll = HyperLogLog::new(12).unwrap();
//!
//! // Add items
//! for i in 0..10_000 {
//!     hll.update(&i);
//! }
//!
//! // Estimate cardinality
//! let estimate = hll.estimate();
//! println!("Estimated cardinality: {}", estimate);
//! // Should be close to 10,000 with ~1.04/sqrt(4096) ≈ 1.6% error
//! ```

use crate::common::{validation, Mergeable, Sketch, SketchError};
use std::hash::{Hash, Hasher};
use twox_hash::XxHash64;

/// HyperLogLog sketch for cardinality estimation
///
/// Uses 2^p registers (each 8 bits) to estimate the number of unique items.
/// Higher precision means more accuracy but more memory usage.
///
/// # Examples
///
/// ```
/// use sketch_oxide::cardinality::HyperLogLog;
/// use sketch_oxide::Sketch;
///
/// let mut hll = HyperLogLog::new(14).unwrap();
/// hll.update(&"user_123");
/// hll.update(&"user_456");
/// hll.update(&"user_123"); // Duplicate
///
/// assert!((hll.estimate() - 2.0).abs() < 1.0);
/// ```
#[derive(Clone, Debug)]
pub struct HyperLogLog {
    /// Precision parameter (4-18)
    /// Determines number of registers: m = 2^p
    precision: u8,

    /// Register array: 2^p registers, 8-bit each
    /// Each register stores the maximum rho (leading zeros + 1) seen for its bucket
    registers: Vec<u8>,
}

impl HyperLogLog {
    /// Minimum precision value
    pub const MIN_PRECISION: u8 = 4;

    /// Maximum precision value
    pub const MAX_PRECISION: u8 = 18;

    /// Creates a new HyperLogLog sketch
    ///
    /// # Arguments
    ///
    /// * `precision` - Precision parameter (4-18), higher = more accurate but more memory
    ///   - precision 4: 16 registers, 16 bytes, ~26% error
    ///   - precision 8: 256 registers, 256 bytes, ~6.5% error
    ///   - precision 10: 1024 registers, 1 KB, ~3.25% error
    ///   - precision 12: 4096 registers, 4 KB, ~1.6% error (recommended)
    ///   - precision 14: 16384 registers, 16 KB, ~0.8% error
    ///   - precision 16: 65536 registers, 64 KB, ~0.4% error
    ///   - precision 18: 262144 registers, 256 KB, ~0.2% error
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if precision < 4 or > 18
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::cardinality::HyperLogLog;
    /// use sketch_oxide::Sketch;
    ///
    /// let hll = HyperLogLog::new(12).unwrap();
    /// assert!(hll.is_empty());
    /// ```
    pub fn new(precision: u8) -> Result<Self, SketchError> {
        if !(Self::MIN_PRECISION..=Self::MAX_PRECISION).contains(&precision) {
            return Err(SketchError::InvalidParameter {
                param: "precision".to_string(),
                value: precision.to_string(),
                constraint: format!(
                    "must be between {} and {}",
                    Self::MIN_PRECISION,
                    Self::MAX_PRECISION
                ),
            });
        }

        let m = 1usize << precision;
        let registers = vec![0u8; m];

        Ok(HyperLogLog {
            precision,
            registers,
        })
    }

    /// Returns the precision parameter
    #[inline]
    pub fn precision(&self) -> u8 {
        self.precision
    }

    /// Returns the number of registers (m = 2^precision)
    #[inline]
    pub fn num_registers(&self) -> usize {
        1 << self.precision
    }

    /// Returns the standard error of the estimate
    ///
    /// The standard error is approximately 1.04 / sqrt(m) where m is the number of registers.
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::cardinality::HyperLogLog;
    ///
    /// let hll = HyperLogLog::new(12).unwrap();
    /// assert!((hll.standard_error() - 0.0163).abs() < 0.001);
    /// ```
    pub fn standard_error(&self) -> f64 {
        1.04 / (self.num_registers() as f64).sqrt()
    }

    /// Updates the sketch with a hashable item
    ///
    /// # Arguments
    ///
    /// * `item` - Any hashable item to add to the sketch
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::cardinality::HyperLogLog;
    ///
    /// let mut hll = HyperLogLog::new(12).unwrap();
    /// hll.update(&"hello");
    /// hll.update(&42);
    /// hll.update(&vec![1, 2, 3]);
    /// ```
    pub fn update<T: Hash>(&mut self, item: &T) {
        let hash = self.hash_item(item);
        self.update_hash(hash);
    }

    /// Hashes an item to a 64-bit value using XXHash64 (faster than DefaultHasher)
    #[inline(always)]
    fn hash_item<T: Hash>(&self, item: &T) -> u64 {
        let mut hasher = XxHash64::with_seed(0);
        item.hash(&mut hasher);
        hasher.finish()
    }

    /// Updates the sketch with a pre-computed hash value
    ///
    /// This is useful when you already have a hash or want to use a specific hash function.
    #[inline]
    pub fn update_hash(&mut self, hash: u64) {
        let idx = (hash >> (64 - self.precision)) as usize;
        let w = hash << self.precision | (1u64 << (self.precision - 1));
        let rho = (w.leading_zeros() + 1) as u8;

        if rho > self.registers[idx] {
            self.registers[idx] = rho;
        }
    }

    /// Computes the raw estimate using harmonic mean
    fn raw_estimate(&self) -> f64 {
        let m = self.num_registers() as f64;

        // Compute harmonic mean of 2^(-register[j])
        let sum: f64 = self
            .registers
            .iter()
            .map(|&r| 2.0_f64.powi(-(r as i32)))
            .sum();

        // Alpha_m constant for bias correction
        let alpha_m = self.alpha();

        alpha_m * m * m / sum
    }

    /// Returns the alpha constant for bias correction based on precision
    fn alpha(&self) -> f64 {
        let m = self.num_registers() as f64;
        match self.num_registers() {
            16 => 0.673,
            32 => 0.697,
            64 => 0.709,
            _ => 0.7213 / (1.0 + 1.079 / m),
        }
    }

    /// Counts the number of zero registers
    fn count_zeros(&self) -> usize {
        self.registers.iter().filter(|&&r| r == 0).count()
    }

    /// Linear counting estimate for small cardinalities
    fn linear_counting(&self, zeros: usize) -> f64 {
        let m = self.num_registers() as f64;
        m * (m / zeros as f64).ln()
    }

    /// Serializes the HyperLogLog to bytes
    ///
    /// Format: [precision: 1 byte][registers: m bytes]
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::cardinality::HyperLogLog;
    ///
    /// let mut hll = HyperLogLog::new(12).unwrap();
    /// hll.update(&"test");
    /// let bytes = hll.to_bytes();
    /// let restored = HyperLogLog::from_bytes(&bytes).unwrap();
    /// ```
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(1 + self.registers.len());
        bytes.push(self.precision);
        bytes.extend_from_slice(&self.registers);
        bytes
    }

    /// Deserializes a HyperLogLog from bytes
    ///
    /// # Errors
    ///
    /// Returns error if bytes are invalid, corrupted, or exceed size limits
    ///
    /// # Validation
    ///
    /// - Checks minimum size (at least 1 byte for precision)
    /// - Validates precision is in range [4, 18]
    /// - Validates total serialized size doesn't exceed safety limits
    /// - Checks byte array length matches expected size for precision
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SketchError> {
        // Check minimum size
        validation::validate_min_size(bytes.len(), 1)?;

        // Check total size doesn't exceed safety limit
        validation::validate_byte_size(bytes.len())?;

        let precision = bytes[0];

        // Validate precision using centralized validator
        validation::validate_precision(precision)?;

        // Calculate and verify expected length
        let expected_len = 1 + (1usize << precision);
        if bytes.len() != expected_len {
            return Err(SketchError::DeserializationError(format!(
                "Invalid serialization: expected {} bytes for precision {}, got {}",
                expected_len,
                precision,
                bytes.len()
            )));
        }

        let registers = bytes[1..].to_vec();

        Ok(HyperLogLog {
            precision,
            registers,
        })
    }

    /// Imports from Redis HyperLogLog sparse format
    ///
    /// Redis uses a specific format for HyperLogLog storage.
    /// This method attempts to parse Redis-exported HLL data.
    ///
    /// # Note
    ///
    /// This is a simplified implementation. Full Redis compatibility
    /// would require handling both sparse and dense representations.
    pub fn from_redis_bytes(bytes: &[u8]) -> Result<Self, SketchError> {
        // Redis HLL header: "HYLL" magic + encoding + registers
        if bytes.len() < 4 {
            return Err(SketchError::DeserializationError(
                "Invalid Redis HLL format: too short".to_string(),
            ));
        }

        // Check magic header "HYLL"
        if &bytes[0..4] != b"HYLL" {
            return Err(SketchError::DeserializationError(
                "Invalid Redis HLL format: missing HYLL header".to_string(),
            ));
        }

        // Redis uses precision 14 (16384 registers) by default
        let precision = 14u8;
        let m = 1usize << precision;

        // For dense representation, registers start at offset 16
        if bytes.len() < 16 + m {
            return Err(SketchError::DeserializationError(
                "Invalid Redis HLL format: insufficient data for dense representation".to_string(),
            ));
        }

        // Extract 6-bit registers from Redis dense format
        let mut registers = vec![0u8; m];
        let data = &bytes[16..];

        // Redis packs registers as 6-bit values
        for (i, reg) in registers.iter_mut().enumerate() {
            let byte_idx = (i * 6) / 8;
            let bit_offset = (i * 6) % 8;

            if byte_idx + 1 < data.len() {
                let val = if bit_offset <= 2 {
                    (data[byte_idx] >> bit_offset) & 0x3F
                } else {
                    ((data[byte_idx] >> bit_offset) | (data[byte_idx + 1] << (8 - bit_offset)))
                        & 0x3F
                };
                *reg = val;
            }
        }

        Ok(HyperLogLog {
            precision,
            registers,
        })
    }

    /// Exports to Redis-compatible format
    ///
    /// Creates a dense representation compatible with Redis PFADD/PFCOUNT.
    pub fn to_redis_bytes(&self) -> Vec<u8> {
        let m = self.num_registers();

        // Header: "HYLL" + encoding byte (1 = dense) + cardinality cache (8 bytes) + padding
        let mut bytes = Vec::with_capacity(16 + (m * 6).div_ceil(8));

        // Magic header
        bytes.extend_from_slice(b"HYLL");

        // Encoding: 1 = dense
        bytes.push(1);

        // Reserved/cardinality cache (11 bytes)
        bytes.extend_from_slice(&[0u8; 11]);

        // Pack 6-bit registers
        let packed_len = (m * 6).div_ceil(8);
        let mut packed = vec![0u8; packed_len];

        for (i, &reg) in self.registers.iter().enumerate() {
            let val = reg.min(63); // 6-bit max
            let bit_idx = i * 6;
            let byte_idx = bit_idx / 8;
            let bit_offset = bit_idx % 8;

            packed[byte_idx] |= val << bit_offset;
            if bit_offset > 2 && byte_idx + 1 < packed_len {
                packed[byte_idx + 1] |= val >> (8 - bit_offset);
            }
        }

        bytes.extend_from_slice(&packed);
        bytes
    }

    /// Returns a reference to the internal registers
    ///
    /// This is useful for debugging or implementing custom operations.
    pub fn registers(&self) -> &[u8] {
        &self.registers
    }
}

impl Sketch for HyperLogLog {
    type Item = u64;

    fn update(&mut self, item: &Self::Item) {
        self.update(item);
    }

    /// Estimates the cardinality with HyperLogLog++ corrections
    ///
    /// Applies small and large range corrections for improved accuracy.
    fn estimate(&self) -> f64 {
        let m = self.num_registers() as f64;
        let raw = self.raw_estimate();

        // Small range correction using linear counting
        if raw <= 2.5 * m {
            let zeros = self.count_zeros();
            if zeros > 0 {
                return self.linear_counting(zeros);
            }
        }

        // Large range correction (for 32-bit hash, not needed for 64-bit)
        // With 64-bit hashes, this threshold is effectively never reached
        let two_pow_32 = (1u64 << 32) as f64;
        if raw > two_pow_32 / 30.0 {
            return -two_pow_32 * (1.0 - raw / two_pow_32).ln();
        }

        raw
    }

    fn is_empty(&self) -> bool {
        self.registers.iter().all(|&r| r == 0)
    }

    fn serialize(&self) -> Vec<u8> {
        self.to_bytes()
    }

    fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        Self::from_bytes(bytes)
    }
}

impl Mergeable for HyperLogLog {
    /// Merges another HyperLogLog into this one
    ///
    /// Takes the maximum of each register pair. Both sketches must have
    /// the same precision.
    ///
    /// # Errors
    ///
    /// Returns error if precisions don't match
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::cardinality::HyperLogLog;
    /// use sketch_oxide::{Sketch, Mergeable};
    ///
    /// let mut hll1 = HyperLogLog::new(12).unwrap();
    /// let mut hll2 = HyperLogLog::new(12).unwrap();
    ///
    /// for i in 0..1000 {
    ///     hll1.update(&i);
    /// }
    /// for i in 500..1500 {
    ///     hll2.update(&i);
    /// }
    ///
    /// hll1.merge(&hll2).unwrap();
    /// // Should estimate ~1500 unique items
    /// ```
    fn merge(&mut self, other: &Self) -> Result<(), SketchError> {
        if self.precision != other.precision {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "Precision mismatch: {} vs {}",
                    self.precision, other.precision
                ),
            });
        }

        for (i, &other_reg) in other.registers.iter().enumerate() {
            if other_reg > self.registers[i] {
                self.registers[i] = other_reg;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_hyperloglog() {
        let hll = HyperLogLog::new(12).unwrap();
        assert!(hll.is_empty());
        assert_eq!(hll.precision(), 12);
        assert_eq!(hll.num_registers(), 4096);
    }

    #[test]
    fn test_invalid_precision() {
        assert!(HyperLogLog::new(3).is_err());
        assert!(HyperLogLog::new(19).is_err());
        assert!(HyperLogLog::new(4).is_ok());
        assert!(HyperLogLog::new(18).is_ok());
    }

    #[test]
    fn test_update() {
        let mut hll = HyperLogLog::new(12).unwrap();
        hll.update(&"hello");
        assert!(!hll.is_empty());
    }

    #[test]
    fn test_estimate_small() {
        let mut hll = HyperLogLog::new(12).unwrap();
        for i in 0..100 {
            hll.update(&i);
        }
        let estimate = hll.estimate();
        assert!(
            (estimate - 100.0).abs() < 20.0,
            "Estimate {} too far from 100",
            estimate
        );
    }

    #[test]
    fn test_estimate_medium() {
        let mut hll = HyperLogLog::new(12).unwrap();
        for i in 0..10_000 {
            hll.update(&i);
        }
        let estimate = hll.estimate();
        let error = (estimate - 10_000.0).abs() / 10_000.0;
        assert!(error < 0.05, "Error {} too high", error);
    }

    #[test]
    fn test_merge() {
        let mut hll1 = HyperLogLog::new(12).unwrap();
        let mut hll2 = HyperLogLog::new(12).unwrap();

        for i in 0..1000 {
            hll1.update(&i);
        }
        for i in 500..1500 {
            hll2.update(&i);
        }

        hll1.merge(&hll2).unwrap();
        let estimate = hll1.estimate();
        let error = (estimate - 1500.0).abs() / 1500.0;
        assert!(
            error < 0.1,
            "Merged estimate {} too far from 1500",
            estimate
        );
    }

    #[test]
    fn test_merge_precision_mismatch() {
        let mut hll1 = HyperLogLog::new(10).unwrap();
        let hll2 = HyperLogLog::new(12).unwrap();
        assert!(hll1.merge(&hll2).is_err());
    }

    #[test]
    fn test_serialization() {
        let mut hll = HyperLogLog::new(12).unwrap();
        for i in 0..1000 {
            hll.update(&i);
        }

        let bytes = hll.to_bytes();
        let restored = HyperLogLog::from_bytes(&bytes).unwrap();

        assert_eq!(hll.precision, restored.precision);
        assert_eq!(hll.registers, restored.registers);
    }

    #[test]
    fn test_standard_error() {
        let hll = HyperLogLog::new(12).unwrap();
        let se = hll.standard_error();
        // 1.04 / sqrt(4096) ≈ 0.01625
        assert!((se - 0.01625).abs() < 0.001);
    }

    #[test]
    fn test_idempotent_updates() {
        let mut hll = HyperLogLog::new(12).unwrap();
        for _ in 0..1000 {
            hll.update(&"same_item");
        }
        let estimate = hll.estimate();
        assert!(
            estimate < 2.0,
            "Duplicate updates should not increase count"
        );
    }
}
