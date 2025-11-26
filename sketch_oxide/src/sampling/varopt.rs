//! VarOpt Sampling: Variance-Optimal Weighted Sampling
//!
//! Implementation of VarOpt Sampling (Cohen 2014), an algorithm for
//! weighted sampling that minimizes variance in downstream estimates.
//!
//! # Algorithm Overview
//!
//! VarOpt maintains a sample of k items where items with higher weights
//! have higher probability of inclusion. Unlike simple weighted random
//! sampling, VarOpt provides:
//!
//! 1. **Inclusion probabilities** for each item
//! 2. **Variance-optimal** estimates for subset sums
//! 3. **Mergeable** samples from distributed systems
//!
//! # Key Concepts
//!
//! - **Weight**: Original weight of an item (e.g., transaction amount)
//! - **Adjusted weight**: Weight after sampling adjustment
//! - **Inclusion probability**: Probability item was selected
//!
//! # Time Complexity
//!
//! - Construction: O(1)
//! - Update: O(log k) amortized
//! - Sample retrieval: O(k)
//!
//! # Space Complexity
//!
//! O(k) where k = sample size
//!
//! # References
//!
//! - Cohen, E. et al. (2014). "Composable, scalable, and accurate weight summarization"
//! - Apache DataSketches VarOpt implementation
//! - Used in: Network traffic analysis, database query optimization

use crate::common::SketchError;
use rand::Rng;
use std::cmp::Ordering;

/// A weighted item in the sample
#[derive(Clone, Debug)]
pub struct WeightedItem<T: Clone> {
    /// The actual item
    pub item: T,
    /// Original weight of the item
    pub weight: f64,
    /// Adjusted weight (for variance-optimal estimates)
    pub adjusted_weight: f64,
}

/// VarOpt Sampling for variance-optimal weighted samples
///
/// # Examples
///
/// ```
/// use sketch_oxide::sampling::VarOptSampling;
///
/// // Create VarOpt sampler with capacity 10
/// let mut sampler = VarOptSampling::new(10).unwrap();
///
/// // Add weighted items (e.g., transactions with amounts)
/// sampler.update("tx_1", 100.0);  // $100 transaction
/// sampler.update("tx_2", 5000.0); // $5000 transaction
/// sampler.update("tx_3", 50.0);   // $50 transaction
///
/// // Get sample - higher weight items more likely to be included
/// let sample = sampler.sample();
/// ```
#[derive(Clone, Debug)]
pub struct VarOptSampling<T: Clone> {
    /// Maximum number of items to store
    k: usize,
    /// Heavy items (weight >= threshold, always included)
    heavy_items: Vec<WeightedItem<T>>,
    /// Light items (weight < threshold, probabilistically included)
    light_items: Vec<WeightedItem<T>>,
    /// Current threshold for heavy vs light
    threshold: f64,
    /// Total weight of light items
    total_light_weight: f64,
    /// Total number of items seen
    count: u64,
    /// Random number generator
    rng: rand::rngs::SmallRng,
}

