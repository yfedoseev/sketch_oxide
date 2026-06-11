//! RadixSpline — a single-pass learned index over sorted keys.
//!
//! RadixSpline (Kipf, Marcus, van Renen, Stoian, Kemper, Kraska & Neumann, "RadixSpline: A
//! Single-Pass Learned Index", aiDM @ SIGMOD 2020) approximates the *CDF* of a sorted key array —
//! the mapping `key → position` — so a lookup can predict a key's position within a guaranteed
//! error bound and then finish with a tiny local search. It has two parts, both built in one pass:
//!
//! - a **linear spline** that traces the CDF with as few points as possible while keeping every
//!   key's interpolated position within `max_error` of its true index (the greedy-spline-corridor
//!   construction); and
//! - a **radix table** mapping each key's high `radix_bits` to the spline segment that contains it,
//!   so the right segment is found in O(1) instead of by binary search.
//!
//! Unlike a B-tree it stores only the spline points and a flat radix table, and unlike a pure model
//! it gives a hard error bound, making the follow-up search a bounded scan.

use crate::common::{Result, SketchError};

/// A learned index over a sorted `u64` key array.
///
/// # Example
/// ```
/// use sketch_oxide::range_filters::RadixSpline;
///
/// let keys: Vec<u64> = (0..10_000u64).map(|i| i * 7 + 3).collect(); // sorted, gappy
/// let rs = RadixSpline::build(&keys, 32, 18).unwrap();
///
/// // Predicted position is within max_error of the true index for every key.
/// let pred = rs.estimate_position(keys[5000]);
/// assert!((pred - 5000.0).abs() <= 32.0);
/// ```
#[derive(Debug, Clone)]
pub struct RadixSpline {
    /// Spline points `(key, position)`, ascending by key.
    spline: Vec<(u64, usize)>,
    /// Radix table: high-bits prefix → first spline index at or after that prefix.
    radix: Vec<u32>,
    radix_shift: u32,
    max_error: usize,
    n: usize,
}

