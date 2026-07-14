//! Exponential Histogram (Enhanced) with Formal Error Bounds
//!
//! A more sophisticated implementation of the Exponential Histogram algorithm
//! (Datar et al. 2002) that provides formal error guarantees and confidence bounds.
//!
//! # Algorithm Overview
//!
//! The Exponential Histogram maintains buckets of exponentially increasing sizes
//! (powers of 2). For each bucket size, we keep at most `k = ceil(1/epsilon)` buckets
//! to maintain the l-canonical form. This structure guarantees that the relative
//! error is bounded by epsilon.
//!
//! # Properties
//!
//! - Space: O((1/epsilon) * log(window_size)) buckets
//! - Query time: O(number of buckets)
//! - Update time: O(log(window_size))
//! - Relative error: at most epsilon
//!
//! # Error Analysis
//!
//! The error comes solely from the oldest partial bucket that straddles the window
//! boundary. By counting half of this bucket, the maximum error is half the largest
//! bucket size. The l-canonical form ensures this is bounded by epsilon * total_count.
//!
//! # References
//!
//! - Datar, Gionis, Indyk, Motwani. "Maintaining Stream Statistics over Sliding Windows"
//!   (SODA 2002)

use crate::common::{Mergeable, Result, Sketch, SketchError};
use crate::streaming::eh_core::{EhBucket, EhCore};

