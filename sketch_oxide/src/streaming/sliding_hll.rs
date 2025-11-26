//! Sliding HyperLogLog: Time-windowed cardinality estimation
//!
//! Sliding HyperLogLog extends the classic HyperLogLog algorithm with temporal
//! awareness, enabling cardinality estimation over sliding time windows. This is
//! essential for real-time analytics, DDoS detection, and streaming applications.
//!
//! # Algorithm Overview
//!
//! The algorithm maintains HyperLogLog registers augmented with timestamp metadata:
//!
//! 1. **Update**: Hash item, update register with both value and timestamp
//! 2. **Window Query**: Filter registers by timestamp, estimate cardinality
//! 3. **Decay**: Remove expired entries to maintain window bounds
//!
//! # Key Innovation: LFPM (List of Future Possible Maxima)
//!
//! While a full LFPM implementation provides O(1) decay, this implementation uses
//! a simpler O(m) approach where m = 2^precision. For typical precision values
//! (4-16), this is still very fast (microseconds).
//!
//! # Time Complexity
//!
//! - Update: O(1)
//! - Window Query: O(m) where m = 2^precision
//! - Decay: O(m)
//! - Merge: O(m)
//!
//! # Space Complexity
//!
//! O(m) where m = 2^precision. Each register stores:
//! - Leading zero count: 1 byte
//! - Timestamp: 8 bytes
//!   Total: ~9m bytes (e.g., 36KB for precision 12)
//!
//! # Production Use Cases (2025)
//!
//! - **Real-time Dashboards**: Unique users in last N minutes
//! - **DDoS Detection**: Unique source IPs in sliding window
//! - **Network Telemetry**: Unique flows over time
//! - **CDN Analytics**: Geographic distribution over time
//! - **Streaming Aggregation**: Time-windowed distinct counts
//!
//! # Examples
//!
//! ```
//! use sketch_oxide::streaming::SlidingHyperLogLog;
//!
//! // Create sketch with precision 12, 1-hour max window
//! let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();
//!
//! // Add items with timestamps
//! hll.update(&"user_123", 1000).unwrap();
//! hll.update(&"user_456", 1030).unwrap();
//! hll.update(&"user_789", 1060).unwrap();
//!
//! // Estimate cardinality in last 60 seconds
//! let estimate = hll.estimate_window(1060, 60);
//! println!("Unique items in window: {:.0}", estimate);
//!
//! // Decay old entries
//! hll.decay(2000, 600).unwrap();
//! ```
//!
//! # References
//!
//! - Chabchoub et al. "Sliding HyperLogLog: Estimating cardinality in a data stream over a sliding window" (2010)
//! - Flajolet et al. "HyperLogLog: the analysis of a near-optimal cardinality estimation algorithm" (2007)

use crate::common::{Mergeable, Sketch, SketchError};
use std::hash::{Hash, Hasher};
use twox_hash::XxHash64;

/// Time-windowed cardinality estimation sketch
///
/// Extends HyperLogLog with temporal awareness for sliding window queries.
/// Maintains registers with timestamp metadata to enable efficient window-based
/// cardinality estimation.
///
/// # Examples
///
/// ```
/// use sketch_oxide::streaming::SlidingHyperLogLog;
///
/// let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();
/// hll.update(&"event_1", 1000).unwrap();
/// hll.update(&"event_2", 1030).unwrap();
///
/// let estimate = hll.estimate_window(1060, 60);
/// println!("Items in last minute: {:.0}", estimate);
/// ```
#[derive(Clone)]
pub struct SlidingHyperLogLog {
    /// Precision parameter (4-16)
    /// Determines number of registers: m = 2^p
    precision: u8,

    /// Registers with timestamp tracking
    /// Each register stores max leading zeros and last update time
    registers: Vec<RegisterWithTime>,

    /// Maximum window size in seconds
    max_window_seconds: u64,

    /// Metadata for tracking and optimization
    metadata: SlidingHLLMetadata,
}

