//! FCM-Sketch — a hierarchical Count-Min with overflow chaining.
//!
//! FCM (Song, Lee, et al., "FCM-Sketch", 2020) is a drop-in Count-Min replacement that spends
//! bits where they are needed. Each row has a wide layer of **small** leaf counters (8-bit)
//! and a narrow layer of **large** overflow counters (32-bit). A key increments its 8-bit
//! leaf until it saturates, after which further increments spill into a shared 32-bit counter
//! — so the long tail costs one byte while heavy keys borrow a wide counter. The estimate is
//! the minimum over rows of the chained value, preserving Count-Min's no-underestimate
//! guarantee at much lower memory for skewed streams.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

/// Cap of an 8-bit leaf counter; reaching it means the key has overflowed into layer 2.
const LEAF_CAP: u8 = u8::MAX;

/// A single FCM row: a wide 8-bit leaf layer and a narrow 32-bit overflow layer.
#[derive(Debug, Clone)]
struct Row {
    leaf: Vec<u8>,
    overflow: Vec<u32>,
}

/// FCM-Sketch over `depth` rows.
///
/// # Example
/// ```
/// use sketch_oxide::frequency::FcmSketch;
///
/// let mut f = FcmSketch::new(4, 4096, 512).unwrap();
/// for _ in 0..100_000 { f.update(b"heavy"); }  // overflows the 8-bit leaf
/// for i in 0..1000u64 { f.update(&i.to_le_bytes()); }
///
/// assert!(f.estimate(b"heavy") >= 100_000); // no underestimate
/// ```
#[derive(Debug, Clone)]
pub struct FcmSketch {
    depth: usize,
    leaf_width: usize,
    overflow_width: usize,
    rows: Vec<Row>,
}

impl FcmSketch {
    /// Creates an FCM-Sketch with `depth` rows, `leaf_width` 8-bit counters and
    /// `overflow_width` 32-bit counters per row (`overflow_width` should be smaller).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if any dimension is 0.
    pub fn new(depth: usize, leaf_width: usize, overflow_width: usize) -> Result<Self> {
        let bad = if depth == 0 {
            Some("depth")
        } else if leaf_width == 0 {
            Some("leaf_width")
        } else if overflow_width == 0 {
            Some("overflow_width")
        } else {
            None
        };
        if let Some(param) = bad {
            return Err(SketchError::InvalidParameter {
                param: param.to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        let rows = (0..depth)
            .map(|_| Row {
                leaf: vec![0u8; leaf_width],
                overflow: vec![0u32; overflow_width],
            })
            .collect();
        Ok(Self {
            depth,
            leaf_width,
            overflow_width,
            rows,
        })
    }

    #[inline]
    fn leaf_idx(&self, item: &[u8], r: usize) -> usize {
        (xxhash(item, (r as u64) << 1) % self.leaf_width as u64) as usize
    }

    #[inline]
    fn overflow_idx(&self, item: &[u8], r: usize) -> usize {
        (xxhash(item, ((r as u64) << 1) | 1) % self.overflow_width as u64) as usize
    }

    /// Records one occurrence of `item`.
    pub fn update(&mut self, item: &[u8]) {
        for r in 0..self.depth {
            let li = self.leaf_idx(item, r);
            if self.rows[r].leaf[li] < LEAF_CAP {
                self.rows[r].leaf[li] += 1;
            } else {
                let oi = self.overflow_idx(item, r);
                self.rows[r].overflow[oi] = self.rows[r].overflow[oi].saturating_add(1);
            }
        }
    }

    /// Per-row chained value for `item`: the leaf count, plus the overflow counter once the
    /// leaf has saturated.
    fn row_value(&self, item: &[u8], r: usize) -> u64 {
        let leaf = self.rows[r].leaf[self.leaf_idx(item, r)];
        if leaf < LEAF_CAP {
            leaf as u64
        } else {
            LEAF_CAP as u64 + self.rows[r].overflow[self.overflow_idx(item, r)] as u64
        }
    }

    /// Estimates the count of `item` (min over rows of the chained value).
    pub fn estimate(&self, item: &[u8]) -> u64 {
        (0..self.depth)
            .map(|r| self.row_value(item, r))
            .min()
            .unwrap_or(0)
    }

    /// Approximate memory in bytes.
    pub fn memory_bytes(&self) -> usize {
        self.depth * (self.leaf_width + self.overflow_width * 4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_dims() {
        assert!(FcmSketch::new(0, 100, 10).is_err());
        assert!(FcmSketch::new(4, 0, 10).is_err());
        assert!(FcmSketch::new(4, 100, 0).is_err());
        assert!(FcmSketch::new(4, 100, 10).is_ok());
    }

    #[test]
    fn small_count_is_exact() {
        let mut f = FcmSketch::new(4, 8192, 512).unwrap();
        for _ in 0..100 {
            f.update(b"cold");
        }
        // 100 < 255 => fits in the leaf, sparse sketch => exact.
        assert_eq!(f.estimate(b"cold"), 100);
    }

    #[test]
    fn never_underestimates_after_overflow() {
        let mut f = FcmSketch::new(4, 4096, 512).unwrap();
        for _ in 0..10_000 {
            f.update(b"x"); // far past the 255 leaf cap
        }
        let e = f.estimate(b"x");
        assert!(e >= 10_000, "estimate {e}");
        assert!(e < 11_000, "estimate {e} overestimated too much");
    }

    #[test]
    fn boundary_at_leaf_cap() {
        let mut f = FcmSketch::new(3, 4096, 256).unwrap();
        for _ in 0..255 {
            f.update(b"edge");
        }
        assert_eq!(f.estimate(b"edge"), 255, "exactly at the leaf cap");
        f.update(b"edge");
        assert_eq!(f.estimate(b"edge"), 256, "one past the cap uses overflow");
    }

    #[test]
    fn min_over_rows_limits_error() {
        let mut f = FcmSketch::new(5, 2048, 256).unwrap();
        for _ in 0..50 {
            f.update(b"target");
        }
        for i in 0..5000u64 {
            f.update(&i.to_le_bytes());
        }
        let e = f.estimate(b"target");
        assert!(e >= 50, "no underestimate");
        assert!(e < 100, "min over rows keeps overestimate small: {e}");
    }

    #[test]
    fn reports_memory() {
        let f = FcmSketch::new(4, 1000, 100).unwrap();
        assert_eq!(f.memory_bytes(), 4 * (1000 + 400));
    }
}

// --- Capability-trait adoption (fable5 doc 01 F3) ---
use crate::common::capabilities::{PointQuery, Update};

impl Update<[u8]> for FcmSketch {
    fn update(&mut self, item: &[u8]) {
        FcmSketch::update(self, item);
    }
}

impl PointQuery<[u8]> for FcmSketch {
    fn query(&self, item: &[u8]) -> u64 {
        self.estimate(item)
    }
}
