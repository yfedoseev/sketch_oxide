//! AMS / Fast-AGMS sketch — second frequency moment (F2), inner products, join sizes.
//!
//! The AMS sketch (Alon, Matias & Szegedy, STOC 1996; "Fast-AGMS" variant) estimates
//! quantities that depend on the squared frequency vector of a stream:
//!
//! - **F2** — `Σ_i f_i²`, the second frequency moment (a.k.a. the self-join size / repeat
//!   rate / Gini-style concentration).
//! - **Inner product** — `Σ_i a_i · b_i` between two streams, i.e. the **join size** of two
//!   relations on a key, estimated from compact sketches without the raw data.
//!
//! # How it works
//!
//! `depth` independent estimators each hold `width` counters. Each item is mapped, per row,
//! to a bucket and a `±1` sign; updates add `sign · count` to the bucket. The per-row
//! estimate of F2 is `Σ_j counter[j]²`; of an inner product, `Σ_j a[j] · b[j]`. Taking the
//! **median** across rows gives the classic `(ε, δ)` guarantee — error `≤ ε·F2` with
//! `width = O(1/ε²)` and failure probability `≤ δ` with `depth = O(log 1/δ)`.
//!
//! The hash seeds are derived deterministically from the row index, so any two
//! `AmsSketch`es with the same `(depth, width)` are inner-product compatible — no shared
//! random state needs to be transmitted.

use crate::common::hash::xxhash;
use crate::common::{Mergeable, Result, Sketch, SketchError};

/// AMS / Fast-AGMS sketch over `depth × width` signed counters.
///
/// # Example
/// ```
/// use sketch_oxide::statistics::AmsSketch;
///
/// let mut a = AmsSketch::new(7, 2048).unwrap();
/// for i in 0..1000u64 {
///     a.update(&i.to_le_bytes(), 1.0); // 1000 distinct keys, each once
/// }
/// // F2 = sum of squared frequencies = 1000 for distinct keys.
/// let f2 = a.f2();
/// assert!((f2 - 1000.0).abs() < 150.0, "F2 estimate {f2}");
/// ```
#[derive(Debug, Clone)]
pub struct AmsSketch {
    depth: usize,
    width: usize,
    counters: Vec<f64>, // depth * width, row-major
}

impl AmsSketch {
    /// Creates a sketch with `depth` rows and `width` counters per row.
    ///
    /// `width ≈ 4/ε²` controls accuracy; `depth ≈ ln(1/δ)` controls confidence (use an odd
    /// depth so the median is well-defined).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `depth` or `width` is 0.
    pub fn new(depth: usize, width: usize) -> Result<Self> {
        if depth == 0 || width == 0 {
            return Err(SketchError::InvalidParameter {
                param: if depth == 0 { "depth" } else { "width" }.to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            depth,
            width,
            counters: vec![0.0; depth * width],
        })
    }

    /// Number of estimator rows.
    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Counters per row.
    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }

    /// Bucket and sign for `item` in row `r`. Seeds are derived from `r`, so they are
    /// identical across sketches of the same shape.
    #[inline]
    fn bucket_and_sign(&self, item: &[u8], r: usize) -> (usize, f64) {
        let hb = xxhash(item, (r as u64) << 1);
        let hs = xxhash(item, ((r as u64) << 1) | 1);
        let bucket = (hb % self.width as u64) as usize;
        let sign = if hs & 1 == 0 { 1.0 } else { -1.0 };
        (bucket, sign)
    }

    /// Adds `count` occurrences of `item` (use a negative `count` for deletions / turnstile
    /// updates).
    pub fn update(&mut self, item: &[u8], count: f64) {
        for r in 0..self.depth {
            let (bucket, sign) = self.bucket_and_sign(item, r);
            self.counters[r * self.width + bucket] += sign * count;
        }
    }

    /// Median over rows; `values` is consumed.
    fn median(mut values: Vec<f64>) -> f64 {
        values.sort_by(f64::total_cmp);
        let n = values.len();
        if n % 2 == 1 {
            values[n / 2]
        } else {
            0.5 * (values[n / 2 - 1] + values[n / 2])
        }
    }

    /// Estimates the second frequency moment `F2 = Σ_i f_i²`.
    pub fn f2(&self) -> f64 {
        let row_estimates: Vec<f64> = (0..self.depth)
            .map(|r| {
                let row = &self.counters[r * self.width..(r + 1) * self.width];
                row.iter().map(|&c| c * c).sum()
            })
            .collect();
        Self::median(row_estimates)
    }

    /// Estimates the inner product `Σ_i a_i · b_i` with another sketch (the **join size** of
    /// the two streams). Both sketches must have the same shape.
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the shapes differ.
    pub fn inner_product(&self, other: &Self) -> Result<f64> {
        self.check_compatible(other)?;
        let row_estimates: Vec<f64> = (0..self.depth)
            .map(|r| {
                let a = &self.counters[r * self.width..(r + 1) * self.width];
                let b = &other.counters[r * self.width..(r + 1) * self.width];
                a.iter().zip(b).map(|(&x, &y)| x * y).sum()
            })
            .collect();
        Ok(Self::median(row_estimates))
    }

    fn check_compatible(&self, other: &Self) -> Result<()> {
        if self.depth != other.depth || self.width != other.width {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "shape mismatch: {}x{} vs {}x{}",
                    self.depth, self.width, other.depth, other.width
                ),
            });
        }
        Ok(())
    }
}

