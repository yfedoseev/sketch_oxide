//! SimHash: Locality-Sensitive Hashing for Near-Duplicate Detection
//!
//! Implementation of SimHash (Charikar 2002), used by Google for web crawling
//! and near-duplicate detection. Unlike MinHash (Jaccard similarity), SimHash
//! estimates cosine similarity and works with weighted features.
//!
//! # Algorithm Overview
//!
//! SimHash creates a fingerprint by:
//! 1. For each feature in the input, compute a hash
//! 2. For each bit position, add +1 if the bit is 1, -1 if 0 (weighted by feature weight)
//! 3. Final fingerprint: bit[i] = 1 if sum[i] > 0, else 0
//!
//! Two documents are similar if their SimHash values have small Hamming distance.
//!
//! # When to Use
//!
//! - **SimHash**: Faster, text-specific, Hamming distance 3-7 detection
//! - **MinHash**: More general, can find 5%+ similarity, better for sets
//!
//! # Time Complexity
//!
//! - Hash computation: O(n) where n = number of features
//! - Hamming distance: O(1) using popcount
//! - Similarity: O(1)
//!
//! # Space Complexity
//!
//! O(1) - Only stores a 64-bit hash value
//!
//! # References
//!
//! - Charikar, M. (2002). "Similarity estimation techniques from rounding algorithms"
//! - Manku, G. S. et al. (2007). "Detecting near-duplicates for web crawling"
//! - Used by: Google (web crawling), Twitter (spam detection)

use crate::common::hash::xxhash;
use crate::common::{Mergeable, Sketch, SketchError};
use std::hash::Hash;

/// SimHash sketch for near-duplicate detection via cosine similarity
///
/// # Examples
///
/// ```
/// use sketch_oxide::similarity::SimHash;
///
/// // Create SimHash instances
/// let mut sh1 = SimHash::new();
/// let mut sh2 = SimHash::new();
///
/// // Add features (e.g., words from documents)
/// sh1.update("hello");
/// sh1.update("world");
/// sh1.update("test");
///
/// sh2.update("hello");
/// sh2.update("world");
/// sh2.update("different");
///
/// // Check similarity using Hamming distance
/// let distance = sh1.hamming_distance(&mut sh2);
/// let similarity = sh1.similarity(&mut sh2);
/// println!("Hamming distance: {}, Similarity: {:.2}", distance, similarity);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimHash {
    /// The fingerprint as 64-bit hash
    fingerprint: u64,
    /// Accumulator for computing the fingerprint (before finalization)
    /// Each index represents a bit position, value is the running sum
    accumulator: Vec<i64>,
    /// Whether the fingerprint has been finalized
    finalized: bool,
    /// Number of features added
    count: usize,
}

impl Default for SimHash {
    fn default() -> Self {
        Self::new()
    }
}

impl SimHash {
    /// Number of bits in the fingerprint
    pub const BITS: usize = 64;

    /// Creates a new SimHash sketch
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::similarity::SimHash;
    /// use sketch_oxide::Sketch;
    ///
    /// let sh = SimHash::new();
    /// assert!(sh.is_empty());
    /// ```
    pub fn new() -> Self {
        SimHash {
            fingerprint: 0,
            accumulator: vec![0i64; Self::BITS],
            finalized: false,
            count: 0,
        }
    }

    /// Updates the SimHash with a new feature (weight = 1)
    ///
    /// # Arguments
    ///
    /// * `feature` - A hashable feature (e.g., word, n-gram, shingle)
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::similarity::SimHash;
    /// use sketch_oxide::Sketch;
    ///
    /// let mut sh = SimHash::new();
    /// sh.update("hello");
    /// sh.update("world");
    /// assert!(!sh.is_empty());
    /// ```
    pub fn update<T: Hash + ?Sized>(&mut self, feature: &T) {
        self.update_weighted(feature, 1);
    }

    /// Updates the SimHash with a weighted feature
    ///
    /// Higher weights make the feature more influential in the final hash.
    ///
    /// # Arguments
    ///
    /// * `feature` - A hashable feature
    /// * `weight` - The weight of this feature (typically word frequency)
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::similarity::SimHash;
    ///
    /// let mut sh = SimHash::new();
    /// // "important" appears 5 times
    /// sh.update_weighted("important", 5);
    /// // "the" appears once
    /// sh.update_weighted("the", 1);
    /// ```
    pub fn update_weighted<T: Hash + ?Sized>(&mut self, feature: &T, weight: i64) {
        if self.finalized {
            // Re-open for updates
            self.finalized = false;
        }

        let hash = self.hash_feature(feature);
        self.count += 1;

        // Update accumulator for each bit
        for i in 0..Self::BITS {
            if (hash >> i) & 1 == 1 {
                self.accumulator[i] += weight;
            } else {
                self.accumulator[i] -= weight;
            }
        }
    }