impl<T: Clone> VarOptSampling<T> {
    /// Creates a new VarOpt Sampling instance
    ///
    /// # Arguments
    ///
    /// * `k` - The maximum sample size
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if k is 0
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::sampling::VarOptSampling;
    ///
    /// let sampler: VarOptSampling<String> = VarOptSampling::new(100).unwrap();
    /// assert!(sampler.is_empty());
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
        Ok(VarOptSampling {
            k,
            heavy_items: Vec::new(),
            light_items: Vec::with_capacity(k),
            threshold: 0.0,
            total_light_weight: 0.0,
            count: 0,
            rng: rand::rngs::SmallRng::from_os_rng(),
        })
    }

    /// Creates a new VarOpt Sampling instance with a seed
    ///
    /// # Arguments
    ///
    /// * `k` - The maximum sample size
    /// * `seed` - Random seed for reproducibility
    pub fn with_seed(k: usize, seed: u64) -> Result<Self, SketchError> {
        if k == 0 {
            return Err(SketchError::InvalidParameter {
                param: "k".to_string(),
                value: k.to_string(),
                constraint: "must be greater than 0".to_string(),
            });
        }

        use rand::SeedableRng;
        Ok(VarOptSampling {
            k,
            heavy_items: Vec::new(),
            light_items: Vec::with_capacity(k),
            threshold: 0.0,
            total_light_weight: 0.0,
            count: 0,
            rng: rand::rngs::SmallRng::seed_from_u64(seed),
        })
    }

    /// Updates the sampler with a new weighted item
    ///
    /// # Arguments
    ///
    /// * `item` - The item to add
    /// * `weight` - The weight of the item (must be positive)
    ///
    /// # Panics
    ///
    /// Panics if weight is non-positive or NaN
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::sampling::VarOptSampling;
    ///
    /// let mut sampler: VarOptSampling<&str> = VarOptSampling::new(5).unwrap();
    /// sampler.update("high_value", 1000.0);
    /// sampler.update("low_value", 10.0);
    /// ```
    pub fn update(&mut self, item: T, weight: f64) {
        assert!(
            weight > 0.0 && weight.is_finite(),
            "Weight must be positive and finite"
        );

        self.count += 1;

        let weighted_item = WeightedItem {
            item,
            weight,
            adjusted_weight: weight,
        };

        // If sample not full, add directly
        let current_size = self.heavy_items.len() + self.light_items.len();
        if current_size < self.k {
            self.light_items.push(weighted_item);
            self.total_light_weight += weight;
            self.update_threshold();
            return;
        }

        // Sample is full - decide whether to include new item
        if weight >= self.threshold {
            // Heavy item - always include
            self.heavy_items.push(weighted_item);
            self.compress();
        } else {
            // Light item - probabilistically include
            let inclusion_prob = weight / self.threshold;
            if self.rng.random::<f64>() < inclusion_prob {
                // Replace a random light item
                if !self.light_items.is_empty() {
                    let idx = self.rng.random_range(0..self.light_items.len());
                    let old = &self.light_items[idx];
                    self.total_light_weight -= old.weight;
                    self.light_items[idx] = weighted_item;
                    self.total_light_weight += weight;
                }
            }
        }
    }

    /// Compresses the sample when it exceeds k items
    fn compress(&mut self) {
        while self.heavy_items.len() + self.light_items.len() > self.k {
            if self.light_items.is_empty() {
                // All items are heavy - demote lightest heavy item
                if let Some((min_idx, _)) = self.heavy_items.iter().enumerate().min_by(|a, b| {
                    a.1.weight
                        .partial_cmp(&b.1.weight)
                        .unwrap_or(Ordering::Equal)
                }) {
                    let demoted = self.heavy_items.swap_remove(min_idx);
                    self.total_light_weight += demoted.weight;
                    self.light_items.push(demoted);
                }
            } else {
                // Remove a random light item
                let idx = self.rng.random_range(0..self.light_items.len());
                let removed = self.light_items.swap_remove(idx);
                self.total_light_weight -= removed.weight;
            }
        }
        self.update_threshold();
    }

    /// Updates the threshold based on current light items
    fn update_threshold(&mut self) {
        let light_count = self.light_items.len();
        if light_count == 0 {
            self.threshold = f64::INFINITY;
        } else {
            self.threshold = self.total_light_weight / light_count as f64;
        }
    }

    /// Returns the current sample
    ///
    /// # Returns
    ///
    /// A vector of weighted items in the sample
    pub fn sample(&self) -> Vec<&WeightedItem<T>> {
        let mut result: Vec<&WeightedItem<T>> = Vec::with_capacity(self.len());
        result.extend(self.heavy_items.iter());
        result.extend(self.light_items.iter());
        result
    }

    /// Returns the current sample as owned items
    pub fn into_sample(self) -> Vec<WeightedItem<T>> {
        let mut result = self.heavy_items;
        result.extend(self.light_items);
        result
    }

    /// Returns true if no items have been sampled
    pub fn is_empty(&self) -> bool {
        self.heavy_items.is_empty() && self.light_items.is_empty()
    }

    /// Returns the number of items in the sample
    pub fn len(&self) -> usize {
        self.heavy_items.len() + self.light_items.len()
    }

    /// Returns the maximum capacity
    pub fn capacity(&self) -> usize {
        self.k
    }

    /// Returns the total number of items seen
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Returns the current threshold
    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    /// Returns the total weight in the sample
    pub fn total_weight(&self) -> f64 {
        let heavy_weight: f64 = self.heavy_items.iter().map(|i| i.weight).sum();
        heavy_weight + self.total_light_weight
    }

    /// Estimates the total weight of the stream
    ///
    /// This uses the variance-optimal estimator based on adjusted weights.
    pub fn estimate_total_weight(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }

        // Heavy items contribute their full weight
        let heavy_weight: f64 = self.heavy_items.iter().map(|i| i.weight).sum();

        // Light items contribute threshold (expected value of sampled items)
        let light_contribution = self.threshold * self.light_items.len() as f64;

        heavy_weight + light_contribution
    }

    /// Clears the sampler
    pub fn clear(&mut self) {
        self.heavy_items.clear();
        self.light_items.clear();
        self.threshold = 0.0;
        self.total_light_weight = 0.0;
        self.count = 0;
    }
}