impl Mergeable for AmsSketch {
    /// Merges another sketch (linear: counters add). Both must share the shape and the
    /// derived seeds, which they do for equal `(depth, width)`.
    fn merge(&mut self, other: &Self) -> Result<()> {
        self.check_compatible(other)?;
        for (a, b) in self.counters.iter_mut().zip(&other.counters) {
            *a += *b;
        }
        Ok(())
    }
}

impl Sketch for AmsSketch {
    type Item = Vec<u8>;

    fn update(&mut self, item: &Self::Item) {
        self.update(item, 1.0);
    }

    /// Returns the estimated F2.
    fn estimate(&self) -> f64 {
        self.f2()
    }

    fn is_empty(&self) -> bool {
        self.counters.iter().all(|&c| c == 0.0)
    }

    fn serialize(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(16 + self.counters.len() * 8);
        b.extend_from_slice(&(self.depth as u64).to_le_bytes());
        b.extend_from_slice(&(self.width as u64).to_le_bytes());
        for &c in &self.counters {
            b.extend_from_slice(&c.to_le_bytes());
        }
        b
    }

    fn deserialize(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 16 {
            return Err(SketchError::DeserializationError("AmsSketch header".into()));
        }
        let depth = u64::from_le_bytes(bytes[0..8].try_into().unwrap()) as usize;
        let width = u64::from_le_bytes(bytes[8..16].try_into().unwrap()) as usize;
        let mut sketch = Self::new(depth, width)?;
        let expected = 16 + depth * width * 8;
        if bytes.len() != expected {
            return Err(SketchError::DeserializationError(format!(
                "expected {expected} bytes, got {}",
                bytes.len()
            )));
        }
        for (i, c) in sketch.counters.iter_mut().enumerate() {
            let off = 16 + i * 8;
            *c = f64::from_le_bytes(bytes[off..off + 8].try_into().unwrap());
        }
        Ok(sketch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_dims() {
        assert!(AmsSketch::new(0, 10).is_err());
        assert!(AmsSketch::new(10, 0).is_err());
        assert!(AmsSketch::new(5, 64).is_ok());
    }

    #[test]
    fn f2_of_distinct_keys() {
        let mut a = AmsSketch::new(9, 4096).unwrap();
        for i in 0..2000u64 {
            a.update(&i.to_le_bytes(), 1.0);
        }
        // distinct keys => F2 = n
        let f2 = a.f2();
        assert!((f2 - 2000.0).abs() < 0.15 * 2000.0, "F2 {f2}");
    }

    #[test]
    fn f2_with_skew() {
        // One hot key with frequency 100, plus 100 singletons: F2 = 100^2 + 100 = 10100.
        let mut a = AmsSketch::new(9, 4096).unwrap();
        a.update(b"hot", 100.0);
        for i in 0..100u64 {
            a.update(&i.to_le_bytes(), 1.0);
        }
        let f2 = a.f2();
        assert!((f2 - 10_100.0).abs() < 0.15 * 10_100.0, "F2 {f2}");
    }

    #[test]
    fn self_inner_product_equals_f2() {
        let mut a = AmsSketch::new(9, 4096).unwrap();
        for i in 0..500u64 {
            a.update(&i.to_le_bytes(), 1.0);
        }
        let ip = a.inner_product(&a).unwrap();
        assert!(
            (ip - a.f2()).abs() < 1e-6,
            "self inner product should equal F2"
        );
    }

    #[test]
    fn inner_product_estimates_join_size() {
        // a has keys 0..1000, b has keys 500..1500; overlap 500..1000 (500 keys),
        // each once on both sides => join size = 500.
        let mut a = AmsSketch::new(11, 8192).unwrap();
        let mut b = AmsSketch::new(11, 8192).unwrap();
        for i in 0..1000u64 {
            a.update(&i.to_le_bytes(), 1.0);
        }
        for i in 500..1500u64 {
            b.update(&i.to_le_bytes(), 1.0);
        }
        let ip = a.inner_product(&b).unwrap();
        assert!((ip - 500.0).abs() < 0.25 * 500.0, "join size estimate {ip}");
    }

    #[test]
    fn merge_adds_streams() {
        let mut a = AmsSketch::new(7, 2048).unwrap();
        let mut b = AmsSketch::new(7, 2048).unwrap();
        for i in 0..1000u64 {
            a.update(&i.to_le_bytes(), 1.0);
        }
        for i in 1000..2000u64 {
            b.update(&i.to_le_bytes(), 1.0);
        }
        a.merge(&b).unwrap();
        let f2 = a.f2();
        assert!((f2 - 2000.0).abs() < 0.2 * 2000.0, "merged F2 {f2}");
    }

    #[test]
    fn incompatible_shapes_error() {
        let a = AmsSketch::new(5, 64).unwrap();
        let b = AmsSketch::new(5, 128).unwrap();
        assert!(a.inner_product(&b).is_err());
    }

    #[test]
    fn serde_round_trip() {
        let mut a = AmsSketch::new(7, 256).unwrap();
        for i in 0..500u64 {
            a.update(&i.to_le_bytes(), 1.0);
        }
        let restored = AmsSketch::deserialize(&a.serialize()).unwrap();
        assert_eq!(restored.f2(), a.f2());
    }
}
