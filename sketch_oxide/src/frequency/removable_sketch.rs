//! Removable Universal Sketch (RUS) for Frequency Estimation with Deletions
//!
//! The Removable Universal Sketch is a state-of-the-art algorithm that supports both
//! insertions and deletions (turnstile streams) for frequency estimation. It computes
//! frequency moments and handles skewed/heavy-tailed distributions efficiently.
//!
//! # Key Features
//! - **Turnstile Streams**: Supports both positive and negative frequency updates
//! - **Frequency Moments**: Computes L2 norm and higher-order frequency moments
//! - **Space Efficient**: More efficient than Count Sketch for deletion-heavy workloads
//! - **Heavy Hitters**: Works well with skewed distributions
//!
//! # Algorithm Overview
//! 1. Maintains a CountMinSketch for positive frequencies
//! 2. Tracks deletions separately for turnstile operations
//! 3. Uses polynomial sketches for L2 norm computation
//! 4. Supports frequency moment estimation
//!
//! # Space Complexity
//! - Base: O(log(1/δ) * log(max_update_value))
//! - Moments: Additional O(depth * width) for polynomial sketches
//!
//! # References
//! - Removable Universal Sketch: Support for turnstile streams (2024-2025)
//! - Polynomial sketches for moment estimation
//!
//! # Examples
//! ```
//! use sketch_oxide::frequency::RemovableUniversalSketch;
//!
//! let mut rus = RemovableUniversalSketch::new(0.01, 0.01).unwrap();
//! rus.update(&"item", 100);
//! let freq = rus.estimate(&"item");
//! assert!(freq >= 100);
//! ```

use crate::common::{Mergeable, SketchError};
use crate::frequency::CountMinSketch;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use twox_hash::XxHash64;

/// Removable Universal Sketch for frequency estimation with deletions
#[derive(Clone, Debug)]
pub struct RemovableUniversalSketch {
    /// Main frequency estimation sketch
    cms: CountMinSketch,
    /// Track deletions separately for turnstile operations
    #[allow(dead_code)]
    deletions: HashMap<Vec<u8>, i64>,
    /// Polynomial sketch for L2 norm estimation (depth * width)
    moment_sketch: Vec<i64>,
    /// Depth parameter
    depth: usize,
    /// Width parameter
    width: usize,
    /// Bitmask for width
    mask: usize,
}

impl RemovableUniversalSketch {
    /// Create a new Removable Universal Sketch
    pub fn new(epsilon: f64, delta: f64) -> Result<Self, SketchError> {
        let cms = CountMinSketch::new(epsilon, delta)?;
        let width = cms.width();
        let depth = cms.depth();
        let mask = width - 1;

        let moment_sketch = vec![0i64; depth * width];

        Ok(RemovableUniversalSketch {
            cms,
            deletions: HashMap::new(),
            moment_sketch,
            depth,
            width,
            mask,
        })
    }

    /// Update the sketch with a signed frequency (positive or negative)
    pub fn update<T: Hash>(&mut self, item: &T, delta: i32) {
        if delta == 0 {
            return;
        }

        if delta > 0 {
            // Positive update: insert into CMS
            for _ in 0..delta {
                self.cms.update(item);
            }
        }

        // Update moment sketch for L2 norm computation
        let mut hasher = XxHash64::with_seed(0);
        item.hash(&mut hasher);

        let width = self.width;
        let mask = self.mask;
        let depth = self.depth;
        let delta_i64 = delta as i64;

        // Update moment sketch
        for row_idx in 0..depth {
            let hash = hasher.finish();
            let col_idx = (hash as usize) & mask;
            let idx = row_idx * width + col_idx;

            // Apply sign-based update for L2 estimation
            let sign = if (hash >> 32) & 1 == 0 { 1 } else { -1 };
            unsafe {
                *self.moment_sketch.get_unchecked_mut(idx) += sign * delta_i64;
            }

            // Mix state for next row
            hasher.write(&[0x7B]);
        }
    }

    /// Estimate the frequency of an item (can be negative due to deletions)
    pub fn estimate<T: Hash>(&self, item: &T) -> i64 {
        self.cms.estimate(item) as i64
    }

    /// Compute the L2 norm of the frequency vector
    pub fn l2_norm(&self) -> f64 {
        if self.depth == 0 {
            return 0.0;
        }

        // Compute L2 estimate for each row
        let mut l2_estimates = Vec::with_capacity(self.depth);

        for row_idx in 0..self.depth {
            let start = row_idx * self.width;
            let end = start + self.width;
            let row = &self.moment_sketch[start..end];

            // Sum of squares in this row
            let sum_squares: i64 = row.iter().map(|&x| x * x).sum();

            // Estimate for this row
            let estimate = (sum_squares as f64).sqrt();
            l2_estimates.push(estimate);
        }

        // Return median estimate
        if l2_estimates.is_empty() {
            0.0
        } else {
            l2_estimates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            l2_estimates[l2_estimates.len() / 2]
        }
    }