// ============================================================================
// TESTS FIRST (TDD Approach)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Test 1: Basic Insert
    // -------------------------------------------------------------------------
    #[test]
    fn test_basic_insert() {
        let mut eh = ExponentialHistogram::new(1000, 0.1).unwrap();

        // Insert single event
        eh.insert(100, 1);
        assert!(!eh.is_empty());
        assert!(eh.num_buckets() >= 1);

        // Insert multiple events
        eh.insert(200, 1);
        eh.insert(300, 1);
        assert!(eh.num_buckets() >= 1);
    }

    // -------------------------------------------------------------------------
    // Test 2: Events In Window
    // -------------------------------------------------------------------------
    #[test]
    fn test_events_in_window() {
        let mut eh = ExponentialHistogram::new(1000, 0.1).unwrap();

        // Add events at timestamps 100, 200, 300
        eh.insert(100, 1);
        eh.insert(200, 1);
        eh.insert(300, 1);

        // At time 500, window covers [0, 500] since window_size=1000
        // All events should be counted
        let (estimate, lower, upper) = eh.count(500);

        assert!(estimate >= 2, "Estimate {} should be >= 2", estimate);
        assert!(lower <= 3, "Lower {} should be <= 3", lower);
        assert!(upper >= 3, "Upper {} should be >= 3", upper);
    }

    // -------------------------------------------------------------------------
    // Test 3: Events Outside Window
    // -------------------------------------------------------------------------
    #[test]
    fn test_events_outside_window() {
        let mut eh = ExponentialHistogram::new(100, 0.1).unwrap();

        // Add events at different timestamps
        eh.insert(10, 1); // Will be outside window at time 200
        eh.insert(20, 1); // Will be outside window at time 200
        eh.insert(150, 1); // Inside window at time 200
        eh.insert(180, 1); // Inside window at time 200

        // At time 200, window covers [100, 200]
        // Only events at 150 and 180 should be fully counted
        let (estimate, _lower, _upper) = eh.count(200);

        // Should be approximately 2 (the in-window events)
        // with possibly partial contribution from straddling bucket
        assert!(estimate >= 1, "Estimate {} should be >= 1", estimate);
        assert!(estimate <= 4, "Estimate {} should be <= 4", estimate);
    }

    // -------------------------------------------------------------------------
    // Test 4: Error Bounds
    // -------------------------------------------------------------------------
    #[test]
    fn test_error_bounds() {
        let epsilon = 0.1;
        let mut eh = ExponentialHistogram::new(10000, epsilon).unwrap();

        // Insert many events
        let actual_count = 100;
        for i in 0..actual_count {
            eh.insert(i * 10, 1);
        }

        // Query at time that includes all events
        let (estimate, lower, upper) = eh.count(actual_count * 10);

        // Verify bounds are consistent
        assert!(
            lower <= estimate,
            "Lower {} should be <= estimate {}",
            lower,
            estimate
        );
        assert!(
            estimate <= upper,
            "Estimate {} should be <= upper {}",
            estimate,
            upper
        );

        // Verify relative error is bounded by epsilon
        let relative_error = (estimate as f64 - actual_count as f64).abs() / actual_count as f64;
        assert!(
            relative_error <= epsilon + 0.05, // Small slack for edge cases
            "Relative error {} should be <= epsilon {} + slack",
            relative_error,
            epsilon
        );
    }

    // -------------------------------------------------------------------------
    // Test 5: Bucket Compression
    // -------------------------------------------------------------------------
    #[test]
    fn test_bucket_compression() {
        let epsilon = 0.5; // k = ceil(1/0.5) = 2
        let mut eh = ExponentialHistogram::new(10000, epsilon).unwrap();

        // Insert many events to trigger compression
        for i in 0..100 {
            eh.insert(i * 10, 1);
        }

        // With k=2, we should have at most 2+1=3 buckets per size
        // Total buckets should be O(k * log(count)) = O(3 * 7) ~ 21
        let num_buckets = eh.num_buckets();
        assert!(
            num_buckets < 50,
            "Expected compressed buckets < 50, got {}",
            num_buckets
        );

        // Verify space efficiency
        let expected_max = ((1.0_f64 / epsilon).ceil() as usize + 1) * 20; // generous bound
        assert!(
            num_buckets <= expected_max,
            "Buckets {} exceed expected max {}",
            num_buckets,
            expected_max
        );
    }

    // -------------------------------------------------------------------------
    // Test 6: Monotonic Timestamps
    // -------------------------------------------------------------------------
    #[test]
    fn test_monotonic_timestamps() {
        let mut eh = ExponentialHistogram::new(1000, 0.1).unwrap();

        eh.insert(100, 1);
        eh.insert(200, 1);

        // Inserting at an earlier timestamp should still work
        // (the implementation should handle this gracefully)
        eh.insert(150, 1);

        // All events should still be countable
        let (estimate, _, _) = eh.count(300);
        assert!(
            estimate >= 2,
            "Should count events even with non-monotonic insertion"
        );
    }

    // -------------------------------------------------------------------------
    // Test 7: Empty Window
    // -------------------------------------------------------------------------
    #[test]
    fn test_empty_window() {
        let eh = ExponentialHistogram::new(100, 0.1).unwrap();

        assert!(eh.is_empty());

        let (estimate, lower, upper) = eh.count(1000);
        assert_eq!(estimate, 0);
        assert_eq!(lower, 0);
        assert_eq!(upper, 0);
    }

    // -------------------------------------------------------------------------
    // Test 8: Single Event
    // -------------------------------------------------------------------------
    #[test]
    fn test_single_event() {
        let mut eh = ExponentialHistogram::new(100, 0.1).unwrap();

        eh.insert(50, 1);

        // Query when event is in window
        let (estimate, lower, upper) = eh.count(100);
        assert_eq!(estimate, 1, "Single event should be counted exactly");
        assert!(lower <= 1);
        assert!(upper >= 1);

        // Query when event is outside window
        let (estimate_outside, _, _) = eh.count(200);
        assert!(estimate_outside <= 1, "Event should be expired or partial");
    }

    // -------------------------------------------------------------------------
    // Test 9: Merge Windows
    // -------------------------------------------------------------------------
    #[test]
    fn test_merge_windows() {
        let mut eh1 = ExponentialHistogram::new(1000, 0.1).unwrap();
        let mut eh2 = ExponentialHistogram::new(1000, 0.1).unwrap();

        // Add events to first histogram
        eh1.insert(100, 1);
        eh1.insert(200, 1);

        // Add events to second histogram
        eh2.insert(300, 1);
        eh2.insert(400, 1);

        // Merge
        eh1.merge(&eh2).unwrap();

        // Should contain events from both
        let (estimate, _, _) = eh1.count(500);
        assert!(
            estimate >= 3,
            "Merged histogram should have >= 3 events, got {}",
            estimate
        );
    }

    // -------------------------------------------------------------------------
    // Test 10: Merge Incompatible Histograms
    // -------------------------------------------------------------------------
    #[test]
    fn test_merge_incompatible() {
        let mut eh1 = ExponentialHistogram::new(1000, 0.1).unwrap();
        let eh2 = ExponentialHistogram::new(500, 0.1).unwrap(); // Different window size

        let result = eh1.merge(&eh2);
        assert!(
            result.is_err(),
            "Should fail to merge incompatible histograms"
        );
    }

    // -------------------------------------------------------------------------
    // Test 11: Invalid Parameters
    // -------------------------------------------------------------------------
    #[test]
    fn test_invalid_parameters() {
        // Zero window size
        assert!(ExponentialHistogram::new(0, 0.1).is_err());

        // Zero epsilon
        assert!(ExponentialHistogram::new(1000, 0.0).is_err());

        // Epsilon >= 1
        assert!(ExponentialHistogram::new(1000, 1.0).is_err());
        assert!(ExponentialHistogram::new(1000, 1.5).is_err());

        // Negative epsilon
        assert!(ExponentialHistogram::new(1000, -0.1).is_err());
    }

    // -------------------------------------------------------------------------
    // Test 12: Insert with Count > 1
    // -------------------------------------------------------------------------
    #[test]
    fn test_insert_multiple_count() {
        let mut eh = ExponentialHistogram::new(1000, 0.1).unwrap();

        // Insert 5 events at once
        eh.insert(100, 5);

        let (estimate, _, _) = eh.count(200);
        assert!(
            estimate >= 4,
            "Should count approximately 5 events, got {}",
            estimate
        );
        assert!(estimate <= 6, "Should not over-count, got {}", estimate);
    }

    // -------------------------------------------------------------------------
    // Test 13: Expire Old Buckets
    // -------------------------------------------------------------------------
    #[test]
    fn test_expire() {
        let mut eh = ExponentialHistogram::new(100, 0.1).unwrap();

        eh.insert(10, 1);
        eh.insert(20, 1);
        eh.insert(200, 1);
        eh.insert(210, 1);

        let buckets_before = eh.num_buckets();
        eh.expire(300);
        let buckets_after = eh.num_buckets();

        // Should have removed old buckets (except possibly one straddling)
        assert!(
            buckets_after <= buckets_before,
            "Expire should not increase buckets"
        );
    }

    // -------------------------------------------------------------------------
    // Test 14: Serialization
    // -------------------------------------------------------------------------
    #[test]
    fn test_serialization() {
        let mut eh = ExponentialHistogram::new(1000, 0.1).unwrap();

        eh.insert(100, 1);
        eh.insert(200, 2);
        eh.insert(300, 3);

        let bytes = eh.serialize();
        let restored = ExponentialHistogram::deserialize(&bytes).unwrap();

        assert_eq!(eh.window_size(), restored.window_size());
        assert_eq!(eh.epsilon(), restored.epsilon());
        assert_eq!(eh.num_buckets(), restored.num_buckets());

        // Counts should match
        let (est1, _, _) = eh.count(400);
        let (est2, _, _) = restored.count(400);
        assert_eq!(est1, est2);
    }

    // -------------------------------------------------------------------------
    // Test 15: Sketch Trait
    // -------------------------------------------------------------------------
    #[test]
    fn test_sketch_trait() {
        let mut eh = ExponentialHistogram::new(1000, 0.1).unwrap();

        // Test update (using Sketch trait explicitly — the capability `Update`
        // impl also matches this item type, so disambiguate).
        Sketch::update(&mut eh, &(100u64, 1u64));
        Sketch::update(&mut eh, &(200u64, 1u64));

        // Test estimate
        let estimate = eh.estimate();
        assert!(
            estimate >= 1.0,
            "Estimate should be >= 1.0, got {}",
            estimate
        );
    }

    // -------------------------------------------------------------------------
    // Test 16: Large Count Decomposition
    // -------------------------------------------------------------------------
    #[test]
    fn test_large_count_decomposition() {
        let mut eh = ExponentialHistogram::new(10000, 0.1).unwrap();

        // Insert a large count that should be decomposed into powers of 2
        eh.insert(100, 15); // 15 = 8 + 4 + 2 + 1 = 4 buckets

        let (estimate, _, _) = eh.count(200);
        assert!(estimate >= 14, "Should count ~15 events, got {}", estimate);
        assert!(estimate <= 16, "Should not over-count, got {}", estimate);
    }

    // -------------------------------------------------------------------------
    // Test 17: Bounds Correctness
    // -------------------------------------------------------------------------
    #[test]
    fn test_bounds_correctness() {
        let epsilon = 0.2;
        let mut eh = ExponentialHistogram::new(1000, epsilon).unwrap();

        for i in 0..50 {
            eh.insert(i * 10, 1);
        }

        let (estimate, lower, upper) = eh.count(600);

        // Lower bound should be achievable (conservative)
        // Upper bound should be achievable (optimistic)
        assert!(lower <= estimate);
        assert!(estimate <= upper);

        // The actual count could be anywhere in [lower, upper]
        // For 50 events in window, bounds should be reasonable
        assert!(lower >= 30, "Lower bound {} too low", lower);
        assert!(upper <= 70, "Upper bound {} too high", upper);
    }

    // -------------------------------------------------------------------------
    // Test 18: Memory Usage
    // -------------------------------------------------------------------------
    #[test]
    fn test_memory_usage() {
        let eh = ExponentialHistogram::new(1000, 0.1).unwrap();
        let base_memory = eh.memory_usage();
        assert!(base_memory > 0);

        let mut eh2 = ExponentialHistogram::new(1000, 0.1).unwrap();
        for i in 0..100 {
            eh2.insert(i * 10, 1);
        }

        let with_data_memory = eh2.memory_usage();
        assert!(with_data_memory > base_memory);
    }

    // -------------------------------------------------------------------------
    // Test 19: Clone
    // -------------------------------------------------------------------------
    #[test]
    fn test_clone() {
        let mut eh = ExponentialHistogram::new(1000, 0.1).unwrap();
        eh.insert(100, 5);

        let cloned = eh.clone();

        let (est1, _, _) = eh.count(200);
        let (est2, _, _) = cloned.count(200);
        assert_eq!(est1, est2);
    }

    // -------------------------------------------------------------------------
    // Test 20: Stress Test
    // -------------------------------------------------------------------------
    #[test]
    fn test_stress() {
        let mut eh = ExponentialHistogram::new(100000, 0.05).unwrap();

        // Insert many events
        for i in 0..10000 {
            eh.insert(i, 1);
        }

        // Query should still work efficiently
        let (estimate, lower, upper) = eh.count(15000);

        // Should have approximately 10000 events in window
        assert!(estimate >= 9000, "Estimate {} too low", estimate);
        assert!(lower <= estimate);
        assert!(estimate <= upper);

        // Space should be bounded
        let epsilon_val: f64 = 0.05;
        let k = (1.0_f64 / epsilon_val).ceil() as usize;
        let max_buckets = (k + 1) * 64; // log2(100000) * (k+1)
        assert!(
            eh.num_buckets() <= max_buckets,
            "Too many buckets: {} > {}",
            eh.num_buckets(),
            max_buckets
        );
    }

    // -------------------------------------------------------------------------
    // Test 21: Malicious num_buckets in deserialize
    // -------------------------------------------------------------------------
    #[test]
    fn test_deserialize_rejects_oversized_num_buckets_without_panic() {
        // Craft a valid 40-byte header claiming a huge bucket count with no
        // bucket data following. Must return Err rather than panicking,
        // ABORTing on `num_buckets * 16` overflow, or OOMing on with_capacity.
        let mut header = Vec::new();
        header.extend_from_slice(&1000u64.to_le_bytes()); // window_size
        header.extend_from_slice(&0.1f64.to_le_bytes()); // epsilon
        header.extend_from_slice(&10u64.to_le_bytes()); // k (ignored)
        header.extend_from_slice(&0u64.to_le_bytes()); // last_timestamp
        header.extend_from_slice(&u64::MAX.to_le_bytes()); // num_buckets (malicious)
        assert!(ExponentialHistogram::deserialize(&header).is_err());

        // A count whose `* 16` overflows usize must also be rejected, not abort.
        let mut overflow = header.clone();
        overflow[32..40].copy_from_slice(&((u64::MAX / 8).to_le_bytes()));
        assert!(ExponentialHistogram::deserialize(&overflow).is_err());
    }
}

