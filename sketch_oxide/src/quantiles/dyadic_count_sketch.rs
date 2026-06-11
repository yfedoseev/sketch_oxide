//! Dyadic Count Sketch (DCS) — approximate quantiles over **turnstile** streams (insertions *and*
//! deletions), following the dyadic-interval framework of Cormode & Muthukrishnan with the Count
//! Sketch refinement of Wang, Luo & Yi ("Quantiles over Data Streams: An Experimental Study").
//!
//! Comparison-based quantile sketches (GK, KLL, t-digest) assume an append-only stream. To support
//! **deletions** (turnstile updates, where an item's weight can go down as well as up) the universe
//! `[0, 2^L)` is decomposed into **dyadic intervals**: level `0` has `2^L` singletons, level `ℓ` has
//! `2^{L-ℓ}` intervals of width `2^ℓ`, up to the single interval at level `L`. Each level keeps a
//! frequency sketch over its interval indices.
//!
//! * **Update** `(x, Δ)`: at every level `ℓ`, add `Δ` to the interval `x >> ℓ`. `Δ` may be negative.
//! * **Rank** `(x)` (count of items `< x`): decompose `[0, x)` into at most `L` dyadic blocks (one per
//!   set bit of `x`) and sum their estimated counts.
//! * **Quantile** `(φ)`: binary-search the universe for the largest `x` whose rank is `≤ φ·n`.
//!
//! DCS uses a **Count Sketch** (signed counters, estimate = median over rows) per level, giving
//! *unbiased* interval-count estimates that tolerate the cancellation from deletions — unlike the
//! one-sided Count-Min of the original "Dyadic Count-Min". Coarse levels with no more intervals than
//! the sketch width are stored **exactly** (cheap and error-free).

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

/// A signed Count Sketch over `u64` keys: `depth` rows of `width` counters, estimate = median.
#[derive(Debug, Clone)]
struct CountSketch {
    width: usize,
    depth: usize,
    seed: u64,
    counts: Vec<i64>, // depth × width
}

impl CountSketch {
    fn new(width: usize, depth: usize, seed: u64) -> Self {
        Self {
            width,
            depth,
            seed,
            counts: vec![0; width * depth],
        }
    }

    /// `(bucket, sign)` for `key` in row `r`.
    fn locate(&self, key: u64, r: usize) -> (usize, i64) {
        let h = xxhash(
            &key.to_le_bytes(),
            self.seed ^ (r as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15),
        );
        let bucket = (h % self.width as u64) as usize;
        let sign = if (h >> 63) & 1 == 0 { 1 } else { -1 };
        (bucket, sign)
    }

    fn update(&mut self, key: u64, delta: i64) {
        for r in 0..self.depth {
            let (b, s) = self.locate(key, r);
            self.counts[r * self.width + b] += delta * s;
        }
    }

    /// Median of the per-row signed estimates (unbiased).
    fn estimate(&self, key: u64) -> i64 {
        let mut vals: Vec<i64> = (0..self.depth)
            .map(|r| {
                let (b, s) = self.locate(key, r);
                self.counts[r * self.width + b] * s
            })
            .collect();
        vals.sort_unstable();
        let m = vals.len() / 2;
        if vals.len() % 2 == 1 {
            vals[m]
        } else {
            (vals[m - 1] + vals[m]) / 2
        }
    }
}

/// One dyadic level: stored exactly when small, otherwise as a Count Sketch.
#[derive(Debug, Clone)]
enum Level {
    Exact(Vec<i64>),
    Sketch(CountSketch),
}

impl Level {
    fn update(&mut self, idx: u64, delta: i64) {
        match self {
            Level::Exact(v) => v[idx as usize] += delta,
            Level::Sketch(s) => s.update(idx, delta),
        }
    }

    fn estimate(&self, idx: u64) -> i64 {
        match self {
            Level::Exact(v) => v[idx as usize],
            Level::Sketch(s) => s.estimate(idx),
        }
    }
}

