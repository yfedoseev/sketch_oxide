//! MinHash: Jaccard Similarity Estimation
//!
//! Implementation of MinHash (Broder 1997), the standard algorithm for
//! estimating Jaccard similarity between sets. As of 2024, there is no
//! better alternative for this specific problem.
//!
//! # Algorithm Overview
//!
//! MinHash approximates the Jaccard similarity |A ∩ B| / |A ∪ B| by:
//! 1. Using k independent hash functions h₁, h₂, ..., hₖ
//! 2. For each set S, storing min_hash[i] = min{hᵢ(x) : x ∈ S}
//! 3. Estimating Jaccard as: (number of matches) / k
//!
//! Key property: P(min_hash_A[i] = min_hash_B[i]) = Jaccard(A, B)
//!
//! # Time Complexity
//!
//! - Construction: O(1)
//! - Update: O(k) where k = num_perm
//! - Jaccard similarity: O(k) comparisons
//! - Merge: O(k) min operations
//!
//! # Space Complexity
//!
//! O(k) storage for k hash values (typically 64-256, default 128)
//!
//! # Accuracy
//!
//! Standard error ≈ 1/√k
//! - k=64: ~12.5% error
//! - k=128: ~8.8% error
//! - k=256: ~6.25% error
//!
//! # References
//!
//! - Broder, A. Z. (1997). "On the resemblance and containment of documents"
//! - Used in: LSH, deduplication, recommendation systems, near-duplicate detection

use crate::common::{Mergeable, Sketch, SketchError};
use std::hash::Hash;

/// MinHash sketch for Jaccard similarity estimation
///
/// # Examples
///
/// ```
/// use sketch_oxide::similarity::MinHash;
/// use sketch_oxide::Mergeable;
///
/// // Create MinHash with 128 hash functions
/// let mut mh1 = MinHash::new(128).unwrap();
/// let mut mh2 = MinHash::new(128).unwrap();
///
/// // Add items to sets
/// for i in 0..100 {
///     mh1.update(&i);
/// }
/// for i in 50..150 {
///     mh2.update(&i);
/// }
///
/// // Estimate Jaccard similarity
/// let similarity = mh1.jaccard_similarity(&mh2).unwrap();
/// // Expected: ~0.33 (50 items overlap, 150 items union)
/// assert!((similarity - 0.33).abs() < 0.1);
///
/// // Merge for union
/// let mut union = mh1.clone();
/// union.merge(&mh2).unwrap();
/// ```
#[derive(Clone, Debug)]
pub struct MinHash {
    /// Number of hash functions (permutations)
    num_perm: usize,
    /// Minimum hash value for each hash function
    /// Initialized to u64::MAX, updated with min(current, hash(item))
    hash_values: Vec<u64>,
    /// Independent seeds for each hash function
    hash_seeds: Vec<u64>,
}

impl MinHash {
    /// Minimum recommended number of permutations
    const MIN_NUM_PERM: usize = 16;