// ============================================================================
// IMPLEMENTATION
// ============================================================================

/// Exponential Histogram with formal error bounds
///
/// Maintains an approximate count over a sliding time window with guaranteed
/// relative error bounded by epsilon. A thin wrapper over the shared
/// [`EhCore`](crate::streaming) bucket engine plus a last-seen timestamp used by the
/// [`Sketch`] trait's parameterless `estimate`.
///
/// # Examples
///
/// ```
/// use sketch_oxide::streaming::ExponentialHistogram;
///
/// let mut eh = ExponentialHistogram::new(60000, 0.01).unwrap(); // 60 second window, 1% error
///
/// // Record events
/// eh.insert(1000, 1);
/// eh.insert(2000, 1);
/// eh.insert(3000, 1);
///
/// // Query count with bounds at time 5000
/// let (estimate, lower, upper) = eh.count(5000);
/// println!("Count: {} (bounds: [{}, {}])", estimate, lower, upper);
/// ```
#[derive(Clone, Debug)]
pub struct ExponentialHistogram {
    /// Shared bucket engine (buckets, window, epsilon, k).
    core: EhCore,
    /// Last timestamp seen, used for the trait-level parameterless estimate.
    last_timestamp: u64,
}

impl ExponentialHistogram {
    /// Creates a new Exponential Histogram
    ///
    /// # Arguments
    ///
    /// * `window_size` - Size of the sliding window in time units
    /// * `epsilon` - Error bound (0.0 to 1.0), e.g., 0.1 for 10% error
    ///
    /// # Errors
    ///
    /// Returns error if parameters are invalid:
    /// - `window_size` must be > 0
    /// - `epsilon` must be in (0, 1)
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::streaming::ExponentialHistogram;
    ///
    /// // 1 hour window with 5% error
    /// let eh = ExponentialHistogram::new(3600, 0.05).unwrap();
    /// ```
    pub fn new(window_size: u64, epsilon: f64) -> Result<Self> {
        Ok(ExponentialHistogram {
            core: EhCore::new(window_size, epsilon)?,
            last_timestamp: 0,
        })
    }