/// Register entry with temporal information
#[derive(Clone, Debug)]
struct RegisterWithTime {
    /// Leading zero count + 1 (rho value)
    value: u8,

    /// Unix timestamp of last update
    timestamp: u64,
}

impl RegisterWithTime {
    fn new() -> Self {
        RegisterWithTime {
            value: 0,
            timestamp: 0,
        }
    }
}

/// Metadata for Sliding HyperLogLog
#[derive(Clone, Debug)]
struct SlidingHLLMetadata {
    /// Last time decay was performed
    last_decay_time: u64,

    /// Total number of updates
    total_updates: u64,
}

impl SlidingHyperLogLog {
    /// Minimum precision value
    pub const MIN_PRECISION: u8 = 4;

    /// Maximum precision value
    pub const MAX_PRECISION: u8 = 16;

    /// Creates a new Sliding HyperLogLog sketch
    ///
    /// # Arguments
    ///
    /// * `precision` - Precision parameter (4-16), higher = more accurate but more memory
    ///   - precision 4: 16 registers, ~144 bytes, ~26% error
    ///   - precision 8: 256 registers, ~2.3 KB, ~6.5% error
    ///   - precision 10: 1024 registers, ~9.2 KB, ~3.25% error
    ///   - precision 12: 4096 registers, ~36 KB, ~1.6% error (recommended)
    ///   - precision 14: 16384 registers, ~147 KB, ~0.8% error
    ///   - precision 16: 65536 registers, ~590 KB, ~0.4% error
    /// * `max_window_seconds` - Maximum window size in seconds
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if precision < 4 or > 16
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::streaming::SlidingHyperLogLog;
    ///
    /// // 1-hour window with precision 12
    /// let hll = SlidingHyperLogLog::new(12, 3600).unwrap();
    /// ```
    pub fn new(precision: u8, max_window_seconds: u64) -> Result<Self, SketchError> {
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
        let registers = vec![RegisterWithTime::new(); m];

        Ok(SlidingHyperLogLog {
            precision,
            registers,
            max_window_seconds,
            metadata: SlidingHLLMetadata {
                last_decay_time: 0,
                total_updates: 0,
            },
        })
    }

    /// Updates the sketch with an item and timestamp
    ///
    /// # Arguments
    ///
    /// * `item` - Any hashable item to add to the sketch
    /// * `timestamp` - Unix timestamp (seconds since epoch)
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::streaming::SlidingHyperLogLog;
    ///
    /// let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();
    /// hll.update(&"user_id", 1000).unwrap();
    /// hll.update(&42, 1030).unwrap();
    /// ```
    pub fn update<T: Hash>(&mut self, item: &T, timestamp: u64) -> Result<(), SketchError> {
        let hash = self.hash_item(item);
        self.update_hash(hash, timestamp);
        self.metadata.total_updates += 1;
        Ok(())
    }

    /// Hashes an item to a 64-bit value using XXHash64
    #[inline(always)]
    fn hash_item<T: Hash>(&self, item: &T) -> u64 {
        let mut hasher = XxHash64::with_seed(0);
        item.hash(&mut hasher);
        hasher.finish()
    }

    /// Updates the sketch with a pre-computed hash value and timestamp
    #[inline]
    fn update_hash(&mut self, hash: u64, timestamp: u64) {
        // Extract register index from first p bits
        let idx = (hash >> (64 - self.precision)) as usize;

        // Compute rho (leading zeros + 1) from remaining bits
        let w = hash << self.precision | (1u64 << (self.precision - 1));
        let rho = (w.leading_zeros() + 1) as u8;

        // Update register if new value is larger OR if same value with newer timestamp
        let register = &mut self.registers[idx];
        if rho > register.value || (rho == register.value && timestamp > register.timestamp) {
            register.value = rho;
            register.timestamp = timestamp;
        }
    }