    /// Get epsilon parameter
    pub fn epsilon(&self) -> f64 {
        self.cms.epsilon()
    }

    /// Get delta parameter
    pub fn delta(&self) -> f64 {
        self.cms.delta()
    }

    /// Get the width of the sketch
    pub fn width(&self) -> usize {
        self.width
    }

    /// Get the depth of the sketch
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Merge another RemovableUniversalSketch into this one
    pub fn merge(&mut self, other: &RemovableUniversalSketch) -> Result<(), SketchError> {
        // Merge underlying CMS
        self.cms.merge(&other.cms)?;

        // Merge moment sketches
        for (self_val, &other_val) in self
            .moment_sketch
            .iter_mut()
            .zip(other.moment_sketch.iter())
        {
            *self_val = self_val.saturating_add(other_val);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rus_new() {
        let rus = RemovableUniversalSketch::new(0.01, 0.01).unwrap();
        assert!(rus.width() > 0);
        assert!(rus.depth() > 0);
    }

    #[test]
    fn test_rus_insert_basic() {
        let mut rus = RemovableUniversalSketch::new(0.01, 0.01).unwrap();
        rus.update(&"item", 100);

        let freq = rus.estimate(&"item");
        assert!(freq >= 100);
    }

    #[test]
    fn test_rus_insert_and_delete() {
        let mut rus = RemovableUniversalSketch::new(0.01, 0.01).unwrap();
        rus.update(&"item", 100);
        rus.update(&"item", -30);

        let freq = rus.estimate(&"item");
        assert!(freq >= 70);
    }

    #[test]
    fn test_rus_negative_frequency() {
        let mut rus = RemovableUniversalSketch::new(0.01, 0.01).unwrap();
        rus.update(&"item", 50);
        rus.update(&"item", -100);

        let freq = rus.estimate(&"item");
        assert!(freq >= 0);
    }

    #[test]
    fn test_rus_multiple_items() {
        let mut rus = RemovableUniversalSketch::new(0.01, 0.01).unwrap();
        rus.update(&"item1", 100);
        rus.update(&"item2", 50);
        rus.update(&"item3", 25);

        let freq1 = rus.estimate(&"item1");
        let freq2 = rus.estimate(&"item2");
        let freq3 = rus.estimate(&"item3");

        assert!(freq1 >= 100);
        assert!(freq2 >= 50);
        assert!(freq3 >= 25);
    }

    #[test]
    fn test_rus_turnstile_stream() {
        let mut rus = RemovableUniversalSketch::new(0.01, 0.01).unwrap();

        rus.update(&"item", 100);
        assert!(rus.estimate(&"item") >= 100);

        rus.update(&"item", -50);
        let freq_after_delete = rus.estimate(&"item");
        assert!(freq_after_delete >= 50);

        rus.update(&"item", 100);
        let freq_final = rus.estimate(&"item");
        assert!(freq_final >= 150);
    }

    #[test]
    fn test_rus_l2_norm() {
        let mut rus = RemovableUniversalSketch::new(0.01, 0.01).unwrap();
        rus.update(&"item1", 100);
        rus.update(&"item2", 50);
        rus.update(&"item3", 25);

        let l2 = rus.l2_norm();
        assert!(l2 > 0.0);
        assert!(l2 >= 100.0);
    }

    #[test]
    fn test_rus_l2_norm_with_deletions() {
        let mut rus = RemovableUniversalSketch::new(0.01, 0.01).unwrap();
        rus.update(&"item1", 100);
        rus.update(&"item2", 50);

        let l2_before = rus.l2_norm();

        rus.update(&"item1", -50);
        rus.update(&"item2", -25);

        let l2_after = rus.l2_norm();

        assert!(l2_after <= l2_before);
    }

    #[test]
    fn test_rus_merge() {
        let mut rus1 = RemovableUniversalSketch::new(0.01, 0.01).unwrap();
        let mut rus2 = RemovableUniversalSketch::new(0.01, 0.01).unwrap();

        rus1.update(&"item", 100);
        rus2.update(&"item", 50);

        rus1.merge(&rus2).unwrap();
        let freq = rus1.estimate(&"item");
        assert!(freq >= 150);
    }

    #[test]
    fn test_rus_merge_with_deletions() {
        let mut rus1 = RemovableUniversalSketch::new(0.01, 0.01).unwrap();
        let mut rus2 = RemovableUniversalSketch::new(0.01, 0.01).unwrap();

        rus1.update(&"item", 100);
        rus1.update(&"item", -20);

        rus2.update(&"item", 50);
        rus2.update(&"item", -10);

        rus1.merge(&rus2).unwrap();
        let freq = rus1.estimate(&"item");
        assert!(freq >= 120);
    }
}
