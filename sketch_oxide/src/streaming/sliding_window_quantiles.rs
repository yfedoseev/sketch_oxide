//! Sliding-window quantiles — approximate quantiles over the most recent `window` items.
//!
//! Monitoring asks `quantile_over_time(p99, 5m)`: the p99 of only the *recent* stream, with old data
//! aged out. This composes the exponential-histogram-over-mergeable-sketch idea of PromSketch's EHKLL
//! (Zhu et al., VLDB 2025): the window is divided into fixed-size **blocks**, each summarized by a
//! [`KllSketch`](crate::quantiles::KllSketch); a query merges the in-window blocks (KLL is additively
//! mergeable) and reads the quantile, while the oldest block is dropped once the window slides past it.
//!
//! The window is *count-based* (the last `window` items) with block-size granularity: the structure
//! retains the last `window / block` completed blocks plus the partially-filled current block, so it
//! covers between `window` and `window + block` items. Memory is `O((window/block)·KLL)`, independent
//! of how long the stream runs.

use crate::common::{Mergeable, Result, SketchError};
use crate::quantiles::KllSketch;
use std::collections::VecDeque;

/// Sliding-window quantile sketch: a ring of per-block KLL summaries.
///
/// # Example
/// ```
/// use sketch_oxide::streaming::SlidingWindowQuantiles;
///
/// let mut swq = SlidingWindowQuantiles::new(10_000, 1_000, 256).unwrap();
/// // Old regime: values around 500.
/// for i in 0..20_000u64 { swq.update((i % 1000) as f64); }
/// // New regime: values around 5500 — fills the whole window.
/// for i in 0..20_000u64 { swq.update(5000.0 + (i % 1000) as f64); }
///
/// // The window now reflects only the recent (high) regime.
/// let median = swq.quantile(0.5).unwrap();
/// assert!(median > 5000.0 && median < 6000.0, "median {median}");
/// ```
#[derive(Debug, Clone)]
pub struct SlidingWindowQuantiles {
    block_size: usize,
    /// Number of completed blocks retained (covers `window` items).
    num_blocks: usize,
    k: u16,
    /// Completed blocks, oldest at the front.
    blocks: VecDeque<KllSketch>,
    current: KllSketch,
    current_count: usize,
}

impl SlidingWindowQuantiles {
    /// Creates a sketch over the last `window` items, summarizing each `block` items with a KLL of
    /// parameter `k` (larger `k` ⇒ more accurate).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `block` is 0 or larger than `window`; propagates KLL `k`
    /// validation.
    pub fn new(window: usize, block: usize, k: u16) -> Result<Self> {
        if block == 0 || block > window {
            return Err(SketchError::InvalidParameter {
                param: "block".to_string(),
                value: block.to_string(),
                constraint: "must be in 1..=window".to_string(),
            });
        }
        let num_blocks = window / block;
        Ok(Self {
            block_size: block,
            num_blocks,
            k,
            blocks: VecDeque::with_capacity(num_blocks + 1),
            current: KllSketch::new(k)?,
            current_count: 0,
        })
    }

    /// Records `value` as the newest item, aging out the oldest block when the window slides.
    pub fn update(&mut self, value: f64) {
        self.current.update(value);
        self.current_count += 1;
        if self.current_count >= self.block_size {
            let finished = std::mem::replace(
                &mut self.current,
                KllSketch::new(self.k).expect("k validated"),
            );
            self.blocks.push_back(finished);
            self.current_count = 0;
            while self.blocks.len() > self.num_blocks {
                self.blocks.pop_front();
            }
        }
    }

    /// Estimated `phi`-quantile over the items currently in the window, or `None` if the window is
    /// empty.
    pub fn quantile(&self, phi: f64) -> Option<f64> {
        let mut acc = KllSketch::new(self.k).ok()?;
        for block in &self.blocks {
            acc.merge(block).ok()?;
        }
        acc.merge(&self.current).ok()?;
        acc.quantile(phi)
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
        assert!(SlidingWindowQuantiles::new(1000, 0, 256).is_err());
        assert!(SlidingWindowQuantiles::new(1000, 2000, 256).is_err());
        assert!(SlidingWindowQuantiles::new(1000, 100, 256).is_ok());
    }

    #[test]
    fn reflects_recent_regime() {
        let mut swq = SlidingWindowQuantiles::new(10_000, 1_000, 256).unwrap();
        for i in 0..30_000u64 {
            swq.update((i % 1000) as f64); // old regime ~ [0, 1000)
        }
        for i in 0..20_000u64 {
            swq.update(5000.0 + (i % 1000) as f64); // new regime ~ [5000, 6000)
        }
        let median = swq.quantile(0.5).unwrap();
        assert!(median > 5000.0 && median < 6000.0, "median {median}");
    }

    #[test]
    fn early_window_reflects_early_values() {
        let mut swq = SlidingWindowQuantiles::new(10_000, 1_000, 256).unwrap();
        for v in 0..8_000u64 {
            swq.update(v as f64); // ramp 0..8000, all within the window
        }
        let median = swq.quantile(0.5).unwrap();
        assert!((median - 4000.0).abs() < 600.0, "median {median}");
    }

    #[test]
    fn window_is_bounded() {
        let mut swq = SlidingWindowQuantiles::new(5_000, 1_000, 128).unwrap();
        for v in 0..100_000u64 {
            swq.update(v as f64);
        }
        // At most window/block completed blocks are kept, regardless of stream length.
        assert!(
            swq.retained_blocks() <= 5,
            "retained {}",
            swq.retained_blocks()
        );
    }

    #[test]
    fn empty_is_none() {
        let swq = SlidingWindowQuantiles::new(1000, 100, 128).unwrap();
        assert!(swq.quantile(0.5).is_none());
    }
}
