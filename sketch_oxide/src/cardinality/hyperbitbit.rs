//! HyperBitBit — an ultra-compact cardinality estimator (Sedgewick, 2016).
//!
//! HyperBitBit estimates the number of distinct items in a stream using just **128 + 6 bits** of
//! state — two 64-bit words and a small exponent — yet stays within ~10% on practical data for
//! cardinalities up to `2^64`. It is a playful minimisation of HyperLogLog: instead of `m` rank
//! registers it keeps a single exponent `lgN` (a running estimate of `log₂` of the cardinality) and a
//! 64-bit `sketch` whose bit `k` is set when an item hashes to bucket `k` *and* its leading-zero rank
//! `r` exceeds `lgN`. A second word `sketch2` tracks the `r > lgN+1` level; once `sketch` is more than
//! half full (popcount > 31), `lgN` is bumped and `sketch2` is promoted to `sketch` — a self-clocking
//! "doubling" that tracks the growing cardinality.
//!
//! The estimate is `2^(lgN + 5.4 + popcount(sketch)/32)`, where `5.4` is Sedgewick's empirically tuned
//! constant. Transcribed from the canonical reference (Sedgewick's algorithm and the stream-lib
//! implementation).
//!
//! # Caveats
//!
//! HyperBitBit is approximate and **does not work well for small cardinalities**; it is also not
//! idempotent (re-inserting an item can change the state, unlike HyperLogLog). Use a precise small-set
//! method below a few thousand, and [`HyperLogLog`](crate::cardinality::HyperLogLog) /
//! [`UltraLogLog`](crate::cardinality::UltraLogLog) when tighter, mergeable estimates are needed.

use std::hash::{Hash, Hasher};
use twox_hash::XxHash64;

/// An ultra-compact HyperBitBit cardinality estimator.
///
/// # Example
/// ```
/// use sketch_oxide::cardinality::HyperBitBit;
///
/// let mut hbb = HyperBitBit::new();
/// for i in 0..1_000_000u64 {
///     hbb.add(&i);
/// }
/// let est = hbb.estimate();
/// assert!((est - 1_000_000.0).abs() < 0.4 * 1_000_000.0, "estimate {est}");
/// ```
#[derive(Debug, Clone)]
pub struct HyperBitBit {
    lg_n: u32,
    sketch: u64,
    sketch2: u64,
}

impl Default for HyperBitBit {
    fn default() -> Self {
        Self::new()
    }
}

impl HyperBitBit {
    /// Creates an empty estimator.
    pub fn new() -> Self {
        Self {
            lg_n: 5,
            sketch: 0,
            sketch2: 0,
        }
    }

    /// Adds one item.
    pub fn add<T: Hash>(&mut self, item: &T) {
        let mut hasher = XxHash64::with_seed(0);
        item.hash(&mut hasher);
        self.add_hash(hasher.finish());
    }

    /// Adds a pre-computed 64-bit hash.
    pub fn add_hash(&mut self, x: u64) {
        // Low 6 bits select the bucket; the leading-zero rank of the remaining bits is the "level".
        let k = (x & 0x3F) as u32;
        let r = (x >> 6).leading_zeros() as i32 - 6;

        if r > self.lg_n as i32 {
            self.sketch |= 1u64 << k;
        }
        if r > self.lg_n as i32 + 1 {
            self.sketch2 |= 1u64 << k;
        }
        if self.sketch.count_ones() > 31 {
            self.sketch = self.sketch2;
            self.sketch2 = 0;
            self.lg_n += 1;
        }
    }

    /// Estimated number of distinct items: `2^(lgN + 5.4 + popcount(sketch)/32)`.
    pub fn estimate(&self) -> f64 {
        let exponent = self.lg_n as f64 + 5.4 + self.sketch.count_ones() as f64 / 32.0;
        2f64.powf(exponent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_starts_small() {
        let hbb = HyperBitBit::new();
        // 2^(5 + 5.4 + 0) ≈ 1351; HyperBitBit is not meaningful at zero/small cardinalities.
        assert!(hbb.estimate() > 0.0);
    }

    fn rel_err(precision_n: u64) -> f64 {
        let mut hbb = HyperBitBit::new();
        for i in 0..precision_n {
            hbb.add(&i);
        }
        let est = hbb.estimate();
        (est - precision_n as f64).abs() / precision_n as f64
    }

    #[test]
    fn accurate_for_large_cardinalities() {
        // HyperBitBit targets large N; allow generous slack for its compact, approximate nature.
        assert!(rel_err(100_000) < 0.4, "100k rel err");
        assert!(rel_err(1_000_000) < 0.4, "1M rel err");
        assert!(rel_err(5_000_000) < 0.4, "5M rel err");
    }

    #[test]
    fn estimate_increases_with_cardinality() {
        let mut hbb = HyperBitBit::new();
        for i in 0..50_000u64 {
            hbb.add(&i);
        }
        let small = hbb.estimate();
        for i in 50_000..2_000_000u64 {
            hbb.add(&i);
        }
        let large = hbb.estimate();
        assert!(
            large > small * 5.0,
            "estimate should grow: {small} -> {large}"
        );
    }
}

/// Capability-trait adoptions (see `crate::common::capabilities`).
mod capability_impls {
    use super::*;
    use crate::common::capabilities::{CardinalityEstimate, Update};
    use std::hash::Hash;

    impl<T: Hash> Update<T> for HyperBitBit {
        fn update(&mut self, item: &T) {
            self.add(item);
        }
    }

    impl CardinalityEstimate for HyperBitBit {
        fn estimate_cardinality(&self) -> f64 {
            self.estimate()
        }
    }
}
