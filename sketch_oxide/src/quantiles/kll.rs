//! KLL Sketch: Nearly optimal quantile approximation (Karnin 2016)
//!
//! KLL Sketch is the standard quantile algorithm in the Apache ecosystem
//! (Druid, Spark, Flink). It provides absolute error guarantees with
//! near-optimal space usage.
//!
//! # Algorithm Overview
//!
//! KLL maintains items in multiple "levels" with exponentially increasing
//! size limits. When a level overflows, items are compacted (sorted,
//! half selected) and promoted to the next level.
//!
//! # Comparison with DDSketch and T-Digest
//!
//! | Algorithm | Error Type | Memory | Best For |
//! |-----------|-----------|--------|----------|
//! | KLL | Absolute (±ε for all ranks) | O(k log(n/k)) | Apache ecosystem |
//! | DDSketch | Relative (±ε × value) | O(log range) | Wide-range data |
//! | T-Digest | Relative at tails | O(compression) | Extreme percentiles |
//!
//! # Time Complexity
//!
//! - Update: O(1) amortized
//! - Quantile: O(k log k)
//! - Merge: O(k log k)
//!
//! # Space Complexity
//!
//! O(k log(n/k)) where k is the accuracy parameter
//!
//! # References
//!
//! - Karnin, Lang, Liberty "Optimal Quantile Approximation in Streams" (2016)
//! - Apache DataSketches KLL
//!
//! # Examples
//!
//! ```
//! use sketch_oxide::quantiles::KllSketch;
//!
//! let mut kll = KllSketch::new(200).unwrap();
//!
//! for i in 0..10_000 {
//!     kll.update(i as f64);
//! }
//!
//! let median = kll.quantile(0.5);
//! let p99 = kll.quantile(0.99);
//! println!("Median: {:?}, P99: {:?}", median, p99);
//! ```

use crate::common::{Mergeable, Sketch, SketchError};

/// KLL Sketch for quantile estimation
///
/// Provides absolute error guarantees: for any quantile q, the returned
/// value is within ε × n ranks of the true answer, where ε ≈ 1/k.
///
/// # Examples
///
/// ```
/// use sketch_oxide::quantiles::KllSketch;
/// use sketch_oxide::Sketch;
///
/// let mut kll = KllSketch::new(200).unwrap();
/// for i in 0..1000 {
///     kll.update(i as f64);
/// }
/// let median = kll.quantile(0.5);
/// assert!(median.is_some());
/// ```
#[derive(Clone, Debug)]
pub struct KllSketch {
    /// Accuracy parameter (higher = more accurate)
    k: u16,
    /// Items at each level
    levels: Vec<Vec<f64>>,
    /// Total items seen
    n: u64,
    /// Minimum value seen
    min_value: f64,
    /// Maximum value seen
    max_value: f64,
    /// Whether compaction is needed
    needs_sort: bool,
}

impl KllSketch {
    /// Minimum k value
    pub const MIN_K: u16 = 8;

    /// Maximum k value
    pub const MAX_K: u16 = 65535;

    /// Default k value (good balance of accuracy and memory)
    pub const DEFAULT_K: u16 = 200;

    /// Growth factor between levels
    const GROWTH_FACTOR: f64 = 2.0;

    /// Creates a new KLL Sketch
    ///
    /// # Arguments
    ///
    /// * `k` - Accuracy parameter (8-65535)
    ///   - Higher k = more accurate, more memory
    ///   - k=200 gives ~1.65% normalized rank error
    ///   - k=100 gives ~3.3% normalized rank error
    ///
    /// # Errors
    ///
    /// Returns error if k is out of valid range
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::quantiles::KllSketch;
    ///
    /// let kll = KllSketch::new(200).unwrap();
    /// ```
    pub fn new(k: u16) -> Result<Self, SketchError> {
        if k < Self::MIN_K {
            return Err(SketchError::InvalidParameter {
                param: "k".to_string(),
                value: k.to_string(),
                constraint: format!("must be at least {}", Self::MIN_K),
            });
        }

        Ok(KllSketch {
            k,
            levels: vec![Vec::with_capacity(k as usize)],
            n: 0,
            min_value: f64::INFINITY,
            max_value: f64::NEG_INFINITY,
            needs_sort: false,
        })
    }

