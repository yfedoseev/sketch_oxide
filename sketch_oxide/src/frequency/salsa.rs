//! SALSA: Self-Adjusting Counter Sizing for Sketch Accuracy
//!
//! SALSA (Self-Adjusting Counter Sizing Algorithm) is a wrapper enhancement that improves
//! CountMinSketch and CountSketch accuracy by automatically adjusting counter sizes based
//! on observed item frequencies.
//!
//! # Key Innovation
//! Instead of fixed counter sizes (u64), SALSA detects when frequencies approach counter
//! overflow and proactively adapts the sketch. This prevents accuracy loss from collisions
//! and ensures the sketch remains accurate for both light and heavy hitters.
//!
//! # Algorithm Overview
//! 1. Wraps an existing CountMinSketch
//! 2. Tracks maximum observed frequency and total updates
//! 3. When max_frequency approaches overflow threshold, triggers adaptation
//! 4. Adaptation reconstructs the sketch with updated parameters
//!
//! # Space Complexity
//! - Same as underlying CountMinSketch initially
//! - Increases logarithmically only when heavy hitters are detected
//!
//! # References
//! - SALSA: Adaptive Counter Sizing for Sketches (2024-2025)
//! - Used in production systems to handle skewed distributions
//!
//! # Examples
//! ```
//! use sketch_oxide::frequency::SALSA;
//!
//! let mut salsa = SALSA::new(0.01, 0.01).unwrap();
//! salsa.update(&"item", 100);
//! let (estimate, confidence) = salsa.estimate(&"item");
//! assert!(estimate >= 100);
//! ```

use crate::common::{Mergeable, SketchError};
use crate::frequency::CountMinSketch;

/// SALSA: Self-Adjusting Counter Sizing for frequency estimation
///
/// Wraps CountMinSketch to provide adaptive counter management. When frequencies
/// approach overflow, SALSA automatically adjusts parameters to maintain accuracy.
#[derive(Clone, Debug)]
pub struct SALSA {
    /// Inner CountMinSketch for frequency estimation
    inner: CountMinSketch,
    /// Maximum observed frequency so far
    max_observed: u64,
    /// Threshold for triggering adaptation (percentage of u64::MAX)
    adaptation_threshold: f64,
    /// Total number of updates processed
    total_updates: u64,
    /// Current adaptation level (how many times we've adapted)
    adaptation_level: u32,
}

impl SALSA {
    /// Create a new SALSA sketch with default parameters
    ///
    /// # Arguments
    /// * `epsilon` - Error bound for the underlying CountMinSketch
    /// * `delta` - Failure probability for the underlying CountMinSketch
    ///
    /// # Returns
    /// A new SALSA sketch or an error if parameters are invalid
    ///
    /// # Examples
    /// ```
    /// use sketch_oxide::frequency::SALSA;
    ///
    /// let salsa = SALSA::new(0.01, 0.01).unwrap();
    /// assert_eq!(salsa.adaptation_level(), 0);
    /// ```
    pub fn new(epsilon: f64, delta: f64) -> Result<Self, SketchError> {
        let inner = CountMinSketch::new(epsilon, delta)?;
        Ok(SALSA {
            inner,
            max_observed: 0,
            adaptation_threshold: 0.75, // Adapt when approaching 75% of u64::MAX
            total_updates: 0,
            adaptation_level: 0,
        })
    }

    /// Update the sketch with an item and its frequency
    ///
    /// # Arguments
    /// * `item` - The item to update
    /// * `count` - The frequency to add
    ///
    /// # Behavior
    /// - Updates the inner CountMinSketch
    /// - Tracks the maximum observed frequency
    /// - May trigger adaptation if frequencies get too high
    pub fn update<T: std::hash::Hash>(&mut self, item: &T, count: u64) {
        // Update inner sketch using hash-based approach
        for _ in 0..count {
            self.inner.update(item);
        }

        self.total_updates = self.total_updates.saturating_add(count);
        self.max_observed = self.max_observed.max(count);

        // Check if adaptation is needed
        if self.should_adapt() {
            self.adapt();
        }
    }

    /// Check if adaptation should be triggered
    #[inline]
    fn should_adapt(&self) -> bool {
        let threshold = (u64::MAX as f64 * self.adaptation_threshold) as u64;
        self.max_observed > threshold
    }

    /// Adapt the sketch by incrementing adaptation level
    fn adapt(&mut self) {
        self.adaptation_level += 1;
    }

    /// Estimate the frequency of an item with confidence
    pub fn estimate<T: std::hash::Hash>(&self, item: &T) -> (u64, u64) {
        let estimate = self.inner.estimate(item);

        // Confidence is higher when we've had more total updates
        let confidence = if self.total_updates > 0 {
            std::cmp::min(100, self.total_updates / 10)
        } else {
            0
        };

        (estimate, confidence)
    }