    /// Returns the window size
    #[inline]
    pub fn window_size(&self) -> u64 {
        self.core.window_size
    }

    /// Returns the error bound (epsilon)
    #[inline]
    pub fn epsilon(&self) -> f64 {
        self.core.epsilon
    }

    /// Returns k value (max buckets per level)
    #[inline]
    pub fn k(&self) -> usize {
        self.core.k
    }

    /// Returns the number of buckets
    #[inline]
    pub fn num_buckets(&self) -> usize {
        self.core.num_buckets()
    }

    /// Inserts an event at the given timestamp with the specified count
    ///
    /// # Arguments
    ///
    /// * `timestamp` - The time at which the event occurred
    /// * `count` - Number of events to record (will be decomposed into powers of 2)
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::streaming::ExponentialHistogram;
    ///
    /// let mut eh = ExponentialHistogram::new(100, 0.1).unwrap();
    /// eh.insert(10, 1);  // Single event
    /// eh.insert(20, 5);  // 5 events at timestamp 20
    /// ```
    pub fn insert(&mut self, timestamp: u64, count: u64) {
        if count == 0 {
            return;
        }
        // Track timestamp for the parameterless estimate.
        self.last_timestamp = self.last_timestamp.max(timestamp);
        self.core.insert(timestamp, count);
    }

