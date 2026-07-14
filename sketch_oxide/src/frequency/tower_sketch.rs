//! TowerSketch — a Count-Min variant with tiered counter widths.
//!
//! Plain Count-Min spends the same number of bits per counter on every key, even though
//! most keys are cold and need only a few bits. TowerSketch (Yang et al., the SALSA /
//! Elastic line of work) stacks rows of *different* counter widths at equal memory per row:
//! a narrow row (8-bit) holds many counters for the long tail, a wider row (32-bit) holds
//! fewer counters for heavy keys. A key is hashed into every row; the estimate is the
//! minimum over the rows whose counter has **not saturated**, which keeps Count-Min's
//! no-underestimate property while packing the cold mass tightly.
//!
//! This is the canonical three-tier tower: 8-bit, 16-bit and 32-bit rows, with
//! proportionally more counters in the narrower rows (equal bytes per row).

use crate::common::hash::xxhash;

/// A three-tier TowerSketch over 8/16/32-bit counter rows.
///
/// # Example
/// ```
/// use sketch_oxide::frequency::TowerSketch;
///
/// let mut t = TowerSketch::new(1024).unwrap();
/// for _ in 0..50_000 { t.update(b"hot"); }   // exceeds the 8-bit row's cap
/// for i in 0..1000u64 { t.update(&i.to_le_bytes()); }
///
/// // No underestimate; wider rows carry the heavy key.
/// assert!(t.estimate(b"hot") >= 50_000);
/// ```
#[derive(Debug, Clone)]
pub struct TowerSketch {
    base_width: usize,
    /// 8-bit row, `4 * base_width` counters (cap 255).
    r8: Vec<u8>,
    /// 16-bit row, `2 * base_width` counters (cap 65535).
    r16: Vec<u16>,
    /// 32-bit row, `base_width` counters.
    r32: Vec<u32>,
}

impl TowerSketch {
    /// Creates a tower with `base_width` 32-bit counters (and `2×`, `4×` that many 16- and
    /// 8-bit counters). All three rows use roughly the same number of bytes.
    ///
    /// # Errors
    /// Returns an error if `base_width` is 0.
    pub fn new(base_width: usize) -> crate::common::Result<Self> {
        if base_width == 0 {
            return Err(crate::common::SketchError::InvalidParameter {
                param: "base_width".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            base_width,
            r8: vec![0u8; base_width * 4],
            r16: vec![0u16; base_width * 2],
            r32: vec![0u32; base_width],
        })
    }

    #[inline]
    fn idx(&self, item: &[u8], seed: u64, modulo: usize) -> usize {
        (xxhash(item, seed) % modulo as u64) as usize
    }

    /// Records one occurrence of `item`.
    pub fn update(&mut self, item: &[u8]) {
        let i8 = self.idx(item, 0, self.r8.len());
        self.r8[i8] = self.r8[i8].saturating_add(1);
        let i16 = self.idx(item, 1, self.r16.len());
        self.r16[i16] = self.r16[i16].saturating_add(1);
        let i32 = self.idx(item, 2, self.r32.len());
        self.r32[i32] = self.r32[i32].saturating_add(1);
    }

    /// Estimates the count of `item`: the minimum over rows whose counter has not saturated.
    pub fn estimate(&self, item: &[u8]) -> u64 {
        let mut best = u64::MAX;
        let v8 = self.r8[self.idx(item, 0, self.r8.len())];
        if v8 < u8::MAX {
            best = best.min(v8 as u64);
        }
        let v16 = self.r16[self.idx(item, 1, self.r16.len())];
        if v16 < u16::MAX {
            best = best.min(v16 as u64);
        }
        let v32 = self.r32[self.idx(item, 2, self.r32.len())];
        if v32 < u32::MAX {
            best = best.min(v32 as u64);
        }
        if best == u64::MAX {
            // Every row saturated: return the largest cap as a lower bound.
            u32::MAX as u64
        } else {
            best
        }
    }

    /// Approximate memory use in bytes.
    pub fn memory_bytes(&self) -> usize {
        self.r8.len() + self.r16.len() * 2 + self.r32.len() * 4
    }

    /// Base width (number of 32-bit counters).
    #[inline]
    pub fn base_width(&self) -> usize {
        self.base_width
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_width() {
        assert!(TowerSketch::new(0).is_err());
        assert!(TowerSketch::new(256).is_ok());
    }

    #[test]
    fn never_underestimates() {
        let mut t = TowerSketch::new(2048).unwrap();
        for _ in 0..300 {
            t.update(b"x"); // exceeds u8 cap (255)
        }
        assert!(t.estimate(b"x") >= 300, "estimate {}", t.estimate(b"x"));
    }

    #[test]
    fn cold_key_exact_when_uncontended() {
        let mut t = TowerSketch::new(4096).unwrap();
        for _ in 0..10 {
            t.update(b"cold");
        }
        // Small count, sparse sketch => likely exact via the 8-bit row.
        assert_eq!(t.estimate(b"cold"), 10);
    }

    #[test]
    fn heavy_key_uses_wider_rows() {
        let mut t = TowerSketch::new(1024).unwrap();
        for _ in 0..100_000 {
            t.update(b"heavy");
        }
        // The 8-bit (and 16-bit) rows saturate; the 32-bit row carries the value.
        let e = t.estimate(b"heavy");
        assert!(e >= 100_000, "heavy estimate {e}");
        assert!(e < 110_000, "heavy estimate {e} overestimated too much");
    }

    #[test]
    fn narrow_rows_have_more_counters() {
        let t = TowerSketch::new(1000).unwrap();
        assert_eq!(t.r8.len(), 4000);
        assert_eq!(t.r16.len(), 2000);
        assert_eq!(t.r32.len(), 1000);
        // Equal bytes per row: 4000*1 == 2000*2 == 1000*4.
        assert_eq!(t.r8.len(), t.r16.len() * 2);
        assert_eq!(t.r8.len(), t.r32.len() * 4);
    }

    #[test]
    fn reports_memory() {
        let t = TowerSketch::new(1000).unwrap();
        // 4000 + 4000 + 4000 bytes.
        assert_eq!(t.memory_bytes(), 12_000);
    }
}

// --- Capability-trait adoption (fable5 doc 01 F3) ---
use crate::common::capabilities::{PointQuery, Update};

impl Update<[u8]> for TowerSketch {
    fn update(&mut self, item: &[u8]) {
        TowerSketch::update(self, item);
    }
}

impl PointQuery<[u8]> for TowerSketch {
    fn query(&self, item: &[u8]) -> u64 {
        self.estimate(item)
    }
}