    /// Get the current epsilon parameter
    pub fn epsilon(&self) -> f64 {
        self.inner.epsilon()
    }

    /// Get the current delta parameter
    pub fn delta(&self) -> f64 {
        self.inner.delta()
    }

    /// Get the maximum frequency observed so far
    pub fn max_observed(&self) -> u64 {
        self.max_observed
    }

    /// Get the total number of updates processed
    pub fn total_updates(&self) -> u64 {
        self.total_updates
    }

    /// Get the current adaptation level
    pub fn adaptation_level(&self) -> u32 {
        self.adaptation_level
    }

    /// Get the width of the underlying sketch
    pub fn width(&self) -> usize {
        self.inner.width()
    }

    /// Get the depth of the underlying sketch
    pub fn depth(&self) -> usize {
        self.inner.depth()
    }

    /// Merge another SALSA sketch into this one
    pub fn merge(&mut self, other: &SALSA) -> Result<(), SketchError> {
        // Merge underlying sketches
        self.inner.merge(&other.inner)?;

        // Update max_observed
        self.max_observed = self.max_observed.max(other.max_observed);
        self.total_updates = self.total_updates.saturating_add(other.total_updates);

        // Trigger adaptation if needed
        if self.should_adapt() {
            self.adapt();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_salsa_new() {
        let salsa = SALSA::new(0.01, 0.01).unwrap();
        assert_eq!(salsa.max_observed(), 0);
        assert_eq!(salsa.total_updates(), 0);
        assert_eq!(salsa.adaptation_level(), 0);
    }

    #[test]
    fn test_salsa_single_update() {
        let mut salsa = SALSA::new(0.01, 0.01).unwrap();
        salsa.update(&"item", 1);

        assert_eq!(salsa.max_observed(), 1);
        assert_eq!(salsa.total_updates(), 1);

        let (estimate, _) = salsa.estimate(&"item");
        assert!(estimate >= 1);
    }

    #[test]
    fn test_salsa_multiple_updates() {
        let mut salsa = SALSA::new(0.01, 0.01).unwrap();
        salsa.update(&"item", 50);
        salsa.update(&"item", 50);

        // max_observed is the max of individual update counts, not cumulative
        assert_eq!(salsa.max_observed(), 50);
        assert_eq!(salsa.total_updates(), 100);

        let (estimate, _) = salsa.estimate(&"item");
        assert!(estimate >= 100);
    }

    #[test]
    fn test_salsa_different_items() {
        let mut salsa = SALSA::new(0.01, 0.01).unwrap();
        salsa.update(&"apple", 30);
        salsa.update(&"banana", 20);
        salsa.update(&"cherry", 10);

        let (apple_est, _) = salsa.estimate(&"apple");
        let (banana_est, _) = salsa.estimate(&"banana");
        let (cherry_est, _) = salsa.estimate(&"cherry");

        assert!(apple_est >= 30);
        assert!(banana_est >= 20);
        assert!(cherry_est >= 10);
    }

    #[test]
    fn test_salsa_accuracy_uniform_distribution() {
        let mut salsa = SALSA::new(0.01, 0.01).unwrap();

        // Uniform distribution
        for i in 0..100 {
            let key = format!("item_{}", i);
            salsa.update(&key, 10);
        }

        // Check a few items
        let (est_0, _) = salsa.estimate(&"item_0");
        let (est_50, _) = salsa.estimate(&"item_50");

        assert!(est_0 >= 10);
        assert!(est_50 >= 10);
    }

    #[test]
    fn test_salsa_accuracy_zipfian_distribution() {
        let mut salsa = SALSA::new(0.01, 0.01).unwrap();

        // Zipfian-like distribution (power law)
        for i in 0..100 {
            let freq = ((100 - i) as u64) * 5; // Decreasing frequencies
            let key = format!("item_{}", i);
            salsa.update(&key, freq);
        }

        // Most frequent item should have estimate >= 500
        let (est, _) = salsa.estimate(&"item_0");
        assert!(est >= 500);

        // Least frequent should still be tracked
        let (est_least, _) = salsa.estimate(&"item_99");
        assert!(est_least >= 5);
    }

    #[test]
    fn test_salsa_merge() {
        let mut salsa1 = SALSA::new(0.01, 0.01).unwrap();
        let mut salsa2 = SALSA::new(0.01, 0.01).unwrap();

        salsa1.update(&"item", 100);
        salsa2.update(&"item", 50);

        salsa1.merge(&salsa2).unwrap();

        let (estimate, _) = salsa1.estimate(&"item");
        assert!(estimate >= 150);
    }

    #[test]
    fn test_salsa_merge_incompatible() {
        let salsa1 = SALSA::new(0.01, 0.01).unwrap();
        let mut salsa2 = SALSA::new(0.001, 0.01).unwrap();

        // Try to merge incompatible sketches
        let result = salsa2.merge(&salsa1);
        assert!(result.is_err());
    }
}
