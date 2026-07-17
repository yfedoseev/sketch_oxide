//! TCM — a graphical sketch for graph streams.
//!
//! TCM (Tang, Liu, Lin et al., "Graph Stream Summarization: From Big Bang to Big Crunch",
//! SIGMOD 2016) is, in effect, a Count-Min sketch whose key space is *edges* of a graph. It
//! keeps `depth` independent `width × width` matrices; an edge `(s, d)` is hashed to a
//! `(row, col)` cell in each matrix, where the row depends only on `s` and the column only
//! on `d`. Because that structure is preserved, the sketch answers not just edge-weight
//! queries but node-level aggregates (out-/in-degree) and is the substrate for reachability
//! and subgraph queries.
//!
//! # Queries
//!
//! - **Edge weight** `(s, d)` — `min` over layers of the hashed cell (Count-Min point query).
//! - **Out-degree** of `s` — `min` over layers of the sum of `s`'s row.
//! - **In-degree** of `d` — `min` over layers of the sum of `d`'s column.
//!
//! All are one-sided overestimates: collisions only ever add weight.

use crate::common::hash::xxhash;
use crate::common::{Mergeable, Result, Sketch, SketchError};

/// A TCM graph-stream sketch: `depth` independent `width × width` weight matrices.
///
/// # Example
/// ```
/// use sketch_oxide::graph::TcmSketch;
///
/// let mut g = TcmSketch::new(4, 256).unwrap();
/// g.add_edge(b"alice", b"bob", 3);
/// g.add_edge(b"alice", b"carol", 2);
/// assert!(g.edge_weight(b"alice", b"bob") >= 3);
/// assert!(g.out_degree(b"alice") >= 5); // 3 + 2 outgoing
/// ```
#[derive(Debug, Clone)]
pub struct TcmSketch {
    depth: usize,
    width: usize,
    /// `depth` matrices, each `width × width`, row-major: `matrices[layer*width*width + r*width + c]`.
    matrices: Vec<u64>,
}

impl TcmSketch {
    /// Creates a sketch with `depth` layers of `width × width` matrices.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `depth` or `width` is 0.
    pub fn new(depth: usize, width: usize) -> Result<Self> {
        if depth == 0 || width == 0 {
            return Err(SketchError::InvalidParameter {
                param: if depth == 0 { "depth" } else { "width" }.to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            depth,
            width,
            matrices: vec![0u64; depth * width * width],
        })
    }

    /// Number of layers.
    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Matrix side length.
    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }

    /// Row index for node `n` in layer `layer`.
    #[inline]
    fn node_index(&self, n: &[u8], layer: usize) -> usize {
        (xxhash(n, layer as u64) % self.width as u64) as usize
    }

    #[inline]
    fn cell(&self, layer: usize, row: usize, col: usize) -> usize {
        layer * self.width * self.width + row * self.width + col
    }

    /// Adds `weight` to edge `src → dst`.
    pub fn add_edge(&mut self, src: &[u8], dst: &[u8], weight: u64) {
        for layer in 0..self.depth {
            let r = self.node_index(src, layer);
            let c = self.node_index(dst, layer);
            let idx = self.cell(layer, r, c);
            self.matrices[idx] += weight;
        }
    }

    /// Estimated total weight of edge `src → dst`.
    pub fn edge_weight(&self, src: &[u8], dst: &[u8]) -> u64 {
        (0..self.depth)
            .map(|layer| {
                let r = self.node_index(src, layer);
                let c = self.node_index(dst, layer);
                self.matrices[self.cell(layer, r, c)]
            })
            .min()
            .unwrap_or(0)
    }

    /// Estimated out-degree (total outgoing weight) of node `src`.
    pub fn out_degree(&self, src: &[u8]) -> u64 {
        (0..self.depth)
            .map(|layer| {
                let r = self.node_index(src, layer);
                let base = layer * self.width * self.width + r * self.width;
                self.matrices[base..base + self.width].iter().sum()
            })
            .min()
            .unwrap_or(0)
    }

    /// Estimated in-degree (total incoming weight) of node `dst`.
    pub fn in_degree(&self, dst: &[u8]) -> u64 {
        (0..self.depth)
            .map(|layer| {
                let c = self.node_index(dst, layer);
                (0..self.width)
                    .map(|r| self.matrices[self.cell(layer, r, c)])
                    .sum()
            })
            .min()
            .unwrap_or(0)
    }

    fn check_compatible(&self, other: &Self) -> Result<()> {
        if self.depth != other.depth || self.width != other.width {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "shape mismatch: {}x{}^2 vs {}x{}^2",
                    self.depth, self.width, other.depth, other.width
                ),
            });
        }
        Ok(())
    }
}

impl Mergeable for TcmSketch {
    /// Merges another sketch (matrices add). Both must have the same shape.
    fn merge(&mut self, other: &Self) -> Result<()> {
        self.check_compatible(other)?;
        for (a, b) in self.matrices.iter_mut().zip(&other.matrices) {
            *a += *b;
        }
        Ok(())
    }
}

impl Sketch for TcmSketch {
    /// An item is a `(src, dst)` edge added with weight 1.
    type Item = (Vec<u8>, Vec<u8>);

    fn update(&mut self, item: &Self::Item) {
        self.add_edge(&item.0, &item.1, 1);
    }

    /// Total weight summarised (sum over one layer's matrix).
    fn estimate(&self) -> f64 {
        let layer_sum: u64 = self.matrices[0..self.width * self.width].iter().sum();
        layer_sum as f64
    }

    fn is_empty(&self) -> bool {
        self.matrices.iter().all(|&c| c == 0)
    }

