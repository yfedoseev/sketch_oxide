//! Count-based sliding-window aggregation over any [`Mergeable`] sketch.
//!
//! `WindowedAggregator<S>` turns any mergeable sketch into a windowed one "for free":
//! keep the last `W` per-pane sketches (one pane per time slice, per micro-batch, etc.)
//! and ask for the merge of everything currently in the window. That gives windowed
//! Theta / CPC / HLL / KLL / t-digest / Count-Min with no per-sketch windowing code.
//!
//! # Algorithm
//!
//! It uses the classic **two-stack** FIFO monoid aggregator (Hammer, "reactive
//! aggregator"; the amortized basis that DABA Lite and FiBA later de-amortize and extend
//! to out-of-order data). Two stacks — a push side (`back`) and a pop side (`front`) —
//! each store, alongside every value, the running aggregate of that stack in FIFO order.
//! A window query combines the two top aggregates in O(1); pushing is O(1); evicting is
//! O(1) amortized (an occasional O(n) "flip" of `back` into `front` is paid off over the
//! pops it enables). FIFO order is preserved in the combine, so it is correct for any
//! associative merge, not only commutative ones.
//!
//! Merging is the monoid operation and an empty sketch is its identity — supplied once at
//! construction and cloned for the empty-window result.
//!
//! # Window model
//!
//! This is a **count-based** window of `W` panes: `push` adds the newest pane and evicts
//! the oldest once `W` is exceeded. To build a *time*-based window, drive it from the
//! [`time`](crate::common::time) convention — advance a [`Watermark`](crate::common::time::Watermark)
//! and `push` one pane per elapsed slice. (Per-element event-time windows with
//! out-of-order tolerance are FiBA's job, in a later wave.)
//!
//! # Example — windowed cardinality over the last 3 panes
//!
//! ```
//! use sketch_oxide::streaming::WindowedAggregator;
//! use sketch_oxide::cardinality::HyperLogLog;
//! use sketch_oxide::common::Sketch;
//!
//! // Empty HLL is the identity element; window holds 3 panes.
//! let identity = HyperLogLog::new(12).unwrap();
//! let mut win = WindowedAggregator::new(3, identity).unwrap();
//!
//! // Each pane is an HLL summarising one time slice.
//! for pane in 0..5u64 {
//!     let mut hll = HyperLogLog::new(12).unwrap();
//!     for i in 0..1000u64 {
//!         hll.update(&(pane * 1000 + i)); // 1000 distinct items per pane
//!     }
//!     win.push(hll).unwrap();
//! }
//!
//! // Only the last 3 panes remain in the window: ~3000 distinct items.
//! let merged = win.query().unwrap();
//! let est = merged.estimate();
//! assert!(est > 2400.0 && est < 3600.0, "windowed estimate was {est}");
//! ```

use crate::common::{Mergeable, Result};

/// One stored pane and the running aggregate of its stack (in FIFO order) up to it.
#[derive(Clone, Debug)]
struct Entry<S> {
    /// The pane value as pushed.
    val: S,
    /// Aggregate of this stack from its FIFO-oldest entry through this one.
    agg: S,
}

/// Count-based sliding-window aggregator over a [`Mergeable`] sketch `S`.
///
/// Holds at most `capacity` panes; [`push`](Self::push) adds the newest and evicts the
/// oldest when full, and [`query`](Self::query) returns the merge of all panes currently
/// in the window.
#[derive(Clone, Debug)]
pub struct WindowedAggregator<S> {
    /// Empty element, cloned to answer an empty-window query.
    identity: S,
    /// Pop side (FIFO-older panes); top is the oldest pane.
    front: Vec<Entry<S>>,
    /// Push side (FIFO-newer panes); top is the newest pane.
    back: Vec<Entry<S>>,
    /// Maximum number of panes retained in the window.
    capacity: usize,
}

impl<S: Mergeable + Clone> WindowedAggregator<S> {
    /// Creates an aggregator over a window of `capacity` panes.
    ///
    /// `identity` must be an empty sketch of the same configuration the pushed panes will
    /// use; it is returned (cloned) when the window is empty.
    ///
    /// # Errors
    /// Returns [`SketchError::InvalidParameter`](crate::common::SketchError::InvalidParameter)
    /// if `capacity` is 0.
    pub fn new(capacity: usize, identity: S) -> Result<Self> {
        if capacity == 0 {
            return Err(crate::common::SketchError::InvalidParameter {
                param: "capacity".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            identity,
            front: Vec::new(),
            back: Vec::new(),
            capacity,
        })
    }

    /// Number of panes currently in the window.
    #[inline]
    pub fn len(&self) -> usize {
        self.front.len() + self.back.len()
    }

    /// Whether the window currently holds no panes.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.front.is_empty() && self.back.is_empty()
    }

    /// The window capacity (maximum number of panes).
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// `a` merged with `b`, without mutating either.
    fn combine(a: &S, b: &S) -> Result<S> {
        let mut merged = a.clone();
        merged.merge(b)?;
        Ok(merged)
    }