    /// Creates a KLL Sketch with default k (200)
    pub fn default_k() -> Self {
        Self::new(Self::DEFAULT_K).unwrap()
    }

    /// Returns the k parameter
    pub fn k(&self) -> u16 {
        self.k
    }

    /// Returns the number of items seen
    pub fn count(&self) -> u64 {
        self.n
    }

    /// Returns the minimum value seen
    pub fn min(&self) -> f64 {
        self.min_value
    }

    /// Returns the maximum value seen
    pub fn max(&self) -> f64 {
        self.max_value
    }

    /// Returns the normalized rank error bound
    ///
    /// The actual rank of the returned quantile value is within
    /// ±(error × n) of the requested rank.
    pub fn normalized_rank_error(&self) -> f64 {
        // Approximate error bound: ~1.65/k for single sketch
        1.65 / self.k as f64
    }

    /// Returns the number of items retained in the sketch
    pub fn num_retained(&self) -> usize {
        self.levels.iter().map(|l| l.len()).sum()
    }

    /// Updates the sketch with a value
    ///
    /// # Arguments
    ///
    /// * `value` - Value to add (NaN and infinity are ignored)
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::quantiles::KllSketch;
    ///
    /// let mut kll = KllSketch::new(200).unwrap();
    /// kll.update(42.0);
    /// kll.update(100.0);
    /// ```
    pub fn update(&mut self, value: f64) {
        if !value.is_finite() {
            return;
        }

        self.n += 1;
        self.min_value = self.min_value.min(value);
        self.max_value = self.max_value.max(value);

        // Add to level 0
        self.levels[0].push(value);
        self.needs_sort = true;

        // Check if compaction is needed
        if self.levels[0].len() >= self.level_capacity(0) {
            self.compact();
        }
    }

    /// Returns the capacity for a given level
    fn level_capacity(&self, level: usize) -> usize {
        // Level 0 has capacity k, each subsequent level has 2x
        let capacity = (self.k as f64) * Self::GROWTH_FACTOR.powi(level as i32);
        capacity.ceil() as usize
    }

    /// Compacts the sketch when level 0 overflows
    fn compact(&mut self) {
        let mut level = 0;

        while level < self.levels.len() && self.levels[level].len() >= self.level_capacity(level) {
            // Sort level if needed
            self.levels[level].sort_by(|a, b| a.partial_cmp(b).unwrap());

            // Select every other item (compaction)
            let compacted: Vec<f64> = self.levels[level]
                .iter()
                .enumerate()
                .filter(|(i, _)| i % 2 == 0)
                .map(|(_, &v)| v)
                .collect();

            // Move compacted items to next level
            if level + 1 >= self.levels.len() {
                self.levels
                    .push(Vec::with_capacity(self.level_capacity(level + 1)));
            }

            self.levels[level + 1].extend(compacted);
            self.levels[level].clear();

            level += 1;
        }

        self.needs_sort = true;
    }

    /// Ensures all levels are sorted for quantile queries
    fn ensure_sorted(&mut self) {
        if self.needs_sort {
            for level in &mut self.levels {
                level.sort_by(|a, b| a.partial_cmp(b).unwrap());
            }
            self.needs_sort = false;
        }
    }

    /// Returns the estimated quantile value
    ///
    /// # Arguments
    ///
    /// * `rank` - The quantile rank (0.0 to 1.0)
    ///
    /// # Returns
    ///
    /// The estimated value at the given quantile, or None if empty
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::quantiles::KllSketch;
    ///
    /// let mut kll = KllSketch::new(200).unwrap();
    /// for i in 0..1000 {
    ///     kll.update(i as f64);
    /// }
    ///
    /// let median = kll.quantile(0.5);
    /// let p99 = kll.quantile(0.99);
    /// ```
    pub fn quantile(&mut self, rank: f64) -> Option<f64> {
        if self.n == 0 {
            return None;
        }

        let rank = rank.clamp(0.0, 1.0);

        if rank == 0.0 {
            return Some(self.min_value);
        }
        if rank == 1.0 {
            return Some(self.max_value);
        }

        self.ensure_sorted();

        // Collect all items with their weights
        let mut items: Vec<(f64, u64)> = Vec::new();
        for (level, level_items) in self.levels.iter().enumerate() {
            let weight = 1u64 << level; // Weight doubles per level
            for &item in level_items {
                items.push((item, weight));
            }
        }

        // Sort by value
        items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // Find the item at the target rank
        let target = (rank * self.n as f64) as u64;
        let mut cumulative = 0u64;

        for (value, weight) in items {
            cumulative += weight;
            if cumulative >= target {
                return Some(value);
            }
        }

        Some(self.max_value)
    }

