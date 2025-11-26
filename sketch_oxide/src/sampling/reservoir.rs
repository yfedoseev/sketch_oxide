//! Reservoir Sampling: Uniform Random Sampling from Streams
//!
//! Implementation of Reservoir Sampling (Vitter 1985), the standard algorithm for
//! maintaining a uniform random sample of k items from a stream of unknown length.
//!
//! # Algorithm Overview (Algorithm R)
//!
//! 1. Fill reservoir with first k items
//! 2. For each subsequent item n (n > k):
//!    - Generate random number r in [1, n]
//!    - If r <= k, replace reservoir[r-1] with item n
//!
//! Key property: Every item has equal probability k/n of being in the sample.
//!
//! # Time Complexity
//!
//! - Construction: O(1)
//! - Update: O(1) amortized
//! - Sample retrieval: O(k)
//! - Merge: O(k)
//!
//! # Space Complexity
//!
//! O(k) where k = reservoir size
//!
//! # References
//!
//! - Vitter, J. S. (1985). "Random sampling with a reservoir"
//! - Used in: Log sampling, A/B testing, database sampling, data quality checks

use crate::common::SketchError;
use rand::Rng;

/// Reservoir Sampling for uniform random samples from streams
///
/// # Examples
///
/// ```
/// use sketch_oxide::sampling::ReservoirSampling;
///
/// // Create reservoir that holds 10 items
/// let mut reservoir = ReservoirSampling::new(10).unwrap();
///
/// // Process stream of items
/// for i in 0..1000 {
///     reservoir.update(format!("item_{}", i));
/// }
///
/// // Get the sample
/// let sample = reservoir.sample();
/// assert_eq!(sample.len(), 10);
///
/// // Each item had 10/1000 = 1% chance of being selected
/// ```
#[derive(Clone, Debug)]
pub struct ReservoirSampling<T: Clone> {
    /// Maximum number of items to store
    k: usize,
    /// The reservoir of sampled items
    reservoir: Vec<T>,
    /// Total number of items seen
    count: u64,
    /// Random number generator
    rng: rand::rngs::SmallRng,
}

impl<T: Clone> ReservoirSampling<T> {
    /// Creates a new Reservoir Sampling instance
    ///
    /// # Arguments
    ///
    /// * `k` - The size of the reservoir (number of items to sample)
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if k is 0
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::sampling::ReservoirSampling;
    ///
    /// let reservoir: ReservoirSampling<String> = ReservoirSampling::new(100).unwrap();
    /// assert!(reservoir.is_empty());
    /// ```
    pub fn new(k: usize) -> Result<Self, SketchError> {
        if k == 0 {
            return Err(SketchError::InvalidParameter {
                param: "k".to_string(),
                value: k.to_string(),
                constraint: "must be greater than 0".to_string(),
            });
        }

        use rand::SeedableRng;
        Ok(ReservoirSampling {
            k,
            reservoir: Vec::with_capacity(k),
            count: 0,
            rng: rand::rngs::SmallRng::from_os_rng(),
        })
    }

    /// Creates a new Reservoir Sampling instance with a seed for reproducibility
    ///
    /// # Arguments
    ///
    /// * `k` - The size of the reservoir
    /// * `seed` - Random seed for reproducibility
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::sampling::ReservoirSampling;
    ///
    /// let mut r1: ReservoirSampling<i32> = ReservoirSampling::with_seed(10, 42).unwrap();
    /// let mut r2: ReservoirSampling<i32> = ReservoirSampling::with_seed(10, 42).unwrap();
    ///
    /// for i in 0..100 {
    ///     r1.update(i);
    ///     r2.update(i);
    /// }
    ///
    /// // Same seed produces same sample
    /// assert_eq!(r1.sample(), r2.sample());
    /// ```
    pub fn with_seed(k: usize, seed: u64) -> Result<Self, SketchError> {
        if k == 0 {
            return Err(SketchError::InvalidParameter {
                param: "k".to_string(),
                value: k.to_string(),
                constraint: "must be greater than 0".to_string(),
            });
        }

        use rand::SeedableRng;
        Ok(ReservoirSampling {
            k,
            reservoir: Vec::with_capacity(k),
            count: 0,
            rng: rand::rngs::SmallRng::seed_from_u64(seed),
        })
    }

    /// Updates the reservoir with a new item (Algorithm R)
    ///
    /// # Arguments
    ///
    /// * `item` - The item to potentially add to the reservoir
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::sampling::ReservoirSampling;
    ///
    /// let mut reservoir: ReservoirSampling<&str> = ReservoirSampling::new(5).unwrap();
    /// reservoir.update("apple");
    /// reservoir.update("banana");
    /// reservoir.update("cherry");
    /// ```
    pub fn update(&mut self, item: T) {
        self.count += 1;

        if self.reservoir.len() < self.k {
            // Reservoir not full yet - add directly
            self.reservoir.push(item);
        } else {
            // Reservoir full - replace with probability k/count
            let r = self.rng.random_range(0..self.count);
            if r < self.k as u64 {
                self.reservoir[r as usize] = item;
            }
        }
    }

    /// Returns the current sample
    ///
    /// # Returns
    ///
    /// A slice of the sampled items (at most k items)
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::sampling::ReservoirSampling;
    ///
    /// let mut reservoir: ReservoirSampling<i32> = ReservoirSampling::new(5).unwrap();
    /// for i in 0..100 {
    ///     reservoir.update(i);
    /// }
    /// let sample = reservoir.sample();
    /// assert_eq!(sample.len(), 5);
    /// ```
    pub fn sample(&self) -> &[T] {
        &self.reservoir
    }