    /// Estimates cardinality over a time window
    ///
    /// Returns the estimated number of unique items observed within the
    /// time window ending at `current_time` and spanning `window_seconds`.
    ///
    /// # Arguments
    ///
    /// * `current_time` - End of the time window (Unix timestamp)
    /// * `window_seconds` - Size of the window in seconds
    ///
    /// # Returns
    ///
    /// Estimated cardinality for the window
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::streaming::SlidingHyperLogLog;
    ///
    /// let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();
    /// for i in 0..1000 {
    ///     hll.update(&i, 1000 + i).unwrap();
    /// }
    ///
    /// // Estimate for last 100 seconds
    /// let estimate = hll.estimate_window(1500, 100);
    /// ```
    pub fn estimate_window(&self, current_time: u64, window_seconds: u64) -> f64 {
        let cutoff_time = current_time.saturating_sub(window_seconds);

        // Count registers and compute harmonic mean for those in window
        let m = self.registers.len() as f64;
        let mut sum = 0.0;
        let mut zeros = 0;

        for register in &self.registers {
            // Register is in window if: timestamp >= cutoff_time AND timestamp <= current_time
            // AND it has a non-zero value
            if register.value > 0
                && register.timestamp >= cutoff_time
                && register.timestamp <= current_time
            {
                // Register is in window and has data
                sum += 2.0_f64.powi(-(register.value as i32));
            } else {
                // Register is empty or outside window
                zeros += 1;
                sum += 1.0; // 2^0 = 1
            }
        }

        // Apply HyperLogLog estimation with bias correction
        let alpha_m = self.alpha();
        let raw_estimate = alpha_m * m * m / sum;

        // Small range correction using linear counting
        if raw_estimate <= 2.5 * m && zeros > 0 {
            return m * (m / zeros as f64).ln();
        }

        raw_estimate
    }

    /// Estimates total cardinality (all history)
    ///
    /// Returns the estimated number of unique items across all time,
    /// ignoring window constraints.
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::streaming::SlidingHyperLogLog;
    ///
    /// let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();
    /// for i in 0..1000 {
    ///     hll.update(&i, 1000).unwrap();
    /// }
    ///
    /// let total = hll.estimate_total();
    /// assert!((total - 1000.0).abs() < 50.0);
    /// ```
    pub fn estimate_total(&self) -> f64 {
        let m = self.registers.len() as f64;
        let mut sum = 0.0;
        let mut zeros = 0;

        for register in &self.registers {
            if register.value > 0 {
                sum += 2.0_f64.powi(-(register.value as i32));
            } else {
                zeros += 1;
                sum += 1.0;
            }
        }

        let alpha_m = self.alpha();
        let raw_estimate = alpha_m * m * m / sum;

        // Small range correction
        if raw_estimate <= 2.5 * m && zeros > 0 {
            return m * (m / zeros as f64).ln();
        }

        raw_estimate
    }

    /// Explicitly decays old entries
    ///
    /// Removes entries older than the specified window. This is useful for
    /// memory management and maintaining accuracy.
    ///
    /// # Arguments
    ///
    /// * `current_time` - Current timestamp
    /// * `window_seconds` - Window size in seconds
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::streaming::SlidingHyperLogLog;
    ///
    /// let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();
    /// hll.update(&"old_item", 1000).unwrap();
    ///
    /// // Decay old entries
    /// hll.decay(5000, 600).unwrap();
    /// ```
    pub fn decay(&mut self, current_time: u64, window_seconds: u64) -> Result<(), SketchError> {
        let cutoff_time = current_time.saturating_sub(window_seconds);

        for register in &mut self.registers {
            if register.timestamp < cutoff_time {
                // Reset expired register
                register.value = 0;
                register.timestamp = 0;
            }
        }

        self.metadata.last_decay_time = current_time;
        Ok(())
    }

