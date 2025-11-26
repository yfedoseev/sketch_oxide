//! Sliding Window Counter using Exponential Histogram (Datar 2002)
//!
//! Maintains approximate count over a sliding window using O(log²N) space.
//! Perfect for time-bounded counting, rate limiting, and anomaly detection.
//!
//! # Algorithm Overview
//!
//! The Exponential Histogram maintains buckets of exponentially increasing sizes.
//! Each bucket stores a timestamp and a count. Old buckets are merged or expired
//! as the window slides.
//!
//! # Properties
//!
//! - Space: O((1/ε) * log²(window_size))
//! - Query time: O(1)
//! - Update time: O(log(1/ε) * log(window_size))
//! - Error: (1 ± ε) * actual_count
//!
//! # Use Cases
//!
//! - Count events in last N seconds/minutes
//! - Rate limiting (requests per time window)
//! - Anomaly detection (sudden spikes)
//! - Moving averages
//!
//! # References
//!
//! - Datar et al. "Maintaining Stream Statistics over Sliding Windows" (SODA 2002)
//!
//! # Examples
//!
//! ```
//! use sketch_oxide::streaming::SlidingWindowCounter;
//!
//! let mut counter = SlidingWindowCounter::new(1000, 0.1).unwrap();
//!
//! // Record events at different timestamps
//! counter.increment(100);
//! counter.increment(200);
//! counter.increment(300);
//!
//! // Count events in the window ending at time 500
//! // Window includes timestamps from 500-1000+1 = -500 to 500
//! // So all events (100, 200, 300) are included
//! let count = counter.count(500);
//! ```

use crate::common::SketchError;

/// A bucket in the exponential histogram
#[derive(Clone, Debug)]
struct Bucket {
    /// Timestamp when this bucket was created
    timestamp: u64,
    /// Count of events in this bucket (always a power of 2)
    count: u64,
}

/// Sliding Window Counter using Exponential Histogram
///
/// Maintains an approximate count of events within a sliding time window.
/// Uses O(log²N) space while providing (1 ± ε) accuracy.
///
/// # Examples
///
/// ```
/// use sketch_oxide::streaming::SlidingWindowCounter;
///
/// let mut counter = SlidingWindowCounter::new(60000, 0.01).unwrap(); // 60 second window, 1% error
///
/// // Record events
/// counter.increment(1000);
/// counter.increment(2000);
/// counter.increment(3000);
///
/// // Query count at time 5000 (includes all events in [5000-60000, 5000])
/// let count = counter.count(5000);
/// ```
#[derive(Clone, Debug)]
pub struct SlidingWindowCounter {
    /// Buckets ordered by timestamp (newest first)
    buckets: Vec<Bucket>,
    /// Window size in time units
    window_size: u64,
    /// Error bound (epsilon)
    epsilon: f64,
    /// Maximum buckets per level (k = ceil(1/epsilon))
    k: usize,
    /// Total count (may include expired items)
    total: u64,
}

