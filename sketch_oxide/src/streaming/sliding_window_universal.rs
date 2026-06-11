//! Sliding-window universal sketch — windowed L2 / heavy-hitters over the most recent items.
//!
//! The companion to [`SlidingWindowQuantiles`](crate::streaming::SlidingWindowQuantiles): PromSketch's
//! EHUniv (Zhu et al., VLDB 2025) puts a [`UnivMon`](crate::universal::UnivMon) universal sketch in
//! each block of an exponential-histogram-style window, so a single structure answers `l2_over_time`
//! (and other additive frequency-moment functions) over the last `window` items. Because UnivMon is
//! additively mergeable, a query merges the in-window blocks and evaluates the function; the oldest
//! block is aged out as the window slides.
//!
//! One UnivMon per *block* (not per item) keeps memory at `O((window/block)·UnivMon)`, independent of
//! stream length — the point of the EH construction for a sketch as heavy as UnivMon. (Heavy-hitter
//! recovery over a merged window is lossy and is left as a follow-up; the moment estimators are exact
//! to the inner sketch's accuracy under merge.)

use crate::common::{Mergeable, Result, SketchError};
use crate::universal::UnivMon;
use std::collections::VecDeque;

/// Sliding-window universal sketch: a ring of per-block UnivMon summaries.
///
/// # Example
/// ```
/// use sketch_oxide::streaming::SlidingWindowUniversal;
///
/// let mut swu = SlidingWindowUniversal::new(4_000, 1_000, 0.1, 0.1).unwrap();
/// // Stream far more distinct items than the window holds.
/// for i in 0..20_000u64 {
///     swu.update(&i.to_le_bytes()).unwrap();
/// }
/// // Each item appears once, so the windowed L2 ≈ √(window) ≈ 63, NOT √20000 ≈ 141.
/// let l2 = swu.estimate_l2();
/// assert!(l2 > 30.0 && l2 < 110.0, "windowed l2 {l2}");
/// ```
#[derive(Debug, Clone)]
pub struct SlidingWindowUniversal {
    block_size: usize,
    num_blocks: usize,
    max_stream_size: u64,
    epsilon: f64,
    delta: f64,
    blocks: VecDeque<UnivMon>,
    current: UnivMon,
    current_count: usize,
}

impl SlidingWindowUniversal {
    /// Creates a windowed universal sketch over the last `window` items, summarizing each `block`
    /// items with a UnivMon of accuracy `(epsilon, delta)`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `block` is 0 or larger than `window`; propagates UnivMon
    /// parameter validation.
    pub fn new(window: usize, block: usize, epsilon: f64, delta: f64) -> Result<Self> {
        if block == 0 || block > window {
            return Err(SketchError::InvalidParameter {
                param: "block".to_string(),
                value: block.to_string(),
                constraint: "must be in 1..=window".to_string(),
            });
        }
        let num_blocks = window / block;
        let max_stream_size = window as u64;
        Ok(Self {
            block_size: block,
            num_blocks,
            max_stream_size,
            epsilon,
            delta,
            blocks: VecDeque::with_capacity(num_blocks + 1),
            current: UnivMon::new(max_stream_size, epsilon, delta)?,
            current_count: 0,
        })
    }

    fn fresh(&self) -> UnivMon {
        UnivMon::new(self.max_stream_size, self.epsilon, self.delta).expect("params validated")
    }

    /// Records one occurrence of `item` as the newest item.
    ///
    /// # Errors
    /// Propagates any error from the underlying UnivMon update.
    pub fn update(&mut self, item: &[u8]) -> Result<()> {
        self.current.update(item, 1.0)?;
        self.current_count += 1;
        if self.current_count >= self.block_size {
            let replacement = self.fresh();
            let finished = std::mem::replace(&mut self.current, replacement);
            self.blocks.push_back(finished);
            self.current_count = 0;
            while self.blocks.len() > self.num_blocks {
                self.blocks.pop_front();
            }
        }
        Ok(())
    }

    /// The merged UnivMon over the items currently in the window.
    fn merged(&self) -> UnivMon {
        let mut acc = self.fresh();
        for block in &self.blocks {
            let _ = acc.merge(block);
        }
        let _ = acc.merge(&self.current);
        acc
    }

    /// Estimated L2 norm (`√F2`) over the items currently in the window.
    pub fn estimate_l2(&self) -> f64 {
        self.merged().estimate_l2()
    }

    /// Number of completed blocks currently retained.
    #[inline]
    pub fn retained_blocks(&self) -> usize {
        self.blocks.len()
    }

    /// Block size.
    #[inline]
    pub fn block_size(&self) -> usize {
        self.block_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(SlidingWindowUniversal::new(1000, 0, 0.1, 0.1).is_err());
        assert!(SlidingWindowUniversal::new(1000, 2000, 0.1, 0.1).is_err());
        assert!(SlidingWindowUniversal::new(1000, 250, 0.1, 0.1).is_ok());
    }

    #[test]
    fn windowed_l2_is_bounded_by_window() {
        let mut swu = SlidingWindowUniversal::new(4_000, 1_000, 0.1, 0.1).unwrap();
        // 20_000 distinct items, each once: full-stream L2 would be √20000 ≈ 141.
        for i in 0..20_000u64 {
            swu.update(&i.to_le_bytes()).unwrap();
        }
        let l2 = swu.estimate_l2();
        // Windowed L2 reflects only ~4000 items ⇒ ≈ √4000 ≈ 63.
        assert!(l2 > 30.0 && l2 < 110.0, "windowed l2 {l2}");
        assert!(
            swu.retained_blocks() <= 4,
            "retained {}",
            swu.retained_blocks()
        );
    }

    #[test]
    fn recent_burst_raises_windowed_l2() {
        // A windowed L2 must rise when a recent heavy item concentrates mass in the window.
        let mut spread = SlidingWindowUniversal::new(4_000, 1_000, 0.1, 0.1).unwrap();
        for i in 0..8_000u64 {
            spread.update(&i.to_le_bytes()).unwrap(); // all distinct ⇒ low L2
        }
        let mut bursty = SlidingWindowUniversal::new(4_000, 1_000, 0.1, 0.1).unwrap();
        for i in 0..4_000u64 {
            bursty.update(&i.to_le_bytes()).unwrap();
        }
        for _ in 0..4_000 {
            bursty.update(b"hot").unwrap(); // 'hot' dominates the window ⇒ high L2
        }
        assert!(
            bursty.estimate_l2() > spread.estimate_l2() * 2.0,
            "bursty l2 {} vs spread l2 {}",
            bursty.estimate_l2(),
            spread.estimate_l2()
        );
    }

    #[test]
    fn empty_l2_is_zero() {
        let swu = SlidingWindowUniversal::new(1000, 250, 0.1, 0.1).unwrap();
        assert_eq!(swu.estimate_l2(), 0.0);
    }
}