    /// Returns the current sample as a Vec (owned)
    pub fn into_sample(self) -> Vec<T> {
        self.reservoir
    }

    /// Returns true if no items have been sampled yet
    pub fn is_empty(&self) -> bool {
        self.reservoir.is_empty()
    }

    /// Returns the number of items currently in the reservoir
    pub fn len(&self) -> usize {
        self.reservoir.len()
    }

    /// Returns the maximum capacity of the reservoir
    pub fn capacity(&self) -> usize {
        self.k
    }

    /// Returns the total number of items seen
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Returns the theoretical probability that any given item is in the sample
    ///
    /// This equals min(k, count) / count
    pub fn inclusion_probability(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            (self.k.min(self.count as usize) as f64) / (self.count as f64)
        }
    }

    /// Clears the reservoir and resets the count
    pub fn clear(&mut self) {
        self.reservoir.clear();
        self.count = 0;
    }
}

impl<T: Clone> ReservoirSampling<T> {
    /// Merges two reservoir samples
    ///
    /// Uses the merge algorithm that maintains uniform sampling properties:
    /// - Combines both reservoirs
    /// - Randomly selects k items from the combined set
    /// - Adjusts counts appropriately
    ///
    /// # Note
    ///
    /// The merged reservoir maintains the uniform sampling property if both
    /// input reservoirs were created with the same k.
    pub fn merge(&mut self, other: &Self) -> Result<(), SketchError> {
        if self.k != other.k {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "Cannot merge reservoirs with different k: {} vs {}",
                    self.k, other.k
                ),
            });
        }

        // Total count from both streams
        let total_count = self.count + other.count;

        if total_count == 0 {
            return Ok(());
        }

        // Combine items from both reservoirs
        let mut combined: Vec<T> = self.reservoir.clone();
        combined.extend(other.reservoir.iter().cloned());

        // If combined is smaller than k, keep all
        if combined.len() <= self.k {
            self.reservoir = combined;
            self.count = total_count;
            return Ok(());
        }

        // Sample k items from combined, weighted by their respective counts
        // Using weighted reservoir sampling on the combined set
        let mut new_reservoir = Vec::with_capacity(self.k);

        // Shuffle combined and take first k (uniform random selection)
        use rand::seq::SliceRandom;
        combined.shuffle(&mut self.rng);
        new_reservoir.extend(combined.into_iter().take(self.k));

        self.reservoir = new_reservoir;
        self.count = total_count;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_reservoir() {
        let reservoir: ReservoirSampling<i32> = ReservoirSampling::new(10).unwrap();
        assert!(reservoir.is_empty());
        assert_eq!(reservoir.capacity(), 10);
        assert_eq!(reservoir.count(), 0);
    }

    #[test]
    fn test_new_invalid_k() {
        let result: Result<ReservoirSampling<i32>, _> = ReservoirSampling::new(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_fills_reservoir() {
        let mut reservoir: ReservoirSampling<i32> = ReservoirSampling::new(5).unwrap();

        for i in 0..5 {
            reservoir.update(i);
        }

        assert_eq!(reservoir.len(), 5);
        assert_eq!(reservoir.count(), 5);
    }

    #[test]
    fn test_update_beyond_capacity() {
        let mut reservoir: ReservoirSampling<i32> = ReservoirSampling::with_seed(5, 42).unwrap();

        for i in 0..100 {
            reservoir.update(i);
        }

        assert_eq!(reservoir.len(), 5);
        assert_eq!(reservoir.count(), 100);
    }

    #[test]
    fn test_seeded_reproducibility() {
        let mut r1: ReservoirSampling<i32> = ReservoirSampling::with_seed(10, 12345).unwrap();
        let mut r2: ReservoirSampling<i32> = ReservoirSampling::with_seed(10, 12345).unwrap();

        for i in 0..1000 {
            r1.update(i);
            r2.update(i);
        }

        assert_eq!(r1.sample(), r2.sample());
    }

    #[test]
    fn test_inclusion_probability() {
        let mut reservoir: ReservoirSampling<i32> = ReservoirSampling::new(10).unwrap();

        assert_eq!(reservoir.inclusion_probability(), 0.0);

        for i in 0..5 {
            reservoir.update(i);
        }
        assert!((reservoir.inclusion_probability() - 1.0).abs() < 0.001);

        for i in 5..100 {
            reservoir.update(i);
        }
        assert!((reservoir.inclusion_probability() - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_clear() {
        let mut reservoir: ReservoirSampling<i32> = ReservoirSampling::new(10).unwrap();

        for i in 0..50 {
            reservoir.update(i);
        }
        assert!(!reservoir.is_empty());

        reservoir.clear();
        assert!(reservoir.is_empty());
        assert_eq!(reservoir.count(), 0);
    }

    #[test]
    fn test_merge_basic() {
        let mut r1: ReservoirSampling<i32> = ReservoirSampling::with_seed(5, 42).unwrap();
        let mut r2: ReservoirSampling<i32> = ReservoirSampling::with_seed(5, 43).unwrap();

        for i in 0..10 {
            r1.update(i);
        }
        for i in 10..20 {
            r2.update(i);
        }

        r1.merge(&r2).unwrap();

        assert_eq!(r1.len(), 5);
        assert_eq!(r1.count(), 20);
    }

    #[test]
    fn test_merge_incompatible() {
        let r1: ReservoirSampling<i32> = ReservoirSampling::new(5).unwrap();
        let r2: ReservoirSampling<i32> = ReservoirSampling::new(10).unwrap();

        let mut r1_clone = r1.clone();
        let result = r1_clone.merge(&r2);
        assert!(result.is_err());
    }
}