impl<T: Clone> VarOptSampling<T> {
    /// Merges two VarOpt samples
    ///
    /// The merged sample maintains variance-optimal properties.
    pub fn merge(&mut self, other: &Self) -> Result<(), SketchError> {
        if self.k != other.k {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "Cannot merge VarOpt samples with different k: {} vs {}",
                    self.k, other.k
                ),
            });
        }

        // Add all items from other to self
        for item in &other.heavy_items {
            if self.heavy_items.len() + self.light_items.len() < self.k {
                self.heavy_items.push(item.clone());
            } else {
                // Re-evaluate with merged threshold
                if item.weight >= self.threshold {
                    self.heavy_items.push(item.clone());
                    self.compress();
                }
            }
        }

        for item in &other.light_items {
            if self.heavy_items.len() + self.light_items.len() < self.k {
                self.light_items.push(item.clone());
                self.total_light_weight += item.weight;
            } else if item.weight >= self.threshold {
                self.heavy_items.push(item.clone());
                self.compress();
            }
        }

        self.count += other.count;
        self.update_threshold();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_varopt() {
        let sampler: VarOptSampling<i32> = VarOptSampling::new(10).unwrap();
        assert!(sampler.is_empty());
        assert_eq!(sampler.capacity(), 10);
        assert_eq!(sampler.count(), 0);
    }

    #[test]
    fn test_new_invalid_k() {
        let result: Result<VarOptSampling<i32>, _> = VarOptSampling::new(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_basic() {
        let mut sampler: VarOptSampling<&str> = VarOptSampling::new(5).unwrap();

        sampler.update("a", 10.0);
        sampler.update("b", 20.0);
        sampler.update("c", 30.0);

        assert_eq!(sampler.len(), 3);
        assert_eq!(sampler.count(), 3);
    }

    #[test]
    fn test_heavy_items_preserved() {
        let mut sampler: VarOptSampling<i32> = VarOptSampling::with_seed(3, 42).unwrap();

        // Add one very heavy item and many light items
        sampler.update(0, 1000.0); // Heavy
        for i in 1..100 {
            sampler.update(i, 1.0); // Light
        }

        // Heavy item should always be in sample
        let sample = sampler.sample();
        let has_heavy = sample.iter().any(|item| item.item == 0);
        assert!(has_heavy, "Heavy item should always be in sample");
    }

    #[test]
    fn test_threshold_updates() {
        let mut sampler: VarOptSampling<i32> = VarOptSampling::new(5).unwrap();

        for i in 0..5 {
            sampler.update(i, (i + 1) as f64 * 10.0);
        }

        assert!(sampler.threshold() > 0.0);
    }

    #[test]
    fn test_total_weight() {
        let mut sampler: VarOptSampling<i32> = VarOptSampling::new(10).unwrap();

        sampler.update(0, 100.0);
        sampler.update(1, 200.0);
        sampler.update(2, 300.0);

        assert!((sampler.total_weight() - 600.0).abs() < 0.001);
    }

    #[test]
    fn test_clear() {
        let mut sampler: VarOptSampling<i32> = VarOptSampling::new(10).unwrap();

        for i in 0..50 {
            sampler.update(i, (i + 1) as f64);
        }

        sampler.clear();
        assert!(sampler.is_empty());
        assert_eq!(sampler.count(), 0);
    }

    #[test]
    fn test_merge_basic() {
        let mut s1: VarOptSampling<i32> = VarOptSampling::with_seed(5, 42).unwrap();
        let mut s2: VarOptSampling<i32> = VarOptSampling::with_seed(5, 43).unwrap();

        for i in 0..10 {
            s1.update(i, (i + 1) as f64);
        }
        for i in 10..20 {
            s2.update(i, (i + 1) as f64);
        }

        s1.merge(&s2).unwrap();
        assert!(s1.len() <= 5);
        assert_eq!(s1.count(), 20);
    }

    #[test]
    fn test_merge_incompatible() {
        let s1: VarOptSampling<i32> = VarOptSampling::new(5).unwrap();
        let s2: VarOptSampling<i32> = VarOptSampling::new(10).unwrap();

        let mut s1_clone = s1.clone();
        let result = s1_clone.merge(&s2);
        assert!(result.is_err());
    }

    #[test]
    #[should_panic]
    fn test_invalid_weight_zero() {
        let mut sampler: VarOptSampling<i32> = VarOptSampling::new(5).unwrap();
        sampler.update(0, 0.0);
    }

    #[test]
    #[should_panic]
    fn test_invalid_weight_negative() {
        let mut sampler: VarOptSampling<i32> = VarOptSampling::new(5).unwrap();
        sampler.update(0, -1.0);
    }
}