    /// Returns the estimated rank of a value
    ///
    /// # Arguments
    ///
    /// * `value` - Value to find rank for
    ///
    /// # Returns
    ///
    /// Estimated fraction of values <= given value (0.0 to 1.0)
    pub fn rank(&mut self, value: f64) -> f64 {
        if self.n == 0 {
            return 0.0;
        }

        if value <= self.min_value {
            return 0.0;
        }
        if value >= self.max_value {
            return 1.0;
        }

        self.ensure_sorted();

        // Count weighted items less than or equal to value
        let mut count = 0u64;
        for (level, level_items) in self.levels.iter().enumerate() {
            let weight = 1u64 << level;
            for &item in level_items {
                if item <= value {
                    count += weight;
                }
            }
        }

        count as f64 / self.n as f64
    }

    /// Returns the CDF (cumulative distribution function)
    ///
    /// Returns pairs of (value, cumulative_rank) for all retained items.
    pub fn cdf(&mut self) -> Vec<(f64, f64)> {
        if self.n == 0 {
            return Vec::new();
        }

        self.ensure_sorted();

        // Collect all items with weights
        let mut items: Vec<(f64, u64)> = Vec::new();
        for (level, level_items) in self.levels.iter().enumerate() {
            let weight = 1u64 << level;
            for &item in level_items {
                items.push((item, weight));
            }
        }

        // Sort by value
        items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // Compute cumulative ranks
        let mut result = Vec::with_capacity(items.len());
        let mut cumulative = 0u64;

        for (value, weight) in items {
            cumulative += weight;
            result.push((value, cumulative as f64 / self.n as f64));
        }

        result
    }

    /// Serializes the KLL Sketch to bytes
    pub fn to_bytes(&mut self) -> Vec<u8> {
        self.ensure_sorted();

        let mut bytes = Vec::new();

        // Header: k (2), n (8), min (8), max (8), num_levels (2)
        bytes.extend_from_slice(&self.k.to_le_bytes());
        bytes.extend_from_slice(&self.n.to_le_bytes());
        bytes.extend_from_slice(&self.min_value.to_le_bytes());
        bytes.extend_from_slice(&self.max_value.to_le_bytes());
        bytes.extend_from_slice(&(self.levels.len() as u16).to_le_bytes());

        // Each level: num_items (4) + items (8 each)
        for level in &self.levels {
            bytes.extend_from_slice(&(level.len() as u32).to_le_bytes());
            for &item in level {
                bytes.extend_from_slice(&item.to_le_bytes());
            }
        }

        bytes
    }

    /// Deserializes a KLL Sketch from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SketchError> {
        if bytes.len() < 28 {
            return Err(SketchError::DeserializationError(
                "Insufficient data for KLL header".to_string(),
            ));
        }

        let k = u16::from_le_bytes(bytes[0..2].try_into().unwrap());
        let n = u64::from_le_bytes(bytes[2..10].try_into().unwrap());
        let min_value = f64::from_le_bytes(bytes[10..18].try_into().unwrap());
        let max_value = f64::from_le_bytes(bytes[18..26].try_into().unwrap());
        let num_levels = u16::from_le_bytes(bytes[26..28].try_into().unwrap()) as usize;

        let mut offset = 28;
        let mut levels = Vec::with_capacity(num_levels);

        for _ in 0..num_levels {
            if offset + 4 > bytes.len() {
                return Err(SketchError::DeserializationError(
                    "Truncated level data".to_string(),
                ));
            }

            let num_items =
                u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;

            if offset + num_items * 8 > bytes.len() {
                return Err(SketchError::DeserializationError(
                    "Truncated item data".to_string(),
                ));
            }

            let mut level = Vec::with_capacity(num_items);
            for _ in 0..num_items {
                let item = f64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
                level.push(item);
                offset += 8;
            }

            levels.push(level);
        }