    /// Creates a new MinHash sketch with specified number of permutations
    ///
    /// # Arguments
    ///
    /// * `num_perm` - Number of hash functions (permutations). Must be >= 16.
    ///   Typical values: 64-256. Higher values give better accuracy but use more memory.
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if:
    /// - `num_perm` is 0
    /// - `num_perm` is less than 16 (minimum for reasonable accuracy)
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::similarity::MinHash;
    ///
    /// let mh = MinHash::new(128).unwrap();
    /// assert!(MinHash::new(0).is_err());
    /// assert!(MinHash::new(8).is_err());
    /// ```
    pub fn new(num_perm: usize) -> Result<Self, SketchError> {
        if num_perm == 0 {
            return Err(SketchError::InvalidParameter {
                param: "num_perm".to_string(),
                value: num_perm.to_string(),
                constraint: "must be greater than 0".to_string(),
            });
        }

        if num_perm < Self::MIN_NUM_PERM {
            return Err(SketchError::InvalidParameter {
                param: "num_perm".to_string(),
                value: num_perm.to_string(),
                constraint: format!(
                    "must be at least {} for reasonable accuracy",
                    Self::MIN_NUM_PERM
                ),
            });
        }

        // Initialize hash values to maximum (no items seen yet)
        let hash_values = vec![u64::MAX; num_perm];

        // Generate independent seeds for each hash function
        // We use a deterministic sequence for reproducibility
        let hash_seeds = (0..num_perm)
            .map(|i| {
                // Generate seed using a simple mixing function
                let mut seed = i as u64;
                seed = seed.wrapping_mul(0x9e3779b97f4a7c15u64); // Golden ratio
                seed ^= seed >> 30;
                seed = seed.wrapping_mul(0xbf58476d1ce4e5b9u64);
                seed ^= seed >> 27;
                seed = seed.wrapping_mul(0x94d049bb133111ebu64);
                seed ^= seed >> 31;
                seed
            })
            .collect();

        Ok(MinHash {
            num_perm,
            hash_values,
            hash_seeds,
        })
    }

    /// Updates the MinHash sketch with a new item
    ///
    /// For each hash function i, computes hash_i(item) and updates
    /// hash_values[i] = min(hash_values[i], hash_i(item))
    ///
    /// # Time Complexity
    ///
    /// O(k) where k = num_perm
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::similarity::MinHash;
    ///
    /// let mut mh = MinHash::new(128).unwrap();
    /// mh.update(&"hello");
    /// mh.update(&42);
    /// mh.update(&vec![1, 2, 3]);
    /// ```
    pub fn update<T: Hash>(&mut self, item: &T) {
        // Hash the item ONCE (streaming, zero allocation), then derive all
        // `num_perm` permuted values by mixing the base hash with each seed via
        // the murmur3 finalizer — a bijection, so each is a valid MinHash
        // permutation. The old code hashed the full key `num_perm` times and
        // heap-allocated a `Vec<u8>` on every one of them (fable5 doc 02 #4:
        // 128 allocs + 128 full hashes per update). Wire format is unchanged
        // (still num_perm + seeds + values); only the produced values differ.
        let base = Self::hash_item(item);
        for i in 0..self.num_perm {
            let hash = Self::permute(base, self.hash_seeds[i]);

            // Update minimum
            if hash < self.hash_values[i] {
                self.hash_values[i] = hash;
            }
        }
    }

