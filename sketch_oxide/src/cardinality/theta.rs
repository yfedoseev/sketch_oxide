//! Theta Sketch - Set Operations
//!
//! Apache DataSketches Theta Sketch implementation.
//! **The only sketch supporting intersection/difference operations.**
//!
//! # Overview
//!
//! Theta Sketch is a probabilistic data structure for cardinality estimation
//! that uniquely supports set operations:
//! - Union: |A ∪ B|
//! - Intersection: |A ∩ B|
//! - Difference: |A - B| (A-not-B)
//!
//! # Algorithm
//!
//! 1. Hash items to uniform u64 values
//! 2. Keep hashes < theta (sampling threshold)
//! 3. When |entries| > k, reduce theta (sampling)
//! 4. Estimate: count * (u64::MAX / theta)
//!
//! # Set Operations
//!
//! - **Union**: Merge entries, use min(theta_a, theta_b)
//! - **Intersection**: Keep common entries, use min(theta_a, theta_b)
//! - **Difference**: Keep A entries not in B, use min(theta_a, theta_b)
//!
//! # Accuracy
//!
//! - Relative error: ~1/sqrt(k) where k = 2^lg_k
//! - Example: lg_k=12 (k=4096) → ~1.6% error
//! - Exact mode when n < k (no sampling)
//!
//! # Production Usage
//!
//! - LinkedIn: 10+ years in production
//! - ClickHouse: 24.1+ (AggregateFunctionThetaSketch)
//! - Yahoo: Large-scale analytics
//!
//! # References
//!
//! - Paper: "Theta Sketch Framework" (Apache DataSketches)
//! - Source: https://datasketches.apache.org/docs/Theta/ThetaSketchFramework.html

use crate::cardinality::theta_core::{NoSummary, ThetaCore};
use crate::error::{Result, SketchError};
use std::hash::{Hash, Hasher};

/// Theta Sketch for cardinality estimation with set operations.
///
/// # Examples
///
/// ```
/// use sketch_oxide::cardinality::ThetaSketch;
///
/// let mut sketch_a = ThetaSketch::new(12).unwrap(); // k = 4096
/// let mut sketch_b = ThetaSketch::new(12).unwrap();
///
/// // Add items
/// for i in 0..100 {
///     sketch_a.update(&i);
/// }
/// for i in 50..150 {
///     sketch_b.update(&i);
/// }
///
/// // Set operations
/// let union = sketch_a.union(&sketch_b).unwrap();
/// let intersection = sketch_a.intersect(&sketch_b).unwrap();
/// let difference = sketch_a.difference(&sketch_b).unwrap();
///
/// println!("Union: {}", union.estimate());
/// println!("Intersection: {}", intersection.estimate());
/// println!("Difference: {}", difference.estimate());
/// ```
#[derive(Clone, Debug)]
pub struct ThetaSketch {
    /// Generic Theta engine with the empty summary (plain set behaviour).
    core: ThetaCore<NoSummary>,

    /// Hash seed for consistency across operations.
    seed: u64,
}

impl ThetaSketch {
    /// Default hash seed (same as Apache DataSketches)
    const DEFAULT_SEED: u64 = 9001;

    /// Creates a new Theta Sketch with specified lg_k.
    ///
    /// # Parameters
    ///
    /// - `lg_k`: log2(k), determines accuracy and memory
    ///   - Valid range: [4, 26]
    ///   - k = 2^lg_k (nominal entries)
    ///   - Memory: ~8k bytes
    ///   - Error: ~1/sqrt(k)
    ///
    /// # Recommended Values
    ///
    /// - lg_k=12 (k=4096): ~1.6% error, 32KB
    /// - lg_k=14 (k=16384): ~0.8% error, 128KB
    /// - lg_k=16 (k=65536): ~0.4% error, 512KB
    ///
    /// # Errors
    ///
    /// Returns `SketchError::InvalidParameter` if lg_k is out of range.
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::cardinality::ThetaSketch;
    ///
    /// let sketch = ThetaSketch::new(12).unwrap();
    /// ```
    pub fn new(lg_k: u8) -> Result<Self> {
        Ok(Self {
            core: ThetaCore::new(lg_k)?,
            seed: Self::DEFAULT_SEED,
        })
    }