        Ok(KllSketch {
            k,
            levels,
            n,
            min_value,
            max_value,
            needs_sort: false,
        })
    }
}

impl Default for KllSketch {
    fn default() -> Self {
        Self::default_k()
    }
}

impl Sketch for KllSketch {
    type Item = f64;

    fn update(&mut self, item: &Self::Item) {
        self.update(*item);
    }

    fn estimate(&self) -> f64 {
        let mut kll = self.clone();
        kll.quantile(0.5).unwrap_or(0.0)
    }

    fn is_empty(&self) -> bool {
        self.n == 0
    }

    fn serialize(&self) -> Vec<u8> {
        let mut kll = self.clone();
        kll.to_bytes()
    }

    fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
        Self::from_bytes(bytes)
    }
}

impl Mergeable for KllSketch {
    /// Merges another KLL Sketch into this one
    ///
    /// Both sketches should have the same k parameter for best results.
    fn merge(&mut self, other: &Self) -> Result<(), SketchError> {
        if self.k != other.k {
            return Err(SketchError::IncompatibleSketches {
                reason: format!("k mismatch: {} vs {}", self.k, other.k),
            });
        }

        // Update min/max
        self.min_value = self.min_value.min(other.min_value);
        self.max_value = self.max_value.max(other.max_value);
        self.n += other.n;

        // Merge levels
        for (level, other_level) in other.levels.iter().enumerate() {
            while self.levels.len() <= level {
                self.levels.push(Vec::new());
            }
            self.levels[level].extend(other_level.iter().copied());
        }

        // Compact if needed
        self.needs_sort = true;
        for level in 0..self.levels.len() {
            if self.levels[level].len() >= self.level_capacity(level) {
                self.compact();
                break;
            }
        }

        Ok(())
    }
}

/// Type alias for float sketches (common use case)
pub type KllFloatSketch = KllSketch;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_kll() {
        let kll = KllSketch::new(200).unwrap();
        assert!(kll.is_empty());
        assert_eq!(kll.k(), 200);
    }

    #[test]
    fn test_invalid_k() {
        assert!(KllSketch::new(5).is_err()); // Too small
    }

    #[test]
    fn test_update() {
        let mut kll = KllSketch::new(200).unwrap();
        kll.update(42.0);
        assert!(!kll.is_empty());
        assert_eq!(kll.count(), 1);
    }

    #[test]
    fn test_quantile_single() {
        let mut kll = KllSketch::new(200).unwrap();
        kll.update(100.0);
        assert_eq!(kll.quantile(0.5), Some(100.0));
    }

    #[test]
    fn test_quantile_uniform() {
        let mut kll = KllSketch::new(200).unwrap();
        for i in 0..1000 {
            kll.update(i as f64);
        }

        let p50 = kll.quantile(0.5).unwrap();
        assert!(
            (p50 - 500.0).abs() < 100.0,
            "Median {} too far from 500",
            p50
        );
    }

    #[test]
    fn test_min_max() {
        let mut kll = KllSketch::new(200).unwrap();
        kll.update(10.0);
        kll.update(100.0);
        kll.update(50.0);

        assert_eq!(kll.min(), 10.0);
        assert_eq!(kll.max(), 100.0);
    }

    #[test]
    fn test_merge() {
        let mut kll1 = KllSketch::new(200).unwrap();
        let mut kll2 = KllSketch::new(200).unwrap();

        for i in 0..500 {
            kll1.update(i as f64);
        }
        for i in 500..1000 {
            kll2.update(i as f64);
        }

        kll1.merge(&kll2).unwrap();
        assert_eq!(kll1.count(), 1000);
    }

    #[test]
    fn test_serialization() {
        let mut kll = KllSketch::new(200).unwrap();
        for i in 0..1000 {
            kll.update(i as f64);
        }

        let bytes = kll.to_bytes();
        let restored = KllSketch::from_bytes(&bytes).unwrap();

        assert_eq!(kll.k(), restored.k());
        assert_eq!(kll.count(), restored.count());
    }
}