    /// Returns the alpha constant for bias correction
    fn alpha(&self) -> f64 {
        let m = self.registers.len() as f64;
        match self.registers.len() {
            16 => 0.673,
            32 => 0.697,
            64 => 0.709,
            _ => 0.7213 / (1.0 + 1.079 / m),
        }
    }

    /// Returns statistics about the sketch
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::streaming::SlidingHyperLogLog;
    ///
    /// let hll = SlidingHyperLogLog::new(12, 3600).unwrap();
    /// let stats = hll.stats();
    /// assert_eq!(stats.precision, 12);
    /// assert_eq!(stats.max_window_seconds, 3600);
    /// ```
    pub fn stats(&self) -> SlidingHLLStats {
        SlidingHLLStats {
            precision: self.precision,
            max_window_seconds: self.max_window_seconds,
            total_updates: self.metadata.total_updates,
        }
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
    pub fn standard_error(&self) -> f64 {
        1.04 / (self.num_registers() as f64).sqrt()
    }

    /// Serializes the sketch to bytes
    ///
    /// Format: [precision: 1 byte][max_window: 8 bytes][registers: m * 9 bytes][metadata]
    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Header
        bytes.push(self.precision);
        bytes.extend_from_slice(&self.max_window_seconds.to_le_bytes());

        // Registers
        for register in &self.registers {
            bytes.push(register.value);
            bytes.extend_from_slice(&register.timestamp.to_le_bytes());
        }

        // Metadata
        bytes.extend_from_slice(&self.metadata.last_decay_time.to_le_bytes());
        bytes.extend_from_slice(&self.metadata.total_updates.to_le_bytes());

        bytes
    }

    /// Deserializes a sketch from bytes
    pub fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        if bytes.len() < 9 {
            return Err(SketchError::DeserializationError(
                "Insufficient bytes".to_string(),
            ));
        }

        let precision = bytes[0];
        if !(Self::MIN_PRECISION..=Self::MAX_PRECISION).contains(&precision) {
            return Err(SketchError::DeserializationError(format!(
                "Invalid precision: {}",
                precision
            )));
        }

        let max_window_seconds =
            u64::from_le_bytes(bytes[1..9].try_into().map_err(|_| {
                SketchError::DeserializationError("Invalid window size".to_string())
            })?);

        let m = 1usize << precision;
        let expected_len = 9 + m * 9 + 16; // header + registers + metadata

        if bytes.len() != expected_len {
            return Err(SketchError::DeserializationError(format!(
                "Expected {} bytes, got {}",
                expected_len,
                bytes.len()
            )));
        }

        // Parse registers
        let mut registers = Vec::with_capacity(m);
        let mut offset = 9;
        for _ in 0..m {
            let value = bytes[offset];
            let timestamp =
                u64::from_le_bytes(bytes[offset + 1..offset + 9].try_into().map_err(|_| {
                    SketchError::DeserializationError("Invalid timestamp".to_string())
                })?);
            registers.push(RegisterWithTime { value, timestamp });
            offset += 9;
        }

        // Parse metadata
        let last_decay_time = u64::from_le_bytes(
            bytes[offset..offset + 8]
                .try_into()
                .map_err(|_| SketchError::DeserializationError("Invalid metadata".to_string()))?,
        );
        let total_updates = u64::from_le_bytes(
            bytes[offset + 8..offset + 16]
                .try_into()
                .map_err(|_| SketchError::DeserializationError("Invalid metadata".to_string()))?,
        );

        Ok(SlidingHyperLogLog {
            precision,
            registers,
            max_window_seconds,
            metadata: SlidingHLLMetadata {
                last_decay_time,
                total_updates,
            },
        })
    }
}

impl Sketch for SlidingHyperLogLog {
    type Item = u64;

    fn update(&mut self, item: &Self::Item) {
        // Use current timestamp if no explicit timestamp provided
        // This makes it compatible with the Sketch trait
        let _ = self.update(item, 0);
    }

    fn estimate(&self) -> f64 {
        self.estimate_total()
    }