    /// Creates a sketch with custom seed (for advanced use).
    pub fn with_seed(lg_k: u8, seed: u64) -> Result<Self> {
        let mut sketch = Self::new(lg_k)?;
        sketch.seed = seed;
        Ok(sketch)
    }

    /// Updates the sketch with a new item.
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::cardinality::ThetaSketch;
    ///
    /// let mut sketch = ThetaSketch::new(12).unwrap();
    /// sketch.update(&"item1");
    /// sketch.update(&42);
    /// sketch.update(&3.14_f64.to_bits());
    /// ```
    pub fn update<T: Hash>(&mut self, item: &T) {
        let hash = self.hash_item(item);
        self.core.update(hash, NoSummary);
    }

    /// Estimates the cardinality.
    ///
    /// # Formula
    ///
    /// - If theta = u64::MAX (no sampling): count
    /// - Otherwise: count * (u64::MAX / theta)
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::cardinality::ThetaSketch;
    ///
    /// let mut sketch = ThetaSketch::new(12).unwrap();
    /// for i in 0..1000 {
    ///     sketch.update(&i);
    /// }
    /// let estimate = sketch.estimate();
    /// assert!((estimate - 1000.0).abs() < 20.0);
    /// ```
    pub fn estimate(&self) -> f64 {
        self.core.estimate()
    }

    /// Returns true if the sketch is empty.
    pub fn is_empty(&self) -> bool {
        self.core.is_empty()
    }

    /// Returns the number of retained entries.
    pub fn num_retained(&self) -> usize {
        self.core.num_retained()
    }

    /// Returns the current theta value.
    ///
    /// - u64::MAX: No sampling (exact mode)
    /// - < u64::MAX: Sampling active
    pub fn get_theta(&self) -> u64 {
        self.core.theta()
    }

    /// Returns the nominal capacity (k).
    pub fn capacity(&self) -> usize {
        self.core.capacity()
    }

    /// Computes union with another sketch: |A ∪ B|
    ///
    /// # Compatibility
    ///
    /// Both sketches must have:
    /// - Same lg_k
    /// - Same seed
    ///
    /// # Algorithm
    ///
    /// 1. new_theta = min(self.theta, other.theta)
    /// 2. new_entries = (self.entries ∪ other.entries) where hash < new_theta
    /// 3. Estimate from merged sketch
    ///
    /// # Properties
    ///
    /// - Commutative: A∪B = B∪A
    /// - Associative: (A∪B)∪C = A∪(B∪C)
    /// - Idempotent: A∪A = A
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::cardinality::ThetaSketch;
    ///
    /// let mut sketch_a = ThetaSketch::new(12).unwrap();
    /// let mut sketch_b = ThetaSketch::new(12).unwrap();
    ///
    /// for i in 0..50 {
    ///     sketch_a.update(&i);
    /// }
    /// for i in 50..100 {
    ///     sketch_b.update(&i);
    /// }
    ///
    /// let union = sketch_a.union(&sketch_b).unwrap();
    /// assert!((union.estimate() - 100.0).abs() < 5.0);
    /// ```
    pub fn union(&self, other: &Self) -> Result<Self> {
        self.check_seed(other)?;
        Ok(Self {
            core: self.core.union(&other.core)?,
            seed: self.seed,
        })
    }