/// A Dyadic Count Sketch for turnstile quantiles over the universe `[0, 2^universe_bits)`.
///
/// # Example
/// ```
/// use sketch_oxide::quantiles::DyadicCountSketch;
///
/// // Universe [0, 1024); Count Sketches 512 wide, depth 5.
/// let mut d = DyadicCountSketch::new(10, 512, 5, 7).unwrap();
/// for x in 0..1000u64 { d.add(x); }   // values 0..1000
/// d.remove(500);                       // turnstile: delete one occurrence of 500
///
/// // ~500 values are below 500 (minus the one we removed at exactly 500, which doesn't count as < 500).
/// let r = d.rank(500);
/// assert!((r - 500.0).abs() < 25.0, "rank {r}");
/// // The median is near 500.
/// let q = d.quantile(0.5);
/// assert!(q.abs_diff(500) < 30, "median {q}");
/// ```
#[derive(Debug, Clone)]
pub struct DyadicCountSketch {
    bits: u32,
    levels: Vec<Level>, // index ℓ covers intervals of width 2^ℓ
}

impl DyadicCountSketch {
    /// Creates a DCS over universe `[0, 2^universe_bits)` with `width`-wide, `depth`-deep Count
    /// Sketches at the levels too large to store exactly.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `universe_bits` is not in `1..=48`, `width == 0`, or
    /// `depth` is not in `1..=16`.
    pub fn new(universe_bits: u32, width: usize, depth: usize, seed: u64) -> Result<Self> {
        if !(1..=48).contains(&universe_bits) {
            return Err(SketchError::InvalidParameter {
                param: "universe_bits".to_string(),
                value: universe_bits.to_string(),
                constraint: "must be in 1..=48".to_string(),
            });
        }
        if width == 0 {
            return Err(SketchError::InvalidParameter {
                param: "width".to_string(),
                value: "0".to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        if !(1..=16).contains(&depth) {
            return Err(SketchError::InvalidParameter {
                param: "depth".to_string(),
                value: depth.to_string(),
                constraint: "must be in 1..=16".to_string(),
            });
        }
        let mut levels = Vec::with_capacity(universe_bits as usize + 1);
        for l in 0..=universe_bits {
            let n_intervals = 1u64 << (universe_bits - l); // intervals at this level
            if n_intervals <= width as u64 {
                levels.push(Level::Exact(vec![0; n_intervals as usize]));
            } else {
                levels.push(Level::Sketch(CountSketch::new(
                    width,
                    depth,
                    seed ^ (l as u64).wrapping_mul(0xD1B5_4A32_D192_ED03),
                )));
            }
        }
        Ok(Self {
            bits: universe_bits,
            levels,
        })
    }

    #[inline]
    fn check(&self, x: u64) -> bool {
        x < (1u64 << self.bits)
    }

    /// Applies a signed weight change `delta` to value `x` (turnstile update).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `x >= 2^universe_bits`.
    pub fn update(&mut self, x: u64, delta: i64) -> Result<()> {
        if !self.check(x) {
            return Err(SketchError::InvalidParameter {
                param: "x".to_string(),
                value: x.to_string(),
                constraint: format!("must be < 2^{}", self.bits),
            });
        }
        for (l, level) in self.levels.iter_mut().enumerate() {
            level.update(x >> l, delta);
        }
        Ok(())
    }

    /// Inserts one occurrence of `x` (`update(x, 1)`). Panics if `x` is out of range.
    pub fn add(&mut self, x: u64) {
        self.update(x, 1).expect("value out of range");
    }

    /// Removes one occurrence of `x` (`update(x, -1)`). Panics if `x` is out of range.
    pub fn remove(&mut self, x: u64) {
        self.update(x, -1).expect("value out of range");
    }

    /// Estimated number of items with value strictly less than `x` (the rank of `x`). `x` is clamped
    /// to `[0, 2^universe_bits]`.
    pub fn rank(&self, x: u64) -> f64 {
        let cap = 1u64 << self.bits;
        let x = x.min(cap);
        let mut r: i64 = 0;
        let mut pos = 0u64;
        // Greedy high-to-low dyadic cover of [0, x): one block per set bit of x.
        for l in (0..=self.bits as usize).rev() {
            let size = 1u64 << l;
            if pos + size <= x {
                r += self.levels[l].estimate(pos >> l);
                pos += size;
            }
        }
        r as f64
    }

    /// Estimated total weight in the sketch (`rank` of the universe top).
    pub fn total(&self) -> f64 {
        self.levels[self.bits as usize].estimate(0) as f64
    }

    /// Estimated point frequency of value `x` (level-0 interval). `x` out of range returns 0.
    pub fn count(&self, x: u64) -> f64 {
        if !self.check(x) {
            return 0.0;
        }
        self.levels[0].estimate(x) as f64
    }

    /// Approximate `φ`-quantile: the smallest value whose rank reaches `φ·n` (`φ` in `[0, 1]`).
    /// Returns a value in `[0, 2^universe_bits)`.
    pub fn quantile(&self, phi: f64) -> u64 {
        let n = self.total();
        if n <= 0.0 {
            return 0;
        }
        let target = phi.clamp(0.0, 1.0) * n;
        // Binary-search the largest x with rank(x) <= target; that x is the φ-quantile value.
        let cap = 1u64 << self.bits;
        let (mut lo, mut hi) = (0u64, cap); // search rank boundary in [0, cap]
        while lo < hi {
            let mid = lo + (hi - lo).div_ceil(2);
            if self.rank(mid) <= target {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        lo.min(cap - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(DyadicCountSketch::new(0, 256, 5, 1).is_err());
        assert!(DyadicCountSketch::new(49, 256, 5, 1).is_err());
        assert!(DyadicCountSketch::new(10, 0, 5, 1).is_err());
        assert!(DyadicCountSketch::new(10, 256, 0, 1).is_err());
        assert!(DyadicCountSketch::new(10, 256, 17, 1).is_err());
        assert!(DyadicCountSketch::new(10, 256, 5, 1).is_ok());
    }

    #[test]
    fn update_validates_range() {
        let mut d = DyadicCountSketch::new(8, 256, 5, 1).unwrap();
        assert!(d.update(255, 1).is_ok());
        assert!(d.update(256, 1).is_err());
    }

    #[test]
    fn rank_and_quantiles_uniform() {
        // Uniform 0..1000 in [0,1024): rank(x) ≈ x, quantiles linear.
        let mut d = DyadicCountSketch::new(10, 1024, 5, 42).unwrap();
        for x in 0..1000u64 {
            d.add(x);
        }
        assert!((d.total() - 1000.0).abs() < 1.0);
        for &x in &[100u64, 250, 500, 750, 900] {
            let r = d.rank(x);
            assert!((r - x as f64).abs() < 20.0, "rank({x}) = {r}");
        }
        let med = d.quantile(0.5);
        assert!(med.abs_diff(500) < 30, "median {med}");
        let p90 = d.quantile(0.9);
        assert!(p90.abs_diff(900) < 30, "p90 {p90}");
    }

    #[test]
    fn turnstile_deletions_reduce_rank() {
        let mut d = DyadicCountSketch::new(10, 1024, 5, 7).unwrap();
        for x in 0..1000u64 {
            d.add(x);
        }
        // Delete everything below 300.
        for x in 0..300u64 {
            d.remove(x);
        }
        assert!((d.total() - 700.0).abs() < 2.0, "total {}", d.total());
        // Few items remain below 300.
        assert!(d.rank(300) < 20.0, "rank(300) = {}", d.rank(300));
        // ~half of the remaining 700 are below 650.
        let r = d.rank(650);
        assert!((r - 350.0).abs() < 30.0, "rank(650) = {r}");
    }

    #[test]
    fn unbiased_estimate_survives_cancellation() {
        // Add then remove a large noisy mass that net-cancels; the median estimator stays unbiased
        // where Count-Min would over-count. Net result: only values 0..100 remain (weight 1 each).
        let mut d = DyadicCountSketch::new(12, 256, 7, 99).unwrap();
        for x in 0..100u64 {
            d.add(x);
        }
        for x in 1000..3000u64 {
            d.update(x, 5).unwrap();
            d.update(x, -5).unwrap();
        }
        assert!((d.total() - 100.0).abs() < 5.0, "total {}", d.total());
        assert!(
            d.rank(100) >= 95.0 && d.rank(100) <= 105.0,
            "rank(100) {}",
            d.rank(100)
        );
    }
}