impl RadixSpline {
    /// Builds a RadixSpline over `keys` (which must be sorted ascending) with a position error bound
    /// of `max_error` and a `radix_bits`-bit radix table.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `keys` is empty, not sorted, `max_error` is 0, or
    /// `radix_bits` is outside `1..=32`.
    pub fn build(keys: &[u64], max_error: usize, radix_bits: u32) -> Result<Self> {
        if keys.is_empty() {
            return Err(SketchError::InvalidParameter {
                param: "keys".to_string(),
                value: "empty".to_string(),
                constraint: "must be non-empty".to_string(),
            });
        }
        if max_error == 0 {
            return Err(SketchError::InvalidParameter {
                param: "max_error".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        if !(1..=32).contains(&radix_bits) {
            return Err(SketchError::InvalidParameter {
                param: "radix_bits".to_string(),
                value: radix_bits.to_string(),
                constraint: "must be in 1..=32".to_string(),
            });
        }
        if keys.windows(2).any(|w| w[0] > w[1]) {
            return Err(SketchError::InvalidParameter {
                param: "keys".to_string(),
                value: "unsorted".to_string(),
                constraint: "must be sorted ascending".to_string(),
            });
        }

        let spline = Self::fit_spline(keys, max_error);
        let radix_shift = 64 - radix_bits;
        let radix = Self::build_radix(&spline, radix_shift, radix_bits);

        Ok(Self {
            spline,
            radix,
            radix_shift,
            max_error,
            n: keys.len(),
        })
    }

    /// Greedy-spline-corridor fit: the fewest points whose linear interpolation keeps every key's
    /// position within `max_error`.
    fn fit_spline(keys: &[u64], max_error: usize) -> Vec<(u64, usize)> {
        let err = max_error as f64;
        let mut spline = vec![(keys[0], 0usize)];
        if keys.len() == 1 {
            return spline;
        }
        let mut base = (keys[0], 0usize);
        let mut prev = (keys[0], 0usize);
        let mut slope_lo = f64::NEG_INFINITY;
        let mut slope_hi = f64::INFINITY;

        for (idx, &k) in keys.iter().enumerate().skip(1) {
            let dk = (k - base.0) as f64;
            if dk == 0.0 {
                // Duplicate key region: just remember the latest position.
                prev = (k, idx);
                continue;
            }
            let want = (idx as f64 - base.1 as f64) / dk; // slope to hit idx exactly
            if want < slope_lo || want > slope_hi {
                // The corridor closed: pin the previous point as a spline knot and restart.
                spline.push(prev);
                base = prev;
                let dk2 = (k - base.0) as f64;
                slope_lo = f64::NEG_INFINITY;
                slope_hi = f64::INFINITY;
                if dk2 > 0.0 {
                    slope_hi = (idx as f64 + err - base.1 as f64) / dk2;
                    slope_lo = (idx as f64 - err - base.1 as f64) / dk2;
                }
            } else {
                let hi = (idx as f64 + err - base.1 as f64) / dk;
                let lo = (idx as f64 - err - base.1 as f64) / dk;
                slope_hi = slope_hi.min(hi);
                slope_lo = slope_lo.max(lo);
            }
            prev = (k, idx);
        }
        // Always anchor the last point.
        let last = (keys[keys.len() - 1], keys.len() - 1);
        if *spline.last().unwrap() != last {
            spline.push(last);
        }
        spline
    }

    /// Radix table: for each high-bits prefix, the index of the first spline point at or after it.
    fn build_radix(spline: &[(u64, usize)], shift: u32, radix_bits: u32) -> Vec<u32> {
        let size = 1usize << radix_bits;
        let mut radix = vec![0u32; size + 1];
        let mut s = 0usize;
        for (prefix, slot) in radix.iter_mut().enumerate() {
            while s < spline.len() && (spline[s].0 >> shift) < prefix as u64 {
                s += 1;
            }
            *slot = s as u32;
        }
        radix
    }

    /// Predicts the (fractional) position of `key`; within `max_error` of the true index for any
    /// key that is present, and a sensible interpolation otherwise.
    pub fn estimate_position(&self, key: u64) -> f64 {
        if key <= self.spline[0].0 {
            return 0.0;
        }
        if key >= self.spline[self.spline.len() - 1].0 {
            return (self.n - 1) as f64;
        }
        // Narrow to a spline segment using the radix table, then binary-search within it.
        let prefix = (key >> self.radix_shift) as usize;
        let lo = self.radix[prefix] as usize;
        let hi = (self.radix[prefix + 1] as usize + 1).min(self.spline.len());
        // Find the segment [seg, seg+1] containing key.
        let seg = match self.spline[lo..hi].binary_search_by(|&(k, _)| k.cmp(&key)) {
            Ok(i) => lo + i,
            Err(i) => (lo + i).saturating_sub(1),
        };
        let seg = seg.min(self.spline.len() - 2);
        let (ka, pa) = self.spline[seg];
        let (kb, pb) = self.spline[seg + 1];
        if kb == ka {
            return pa as f64;
        }
        let t = (key - ka) as f64 / (kb - ka) as f64;
        (pa as f64 + t * (pb as f64 - pa as f64)).clamp(0.0, (self.n - 1) as f64)
    }

    /// The inclusive index window `[lo, hi]` guaranteed to contain `key`'s true position (the
    /// prediction ± `max_error`), ready for a bounded local search.
    pub fn search_bound(&self, key: u64) -> (usize, usize) {
        let pred = self.estimate_position(key);
        let lo = (pred as isize - self.max_error as isize).max(0) as usize;
        let hi = ((pred as usize) + self.max_error).min(self.n - 1);
        (lo, hi)
    }

    /// Number of spline points (the index's size).
    #[inline]
    pub fn num_spline_points(&self) -> usize {
        self.spline.len()
    }

    /// The configured error bound.
    #[inline]
    pub fn max_error(&self) -> usize {
        self.max_error
    }

    /// Number of indexed keys.
    #[inline]
    pub fn len(&self) -> usize {
        self.n
    }

    /// Whether the index is empty (always false — `build` rejects empty input).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_input() {
        assert!(RadixSpline::build(&[], 32, 18).is_err());
        assert!(RadixSpline::build(&[1, 2, 3], 0, 18).is_err());
        assert!(RadixSpline::build(&[1, 2, 3], 32, 0).is_err());
        assert!(RadixSpline::build(&[3, 2, 1], 32, 18).is_err()); // unsorted
        assert!(RadixSpline::build(&[1, 2, 3], 32, 18).is_ok());
    }

    #[test]
    fn error_bound_holds_for_linear_keys() {
        let keys: Vec<u64> = (0..20_000u64).collect();
        let rs = RadixSpline::build(&keys, 16, 18).unwrap();
        for (i, &k) in keys.iter().enumerate() {
            let pred = rs.estimate_position(k);
            assert!(
                (pred - i as f64).abs() <= 16.0,
                "key {k} idx {i}: pred {pred} out of error bound"
            );
        }
        // A linear CDF should compress to very few spline points.
        assert!(
            rs.num_spline_points() < 10,
            "spline points {}",
            rs.num_spline_points()
        );
    }

    #[test]
    fn error_bound_holds_for_gappy_keys() {
        let keys: Vec<u64> = (0..15_000u64).map(|i| i * 7 + 3).collect();
        let max_err = 32;
        let rs = RadixSpline::build(&keys, max_err, 16).unwrap();
        for (i, &k) in keys.iter().enumerate() {
            let pred = rs.estimate_position(k);
            assert!(
                (pred - i as f64).abs() <= max_err as f64,
                "idx {i}: pred {pred}"
            );
        }
    }

    #[test]
    fn error_bound_holds_for_clustered_keys() {
        // Non-uniform: dense low region, sparse high region — stresses the spline corridor.
        let mut keys: Vec<u64> = (0..5000u64).collect();
        keys.extend((0..5000u64).map(|i| 1_000_000 + i * 1000));
        keys.sort_unstable();
        let max_err = 24;
        let rs = RadixSpline::build(&keys, max_err, 16).unwrap();
        for (i, &k) in keys.iter().enumerate() {
            let pred = rs.estimate_position(k);
            assert!(
                (pred - i as f64).abs() <= max_err as f64,
                "idx {i}: pred {pred}"
            );
        }
    }

    #[test]
    fn search_bound_contains_true_position() {
        let keys: Vec<u64> = (0..10_000u64).map(|i| i * 13).collect();
        let rs = RadixSpline::build(&keys, 20, 16).unwrap();
        for (i, &k) in keys.iter().enumerate().step_by(97) {
            let (lo, hi) = rs.search_bound(k);
            assert!(lo <= i && i <= hi, "true {i} not in [{lo}, {hi}]");
        }
    }

    #[test]
    fn endpoints_clamp() {
        let keys: Vec<u64> = (10..100u64).collect();
        let rs = RadixSpline::build(&keys, 4, 8).unwrap();
        assert_eq!(rs.estimate_position(0), 0.0); // below the smallest key
        assert_eq!(rs.estimate_position(1000), (keys.len() - 1) as f64); // above the largest
    }
}
