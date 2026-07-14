//! Odd Sketch — compact symmetric-difference and similarity estimation via parity bits.
//!
//! An Odd Sketch (Mitzenmacher, Pagh, Pham, "Efficient Estimation of the number of distinct values
//! and Jaccard similarity via Odd Sketches", WWW 2014) is a bit array in which each inserted item
//! *toggles* (XORs) one bit. Bit `i` therefore holds the **parity** of the number of items hashing
//! to `i`. Two facts make this powerful:
//!
//! - **XOR composes symmetric difference.** `odd(A) XOR odd(B) = odd(A △ B)`: an item in `A ∩ B`
//!   toggles its bit in both sketches and cancels, while an item unique to one side survives. So the
//!   combined sketch is exactly the Odd Sketch of the symmetric difference.
//! - **Set bits estimate cardinality.** With `n` distinct items toggled into `m` bits, the expected
//!   number of set bits is `m/2 · (1 − (1 − 2/m)^n) ≈ m/2 · (1 − e^{−2n/m})`. Inverting gives the
//!   estimator `n̂ = −(m/2) · ln(1 − 2b/m)` for `b` set bits.
//!
//! Combining the two, `|A △ B|` is estimated from the XOR sketch, and the Jaccard similarity follows
//! from the exact set sizes via `J = (|A| + |B| − |A △ B|) / (|A| + |B| + |A △ B|)`. The sketch is
//! accurate while the symmetric difference stays well below `m/2`; size `m` accordingly.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

/// Fixed hashing seed so independently-built sketches agree on bit positions.
const ODD_SEED: u64 = 0x0DD5_E7C0_FFEE_5EED;

/// A parity-bit Odd Sketch over `num_bits` bits.
///
/// # Example
/// ```
/// use sketch_oxide::similarity::OddSketch;
///
/// let mut a = OddSketch::new(4096).unwrap();
/// let mut b = OddSketch::new(4096).unwrap();
/// for i in 0..1000u64 { a.insert(&i.to_le_bytes()); }       // A = {0..1000}
/// for i in 300..1300u64 { b.insert(&i.to_le_bytes()); }     // B = {300..1300}
///
/// // True symmetric difference is {0..300} ∪ {1000..1300} = 600 items.
/// let sym = a.symmetric_difference_size(&b).unwrap();
/// assert!((sym - 600.0).abs() < 60.0, "sym diff {sym}");
///
/// // True Jaccard = 700 / 1300 ≈ 0.538.
/// let j = a.jaccard(&b, 1000, 1000).unwrap();
/// assert!((j - 0.538).abs() < 0.05, "jaccard {j}");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OddSketch {
    /// One bit per position, packed into `u64` words.
    words: Vec<u64>,
    num_bits: usize,
}