    /// Estimates Jaccard similarity between this sketch and another
    ///
    /// Jaccard similarity = |A ∩ B| / |A ∪ B|
    ///
    /// Estimated as: (number of matching hash values) / num_perm
    ///
    /// # Arguments
    ///
    /// * `other` - Another MinHash sketch to compare with
    ///
    /// # Returns
    ///
    /// - Similarity in range [0.0, 1.0]
    /// - 0.0 for two empty sketches (no items)
    /// - 1.0 for identical sets
    ///
    /// # Errors
    ///
    /// Returns `IncompatibleSketches` if num_perm differs
    ///
    /// # Time Complexity
    ///
    /// O(k) where k = num_perm
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::similarity::MinHash;
    ///
    /// let mut mh1 = MinHash::new(128).unwrap();
    /// let mut mh2 = MinHash::new(128).unwrap();
    ///
    /// // Identical sets
    /// for i in 0..100 {
    ///     mh1.update(&i);
    ///     mh2.update(&i);
    /// }
    /// assert!((mh1.jaccard_similarity(&mh2).unwrap() - 1.0).abs() < 0.01);
    ///
    /// // Disjoint sets
    /// let mut mh3 = MinHash::new(128).unwrap();
    /// for i in 200..300 {
    ///     mh3.update(&i);
    /// }
    /// assert!(mh1.jaccard_similarity(&mh3).unwrap() < 0.05);
    /// ```
    pub fn jaccard_similarity(&self, other: &Self) -> Result<f64, SketchError> {
        if self.num_perm != other.num_perm {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "Cannot compare MinHash with different num_perm: {} vs {}",
                    self.num_perm, other.num_perm
                ),
            });
        }

        // Check if both sketches are empty (all values are MAX)
        let self_empty = self.hash_values.iter().all(|&v| v == u64::MAX);
        let other_empty = other.hash_values.iter().all(|&v| v == u64::MAX);

        if self_empty && other_empty {
            // Two empty sets: Jaccard is undefined, return 0.0
            return Ok(0.0);
        }

        if self_empty || other_empty {
            // One empty, one non-empty: Jaccard is 0.0
            return Ok(0.0);
        }

        // Count matching hash values
        // According to MinHash theory: P(min_h(A) = min_h(B)) = |A ∩ B| / |A ∪ B|
        // We simply count where the hash values are equal
        let matches = self
            .hash_values
            .iter()
            .zip(other.hash_values.iter())
            .filter(|&(&a, &b)| a == b)
            .count();

        // Estimate Jaccard similarity
        let similarity = matches as f64 / self.num_perm as f64;
        Ok(similarity)
    }

    /// Hashes an item once to a 64-bit base value, streaming its bytes through
    /// xxHash64 with no intermediate allocation.
    #[inline]
    fn hash_item<T: Hash>(item: &T) -> u64 {
        use std::hash::Hasher as StdHasher;
        let mut hasher = twox_hash::XxHash64::with_seed(0);
        item.hash(&mut hasher);
        hasher.finish()
    }

    /// Derives the permutation-`seed` MinHash value from a base hash by mixing
    /// with the seed through the murmur3 64-bit finalizer. `fmix64` is a
    /// bijection over `u64`, so `fmix64(base ^ seed)` is a valid random
    /// permutation of the hash universe for each distinct seed.
    #[inline]
    fn permute(base: u64, seed: u64) -> u64 {
        let mut x = base ^ seed;
        x ^= x >> 33;
        x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
        x ^= x >> 33;
        x = x.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
        x ^= x >> 33;
        x
    }

    /// Returns the number of permutations (hash functions) used
    pub fn num_perm(&self) -> usize {
        self.num_perm
    }

    /// Returns the MinHash signature: the minimum hash value for each permutation.
    pub fn hashes(&self) -> &[u64] {
        &self.hash_values
    }

    /// Number of permutations in the industry-standard LLM-dedup preset:
    /// FineWeb / datatrove use **14 bands × 8 rows = 112** hashes over 5-grams
    /// at a ~0.75 Jaccard threshold (fable5 doc 03 C.4).
    pub const DATATROVE_NUM_PERM: usize = 112;

    /// Number of LSH bands in the datatrove/FineWeb preset.
    pub const DATATROVE_BANDS: usize = 14;

    /// Number of rows per LSH band in the datatrove/FineWeb preset.
    pub const DATATROVE_ROWS: usize = 8;

    /// Creates a MinHash configured to be signature-compatible with the
    /// FineWeb / datatrove LLM-deduplication pipeline (112 permutations).
    ///
    /// Pair with an LSH index of [`DATATROVE_BANDS`](Self::DATATROVE_BANDS) ×
    /// [`DATATROVE_ROWS`](Self::DATATROVE_ROWS) bands/rows.
    #[must_use]
    pub fn datatrove() -> Self {
        // 112 >= MIN_NUM_PERM, so this never fails.
        Self::new(Self::DATATROVE_NUM_PERM).expect("112 is a valid num_perm")
    }

    /// Exports the signature as a little-endian byte vector — the on-wire form
    /// consumed by vector databases as a bring-your-own signature (e.g. Milvus
    /// `MINHASH_LSH` / `BINARY_VECTOR`) and by LSHBloom-style combinators
    /// (fable5 doc 03 C.4). Length is `8 * num_perm` bytes.
    #[must_use]
    pub fn to_binary_vector(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.hash_values.len() * 8);
        for &h in &self.hash_values {
            out.extend_from_slice(&h.to_le_bytes());
        }
        out
    }
}

