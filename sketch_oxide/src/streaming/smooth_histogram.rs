//! Smooth Histogram — (1±ε) sliding-window aggregates for smooth functions.
//!
//! The Smooth Histogram (Braverman & Ostrovsky, "Smooth Histograms for Sliding Windows", FOCS
//! 2007) is a general framework for approximating a function `f` over the last `W` elements of a
//! stream when `f` is *smooth* — informally, when shrinking the window from the old end never
//! increases `f`, and once two suffixes are close they stay close. It keeps a sparse set of
//! **checkpoints**: each checkpoint stores `f` evaluated from its start time to *now*, and the set
//! is pruned so consecutive checkpoints are within `(1−ε)` of each other. A window query returns the
//! checkpoint whose start sits just before the window boundary, which the smoothness guarantees is
//! within `(1±ε)` of the true windowed value — all in `O((1/ε)·log R)` space, never the full window.
//!
//! This implementation instantiates the framework for the canonical smooth aggregate, the **sum of
//! non-negative values** over the last `W` elements (each checkpoint holds the running sum from its
//! start). The same checkpoint machinery generalizes to any smooth `f` (distinct counts, `Lp`
//! norms, longest-increasing-subsequence) by storing a per-checkpoint sketch instead of a scalar —
//! a documented follow-up.

use crate::common::{Result, SketchError};

/// A Smooth Histogram approximating the sum of the last `window` non-negative values.
///
/// # Example
/// ```
/// use sketch_oxide::streaming::SmoothHistogramSum;
///
/// let mut sh = SmoothHistogramSum::new(0.05, 1000).unwrap();
/// for i in 0..5000u64 { sh.update((i % 10) as f64); }
/// // The last 1000 values average ~4.5, so the windowed sum is ≈ 4500.
/// let est = sh.query();
/// assert!((est - 4500.0).abs() < 0.15 * 4500.0, "windowed sum {est}");
/// ```
#[derive(Debug, Clone)]
pub struct SmoothHistogramSum {
    epsilon: f64,
    window: usize,
    /// Checkpoints `(start_index, sum_from_start_to_now)`, ascending by start, descending by sum.
    checkpoints: Vec<(usize, f64)>,
    n: usize,
}