    /// Returns the count estimate with bounds for the window ending at current_time
    ///
    /// # Arguments
    ///
    /// * `current_time` - The current time (end of the window)
    ///
    /// # Returns
    ///
    /// A tuple of (estimate, lower_bound, upper_bound) where:
    /// - `estimate`: The best estimate of the count
    /// - `lower_bound`: Conservative lower bound
    /// - `upper_bound`: Optimistic upper bound
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::streaming::ExponentialHistogram;
    ///
    /// let mut eh = ExponentialHistogram::new(100, 0.1).unwrap();
    /// eh.insert(50, 1);
    /// eh.insert(60, 1);
    ///
    /// let (est, lower, upper) = eh.count(100);
    /// println!("Count: {} in [{}, {}]", est, lower, upper);
    /// ```
    pub fn count(&self, current_time: u64) -> (u64, u64, u64) {
        self.core.count_with_bounds(current_time)
    }

    /// Expires old buckets outside the window
    ///
    /// Removes buckets that are entirely outside the window, keeping at most
    /// one bucket that straddles the window boundary for accuracy.
    ///
    /// # Arguments
    ///
    /// * `current_time` - The current time
    pub fn expire(&mut self, current_time: u64) {
        self.core.expire(current_time);
    }

    /// Clears all buckets
    pub fn clear(&mut self) {
        self.core.clear();
        self.last_timestamp = 0;
    }

    /// Returns the theoretical error bound
    #[inline]
    pub fn error_bound(&self) -> f64 {
        self.core.epsilon
    }

    /// Returns memory usage in bytes (approximate)
    pub fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>() + self.core.num_buckets() * std::mem::size_of::<EhBucket>()
    }
}

// ============================================================================
// Sketch Trait Implementation
// ============================================================================

impl Sketch for ExponentialHistogram {
    /// Item type is (timestamp, count)
    type Item = (u64, u64);

    fn update(&mut self, item: &Self::Item) {
        self.insert(item.0, item.1);
    }