impl Sketch for MinHash {
    /// Item type (MinHash works with any hashable type)
    /// We use u64 as the nominal type, but update is generic
    type Item = u64;

    /// Update is provided as a generic method
    ///
    /// Use the generic `update<T: Hash>(&mut self, item: &T)` method instead.
    fn update(&mut self, item: &Self::Item) {
        // Call the generic update method
        self.update(item);
    }

    /// Estimate returns 0.0 as MinHash requires another sketch for similarity
    ///
    /// This is a placeholder to satisfy the Sketch trait.
    /// Use `jaccard_similarity(&self, other: &Self) -> Result<f64, SketchError>` instead.
    fn estimate(&self) -> f64 {
        0.0
    }

    /// Check if the sketch is empty (no items added)
    fn is_empty(&self) -> bool {
        self.hash_values.iter().all(|&v| v == u64::MAX)
    }

    /// Serialize the sketch to bytes
    fn serialize(&self) -> Vec<u8> {
        // Format: [num_perm:8][hash_seeds][hash_values]
        let mut bytes = Vec::new();

        // Write num_perm
        bytes.extend_from_slice(&(self.num_perm as u64).to_le_bytes());

        // Write hash seeds
        for &seed in &self.hash_seeds {
            bytes.extend_from_slice(&seed.to_le_bytes());
        }

        // Write hash values
        for &value in &self.hash_values {
            bytes.extend_from_slice(&value.to_le_bytes());
        }

        bytes
    }

    /// Deserialize a sketch from bytes
    fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        if bytes.len() < 8 {
            return Err(SketchError::DeserializationError(
                "Insufficient data for num_perm".to_string(),
            ));
        }

        // Read num_perm
        let num_perm = u64::from_le_bytes(
            bytes[0..8]
                .try_into()
                .map_err(|_| SketchError::DeserializationError("Invalid num_perm".to_string()))?,
        ) as usize;

        // `num_perm` is attacker-controlled (a full u64 truncated to usize). The
        // original `8 + (num_perm * 8 * 2)` is unchecked: for a crafted huge
        // `num_perm` it overflows, which under overflow-checks=true ABORTS the
        // process (DoS) and otherwise wraps to a small value that passes this
        // length check, then drives two giant `Vec::with_capacity(num_perm)`
        // allocations below (OOM). Compute the expected length with checked
        // arithmetic and reject on overflow. Once `bytes.len() == expected_len`
        // holds, `num_perm * 16 <= bytes.len()`, so the allocations are bounded
        // by the actual input size.
        let expected_len = num_perm
            .checked_mul(16) // seeds + values, 8 bytes each
            .and_then(|n| n.checked_add(8)) // + num_perm header
            .ok_or_else(|| SketchError::DeserializationError("num_perm too large".to_string()))?;
        if bytes.len() != expected_len {
            return Err(SketchError::DeserializationError(format!(
                "Invalid data length: expected {}, got {}",
                expected_len,
                bytes.len()
            )));
        }

        // Read hash seeds
        let mut hash_seeds = Vec::with_capacity(num_perm);
        for i in 0..num_perm {
            let offset = 8 + (i * 8);
            let seed = u64::from_le_bytes(
                bytes[offset..offset + 8]
                    .try_into()
                    .map_err(|_| SketchError::DeserializationError("Invalid seed".to_string()))?,
            );
            hash_seeds.push(seed);
        }

        // Read hash values
        let mut hash_values = Vec::with_capacity(num_perm);
        for i in 0..num_perm {
            let offset = 8 + (num_perm * 8) + (i * 8);
            let value = u64::from_le_bytes(bytes[offset..offset + 8].try_into().map_err(|_| {
                SketchError::DeserializationError("Invalid hash value".to_string())
            })?);
            hash_values.push(value);
        }

