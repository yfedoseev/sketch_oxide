//! GSS — the Graph Stream Sketch.
//!
//! GSS (Gou, Zou, Zhao & Yang, "Fast and Accurate Graph Stream Summarization", ICDE 2019) is the
//! accuracy successor to [`TcmSketch`](crate::graph::TcmSketch). TCM hashes each node to a row /
//! column and sums weights there, so two different edges landing in the same cell are
//! indistinguishable and the cell over-counts badly. GSS fixes this by storing a **fingerprint**
//! of the endpoints in every cell: an edge `(s, d)` maps to cell `(addr(s), addr(d))` and the
//! cell records `(fp(s), fp(d), weight)`. A query only credits a cell whose stored fingerprints
//! match, so unrelated edges sharing a cell no longer collide — error appears only on the rare
//! event that two distinct edges share *both* address and fingerprint.
//!
//! Each cell holds a few fingerprint slots (`room`); the small fraction of edges that find their
//! cell full spill into an overflow **buffer**, so no weight is ever lost.
//!
//! # Queries (all one-sided overestimates, never underestimates)
//!
//! - **Edge weight** `(s, d)` — the weight of the matching fingerprint slot (or buffer entry).
//! - **Out-degree** of `s` — summed weight of source-`s` fingerprints across its matrix row.
//! - **In-degree** of `d` — summed weight of dest-`d` fingerprints across its matrix column.
//!
//! # Layout note
//!
//! Edges map to the single cell `(addr(s), addr(d))` with a bucket of `room` slots plus an
//! overflow buffer — the clear, verifiable form of GSS's contract. The paper's **square hashing**
//! (mapping each node to a *range* of candidate rooms via a sequence value, to spread load and
//! shrink the buffer) is a placement optimization over this same fingerprint contract and is left
//! as a follow-up; it changes where an edge is stored, not which weight a query returns.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use std::collections::HashMap;

/// One fingerprint slot in a matrix cell.
#[derive(Debug, Clone, Copy)]
struct Slot {
    fp_s: u16,
    fp_d: u16,
    weight: u64,
}

/// The address (matrix row/column) and fingerprint of a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Addr {
    cell: u32,
    fp: u16,
}

/// A Graph Stream Sketch: an `side × side` fingerprinted matrix with `room` slots per cell and a
/// buffer for overflow.
///
/// # Example
/// ```
/// use sketch_oxide::graph::GssSketch;
///
/// let mut g = GssSketch::new(64, 4).unwrap();
/// g.add_edge(b"alice", b"bob", 3);
/// g.add_edge(b"alice", b"carol", 2);
/// assert!(g.edge_weight(b"alice", b"bob") >= 3);
/// assert_eq!(g.edge_weight(b"alice", b"dave"), 0); // never inserted
/// assert!(g.out_degree(b"alice") >= 5);            // 3 + 2 outgoing
/// ```
#[derive(Debug, Clone)]
pub struct GssSketch {
    side: usize,
    room: usize,
    /// `side × side` cells, row-major; each cell is a bucket of up to `room` slots.
    cells: Vec<Vec<Slot>>,
    /// Overflow for edges whose cell bucket was full: `(src, dst) → weight`.
    buffer: HashMap<(Addr, Addr), u64>,
}

