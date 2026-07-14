//! Per-flow quantiles (M4) — quantile estimates for millions of concurrent flows in fixed space.
//!
//! [`PerKeyQuantiles`](crate::quantiles::PerKeyQuantiles) tracks an exact summary for a bounded set of
//! *heavy* keys. The M4 framework (Wang et al., ICDE 2024) instead answers per-flow quantiles for
//! *every* flow in fixed space by sketching: each flow hashes to one cell per row of a `d × w` table,
//! and every cell holds a [`DDSketch`](crate::quantiles::DDSketch) that accumulates the values of all
//! flows mapping there. A flow's own values land in all `d` of its cells, each contaminated by a
//! different set of other flows; the **MIN technique** answers a query from the flow's *least*
//! contaminated cell — the one with the smallest total count — which most closely reflects the flow
//! alone.
//!
//! This trades exactness for unbounded flow cardinality: "what is the p99 latency of flow X?" across
//! millions of flows, with `O(d·w)` memory independent of the number of flows. A clean flow (no hash
//! collisions in its best cell) is answered at the inner sketch's accuracy; collisions only ever add
//! mass, so the min-count cell is the tightest available estimate.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use crate::quantiles::DDSketch;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Base seed mixed with the row index to derive each row's hash function.
const ROW_SEED_BASE: u64 = 0x4D34_5157_5541_4421; // "M4QUAD!"

/// A per-flow quantile sketch: `depth` rows of `width` DDSketch cells.
///
/// # Example
/// ```
/// use sketch_oxide::quantiles::PerFlowQuantiles;
///
/// let mut pfq = PerFlowQuantiles::new(4, 512, 0.01).unwrap();
/// for v in 0..1000u64 { pfq.update(&"flow-a", v as f64); }     // a: latencies 0..1000
/// for v in 2000..3000u64 { pfq.update(&"flow-b", v as f64); }  // b: latencies 2000..3000
///
/// assert!((pfq.quantile(&"flow-a", 0.5).unwrap() - 500.0).abs() < 80.0);
/// assert!((pfq.quantile(&"flow-b", 0.5).unwrap() - 2500.0).abs() < 80.0);
/// ```
#[derive(Debug, Clone)]
pub struct PerFlowQuantiles {
    depth: usize,
    width: usize,
    /// `depth × width` cells (row-major), each a DDSketch plus its total count.
    cells: Vec<(DDSketch, u64)>,
}

impl PerFlowQuantiles {
    /// Creates a sketch with `depth` rows of `width` cells, each a DDSketch of `relative_accuracy`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `depth` or `width` is 0; propagates DDSketch accuracy
    /// validation.
    pub fn new(depth: usize, width: usize, relative_accuracy: f64) -> Result<Self> {
        if depth == 0 {
            return Err(SketchError::InvalidParameter {
                param: "depth".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        if width == 0 {
            return Err(SketchError::InvalidParameter {
                param: "width".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        let proto = DDSketch::new(relative_accuracy)?;
        Ok(Self {
            depth,
            width,
            cells: vec![(proto, 0u64); depth * width],
        })
    }

    /// Cell index of `flow` in row `row`.
    #[inline]
    fn index<F: Hash>(&self, flow: &F, row: usize) -> usize {
        let mut hasher = DefaultHasher::new();
        flow.hash(&mut hasher);
        let seed = ROW_SEED_BASE.wrapping_add(row as u64);
        let h = xxhash(&hasher.finish().to_le_bytes(), seed);
        row * self.width + (h % self.width as u64) as usize
    }

    /// Records `value` for `flow`.
    pub fn update<F: Hash>(&mut self, flow: &F, value: f64) {
        for row in 0..self.depth {
            let idx = self.index(flow, row);
            let cell = &mut self.cells[idx];
            cell.0.add(value);
            cell.1 += 1;
        }
    }

    /// Estimated `phi`-quantile of `flow`, taken from its least-contaminated (min-count) cell.
    /// Returns `None` if `flow` has no recorded values.
    pub fn quantile<F: Hash>(&self, flow: &F, phi: f64) -> Option<f64> {
        let mut best: Option<&(DDSketch, u64)> = None;
        for row in 0..self.depth {
            let cell = &self.cells[self.index(flow, row)];
            best = match best {
                Some(b) if b.1 <= cell.1 => Some(b),
                _ => Some(cell),
            };
        }
        best.and_then(|(sketch, _)| sketch.quantile(phi))
    }

    /// Number of rows `d`.
    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Cells per row `w`.
    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }
}

// No capability-trait match: this is a keyed multiplexer — `update(flow, value)`
// and `quantile(flow, phi)` both take a per-flow key, so neither the scalar
// `Update<f64>` nor `QuantileQuery` (single-argument) applies. No serialization.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(PerFlowQuantiles::new(0, 256, 0.01).is_err());
        assert!(PerFlowQuantiles::new(4, 0, 0.01).is_err());
        assert!(PerFlowQuantiles::new(4, 256, 0.01).is_ok());
    }

    #[test]
    fn tracks_per_flow_medians() {
        let mut pfq = PerFlowQuantiles::new(4, 512, 0.01).unwrap();
        for v in 0..1000u64 {
            pfq.update(&"a", v as f64);
        }
        for v in 2000..3000u64 {
            pfq.update(&"b", v as f64);
        }
        assert!((pfq.quantile(&"a", 0.5).unwrap() - 500.0).abs() < 80.0);
        assert!((pfq.quantile(&"b", 0.5).unwrap() - 2500.0).abs() < 80.0);
    }

    #[test]
    fn per_flow_tail_quantiles_amid_many_flows() {
        let mut pfq = PerFlowQuantiles::new(5, 1024, 0.01).unwrap();
        // One flow of interest with a known distribution.
        for v in 0..10_000u64 {
            pfq.update(&"hot", v as f64);
        }
        // Thousands of light background flows.
        for f in 0..5000u64 {
            pfq.update(&f, (f % 100) as f64);
        }
        let p99 = pfq.quantile(&"hot", 0.99).unwrap();
        // True p99 of 0..10000 is ~9900; the min-count cell keeps it close.
        assert!((p99 - 9900.0).abs() < 600.0, "p99 {p99}");
        let p50 = pfq.quantile(&"hot", 0.5).unwrap();
        assert!((p50 - 5000.0).abs() < 600.0, "p50 {p50}");
    }

    #[test]
    fn unseen_flow_is_none() {
        let pfq = PerFlowQuantiles::new(4, 256, 0.01).unwrap();
        assert!(pfq.quantile(&"missing", 0.5).is_none());
    }
}
