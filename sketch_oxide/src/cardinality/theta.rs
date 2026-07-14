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

    /// Serializes the sketch to bytes (fable5 doc 01 F2 — Theta previously had
    /// no serialization despite union/intersect/difference being its selling
    /// point).
    ///
    /// Format: `[lg_k:1][seed:8][theta:8][num_entries:8][entry:8]*` (all LE).
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        use crate::common::WriteBuf;
        let entries = self.core.entries();
        let mut buf = WriteBuf::with_capacity(25 + entries.len() * 8);
        buf.write_u8(self.core.lg_k());
        buf.write_u64_le(self.seed);
        buf.write_u64_le(self.core.theta());
        buf.write_u64_le(entries.len() as u64);
        for &hash in entries.keys() {
            buf.write_u64_le(hash);
        }
        buf.into_bytes()
    }

    /// Deserializes a sketch from bytes produced by [`to_bytes`](Self::to_bytes).
    ///
    /// # Errors
    /// Returns `DeserializationError` on truncated/invalid input (panic-free) or
    /// `InvalidParameter` if the stored `lg_k` is out of range.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        use crate::common::ReadCursor;
        let mut cur = ReadCursor::new(bytes);
        let lg_k = cur.read_u8()?;
        let seed = cur.read_u64_le()?;
        let theta = cur.read_u64_le()?;
        // Each entry is 8 bytes; count is validated against the remaining input.
        let count = cur.read_len_prefixed_count(8)?;
        let mut entries = std::collections::HashMap::with_capacity(count);
        for _ in 0..count {
            entries.insert(cur.read_u64_le()?, NoSummary);
        }
        Ok(ThetaSketch {
            core: ThetaCore::from_raw_parts(lg_k, theta, entries)?,
            seed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_round_trips_and_supports_merge_after_deserialize() {
        // The canonical Theta workflow (fable5 doc 01 F2): build, serialize,
        // deserialize, then union — must preserve the estimate and set ops.
        let mut a = ThetaSketch::new(12).unwrap();
        for i in 0..5000u64 {
            a.update(&i);
        }
        let bytes = a.to_bytes();
        let restored = ThetaSketch::from_bytes(&bytes).expect("round-trip");
        assert_eq!(restored.num_retained(), a.num_retained());
        assert!((restored.estimate() - a.estimate()).abs() < 1e-9);

        // Union of the deserialized sketch with an overlapping one.
        let mut b = ThetaSketch::new(12).unwrap();
        for i in 2500..7500u64 {
            b.update(&i);
        }
        let union = restored.union(&b).unwrap();
        // True union cardinality is 7500; Theta estimate within a few %.
        assert!((union.estimate() - 7500.0).abs() / 7500.0 < 0.1);
    }

    #[test]
    fn deserialize_rejects_malformed_bytes_without_panic() {
        assert!(ThetaSketch::from_bytes(&[]).is_err());
        assert!(ThetaSketch::from_bytes(&[12]).is_err()); // lg_k only, truncated
        // lg_k=12, seed, theta, then a huge entry count with no data.
        let mut b = vec![12u8];
        b.extend_from_slice(&0u64.to_le_bytes());
        b.extend_from_slice(&u64::MAX.to_le_bytes());
        b.extend_from_slice(&u64::MAX.to_le_bytes()); // count
        assert!(ThetaSketch::from_bytes(&b).is_err());
    }

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

/// Capability-trait adoptions (see `crate::common::capabilities`).
mod capability_impls {
    use super::*;
    use crate::common::capabilities::{CardinalityEstimate, Update};
    use std::hash::Hash;

    impl<T: Hash> Update<T> for ThetaSketch {
        fn update(&mut self, item: &T) {
            ThetaSketch::update(self, item);
        }
    }

    impl CardinalityEstimate for ThetaSketch {
        fn estimate_cardinality(&self) -> f64 {
            self.estimate()
        }
    }

    impl crate::common::capabilities::Serializable for ThetaSketch {
        fn to_bytes(&self) -> crate::common::Result<Vec<u8>> {
            Ok(ThetaSketch::to_bytes(self))
        }
        fn from_bytes(bytes: &[u8]) -> crate::common::Result<Self> {
            ThetaSketch::from_bytes(bytes)
        }
    }
}