    fn serialize(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(16 + self.matrices.len() * 8);
        b.extend_from_slice(&(self.depth as u64).to_le_bytes());
        b.extend_from_slice(&(self.width as u64).to_le_bytes());
        for &c in &self.matrices {
            b.extend_from_slice(&c.to_le_bytes());
        }
        b
    }

    fn deserialize(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 16 {
            return Err(SketchError::DeserializationError("TcmSketch header".into()));
        }
        let depth = u64::from_le_bytes(bytes[0..8].try_into().unwrap()) as usize;
        let width = u64::from_le_bytes(bytes[8..16].try_into().unwrap()) as usize;
        // `depth`/`width` are attacker-controlled (full u64 → usize). Validate the
        // byte length with checked arithmetic BEFORE constructing the sketch:
        // `Self::new` allocates `vec![0u64; depth * width * width]`, so a crafted
        // triple product would either overflow (aborting under
        // overflow-checks=true) or drive a giant allocation (OOM) before any
        // length check could reject it. `expected == bytes.len()` guarantees
        // `depth * width * width * 8 <= bytes.len()`, bounding the allocation.
        let expected = depth
            .checked_mul(width)
            .and_then(|n| n.checked_mul(width))
            .and_then(|cells| cells.checked_mul(8))
            .and_then(|n| n.checked_add(16))
            .ok_or_else(|| {
                SketchError::DeserializationError("matrix grid size overflow".to_string())
            })?;
        if bytes.len() != expected {
            return Err(SketchError::DeserializationError(format!(
                "expected {expected} bytes, got {}",
                bytes.len()
            )));
        }
        let mut sketch = Self::new(depth, width)?;
        for (i, c) in sketch.matrices.iter_mut().enumerate() {
            let off = 16 + i * 8;
            *c = u64::from_le_bytes(bytes[off..off + 8].try_into().unwrap());
        }
        Ok(sketch)
    }
}

impl crate::common::Serializable for TcmSketch {
    /// Delegates to the existing [`Sketch::serialize`] wire format.
    fn to_bytes(&self) -> Result<Vec<u8>> {
        Ok(<Self as Sketch>::serialize(self))
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        <Self as Sketch>::deserialize(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_dims() {
        assert!(TcmSketch::new(0, 16).is_err());
        assert!(TcmSketch::new(4, 0).is_err());
        assert!(TcmSketch::new(4, 16).is_ok());
    }

    #[test]
    fn deserialize_rejects_oversized_dims_without_panic() {
        // Regression: `depth`/`width` drive `vec![0u64; depth*width*width]`
        // inside `Self::new`, which previously ran BEFORE the length check.
        // Overflowing product.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&u64::MAX.to_le_bytes()); // depth
        bytes.extend_from_slice(&u64::MAX.to_le_bytes()); // width
        bytes.extend_from_slice(&[0u8; 8]);
        assert!(
            TcmSketch::deserialize(&bytes).is_err(),
            "must error, not overflow/OOM"
        );

        // Large-but-non-overflowing product with a truncated tail.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&4u64.to_le_bytes()); // depth
        bytes.extend_from_slice(&100_000u64.to_le_bytes()); // width -> 4*1e10 cells
        bytes.extend_from_slice(&[0u8; 8]);
        assert!(TcmSketch::deserialize(&bytes).is_err());

        // Truncated header.
        assert!(TcmSketch::deserialize(&[0u8; 8]).is_err());
    }

    #[test]
    fn edge_weight_never_underestimates() {
        let mut g = TcmSketch::new(4, 256).unwrap();
        g.add_edge(b"a", b"b", 5);
        g.add_edge(b"a", b"b", 3);
        assert!(g.edge_weight(b"a", b"b") >= 8);
    }

    #[test]
    fn out_and_in_degree() {
        let mut g = TcmSketch::new(5, 512).unwrap();
        g.add_edge(b"a", b"b", 3);
        g.add_edge(b"a", b"c", 2);
        g.add_edge(b"x", b"b", 4);
        assert!(g.out_degree(b"a") >= 5, "out-degree a >= 5");
        assert!(g.in_degree(b"b") >= 7, "in-degree b >= 7 (3 + 4)");
    }

    #[test]
    fn distinct_edges_recovered() {
        let mut g = TcmSketch::new(5, 512).unwrap();
        for i in 0..200u64 {
            g.add_edge(&i.to_le_bytes(), &(i + 1).to_le_bytes(), 1);
        }
        // Each distinct edge has weight 1; Count-Min may overestimate slightly.
        let w = g.edge_weight(&50u64.to_le_bytes(), &51u64.to_le_bytes());
        assert!((1..=5).contains(&w), "edge weight {w}");
    }

    #[test]
    fn merge_adds_graphs() {
        let mut a = TcmSketch::new(4, 256).unwrap();
        let mut b = TcmSketch::new(4, 256).unwrap();
        a.add_edge(b"a", b"b", 3);
        b.add_edge(b"a", b"b", 4);
        a.merge(&b).unwrap();
        assert!(a.edge_weight(b"a", b"b") >= 7);
    }

    #[test]
    fn incompatible_shapes_error() {
        let mut a = TcmSketch::new(4, 16).unwrap();
        let b = TcmSketch::new(4, 32).unwrap();
        assert!(a.merge(&b).is_err());
    }

    #[test]
    fn serde_round_trip() {
        let mut g = TcmSketch::new(3, 32).unwrap();
        g.add_edge(b"a", b"b", 9);
        g.add_edge(b"c", b"d", 4);
        let restored = TcmSketch::deserialize(&g.serialize()).unwrap();
        assert_eq!(restored.edge_weight(b"a", b"b"), g.edge_weight(b"a", b"b"));
        assert_eq!(restored.edge_weight(b"c", b"d"), g.edge_weight(b"c", b"d"));
    }
}
