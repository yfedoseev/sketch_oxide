//! Linear Counting — a simple, accurate cardinality sketch for small-to-moderate sets.
//!
//! Linear Counting (Whang, Vander-Zanden & Taylor, "A linear-time probabilistic counting
//! algorithm for database applications", TODS 1990) is the simplest distinct-count sketch: a
//! bitmap of `m` bits where each item sets bit `hash(item) mod m`. The cardinality is
//! recovered from how *full* the bitmap is — `n̂ = −m·ln(z/m)` where `z` is the number of bits
//! still zero. For load factors up to a few, it is more accurate than HyperLogLog and exact
//! when no collisions occur, which is exactly why HLL uses it as its small-range correction.
//! It is also fully mergeable (bitwise OR).

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

/// A Linear Counting bitmap cardinality estimator.
///
/// # Example
/// ```
/// use sketch_oxide::cardinality::LinearCounting;
///
/// let mut lc = LinearCounting::new(1 << 16).unwrap(); // 65536-bit bitmap
/// for i in 0..5000u64 { lc.add(&i.to_le_bytes()); }
/// let est = lc.estimate();
/// assert!((est - 5000.0).abs() < 100.0, "estimate {est}");
/// ```
#[derive(Debug, Clone)]
pub struct LinearCounting {
    num_bits: usize,
    bits: Vec<u64>,
    set_count: usize,
}

impl LinearCounting {
    /// Creates a bitmap of `num_bits` bits (rounded up to a multiple of 64).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `num_bits` is 0.
    pub fn new(num_bits: usize) -> Result<Self> {
        if num_bits == 0 {
            return Err(SketchError::InvalidParameter {
                param: "num_bits".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            num_bits,
            bits: vec![0u64; num_bits.div_ceil(64)],
            set_count: 0,
        })
    }

    /// Adds an item.
    pub fn add(&mut self, item: &[u8]) {
        let i = (xxhash(item, 0) % self.num_bits as u64) as usize;
        let word = i / 64;
        let mask = 1u64 << (i % 64);
        if self.bits[word] & mask == 0 {
            self.bits[word] |= mask;
            self.set_count += 1;
        }
    }

    /// Number of zero bits remaining.
    #[inline]
    fn zeros(&self) -> usize {
        self.num_bits - self.set_count
    }

    /// Estimated number of distinct items.
    ///
    /// Returns the linear-counting estimate `−m·ln(z/m)`. When the bitmap is saturated
    /// (`z == 0`) the estimate is reported as if one bit remained, an underestimate beyond
    /// the sketch's useful range.
    pub fn estimate(&self) -> f64 {
        let m = self.num_bits as f64;
        let z = self.zeros().max(1) as f64; // avoid ln(0) on saturation
        -m * (z / m).ln()
    }

    /// Fraction of bits set (a saturation/health indicator; accuracy degrades past ~0.7).
    pub fn load_factor(&self) -> f64 {
        self.set_count as f64 / self.num_bits as f64
    }

    /// Whether nothing has been added.
    pub fn is_empty(&self) -> bool {
        self.set_count == 0
    }

    /// Merges another bitmap of the same size (bitwise OR).
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the bitmaps differ in size.
    pub fn merge(&mut self, other: &Self) -> Result<()> {
        if self.num_bits != other.num_bits {
            return Err(SketchError::IncompatibleSketches {
                reason: format!("size mismatch: {} vs {}", self.num_bits, other.num_bits),
            });
        }
        self.set_count = 0;
        for (a, b) in self.bits.iter_mut().zip(&other.bits) {
            *a |= *b;
            self.set_count += a.count_ones() as usize;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_bits() {
        assert!(LinearCounting::new(0).is_err());
        assert!(LinearCounting::new(1024).is_ok());
    }

    #[test]
    fn exact_for_tiny_sets() {
        let mut lc = LinearCounting::new(1 << 16).unwrap();
        for i in 0..10u64 {
            lc.add(&i.to_le_bytes());
        }
        // No collisions at this load => estimate rounds to 10.
        assert!(
            (lc.estimate() - 10.0).abs() < 0.5,
            "estimate {}",
            lc.estimate()
        );
    }

    #[test]
    fn accurate_at_moderate_load() {
        let mut lc = LinearCounting::new(1 << 16).unwrap();
        for i in 0..20_000u64 {
            lc.add(&i.to_le_bytes());
        }
        let est = lc.estimate();
        assert!((est - 20_000.0).abs() < 0.05 * 20_000.0, "estimate {est}");
    }

    #[test]
    fn duplicates_dont_count() {
        let mut lc = LinearCounting::new(1 << 14).unwrap();
        for _ in 0..1000 {
            lc.add(b"same");
        }
        assert!(lc.estimate() < 2.0);
    }

    #[test]
    fn merge_unions_sets() {
        let mut a = LinearCounting::new(1 << 16).unwrap();
        let mut b = LinearCounting::new(1 << 16).unwrap();
        for i in 0..5000u64 {
            a.add(&i.to_le_bytes());
        }
        for i in 2500..7500u64 {
            b.add(&i.to_le_bytes());
        }
        a.merge(&b).unwrap();
        // Union of [0,5000) and [2500,7500) = 7500 distinct.
        assert!(
            (a.estimate() - 7500.0).abs() < 0.05 * 7500.0,
            "merged {}",
            a.estimate()
        );
    }

    #[test]
    fn merge_size_mismatch_errors() {
        let mut a = LinearCounting::new(1024).unwrap();
        let b = LinearCounting::new(2048).unwrap();
        assert!(a.merge(&b).is_err());
    }
}

/// Capability-trait adoptions (see `crate::common::capabilities`).
mod capability_impls {
    use super::*;
    use crate::common::capabilities::{CardinalityEstimate, Update};

    impl Update<[u8]> for LinearCounting {
        fn update(&mut self, item: &[u8]) {
            self.add(item);
        }
    }

    impl CardinalityEstimate for LinearCounting {
        fn estimate_cardinality(&self) -> f64 {
            self.estimate()
        }
    }
}