impl SmoothHistogramSum {
    /// Creates a Smooth Histogram with relative error `epsilon` over a window of `window` elements.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `epsilon` is not in `(0, 1)` or `window` is 0.
    pub fn new(epsilon: f64, window: usize) -> Result<Self> {
        if !(epsilon > 0.0 && epsilon < 1.0) {
            return Err(SketchError::InvalidParameter {
                param: "epsilon".to_string(),
                value: epsilon.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }
        if window == 0 {
            return Err(SketchError::InvalidParameter {
                param: "window".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            epsilon,
            window,
            checkpoints: Vec::new(),
            n: 0,
        })
    }

    /// Adds a non-negative value to the stream.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `value` is negative or not finite.
    pub fn update(&mut self, value: f64) -> Result<()> {
        if !value.is_finite() || value < 0.0 {
            return Err(SketchError::InvalidParameter {
                param: "value".to_string(),
                value: value.to_string(),
                constraint: "must be finite and non-negative".to_string(),
            });
        }
        for c in &mut self.checkpoints {
            c.1 += value;
        }
        self.checkpoints.push((self.n, value));
        self.n += 1;
        self.prune();
        // Drop checkpoints whose entire range has aged out of the window plus one straddler.
        self.trim_expired();
        Ok(())
    }

    /// Braverman–Ostrovsky pruning: keep the checkpoint set sparse by deleting the middle of any
    /// three consecutive checkpoints whose outer two are within `(1−ε)`.
    fn prune(&mut self) {
        let mut i = 0;
        while i + 2 < self.checkpoints.len() {
            if self.checkpoints[i + 2].1 >= (1.0 - self.epsilon) * self.checkpoints[i].1 {
                self.checkpoints.remove(i + 1);
            } else {
                i += 1;
            }
        }
    }

    /// Removes checkpoints that are strictly older than the window, keeping one straddling
    /// checkpoint so a query always has a start at or before the window boundary.
    fn trim_expired(&mut self) {
        let ws = self.n.saturating_sub(self.window);
        // Find the last checkpoint with start <= ws; everything strictly before it is redundant.
        let mut keep_from = 0;
        for (idx, &(start, _)) in self.checkpoints.iter().enumerate() {
            if start <= ws {
                keep_from = idx;
            } else {
                break;
            }
        }
        if keep_from > 0 {
            self.checkpoints.drain(0..keep_from);
        }
    }

    /// Estimated sum over the last `window` elements (within `(1±ε)` of the true windowed sum).
    pub fn query(&self) -> f64 {
        if self.checkpoints.is_empty() {
            return 0.0;
        }
        let ws = self.n.saturating_sub(self.window);
        // The checkpoint whose start is the latest one at or before the window boundary.
        let mut chosen = self.checkpoints[0].1;
        for &(start, sum) in &self.checkpoints {
            if start <= ws {
                chosen = sum;
            } else {
                break;
            }
        }
        chosen
    }

    /// Number of checkpoints currently retained (the sketch's space).
    #[inline]
    pub fn num_checkpoints(&self) -> usize {
        self.checkpoints.len()
    }

    /// Total elements seen.
    #[inline]
    pub fn count(&self) -> usize {
        self.n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exact windowed sum via a full buffer, for comparison.
    fn exact_window_sum(values: &[f64], window: usize) -> f64 {
        let start = values.len().saturating_sub(window);
        values[start..].iter().sum()
    }

    #[test]
    fn rejects_bad_params() {
        assert!(SmoothHistogramSum::new(0.0, 100).is_err());
        assert!(SmoothHistogramSum::new(1.0, 100).is_err());
        assert!(SmoothHistogramSum::new(0.1, 0).is_err());
        assert!(SmoothHistogramSum::new(0.1, 100).is_ok());
    }

    #[test]
    fn rejects_bad_values() {
        let mut sh = SmoothHistogramSum::new(0.1, 100).unwrap();
        assert!(sh.update(-1.0).is_err());
        assert!(sh.update(f64::NAN).is_err());
        assert!(sh.update(3.0).is_ok());
    }

    #[test]
    fn approximates_uniform_window() {
        let eps = 0.05;
        let mut sh = SmoothHistogramSum::new(eps, 1000).unwrap();
        let values: Vec<f64> = (0..6000u64).map(|i| (i % 10) as f64).collect();
        for &v in &values {
            sh.update(v).unwrap();
        }
        let exact = exact_window_sum(&values, 1000);
        let est = sh.query();
        assert!(
            (est - exact).abs() <= 2.0 * eps * exact,
            "est {est} vs exact {exact}"
        );
    }

    #[test]
    fn approximates_at_multiple_points() {
        let eps = 0.05;
        let window = 500;
        let mut sh = SmoothHistogramSum::new(eps, window).unwrap();
        let mut values = Vec::new();
        for i in 0..4000u64 {
            let v = ((i * 37 % 23) + 1) as f64;
            values.push(v);
            sh.update(v).unwrap();
            if i % 250 == 0 && i as usize >= window {
                let exact = exact_window_sum(&values, window);
                let est = sh.query();
                assert!(
                    (est - exact).abs() <= 2.0 * eps * exact + 1.0,
                    "at {i}: est {est} vs exact {exact}"
                );
            }
        }
    }

    #[test]
    fn checkpoint_count_is_sublinear() {
        let mut sh = SmoothHistogramSum::new(0.1, 100_000).unwrap();
        for _ in 0..100_000 {
            sh.update(1.0).unwrap();
        }
        // O((1/eps) log R) checkpoints, far below the window size.
        assert!(
            sh.num_checkpoints() < 500,
            "checkpoints {}",
            sh.num_checkpoints()
        );
    }

    #[test]
    fn window_larger_than_stream_sums_all() {
        let mut sh = SmoothHistogramSum::new(0.1, 10_000).unwrap();
        for _ in 0..100 {
            sh.update(2.0).unwrap();
        }
        // Only 100 elements seen, window 10000 → full sum 200.
        assert!((sh.query() - 200.0).abs() < 0.1 * 200.0);
    }
}