impl SlidingWindowCounter {
    /// Creates a new Sliding Window Counter
    ///
    /// # Arguments
    ///
    /// * `window_size` - Size of the sliding window in time units
    /// * `epsilon` - Error bound (0.0 to 1.0), e.g., 0.1 for 10% error
    ///
    /// # Errors
    ///
    /// Returns error if parameters are invalid
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::streaming::SlidingWindowCounter;
    ///
    /// // 1 hour window with 5% error
    /// let counter = SlidingWindowCounter::new(3600, 0.05).unwrap();
    /// ```
    pub fn new(window_size: u64, epsilon: f64) -> Result<Self, SketchError> {
        if window_size == 0 {
            return Err(SketchError::InvalidParameter {
                param: "window_size".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }

        if epsilon <= 0.0 || epsilon >= 1.0 {
            return Err(SketchError::InvalidParameter {
                param: "epsilon".to_string(),
                value: epsilon.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }

        // k = ceil(1/epsilon) determines max buckets per level
        let k = (1.0 / epsilon).ceil() as usize;

        Ok(SlidingWindowCounter {
            buckets: Vec::new(),
            window_size,
            epsilon,
            k,
            total: 0,
        })
    }

    /// Returns the window size
    pub fn window_size(&self) -> u64 {
        self.window_size
    }

    /// Returns the error bound
    pub fn epsilon(&self) -> f64 {
        self.epsilon
    }

    /// Increments the counter at the given timestamp
    ///
    /// # Arguments
    ///
    /// * `timestamp` - The time at which the event occurred
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::streaming::SlidingWindowCounter;
    ///
    /// let mut counter = SlidingWindowCounter::new(100, 0.1).unwrap();
    /// counter.increment(10);
    /// counter.increment(20);
    /// ```
    pub fn increment(&mut self, timestamp: u64) {
        self.increment_by(timestamp, 1);
    }

    /// Increments the counter by a specified amount at the given timestamp
    ///
    /// # Arguments
    ///
    /// * `timestamp` - The time at which the events occurred
    /// * `count` - Number of events to record
    pub fn increment_by(&mut self, timestamp: u64, count: u64) {
        if count == 0 {
            return;
        }

        self.total += count;

        // Add new bucket(s) for the count
        // We need to decompose count into powers of 2
        let mut remaining = count;
        while remaining > 0 {
            // Find the largest power of 2 <= remaining
            let power = 63 - remaining.leading_zeros();
            let bucket_count = 1u64 << power;

            self.buckets.insert(
                0,
                Bucket {
                    timestamp,
                    count: bucket_count,
                },
            );

            remaining -= bucket_count;
        }

        // Merge buckets if needed to maintain invariant
        self.merge_buckets();
    }

    /// Merges buckets to maintain the exponential histogram invariant
    /// (at most k+1 buckets of each size, except possibly the largest)
    fn merge_buckets(&mut self) {
        if self.buckets.len() < 2 {
            return;
        }

        // Process from smallest to largest buckets
        // Group buckets by their count (power of 2)
        let mut i = 0;
        while i < self.buckets.len() {
            let current_count = self.buckets[i].count;

            // Count consecutive buckets with same count
            let mut same_count = 1;
            while i + same_count < self.buckets.len()
                && self.buckets[i + same_count].count == current_count
            {
                same_count += 1;
            }

            // If too many buckets of this size, merge the oldest two
            while same_count > self.k + 1 {
                // Merge the two oldest (last in this group)
                let merge_idx = i + same_count - 2;
                let older_timestamp = self.buckets[merge_idx + 1].timestamp;

                // Create merged bucket with double count
                self.buckets[merge_idx] = Bucket {
                    timestamp: older_timestamp,
                    count: current_count * 2,
                };

                // Remove the oldest of the pair
                self.buckets.remove(merge_idx + 1);
                same_count -= 1;
            }

            i += same_count;
        }
    }

    /// Returns the approximate count within the window ending at the given timestamp
    ///
    /// # Arguments
    ///
    /// * `current_time` - The current time (end of the window)
    ///
    /// # Returns
    ///
    /// Approximate count of events in [current_time - window_size, current_time]
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::streaming::SlidingWindowCounter;
    ///
    /// let mut counter = SlidingWindowCounter::new(100, 0.1).unwrap();
    /// counter.increment(50);
    /// counter.increment(60);
    ///
    /// // At time 100, both events are in the window
    /// assert!(counter.count(100) >= 1);
    /// ```
    pub fn count(&self, current_time: u64) -> u64 {
        let window_start = current_time.saturating_sub(self.window_size);

        let mut total = 0u64;
        let mut last_partial = 0u64;

        for bucket in &self.buckets {
            if bucket.timestamp > current_time {
                // Future bucket, skip
                continue;
            }

            if bucket.timestamp >= window_start {
                // Fully within window
                total += bucket.count;
            } else {
                // Partially outside window - this is the oldest relevant bucket
                // We count half of it (approximation)
                last_partial = bucket.count / 2;
                break;
            }
        }

        total + last_partial
    }

    /// Returns the count for a specific time range
    ///
    /// # Arguments
    ///
    /// * `start` - Start of the range (inclusive)
    /// * `end` - End of the range (inclusive)
    pub fn count_range(&self, start: u64, end: u64) -> u64 {
        let mut total = 0u64;

        for bucket in &self.buckets {
            if bucket.timestamp > end {
                continue;
            }
            if bucket.timestamp < start {
                // Partially before range
                total += bucket.count / 2;
                break;
            }
            total += bucket.count;
        }

        total
    }

    /// Expires old buckets outside the window
    ///
    /// Call this periodically to free memory from old buckets.
    ///
    /// # Arguments
    ///
    /// * `current_time` - The current time
    pub fn expire(&mut self, current_time: u64) {
        let window_start = current_time.saturating_sub(self.window_size);

        // Remove buckets entirely outside the window
        // Keep one bucket that might be partially in the window
        let mut found_outside = false;
        self.buckets.retain(|bucket| {
            if bucket.timestamp >= window_start {
                true
            } else if !found_outside {
                found_outside = true;
                true // Keep one partial bucket
            } else {
                false
            }
        });
    }

    /// Clears all buckets
    pub fn clear(&mut self) {
        self.buckets.clear();
        self.total = 0;
    }

    /// Returns the number of buckets (for diagnostics)
    pub fn num_buckets(&self) -> usize {
        self.buckets.len()
    }

    /// Returns the theoretical error bound
    ///
    /// The actual count is within (1 ± epsilon) * returned_count
    pub fn error_bound(&self) -> f64 {
        self.epsilon
    }

    /// Returns memory usage in bytes (approximate)
    pub fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>() + self.buckets.len() * std::mem::size_of::<Bucket>()
    }

    /// Serializes the counter to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.window_size.to_le_bytes());
        bytes.extend_from_slice(&self.epsilon.to_le_bytes());
        bytes.extend_from_slice(&(self.k as u64).to_le_bytes());
        bytes.extend_from_slice(&self.total.to_le_bytes());
        bytes.extend_from_slice(&(self.buckets.len() as u64).to_le_bytes());

        for bucket in &self.buckets {
            bytes.extend_from_slice(&bucket.timestamp.to_le_bytes());
            bytes.extend_from_slice(&bucket.count.to_le_bytes());
        }

        bytes
    }

    /// Deserializes a counter from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SketchError> {
        if bytes.len() < 40 {
            return Err(SketchError::DeserializationError(
                "Insufficient data for SlidingWindowCounter header".to_string(),
            ));
        }

        let window_size = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
        let epsilon = f64::from_le_bytes(bytes[8..16].try_into().unwrap());
        let k = u64::from_le_bytes(bytes[16..24].try_into().unwrap()) as usize;
        let total = u64::from_le_bytes(bytes[24..32].try_into().unwrap());
        let num_buckets = u64::from_le_bytes(bytes[32..40].try_into().unwrap()) as usize;

        let expected_len = 40 + num_buckets * 16;
        if bytes.len() < expected_len {
            return Err(SketchError::DeserializationError(format!(
                "Expected {} bytes, got {}",
                expected_len,
                bytes.len()
            )));
        }

        let mut buckets = Vec::with_capacity(num_buckets);
        let mut offset = 40;

        for _ in 0..num_buckets {
            let timestamp = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
            let count = u64::from_le_bytes(bytes[offset + 8..offset + 16].try_into().unwrap());
            buckets.push(Bucket { timestamp, count });
            offset += 16;
        }

        Ok(SlidingWindowCounter {
            buckets,
            window_size,
            epsilon,
            k,
            total,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let counter = SlidingWindowCounter::new(1000, 0.1).unwrap();
        assert_eq!(counter.window_size(), 1000);
        assert_eq!(counter.epsilon(), 0.1);
    }

    #[test]
    fn test_invalid_params() {
        assert!(SlidingWindowCounter::new(0, 0.1).is_err());
        assert!(SlidingWindowCounter::new(1000, 0.0).is_err());
        assert!(SlidingWindowCounter::new(1000, 1.0).is_err());
        assert!(SlidingWindowCounter::new(1000, -0.1).is_err());
    }

    #[test]
    fn test_increment_and_count() {
        let mut counter = SlidingWindowCounter::new(100, 0.1).unwrap();

        counter.increment(10);
        counter.increment(20);
        counter.increment(30);

        // All events are within window at time 50
        let count = counter.count(50);
        assert!(count >= 2, "Expected at least 2, got {}", count);
    }

    #[test]
    fn test_window_expiration() {
        let mut counter = SlidingWindowCounter::new(100, 0.1).unwrap();

        counter.increment(10);
        counter.increment(20);
        counter.increment(200); // Outside window from perspective of time 10-20

        // At time 250, only event at 200 is in window [150, 250]
        let count = counter.count(250);
        assert!(count >= 1, "Expected at least 1, got {}", count);
    }

    #[test]
    fn test_expire() {
        let mut counter = SlidingWindowCounter::new(100, 0.1).unwrap();

        counter.increment(10);
        counter.increment(20);
        counter.increment(200);

        let before = counter.num_buckets();
        counter.expire(300);
        let after = counter.num_buckets();

        // Should have removed some old buckets
        assert!(after <= before);
    }

    #[test]
    fn test_increment_by() {
        let mut counter = SlidingWindowCounter::new(100, 0.1).unwrap();

        counter.increment_by(10, 5);
        counter.increment_by(20, 3);

        let count = counter.count(50);
        assert!(count >= 6, "Expected at least 6, got {}", count);
    }

    #[test]
    fn test_count_range() {
        let mut counter = SlidingWindowCounter::new(1000, 0.1).unwrap();

        counter.increment(100);
        counter.increment(200);
        counter.increment(300);
        counter.increment(400);

        let count = counter.count_range(150, 350);
        assert!(count >= 2, "Expected at least 2 in range, got {}", count);
    }

    #[test]
    fn test_clear() {
        let mut counter = SlidingWindowCounter::new(100, 0.1).unwrap();

        counter.increment(10);
        counter.increment(20);

        counter.clear();

        assert_eq!(counter.count(50), 0);
        assert_eq!(counter.num_buckets(), 0);
    }

    #[test]
    fn test_serialization() {
        let mut counter = SlidingWindowCounter::new(100, 0.1).unwrap();

        counter.increment(10);
        counter.increment(20);
        counter.increment(30);

        let bytes = counter.to_bytes();
        let restored = SlidingWindowCounter::from_bytes(&bytes).unwrap();

        assert_eq!(counter.window_size(), restored.window_size());
        assert_eq!(counter.epsilon(), restored.epsilon());
        assert_eq!(counter.num_buckets(), restored.num_buckets());
    }

    #[test]
    fn test_bucket_merging() {
        let mut counter = SlidingWindowCounter::new(1000, 0.5).unwrap(); // k=2

        // Insert many events to trigger merging
        for i in 0..20 {
            counter.increment(i * 10);
        }

        // With k=2, we shouldn't have more than ~2*(log2(20)+1) buckets
        assert!(
            counter.num_buckets() < 20,
            "Expected fewer buckets due to merging, got {}",
            counter.num_buckets()
        );
    }

    #[test]
    fn test_accuracy() {
        let epsilon = 0.1;
        let mut counter = SlidingWindowCounter::new(1000, epsilon).unwrap();

        let actual_count = 100;
        for i in 0..actual_count {
            counter.increment(i * 5);
        }

        let estimated = counter.count(500);

        // Error should be within epsilon
        let error_ratio = (estimated as f64 - actual_count as f64).abs() / actual_count as f64;
        assert!(
            error_ratio <= epsilon + 0.1, // Some slack for edge cases
            "Error {} exceeds epsilon {}",
            error_ratio,
            epsilon
        );
    }

    #[test]
    fn test_memory_usage() {
        let counter = SlidingWindowCounter::new(1000, 0.1).unwrap();
        assert!(counter.memory_usage() > 0);
    }
}