        Ok(MinHash {
            num_perm,
            hash_values,
            hash_seeds,
        })
    }
}

impl Mergeable for MinHash {
    /// Merges another MinHash sketch into this one
    ///
    /// After merging, this sketch represents the union of both sets.
    /// For each hash function i: hash_values[i] = min(self[i], other[i])
    ///
    /// # Arguments
    ///
    /// * `other` - Another MinHash sketch to merge
    ///
    /// # Errors
    ///
    /// Returns `IncompatibleSketches` if num_perm differs
    ///
    /// # Time Complexity
    ///
    /// O(k) where k = num_perm
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::similarity::MinHash;
    /// use sketch_oxide::Mergeable;
    ///
    /// let mut mh1 = MinHash::new(128).unwrap();
    /// let mut mh2 = MinHash::new(128).unwrap();
    ///
    /// for i in 0..50 {
    ///     mh1.update(&i);
    /// }
    /// for i in 50..100 {
    ///     mh2.update(&i);
    /// }
    ///
    /// mh1.merge(&mh2).unwrap();
    /// // mh1 now represents the union {0..100}
    /// ```
    fn merge(&mut self, other: &Self) -> Result<(), SketchError> {
        if self.num_perm != other.num_perm {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "Cannot merge MinHash with different num_perm: {} vs {}",
                    self.num_perm, other.num_perm
                ),
            });
        }

        // Take minimum for each hash function (union operation)
        for i in 0..self.num_perm {
            self.hash_values[i] = self.hash_values[i].min(other.hash_values[i]);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn datatrove_preset_and_binary_export() {
        let mut mh = MinHash::datatrove();
        assert_eq!(mh.num_perm(), 112);
        assert_eq!(
            MinHash::DATATROVE_BANDS * MinHash::DATATROVE_ROWS,
            MinHash::DATATROVE_NUM_PERM
        );
        for i in 0..1000u64 {
            mh.update(&i);
        }
        // Binary vector is the LE-encoded signature: 8 bytes per permutation.
        let bin = mh.to_binary_vector();
        assert_eq!(bin.len(), 112 * 8);
        // First 8 bytes decode to the first signature value.
        let first = u64::from_le_bytes(bin[0..8].try_into().unwrap());
        assert_eq!(first, mh.hashes()[0]);
    }

    #[test]
    fn hash_once_permutation_preserves_jaccard_accuracy() {
        // The hash-once + fmix64 permutation family must still estimate Jaccard
        // accurately. Two sets with a known overlap of 0.5 should estimate close.
        let mut a = MinHash::new(256).unwrap();
        let mut b = MinHash::new(256).unwrap();
        for i in 0..1000u64 {
            a.update(&i);
        }
        for i in 500..1500u64 {
            b.update(&i);
        }
        // |A ∩ B| = 500, |A ∪ B| = 1500 -> Jaccard = 1/3.
        let est = a.jaccard_similarity(&b).unwrap();
        assert!(
            (est - (1.0 / 3.0)).abs() < 0.06,
            "Jaccard estimate {est} too far from 1/3"
        );
    }

    #[test]
    fn deserialize_rejects_overflowing_num_perm_without_panic() {
        // Regression: a crafted `num_perm` near u64::MAX made the old
        // `8 + (num_perm * 8 * 2)` overflow, aborting under overflow-checks or
        // wrapping past the length check into a giant `Vec::with_capacity` (OOM).
        // Must be rejected cleanly.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&u64::MAX.to_le_bytes()); // num_perm
        bytes.extend_from_slice(&[0u8; 16]); // short tail
        assert!(
            MinHash::deserialize(&bytes).is_err(),
            "must error, not overflow/OOM"
        );

        // A large-but-non-overflowing count with a truncated tail must also error.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1_000_000u64.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 16]);
        assert!(MinHash::deserialize(&bytes).is_err());

        // Truncated header.
        assert!(MinHash::deserialize(&[0u8; 4]).is_err());
    }

    #[test]
    fn deserialize_round_trips_populated_sketch() {
        let mut mh = MinHash::new(64).unwrap();
        for i in 0..500u64 {
            mh.update(&i);
        }
        let bytes = mh.serialize();
        let restored = MinHash::deserialize(&bytes).expect("round-trip");
        assert_eq!(restored.num_perm(), mh.num_perm());
        assert_eq!(restored.hashes(), mh.hashes());
        assert!((restored.jaccard_similarity(&mh).unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_new_minhash() {
        let mh = MinHash::new(128);
        assert!(mh.is_ok());

        let mh = mh.unwrap();
        assert_eq!(mh.num_perm(), 128);
        assert_eq!(mh.hash_values.len(), 128);
        assert_eq!(mh.hash_seeds.len(), 128);

        // All hash values should be initialized to MAX
        assert!(mh.hash_values.iter().all(|&v| v == u64::MAX));
    }

    #[test]
    fn test_update() {
        let mut mh = MinHash::new(128).unwrap();
        mh.update(&"test");

        // After update, some hash values should be less than MAX
        assert!(mh.hash_values.iter().any(|&v| v != u64::MAX));
    }

    #[test]
    fn test_identical_sets() {
        let mut mh1 = MinHash::new(128).unwrap();
        let mut mh2 = MinHash::new(128).unwrap();

        for i in 0..100 {
            mh1.update(&i);
            mh2.update(&i);
        }

        let sim = mh1.jaccard_similarity(&mh2).unwrap();
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_disjoint_sets() {
        let mut mh1 = MinHash::new(128).unwrap();
        let mut mh2 = MinHash::new(128).unwrap();

        for i in 0..100 {
            mh1.update(&i);
        }
        for i in 100..200 {
            mh2.update(&i);
        }

        let sim = mh1.jaccard_similarity(&mh2).unwrap();
        assert!(sim < 0.05);
    }

    #[test]
    fn test_merge() {
        let mut mh1 = MinHash::new(128).unwrap();
        let mut mh2 = MinHash::new(128).unwrap();

        for i in 0..50 {
            mh1.update(&i);
        }
        for i in 50..100 {
            mh2.update(&i);
        }

        mh1.merge(&mh2).unwrap();

        // Create reference union
        let mut mh_union = MinHash::new(128).unwrap();
        for i in 0..100 {
            mh_union.update(&i);
        }

        let sim = mh1.jaccard_similarity(&mh_union).unwrap();
        assert!(sim > 0.95);
    }

    #[test]
    fn test_hash_seeds_are_different() {
        let mh = MinHash::new(128).unwrap();

        // Seeds should be different from each other
        let unique_seeds: std::collections::HashSet<_> = mh.hash_seeds.iter().collect();
        assert_eq!(unique_seeds.len(), 128, "Seeds should be unique");
    }
}

// ---------------------------------------------------------------------------
// Capability-trait adoptions (fable5 doc 01 F3 "split the `Sketch` trait").
// MinHash ingests any hashable item and has a fully working, overflow-hardened
// `Sketch` serialize/deserialize, so it cleanly satisfies `Update` and
// `Serializable`. It has no cardinality/quantile/point/membership semantics
// (it estimates Jaccard similarity between two sketches), so those are skipped.
// ---------------------------------------------------------------------------
use crate::common::{Serializable, Update};

impl<T: Hash> Update<T> for MinHash {
    fn update(&mut self, item: &T) {
        MinHash::update(self, item);
    }
}

impl Serializable for MinHash {
    fn to_bytes(&self) -> crate::common::Result<Vec<u8>> {
        Ok(<Self as Sketch>::serialize(self))
    }

    fn from_bytes(bytes: &[u8]) -> crate::common::Result<Self> {
        <Self as Sketch>::deserialize(bytes)
    }
}