    fn is_empty(&self) -> bool {
        self.registers.iter().all(|r| r.value == 0)
    }

    fn serialize(&self) -> Vec<u8> {
        self.serialize()
    }

    fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        Self::deserialize(bytes)
    }
}

impl Mergeable for SlidingHyperLogLog {
    /// Merges another Sliding HyperLogLog into this one
    ///
    /// Takes the maximum value from each register pair, preserving the timestamp
    /// of the maximum value. Both sketches must have the same precision.
    ///
    /// # Errors
    ///
    /// Returns error if precisions or window sizes don't match
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::streaming::SlidingHyperLogLog;
    /// use sketch_oxide::Mergeable;
    ///
    /// let mut hll1 = SlidingHyperLogLog::new(12, 3600).unwrap();
    /// let mut hll2 = SlidingHyperLogLog::new(12, 3600).unwrap();
    ///
    /// for i in 0..100 {
    ///     hll1.update(&i, 1000).unwrap();
    /// }
    /// for i in 50..150 {
    ///     hll2.update(&i, 1000).unwrap();
    /// }
    ///
    /// hll1.merge(&hll2).unwrap();
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

        if self.max_window_seconds != other.max_window_seconds {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "Window size mismatch: {} vs {}",
                    self.max_window_seconds, other.max_window_seconds
                ),
            });
        }

        for (i, other_reg) in other.registers.iter().enumerate() {
            let self_reg = &mut self.registers[i];

            // Take the maximum value, preserving its timestamp
            if other_reg.value > self_reg.value {
                self_reg.value = other_reg.value;
                self_reg.timestamp = other_reg.timestamp;
            } else if other_reg.value == self_reg.value && other_reg.timestamp > self_reg.timestamp
            {
                // Same value, keep newer timestamp
                self_reg.timestamp = other_reg.timestamp;
            }
        }

        // Update metadata
        self.metadata.total_updates += other.metadata.total_updates;
        self.metadata.last_decay_time = self
            .metadata
            .last_decay_time
            .max(other.metadata.last_decay_time);

        Ok(())
    }
}

/// Statistics for Sliding HyperLogLog
#[derive(Debug, Clone)]
pub struct SlidingHLLStats {
    /// Precision parameter
    pub precision: u8,

    /// Maximum window size in seconds
    pub max_window_seconds: u64,

    /// Total number of updates
    pub total_updates: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_sliding_hll() {
        let hll = SlidingHyperLogLog::new(12, 3600).unwrap();
        let stats = hll.stats();
        assert_eq!(stats.precision, 12);
        assert_eq!(stats.max_window_seconds, 3600);
        assert_eq!(stats.total_updates, 0);
    }

    #[test]
    fn test_invalid_precision() {
        assert!(SlidingHyperLogLog::new(3, 3600).is_err());
        assert!(SlidingHyperLogLog::new(17, 3600).is_err());
    }

    #[test]
    fn test_update_and_estimate() {
        let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();

        for i in 0..100 {
            hll.update(&i, 1000).unwrap();
        }

        let estimate = hll.estimate_total();
        assert!(
            (estimate - 100.0).abs() < 30.0,
            "Estimate {} too far from 100",
            estimate
        );
    }

    #[test]
    fn test_window_estimation() {
        let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();

        for i in 0..50 {
            hll.update(&i, 1000).unwrap();
        }
        for i in 50..100 {
            hll.update(&i, 2000).unwrap();
        }

        // Window covering only second batch
        let estimate = hll.estimate_window(2500, 600);
        assert!(estimate >= 20.0, "Expected items in window");
    }

    #[test]
    fn test_serialization() {
        let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();
        for i in 0..100 {
            hll.update(&i, 1000).unwrap();
        }

        let bytes = hll.serialize();
        let restored = SlidingHyperLogLog::deserialize(&bytes).unwrap();

        assert_eq!(hll.precision, restored.precision);
        assert_eq!(hll.max_window_seconds, restored.max_window_seconds);
    }
}