    /// Finalizes and returns the fingerprint
    ///
    /// Call this after adding all features. The fingerprint is computed
    /// by setting each bit to 1 if the accumulator sum is positive.
    ///
    /// # Returns
    ///
    /// The 64-bit SimHash fingerprint
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::similarity::SimHash;
    ///
    /// let mut sh = SimHash::new();
    /// sh.update("hello");
    /// sh.update("world");
    /// let fp = sh.fingerprint();
    /// println!("Fingerprint: {:016x}", fp);
    /// ```
    pub fn fingerprint(&mut self) -> u64 {
        if !self.finalized {
            self.compute_fingerprint();
        }
        self.fingerprint
    }

    /// Computes the fingerprint from the accumulator
    fn compute_fingerprint(&mut self) {
        self.fingerprint = 0;
        for i in 0..Self::BITS {
            if self.accumulator[i] > 0 {
                self.fingerprint |= 1u64 << i;
            }
        }
        self.finalized = true;
    }

    /// Computes Hamming distance to another SimHash
    ///
    /// Hamming distance is the number of bit positions where the fingerprints differ.
    /// Lower distance means more similar documents.
    ///
    /// # Arguments
    ///
    /// * `other` - Another SimHash to compare with
    ///
    /// # Returns
    ///
    /// Number of differing bits (0-64)
    ///
    /// # Note
    ///
    /// Both SimHashes are finalized automatically if needed.
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::similarity::SimHash;
    ///
    /// let mut sh1 = SimHash::new();
    /// let mut sh2 = SimHash::new();
    ///
    /// sh1.update("hello");
    /// sh1.update("world");
    ///
    /// sh2.update("hello");
    /// sh2.update("world");
    ///
    /// assert_eq!(sh1.hamming_distance(&mut sh2), 0); // Identical
    /// ```
    pub fn hamming_distance(&mut self, other: &mut SimHash) -> u32 {
        let fp1 = self.fingerprint();
        let fp2 = other.fingerprint();
        (fp1 ^ fp2).count_ones()
    }

    /// Computes similarity as (64 - hamming_distance) / 64
    ///
    /// # Returns
    ///
    /// Similarity score in range [0.0, 1.0]
    /// - 1.0 = identical fingerprints
    /// - 0.0 = completely different (all 64 bits differ)
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::similarity::SimHash;
    ///
    /// let mut sh1 = SimHash::new();
    /// let mut sh2 = SimHash::new();
    ///
    /// sh1.update("hello");
    /// sh2.update("hello");
    ///
    /// assert!(sh1.similarity(&mut sh2) > 0.9);
    /// ```
    pub fn similarity(&mut self, other: &mut SimHash) -> f64 {
        let distance = self.hamming_distance(other);
        (Self::BITS as u32 - distance) as f64 / Self::BITS as f64
    }

    /// Computes similarity from pre-computed fingerprints (immutable)
    ///
    /// Use this when you want to compare without modifying the sketches.
    ///
    /// # Arguments
    ///
    /// * `fp1` - First fingerprint
    /// * `fp2` - Second fingerprint
    ///
    /// # Returns
    ///
    /// Similarity score in range [0.0, 1.0]
    pub fn similarity_from_fingerprints(fp1: u64, fp2: u64) -> f64 {
        let distance = (fp1 ^ fp2).count_ones();
        (Self::BITS as u32 - distance) as f64 / Self::BITS as f64
    }

    /// Computes Hamming distance from pre-computed fingerprints
    pub fn hamming_distance_from_fingerprints(fp1: u64, fp2: u64) -> u32 {
        (fp1 ^ fp2).count_ones()
    }

    /// Returns the number of features added
    pub fn len(&self) -> usize {
        self.count
    }

    /// Returns true if no features have been added
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Hashes a feature to 64-bit value
    fn hash_feature<T: Hash + ?Sized>(&self, feature: &T) -> u64 {
        use std::hash::Hasher as StdHasher;

        struct ByteHasher {
            bytes: Vec<u8>,
        }

        impl StdHasher for ByteHasher {
            fn finish(&self) -> u64 {
                0
            }

            fn write(&mut self, bytes: &[u8]) {
                self.bytes.extend_from_slice(bytes);
            }
        }

        let mut hasher = ByteHasher { bytes: Vec::new() };
        feature.hash(&mut hasher);
        xxhash(&hasher.bytes, 0)
    }

    /// Serializes the SimHash to bytes
    pub fn to_bytes(&mut self) -> Vec<u8> {
        let fp = self.fingerprint();
        let mut bytes = Vec::with_capacity(8 + 8 + 1 + Self::BITS * 8);

        // Write fingerprint
        bytes.extend_from_slice(&fp.to_le_bytes());

        // Write count
        bytes.extend_from_slice(&(self.count as u64).to_le_bytes());

        // Write finalized flag
        bytes.push(self.finalized as u8);

        // Write accumulator
        for &val in &self.accumulator {
            bytes.extend_from_slice(&val.to_le_bytes());
        }

        bytes
    }

    /// Deserializes a SimHash from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SketchError> {
        let min_len = 8 + 8 + 1 + Self::BITS * 8;
        if bytes.len() < min_len {
            return Err(SketchError::DeserializationError(format!(
                "Insufficient data: expected at least {} bytes, got {}",
                min_len,
                bytes.len()
            )));
        }