impl OddSketch {
    /// Creates an empty sketch with `num_bits` parity bits.
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
            words: vec![0u64; num_bits.div_ceil(64)],
            num_bits,
        })
    }

    /// Toggles the bit for `item` (XOR semantics: inserting an item twice cancels).
    pub fn insert(&mut self, item: &[u8]) {
        let pos = (xxhash(item, ODD_SEED) % self.num_bits as u64) as usize;
        self.words[pos / 64] ^= 1u64 << (pos % 64);
    }

    /// Number of set (odd-parity) bits.
    pub fn set_bits(&self) -> usize {
        self.words.iter().map(|w| w.count_ones() as usize).sum()
    }

    /// Estimated number of distinct odd-multiplicity items: `n̂ = −(m/2)·ln(1 − 2b/m)`.
    ///
    /// Returns `f64::INFINITY` if the sketch is saturated (`2b ≥ m`), which signals the symmetric
    /// difference has outgrown this sketch size.
    pub fn estimate_size(&self) -> f64 {
        let m = self.num_bits as f64;
        let b = self.set_bits() as f64;
        let frac = 1.0 - 2.0 * b / m;
        if frac <= 0.0 {
            f64::INFINITY
        } else {
            -(m / 2.0) * frac.ln()
        }
    }

    /// XOR this sketch with `other` in place; afterwards it is the Odd Sketch of the symmetric
    /// difference.
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the bit sizes differ.
    pub fn xor_with(&mut self, other: &Self) -> Result<()> {
        if self.num_bits != other.num_bits {
            return Err(SketchError::IncompatibleSketches {
                reason: format!("bit sizes differ: {} vs {}", self.num_bits, other.num_bits),
            });
        }
        for (w, o) in self.words.iter_mut().zip(&other.words) {
            *w ^= *o;
        }
        Ok(())
    }

    /// Estimated size of the symmetric difference `|A △ B|` of the two underlying sets.
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the bit sizes differ.
    pub fn symmetric_difference_size(&self, other: &Self) -> Result<f64> {
        let mut x = self.clone();
        x.xor_with(other)?;
        Ok(x.estimate_size())
    }

    /// Estimated Jaccard similarity, given the exact set sizes `size_a`, `size_b`:
    /// `J = (|A| + |B| − |A △ B|) / (|A| + |B| + |A △ B|)`.
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the bit sizes differ.
    pub fn jaccard(&self, other: &Self, size_a: u64, size_b: u64) -> Result<f64> {
        let sym = self.symmetric_difference_size(other)?;
        let total = size_a as f64 + size_b as f64;
        if total == 0.0 {
            return Ok(1.0);
        }
        // Clamp: sym cannot exceed the total, nor be negative.
        let sym = sym.clamp(0.0, total);
        Ok(((total - sym) / (total + sym)).clamp(0.0, 1.0))
    }

    /// Number of parity bits.
    #[inline]
    pub fn num_bits(&self) -> usize {
        self.num_bits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(range: std::ops::Range<u64>, bits: usize) -> OddSketch {
        let mut s = OddSketch::new(bits).unwrap();
        for i in range {
            s.insert(&i.to_le_bytes());
        }
        s
    }

    #[test]
    fn rejects_zero_bits() {
        assert!(OddSketch::new(0).is_err());
        assert!(OddSketch::new(1024).is_ok());
    }

    #[test]
    fn double_insert_cancels() {
        let mut s = OddSketch::new(1024).unwrap();
        s.insert(b"hello");
        assert_eq!(s.set_bits(), 1);
        s.insert(b"hello");
        assert_eq!(s.set_bits(), 0);
    }

    #[test]
    fn estimates_set_size() {
        let s = build(0..1000, 8192);
        let est = s.estimate_size();
        assert!((est - 1000.0).abs() < 100.0, "size estimate {est}");
    }

    #[test]
    fn estimates_symmetric_difference() {
        let a = build(0..1000, 8192);
        let b = build(300..1300, 8192);
        // True |A △ B| = 600.
        let sym = a.symmetric_difference_size(&b).unwrap();
        assert!((sym - 600.0).abs() < 60.0, "sym diff {sym}");
    }

    #[test]
    fn identical_sets_have_zero_difference() {
        let a = build(0..500, 4096);
        let b = build(0..500, 4096);
        let sym = a.symmetric_difference_size(&b).unwrap();
        assert!(sym.abs() < 1e-9, "sym diff {sym}");
        assert!((a.jaccard(&b, 500, 500).unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn estimates_jaccard() {
        let a = build(0..1000, 8192);
        let b = build(300..1300, 8192);
        // True Jaccard = |∩| / |∪| = 700 / 1300 ≈ 0.538.
        let j = a.jaccard(&b, 1000, 1000).unwrap();
        assert!((j - 0.538).abs() < 0.05, "jaccard {j}");
    }

    #[test]
    fn disjoint_sets() {
        let a = build(0..500, 8192);
        let b = build(1000..1500, 8192);
        let j = a.jaccard(&b, 500, 500).unwrap();
        assert!(j < 0.05, "jaccard {j}");
    }

    #[test]
    fn incompatible_sizes_error() {
        let a = OddSketch::new(1024).unwrap();
        let b = OddSketch::new(2048).unwrap();
        assert!(a.symmetric_difference_size(&b).is_err());
    }
}

// ---------------------------------------------------------------------------
// Capability-trait adoptions (fable5 doc 01 F3 "split the `Sketch` trait").
// OddSketch ingests raw byte items (`insert`) and estimates the number of
// distinct items of odd multiplicity (`estimate_size`), so it satisfies
// `Update<[u8]>` and `CardinalityEstimate`. It has no `Sketch` serialize and no
// quantile/point/membership semantics.
// ---------------------------------------------------------------------------
use crate::common::{CardinalityEstimate, Update};

impl Update<[u8]> for OddSketch {
    fn update(&mut self, item: &[u8]) {
        self.insert(item);
    }
}

impl CardinalityEstimate for OddSketch {
    fn estimate_cardinality(&self) -> f64 {
        self.estimate_size()
    }
}