    fn estimate(&self) -> f64 {
        // Return estimate at last timestamp
        let (estimate, _, _) = self.count(self.last_timestamp);
        estimate as f64
    }

    fn is_empty(&self) -> bool {
        self.core.num_buckets() == 0
    }

    fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Header (format unchanged across the EhCore refactor for FFI compatibility)
        bytes.extend_from_slice(&self.core.window_size.to_le_bytes());
        bytes.extend_from_slice(&self.core.epsilon.to_le_bytes());
        bytes.extend_from_slice(&(self.core.k as u64).to_le_bytes());
        bytes.extend_from_slice(&self.last_timestamp.to_le_bytes());
        bytes.extend_from_slice(&(self.core.num_buckets() as u64).to_le_bytes());

        // Buckets
        for bucket in &self.core.buckets {
            bytes.extend_from_slice(&bucket.timestamp.to_le_bytes());
            bytes.extend_from_slice(&bucket.count.to_le_bytes());
        }

        bytes
    }

    fn deserialize(bytes: &[u8]) -> Result<Self> {
        const HEADER_SIZE: usize = 40; // 5 * 8 bytes

        if bytes.len() < HEADER_SIZE {
            return Err(SketchError::DeserializationError(
                "Insufficient data for ExponentialHistogram header".to_string(),
            ));
        }

        let window_size = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
        let epsilon = f64::from_le_bytes(bytes[8..16].try_into().unwrap());
        // bytes[16..24] held `k`; it is recomputed from epsilon by EhCore.
        let last_timestamp = u64::from_le_bytes(bytes[24..32].try_into().unwrap());
        let num_buckets = u64::from_le_bytes(bytes[32..40].try_into().unwrap()) as usize;

        // `num_buckets` is attacker-controlled. Use checked arithmetic so a crafted
        // count cannot overflow `num_buckets * 16` (which ABORTS under
        // overflow-checks = a DoS), and require the buckets to actually be present in
        // the input before the `Vec::with_capacity` below so a huge count cannot drive
        // an OOM allocation (count * 16 <= remaining bytes).
        let buckets_bytes = num_buckets.checked_mul(16).ok_or_else(|| {
            SketchError::DeserializationError("bucket table size overflow".to_string())
        })?;
        let expected_len = HEADER_SIZE.checked_add(buckets_bytes).ok_or_else(|| {
            SketchError::DeserializationError("bucket table size overflow".to_string())
        })?;
        if bytes.len() < expected_len {
            return Err(SketchError::DeserializationError(format!(
                "Expected {} bytes, got {}",
                expected_len,
                bytes.len()
            )));
        }

        let mut buckets = Vec::with_capacity(num_buckets);
        let mut offset = HEADER_SIZE;

        for _ in 0..num_buckets {
            let timestamp = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
            let count = u64::from_le_bytes(bytes[offset + 8..offset + 16].try_into().unwrap());
            buckets.push(EhBucket { timestamp, count });
            offset += 16;
        }

        Ok(ExponentialHistogram {
            core: EhCore::from_parts(window_size, epsilon, buckets),
            last_timestamp,
        })
    }
}

// ============================================================================
// Mergeable Trait Implementation
// ============================================================================

impl Mergeable for ExponentialHistogram {
    fn merge(&mut self, other: &Self) -> Result<()> {
        self.core.merge_from(&other.core)?;
        self.last_timestamp = self.last_timestamp.max(other.last_timestamp);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Capability-trait adoptions (fable5 doc 01 F3 "split the `Sketch` trait").
// ExponentialHistogram ingests `(timestamp, count)` events — exactly its
// `Sketch::Item` — so it satisfies `Update<(u64, u64)>` (delegating to the
// existing infallible `Sketch::update`). Its serialize/deserialize round-trip
// works, so it also satisfies `Serializable`. `Sketch::estimate` returns the
// count at the last timestamp (not a set cardinality), and `count` needs a
// current-time argument, so those query traits are skipped.
// ---------------------------------------------------------------------------
use crate::common::{Serializable, Update};

impl Update<(u64, u64)> for ExponentialHistogram {
    fn update(&mut self, item: &(u64, u64)) {
        <Self as Sketch>::update(self, item);
    }
}

impl Serializable for ExponentialHistogram {
    fn to_bytes(&self) -> Result<Vec<u8>> {
        Ok(<Self as Sketch>::serialize(self))
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        <Self as Sketch>::deserialize(bytes)
    }
}