    /// Pushes the newest pane, evicting the oldest if the window is full.
    ///
    /// # Errors
    /// Propagates a merge error if `value` is incompatible with the panes already in the
    /// window (e.g. different precision/configuration).
    pub fn push(&mut self, value: S) -> Result<()> {
        // Running aggregate of the back stack in FIFO order: combine(prev_top, value).
        let agg = match self.back.last() {
            Some(top) => Self::combine(&top.agg, &value)?,
            None => value.clone(),
        };
        self.back.push(Entry { val: value, agg });

        if self.len() > self.capacity {
            self.evict()?;
        }
        Ok(())
    }

    /// Removes the oldest pane (FIFO dequeue), flipping `back` into `front` if needed.
    fn evict(&mut self) -> Result<()> {
        if self.front.is_empty() {
            self.flip()?;
        }
        // After a flip the front holds every pane; drop its top (the oldest).
        self.front.pop();
        Ok(())
    }

    /// Moves all of `back` into `front`, reversing order so `front`'s top is the oldest
    /// pane and each `front` aggregate covers that entry through the FIFO-oldest pane.
    fn flip(&mut self) -> Result<()> {
        debug_assert!(self.front.is_empty());
        while let Some(entry) = self.back.pop() {
            // front aggregate is combine(value, previous_front_top) to keep FIFO order.
            let agg = match self.front.last() {
                Some(top) => Self::combine(&entry.val, &top.agg)?,
                None => entry.val.clone(),
            };
            self.front.push(Entry {
                val: entry.val,
                agg,
            });
        }
        Ok(())
    }

    /// Returns the merge of every pane currently in the window (the identity if empty).
    ///
    /// # Errors
    /// Propagates a merge error if the two stack aggregates are incompatible (should not
    /// happen when all pushed panes share a configuration).
    pub fn query(&self) -> Result<S> {
        match (self.front.last(), self.back.last()) {
            (None, None) => Ok(self.identity.clone()),
            (Some(f), None) => Ok(f.agg.clone()),
            (None, Some(b)) => Ok(b.agg.clone()),
            // front holds the FIFO-older panes, back the newer ones.
            (Some(f), Some(b)) => Self::combine(&f.agg, &b.agg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cardinality::HyperLogLog;
    use crate::common::Sketch;

    /// Builds an HLL covering the half-open id range `[start, end)`.
    fn hll_range(start: u64, end: u64) -> HyperLogLog {
        let mut h = HyperLogLog::new(12).unwrap();
        for i in start..end {
            h.update(&i);
        }
        h
    }

    #[test]
    fn empty_window_returns_identity() {
        let win = WindowedAggregator::new(3, HyperLogLog::new(12).unwrap()).unwrap();
        assert!(win.is_empty());
        assert_eq!(win.len(), 0);
        assert_eq!(win.query().unwrap().estimate(), 0.0);
    }

    #[test]
    fn capacity_zero_rejected() {
        assert!(WindowedAggregator::new(0, HyperLogLog::new(12).unwrap()).is_err());
    }

    #[test]
    fn within_capacity_is_full_union() {
        let mut win = WindowedAggregator::new(3, HyperLogLog::new(12).unwrap()).unwrap();
        win.push(hll_range(0, 1000)).unwrap();
        win.push(hll_range(1000, 2000)).unwrap();
        assert_eq!(win.len(), 2);
        let est = win.query().unwrap().estimate();
        assert!(est > 1700.0 && est < 2300.0, "union estimate {est}");
    }

    #[test]
    fn eviction_drops_oldest_pane() {
        // Capacity 1: each push fully replaces the window.
        let mut win = WindowedAggregator::new(1, HyperLogLog::new(12).unwrap()).unwrap();
        win.push(hll_range(0, 100)).unwrap();
        assert!(win.query().unwrap().estimate() < 130.0);

        win.push(hll_range(0, 5000)).unwrap();
        assert!(
            win.query().unwrap().estimate() > 4000.0,
            "only newest pane in window"
        );

        win.push(hll_range(0, 10)).unwrap();
        assert!(
            win.query().unwrap().estimate() < 25.0,
            "big pane should have been evicted"
        );
        assert_eq!(win.len(), 1);
    }

    #[test]
    fn slides_to_last_w_panes_across_flips() {
        // Capacity 3, eight disjoint panes -> window holds panes 5,6,7 (~3000 distinct).
        let mut win = WindowedAggregator::new(3, HyperLogLog::new(12).unwrap()).unwrap();
        for pane in 0..8u64 {
            win.push(hll_range(pane * 1000, pane * 1000 + 1000))
                .unwrap();
        }
        assert_eq!(win.len(), 3);
        let est = win.query().unwrap().estimate();
        assert!(est > 2400.0 && est < 3600.0, "last-3-panes estimate {est}");
    }

    #[test]
    fn interleaved_push_query_stays_consistent() {
        // Exercise repeated flips: query after every push.
        let mut win = WindowedAggregator::new(2, HyperLogLog::new(12).unwrap()).unwrap();
        for pane in 0..6u64 {
            win.push(hll_range(pane * 500, pane * 500 + 500)).unwrap();
            assert!(win.len() <= 2);
            let est = win.query().unwrap().estimate();
            // At most two panes of 500 distinct items => well under 1100.
            assert!(est < 1100.0, "pane {pane}: estimate {est}");
        }
    }
}