        let fingerprint =
            u64::from_le_bytes(bytes[0..8].try_into().map_err(|_| {
                SketchError::DeserializationError("Invalid fingerprint".to_string())
            })?);

        let count = u64::from_le_bytes(
            bytes[8..16]
                .try_into()
                .map_err(|_| SketchError::DeserializationError("Invalid count".to_string()))?,
        ) as usize;

        let finalized = bytes[16] != 0;

        let mut accumulator = Vec::with_capacity(Self::BITS);
        for i in 0..Self::BITS {
            let offset = 17 + i * 8;
            let val = i64::from_le_bytes(bytes[offset..offset + 8].try_into().map_err(|_| {
                SketchError::DeserializationError("Invalid accumulator".to_string())
            })?);
            accumulator.push(val);
        }

        Ok(SimHash {
            fingerprint,
            accumulator,
            finalized,
            count,
        })
    }
}

impl Sketch for SimHash {
    type Item = String;

    fn update(&mut self, item: &Self::Item) {
        self.update(item);
    }

    /// Returns the fingerprint as f64 (not meaningful, use fingerprint() instead)
    fn estimate(&self) -> f64 {
        self.fingerprint as f64
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }

    fn serialize(&self) -> Vec<u8> {
        let mut sh = self.clone();
        sh.to_bytes()
    }

    fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        Self::from_bytes(bytes)
    }
}

impl Mergeable for SimHash {
    /// Merges another SimHash into this one by combining accumulators
    ///
    /// This is useful for combining SimHash sketches from different
    /// parts of a document or from distributed processing.
    ///
    /// # Note
    ///
    /// Both sketches must be in an unfinalized state for accurate merging.
    /// If either is finalized, the merge combines accumulators but the
    /// result should be re-finalized.
    fn merge(&mut self, other: &Self) -> Result<(), SketchError> {
        // Combine accumulators
        for i in 0..Self::BITS {
            self.accumulator[i] += other.accumulator[i];
        }
        self.count += other.count;
        self.finalized = false; // Need to recompute fingerprint
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_simhash() {
        let sh = SimHash::new();
        assert!(sh.is_empty());
        assert_eq!(sh.len(), 0);
    }

    #[test]
    fn test_update() {
        let mut sh = SimHash::new();
        sh.update("hello");
        assert!(!sh.is_empty());
        assert_eq!(sh.len(), 1);
    }

    #[test]
    fn test_identical_inputs() {
        let mut sh1 = SimHash::new();
        let mut sh2 = SimHash::new();

        sh1.update("hello");
        sh1.update("world");

        sh2.update("hello");
        sh2.update("world");

        assert_eq!(sh1.hamming_distance(&mut sh2), 0);
        assert!((sh1.similarity(&mut sh2) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_different_inputs() {
        let mut sh1 = SimHash::new();
        let mut sh2 = SimHash::new();

        sh1.update("hello");
        sh1.update("world");

        sh2.update("completely");
        sh2.update("different");

        // Should have some Hamming distance
        let distance = sh1.hamming_distance(&mut sh2);
        assert!(distance > 0);
    }

    #[test]
    fn test_similar_inputs() {
        let mut sh1 = SimHash::new();
        let mut sh2 = SimHash::new();

        // Same base, slight difference
        for word in ["the", "quick", "brown", "fox", "jumps"] {
            sh1.update(word);
            sh2.update(word);
        }

        sh1.update("over");
        sh2.update("under"); // Different word

        // Should be quite similar
        let similarity = sh1.similarity(&mut sh2);
        assert!(similarity > 0.7);
    }

    #[test]
    fn test_weighted_update() {
        let mut sh = SimHash::new();
        sh.update_weighted("important", 10);
        sh.update_weighted("noise", 1);

        assert_eq!(sh.len(), 2);
    }

    #[test]
    fn test_merge() {
        let mut sh1 = SimHash::new();
        let mut sh2 = SimHash::new();

        sh1.update("hello");
        sh2.update("world");

        let mut combined = SimHash::new();
        combined.update("hello");
        combined.update("world");

        sh1.merge(&sh2).unwrap();

        // Merged should be same as combined
        assert_eq!(sh1.fingerprint(), combined.fingerprint());
    }

    #[test]
    fn test_serialization() {
        let mut sh = SimHash::new();
        sh.update("hello");
        sh.update("world");

        let bytes = sh.to_bytes();
        let restored = SimHash::from_bytes(&bytes).unwrap();

        assert_eq!(sh.fingerprint, restored.fingerprint);
        assert_eq!(sh.count, restored.count);
    }

    #[test]
    fn test_fingerprint_from_static() {
        let fp1 = 0b1010101010101010u64;
        let fp2 = 0b1010101010101011u64;

        let distance = SimHash::hamming_distance_from_fingerprints(fp1, fp2);
        assert_eq!(distance, 1);

        let similarity = SimHash::similarity_from_fingerprints(fp1, fp2);
        assert!((similarity - 63.0 / 64.0).abs() < 0.001);
    }
}