impl GssSketch {
    /// Creates a sketch with a `side × side` matrix and `room` fingerprint slots per cell.
    ///
    /// Larger `side` lowers address collisions; larger `room` shrinks the overflow buffer.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `side` or `room` is 0.
    pub fn new(side: usize, room: usize) -> Result<Self> {
        if side == 0 || room == 0 {
            return Err(SketchError::InvalidParameter {
                param: if side == 0 { "side" } else { "room" }.to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        Ok(Self {
            side,
            room,
            cells: vec![Vec::new(); side * side],
            buffer: HashMap::new(),
        })
    }

    /// Address + fingerprint of a node. The address and fingerprint use independent hashes so a
    /// fingerprint match is real evidence of the node, not an echo of the address.
    #[inline]
    fn addr(&self, node: &[u8]) -> Addr {
        let cell = (xxhash(node, 0) % self.side as u64) as u32;
        let fp = xxhash(node, 1) as u16;
        Addr { cell, fp }
    }

    /// Adds `weight` to the edge `(s, d)`.
    pub fn add_edge(&mut self, s: &[u8], d: &[u8], weight: u64) {
        let a = self.addr(s);
        let b = self.addr(d);
        let idx = a.cell as usize * self.side + b.cell as usize;
        let cell = &mut self.cells[idx];
        for slot in cell.iter_mut() {
            if slot.fp_s == a.fp && slot.fp_d == b.fp {
                slot.weight = slot.weight.saturating_add(weight);
                return;
            }
        }
        if cell.len() < self.room {
            cell.push(Slot {
                fp_s: a.fp,
                fp_d: b.fp,
                weight,
            });
            return;
        }
        *self.buffer.entry((a, b)).or_insert(0) += weight;
    }

    /// Estimated weight of edge `(s, d)` (0 if never inserted). One-sided overestimate.
    pub fn edge_weight(&self, s: &[u8], d: &[u8]) -> u64 {
        let a = self.addr(s);
        let b = self.addr(d);
        let idx = a.cell as usize * self.side + b.cell as usize;
        for slot in &self.cells[idx] {
            if slot.fp_s == a.fp && slot.fp_d == b.fp {
                return slot.weight;
            }
        }
        self.buffer.get(&(a, b)).copied().unwrap_or(0)
    }

    /// Estimated out-degree (summed outgoing weight) of `s`. One-sided overestimate.
    pub fn out_degree(&self, s: &[u8]) -> u64 {
        let a = self.addr(s);
        let mut total = 0u64;
        let row = a.cell as usize * self.side;
        for col in 0..self.side {
            for slot in &self.cells[row + col] {
                if slot.fp_s == a.fp {
                    total = total.saturating_add(slot.weight);
                }
            }
        }
        for ((src, _), w) in &self.buffer {
            if *src == a {
                total = total.saturating_add(*w);
            }
        }
        total
    }

    /// Estimated in-degree (summed incoming weight) of `d`. One-sided overestimate.
    pub fn in_degree(&self, d: &[u8]) -> u64 {
        let b = self.addr(d);
        let mut total = 0u64;
        for row in 0..self.side {
            for slot in &self.cells[row * self.side + b.cell as usize] {
                if slot.fp_d == b.fp {
                    total = total.saturating_add(slot.weight);
                }
            }
        }
        for ((_, dst), w) in &self.buffer {
            if *dst == b {
                total = total.saturating_add(*w);
            }
        }
        total
    }

    /// Number of edges held in the overflow buffer (cells that were full). Useful for tuning
    /// `side` / `room`.
    #[inline]
    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }

    /// Matrix side length.
    #[inline]
    pub fn side(&self) -> usize {
        self.side
    }

    /// Slots per cell.
    #[inline]
    pub fn room(&self) -> usize {
        self.room
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(GssSketch::new(0, 4).is_err());
        assert!(GssSketch::new(64, 0).is_err());
        assert!(GssSketch::new(64, 4).is_ok());
    }

    #[test]
    fn edge_weight_accumulates() {
        let mut g = GssSketch::new(64, 4).unwrap();
        g.add_edge(b"a", b"b", 5);
        g.add_edge(b"a", b"b", 7);
        assert_eq!(g.edge_weight(b"a", b"b"), 12);
    }

    #[test]
    fn absent_edge_is_zero_when_no_collision() {
        let mut g = GssSketch::new(256, 4).unwrap();
        g.add_edge(b"a", b"b", 5);
        // With a large matrix, an unrelated edge almost never collides → exact zero.
        assert_eq!(g.edge_weight(b"x", b"y"), 0);
    }

    #[test]
    fn no_underestimate_on_many_edges() {
        // GSS is a one-sided sketch: queries must never be BELOW the true weight.
        let mut g = GssSketch::new(128, 4).unwrap();
        let mut truth = std::collections::HashMap::new();
        for i in 0..3000u64 {
            let s = format!("s{}", i % 200);
            let d = format!("d{}", i % 300);
            g.add_edge(s.as_bytes(), d.as_bytes(), 1);
            *truth.entry((s, d)).or_insert(0u64) += 1;
        }
        for ((s, d), w) in &truth {
            assert!(
                g.edge_weight(s.as_bytes(), d.as_bytes()) >= *w,
                "underestimate for ({s},{d}): got {} < {w}",
                g.edge_weight(s.as_bytes(), d.as_bytes())
            );
        }
    }

    #[test]
    fn degrees_are_lower_bounded_by_truth() {
        let mut g = GssSketch::new(128, 4).unwrap();
        for i in 0..20u64 {
            g.add_edge(b"hub", format!("n{i}").as_bytes(), 2);
        }
        // True out-degree of "hub" is 20 * 2 = 40; GSS never underestimates.
        assert!(
            g.out_degree(b"hub") >= 40,
            "out_degree {}",
            g.out_degree(b"hub")
        );
        // Each "n{i}" has in-degree 2 from the hub.
        assert!(g.in_degree(b"n0") >= 2);
    }

    #[test]
    fn overflow_goes_to_buffer_not_lost() {
        // Force every edge into ONE cell (side 1) with room 2 → most edges overflow to buffer,
        // and every edge weight is still recoverable.
        let mut g = GssSketch::new(1, 2).unwrap();
        for i in 0..50u64 {
            g.add_edge(format!("s{i}").as_bytes(), format!("d{i}").as_bytes(), 1);
        }
        assert!(g.buffer_len() > 0, "expected overflow into the buffer");
        // Pick an edge that overflowed and confirm its weight survived.
        let mut recovered = 0;
        for i in 0..50u64 {
            if g.edge_weight(format!("s{i}").as_bytes(), format!("d{i}").as_bytes()) >= 1 {
                recovered += 1;
            }
        }
        assert_eq!(recovered, 50, "every edge weight must be recoverable");
    }

    #[test]
    fn fingerprints_separate_edges_in_same_cell() {
        // side 1 forces all edges into the single cell; fingerprints must still keep distinct
        // edges' weights apart (this is GSS's whole advantage over TCM).
        let mut g = GssSketch::new(1, 16).unwrap();
        g.add_edge(b"alice", b"bob", 10);
        g.add_edge(b"carol", b"dave", 20);
        assert_eq!(g.edge_weight(b"alice", b"bob"), 10);
        assert_eq!(g.edge_weight(b"carol", b"dave"), 20);
    }
}
