//! Sliding-window uniform sampling.
//!
//! Maintaining a uniform random sample of the **last `W`** stream elements is harder than plain
//! reservoir sampling: the sampled element can expire and must be replaced, yet the stream is seen
//! once. The classic solution (Babcock, Datar & Motwani, "Sampling From a Moving Window over
//! Streaming Data", SODA 2002) gives every element a random priority and keeps the in-window
//! element of **minimum priority** as the sample — which is uniform because the priorities are
//! i.i.d. The trick is to store only the elements that could ever *become* that minimum as the
//! window slides: an element is kept only while no later element has a smaller priority, so the
//! retained set is a monotonic "staircase" (a deque of strictly increasing priorities) whose front
//! is always the current sample.
//!
//! This maintains a single uniform sample (`k = 1`); a `k`-sample variant keeps `k` parallel
//! staircases and is a documented follow-up.

use crate::common::SketchError;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::VecDeque;

#[derive(Debug, Clone)]
struct Entry<T> {
    index: u64,
    priority: f64,
    value: T,
}

/// A uniform sampler over the last `window` elements of a stream.
///
/// # Example
/// ```
/// use sketch_oxide::sampling::SlidingWindowSample;
///
/// let mut s = SlidingWindowSample::with_seed(100, 42).unwrap();
/// for i in 0..10_000u64 { s.push(i); }
/// // The sample is always one of the last 100 elements.
/// let v = *s.sample().unwrap();
/// assert!(v >= 9_900);
/// ```
#[derive(Debug, Clone)]
pub struct SlidingWindowSample<T: Clone> {
    window: u64,
    n: u64,
    /// Monotonic deque: strictly increasing priority front→back; front is the current sample.
    deque: VecDeque<Entry<T>>,
    rng: SmallRng,
}

impl<T: Clone> SlidingWindowSample<T> {
    /// Creates a sampler over a window of `window` elements, seeded from the OS.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `window` is 0.
    pub fn new(window: u64) -> Result<Self, SketchError> {
        Self::build(window, SmallRng::from_os_rng())
    }

    /// Creates a reproducible sampler over a window of `window` elements with the given `seed`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `window` is 0.
    pub fn with_seed(window: u64, seed: u64) -> Result<Self, SketchError> {
        Self::build(window, SmallRng::seed_from_u64(seed))
    }

    fn build(window: u64, rng: SmallRng) -> Result<Self, SketchError> {
        if window == 0 {
            return Err(SketchError::InvalidParameter {
                param: "window".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            window,
            n: 0,
            deque: VecDeque::new(),
            rng,
        })
    }

    /// Feeds one element to the stream.
    pub fn push(&mut self, value: T) {
        let index = self.n;
        self.n += 1;
        let priority: f64 = self.rng.random();
        // Drop tail entries that can never again be the minimum (this newer one is smaller).
        while let Some(back) = self.deque.back() {
            if back.priority >= priority {
                self.deque.pop_back();
            } else {
                break;
            }
        }
        self.deque.push_back(Entry {
            index,
            priority,
            value,
        });
        // Expire the front while it has fallen out of the window.
        let cutoff = self.n.saturating_sub(self.window);
        while let Some(front) = self.deque.front() {
            if front.index < cutoff {
                self.deque.pop_front();
            } else {
                break;
            }
        }
    }

    /// The current uniform sample of the last `window` elements (`None` before the first push).
    pub fn sample(&self) -> Option<&T> {
        self.deque.front().map(|e| &e.value)
    }

    /// Total elements seen.
    #[inline]
    pub fn count(&self) -> u64 {
        self.n
    }

    /// Window size.
    #[inline]
    pub fn window(&self) -> u64 {
        self.window
    }

    /// Number of retained candidates (the staircase size; at most `window`).
    #[inline]
    pub fn retained(&self) -> usize {
        self.deque.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_window() {
        assert!(SlidingWindowSample::<u64>::new(0).is_err());
        assert!(SlidingWindowSample::<u64>::new(10).is_ok());
    }

    #[test]
    fn empty_has_no_sample() {
        let s = SlidingWindowSample::<u64>::with_seed(10, 1).unwrap();
        assert!(s.sample().is_none());
    }

    #[test]
    fn sample_is_always_in_window() {
        let mut s = SlidingWindowSample::with_seed(100, 7).unwrap();
        for i in 0..100_000u64 {
            s.push(i);
            let v = *s.sample().unwrap();
            // The sample's value (== its index here) is within the last `window` indices.
            assert!(v + 100 > i, "sample {v} not in window ending at {i}");
            assert!(v <= i);
        }
    }

    #[test]
    fn staircase_is_bounded_by_window() {
        let mut s = SlidingWindowSample::with_seed(50, 3).unwrap();
        for i in 0..10_000u64 {
            s.push(i);
            assert!(s.retained() as u64 <= 50);
        }
    }

    #[test]
    fn sample_is_approximately_uniform() {
        // Over many seeds, the final sample's offset within the window averages near the midpoint.
        let window = 1000u64;
        let n = 50_000u64;
        let mut offsets = Vec::new();
        for seed in 0..200u64 {
            let mut s = SlidingWindowSample::with_seed(window, seed).unwrap();
            for i in 0..n {
                s.push(i);
            }
            let v = *s.sample().unwrap();
            offsets.push((n - 1 - v) as f64); // 0 = newest, window-1 = oldest
        }
        let avg = offsets.iter().sum::<f64>() / offsets.len() as f64;
        let expected = (window - 1) as f64 / 2.0;
        assert!(
            (avg - expected).abs() < 0.12 * expected,
            "mean offset {avg} far from {expected}"
        );
    }

    #[test]
    fn deterministic_with_seed() {
        let run = || {
            let mut s = SlidingWindowSample::with_seed(20, 99).unwrap();
            for i in 0..5000u64 {
                s.push(i);
            }
            *s.sample().unwrap()
        };
        assert_eq!(run(), run());
    }
}