    /// Computes intersection with another sketch: |A ∩ B|
    ///
    /// # Algorithm
    ///
    /// 1. new_theta = min(self.theta, other.theta)
    /// 2. new_entries = (self.entries ∩ other.entries) where hash < new_theta
    /// 3. Estimate from intersection sketch
    ///
    /// # Properties
    ///
    /// - Commutative: A∩B = B∩A
    /// - Associative: (A∩B)∩C = A∩(B∩C)
    /// - Idempotent: A∩A = A
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::cardinality::ThetaSketch;
    ///
    /// let mut sketch_a = ThetaSketch::new(12).unwrap();
    /// let mut sketch_b = ThetaSketch::new(12).unwrap();
    ///
    /// for i in 0..75 {
    ///     sketch_a.update(&i);
    /// }
    /// for i in 25..100 {
    ///     sketch_b.update(&i);
    /// }
    ///
    /// let intersection = sketch_a.intersect(&sketch_b).unwrap();
    /// assert!((intersection.estimate() - 50.0).abs() < 5.0);
    /// ```
    pub fn intersect(&self, other: &Self) -> Result<Self> {
        self.check_seed(other)?;
        Ok(Self {
            core: self.core.intersect(&other.core)?,
            seed: self.seed,
        })
    }

    /// Computes difference: |A - B| (items in A but not in B)
    ///
    /// # Algorithm
    ///
    /// 1. new_theta = min(self.theta, other.theta)
    /// 2. new_entries = (self.entries - other.entries) where hash < new_theta
    /// 3. Estimate from difference sketch
    ///
    /// # Properties
    ///
    /// - NOT commutative: A-B ≠ B-A (in general)
    /// - A-A = ∅
    /// - A-∅ = A
    /// - ∅-B = ∅
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::cardinality::ThetaSketch;
    ///
    /// let mut sketch_a = ThetaSketch::new(12).unwrap();
    /// let mut sketch_b = ThetaSketch::new(12).unwrap();
    ///
    /// for i in 0..75 {
    ///     sketch_a.update(&i);
    /// }
    /// for i in 25..100 {
    ///     sketch_b.update(&i);
    /// }
    ///
    /// let difference = sketch_a.difference(&sketch_b).unwrap();
    /// assert!((difference.estimate() - 25.0).abs() < 5.0);
    /// ```
    pub fn difference(&self, other: &Self) -> Result<Self> {
        self.check_seed(other)?;
        Ok(Self {
            core: self.core.difference(&other.core)?,
            seed: self.seed,
        })
    }

    // ============================================================================
    // Private Methods
    // ============================================================================

    /// Checks that two sketches share a hash seed (capacity/`lg_k` is checked by the core).
    fn check_seed(&self, other: &Self) -> Result<()> {
        if self.seed != other.seed {
            return Err(SketchError::IncompatibleSketches {
                reason: format!("seed mismatch: {} vs {}", self.seed, other.seed),
            });
        }
        Ok(())
    }

    /// Hashes an item to u64 using xxHash-like algorithm.
    fn hash_item<T: Hash>(&self, item: &T) -> u64 {
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();
        self.seed.hash(&mut hasher);
        item.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_creation() {
        let sketch = ThetaSketch::new(12).unwrap();
        assert_eq!(sketch.capacity(), 4096); // k = 2^12
        assert_eq!(sketch.get_theta(), u64::MAX);
        assert!(sketch.is_empty());
    }

    #[test]
    fn test_hash_consistency() {
        let sketch = ThetaSketch::new(12).unwrap();
        let hash1 = sketch.hash_item(&"test");
        let hash2 = sketch.hash_item(&"test");
        assert_eq!(hash1, hash2, "Hash should be deterministic");
    }

    #[test]
    fn test_seed_affects_hash() {
        let sketch1 = ThetaSketch::new(12).unwrap();
        let sketch2 = ThetaSketch::with_seed(12, 1234).unwrap();

        let hash1 = sketch1.hash_item(&"test");
        let hash2 = sketch2.hash_item(&"test");

        assert_ne!(
            hash1, hash2,
            "Different seeds should produce different hashes"
        );
    }
}
