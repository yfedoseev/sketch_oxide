//! HyperANF — approximate neighborhood function of a graph (Boldi, Rosa & Vigna, WWW 2011).
//!
//! The neighborhood function `N(t)` counts the pairs of nodes within distance `t`; its growth
//! characterizes a graph's connectivity and small-world structure (it is how Facebook measured "four
//! degrees of separation"). Computing it exactly needs an all-pairs BFS. HyperANF estimates it in near
//! linear space by giving every node a [`HyperLogLog`](crate::cardinality::HyperLogLog) of the set of
//! nodes it can reach, and growing those sets one BFS ring at a time: at round `t` each node unions in
//! its neighbors' round-`t−1` sketches, so its sketch becomes the set of nodes within distance `t`.
//! Summing the per-node cardinalities gives `N(t)`; a single node's cardinality gives its
//! ball size `|B(v, t)|`.
//!
//! Because HyperLogLog is idempotent under union, revisiting a node never double-counts it — the same
//! property that makes the iteration converge to the true reachable set.

use crate::cardinality::HyperLogLog;
use crate::common::cursor::{Framing, SketchId, WriteBuf};
use crate::common::{Mergeable, Result, Serializable, Sketch, SketchError};
use std::collections::HashMap;

/// Approximate neighborhood-function estimator over an undirected graph of `u64` vertices.
///
/// # Example
/// ```
/// use sketch_oxide::graph::HyperAnf;
///
/// // A 6-clique: every node reaches all 6 within one hop.
/// let mut anf = HyperAnf::new(12).unwrap();
/// for a in 0..6u64 {
///     for b in (a + 1)..6 {
///         anf.add_edge(a, b);
///     }
/// }
/// let nf = anf.neighborhood_function(2);
/// assert!((nf[0] - 6.0).abs() < 1.0);          // N(0) = 6 (each node reaches itself)
/// assert!((nf[1] - 36.0).abs() < 4.0);         // N(1) = 36 (all ordered pairs)
/// ```
#[derive(Debug, Clone)]
pub struct HyperAnf {
    precision: u8,
    adjacency: HashMap<u64, Vec<u64>>,
}

impl HyperAnf {
    /// Creates an estimator whose per-node sketches use HyperLogLog `precision`.
    ///
    /// # Errors
    /// Propagates [`HyperLogLog`] precision validation.
    pub fn new(precision: u8) -> Result<Self> {
        HyperLogLog::new(precision)?; // validate precision once
        Ok(Self {
            precision,
            adjacency: HashMap::new(),
        })
    }

    /// Adds an undirected edge `(u, v)`. Self-loops are ignored; duplicate edges are harmless.
    pub fn add_edge(&mut self, u: u64, v: u64) {
        if u == v {
            return;
        }
        self.adjacency.entry(u).or_default().push(v);
        self.adjacency.entry(v).or_default().push(u);
    }

    /// A fresh per-node sketch initialized with that node.
    fn singleton(&self, node: u64) -> HyperLogLog {
        let mut hll = HyperLogLog::new(self.precision).expect("precision validated");
        hll.update(&node);
        hll
    }

    /// The estimated neighborhood function `N(0..=max_distance)`, where `N(t)` is the number of
    /// ordered pairs of nodes within distance `t` (including each node with itself).
    pub fn neighborhood_function(&self, max_distance: usize) -> Vec<f64> {
        let nodes: Vec<u64> = self.adjacency.keys().copied().collect();
        let mut sketches: HashMap<u64, HyperLogLog> =
            nodes.iter().map(|&v| (v, self.singleton(v))).collect();

        let mut out = Vec::with_capacity(max_distance + 1);
        out.push(sketches.values().map(|s| s.estimate()).sum());

        for _ in 1..=max_distance {
            let mut next = sketches.clone();
            for &v in &nodes {
                if let Some(neighbors) = self.adjacency.get(&v) {
                    for &u in neighbors {
                        if let Some(us) = sketches.get(&u) {
                            let _ = next.get_mut(&v).unwrap().merge(us);
                        }
                    }
                }
            }
            sketches = next;
            out.push(sketches.values().map(|s| s.estimate()).sum());
        }
        out
    }

    /// Estimated ball size `|B(v, t)|`: the number of nodes within distance `t` of `v`.
    pub fn ball_size(&self, v: u64, t: usize) -> f64 {
        if !self.adjacency.contains_key(&v) {
            return 0.0;
        }
        let nodes: Vec<u64> = self.adjacency.keys().copied().collect();
        let mut sketches: HashMap<u64, HyperLogLog> =
            nodes.iter().map(|&n| (n, self.singleton(n))).collect();
        for _ in 0..t {
            let mut next = sketches.clone();
            for &n in &nodes {
                if let Some(neighbors) = self.adjacency.get(&n) {
                    for &u in neighbors {
                        if let Some(us) = sketches.get(&u) {
                            let _ = next.get_mut(&n).unwrap().merge(us);
                        }
                    }
                }
            }
            sketches = next;
        }
        sketches.get(&v).map_or(0.0, |s| s.estimate())
    }

    /// Number of vertices that have at least one incident edge.
    pub fn num_vertices(&self) -> usize {
        self.adjacency.len()
    }
}

impl Serializable for HyperAnf {
    /// Framed layout: `precision:u8`, `num_nodes:u64`, then per node
    /// `id:u64, degree:u64, neighbors:u64×degree`.
    fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut b = WriteBuf::new();
        b.write_u8(self.precision);
        b.write_u64_le(self.adjacency.len() as u64);
        for (&node, neighbors) in &self.adjacency {
            b.write_u64_le(node);
            b.write_u64_le(neighbors.len() as u64);
            for &n in neighbors {
                b.write_u64_le(n);
            }
        }
        Ok(Framing::new(SketchId::HYPER_ANF, 1).frame(&b.into_bytes()))
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let (_, mut cur) = Framing::parse(bytes, SketchId::HYPER_ANF)?;
        let precision = cur.read_u8()?;
        let mut anf = Self::new(precision)?;
        // Each node record is at least id + degree = 16 bytes.
        let num_nodes = cur.read_len_prefixed_count(16)?;
        for _ in 0..num_nodes {
            let node = cur.read_u64_le()?;
            let degree = cur.read_len_prefixed_count(8)?;
            let mut neighbors = Vec::with_capacity(degree);
            for _ in 0..degree {
                neighbors.push(cur.read_u64_le()?);
            }
            if anf.adjacency.insert(node, neighbors).is_some() {
                return Err(SketchError::DeserializationError(format!(
                    "duplicate node {node} in HyperAnf adjacency"
                )));
            }
        }
        if cur.remaining() != 0 {
            return Err(SketchError::DeserializationError(
                "trailing bytes after HyperAnf payload".to_string(),
            ));
        }
        Ok(anf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds an undirected path `0-1-…-(n-1)`.
    fn path(anf: &mut HyperAnf, n: u64) {
        for i in 0..n - 1 {
            anf.add_edge(i, i + 1);
        }
    }

    /// Exact neighborhood function of a path on `n` nodes at distance `t`.
    fn exact_path_nf(n: i64, t: i64) -> f64 {
        let mut total = 0i64;
        for v in 0..n {
            let lo = (v - t).max(0);
            let hi = (v + t).min(n - 1);
            total += hi - lo + 1;
        }
        total as f64
    }

    #[test]
    fn rejects_bad_precision() {
        assert!(HyperAnf::new(3).is_err());
        assert!(HyperAnf::new(12).is_ok());
    }

    #[test]
    fn path_neighborhood_function() {
        let mut anf = HyperAnf::new(12).unwrap();
        path(&mut anf, 100);
        let nf = anf.neighborhood_function(5);
        for t in 0..=5 {
            let truth = exact_path_nf(100, t as i64);
            assert!(
                (nf[t] - truth).abs() < 0.08 * truth + 2.0,
                "t={t}: est {}, truth {truth}",
                nf[t]
            );
        }
    }

    #[test]
    fn path_eventually_reaches_all_pairs() {
        let mut anf = HyperAnf::new(12).unwrap();
        path(&mut anf, 60);
        let nf = anf.neighborhood_function(80); // diameter 59 < 80
        let n2 = 60.0 * 60.0;
        assert!(
            (nf[80] - n2).abs() < 0.06 * n2,
            "reached {}, want {n2}",
            nf[80]
        );
    }

    #[test]
    fn ball_size_on_path() {
        let mut anf = HyperAnf::new(12).unwrap();
        path(&mut anf, 100);
        // The middle node reaches 2t+1 nodes within distance t (away from the ends).
        let b = anf.ball_size(50, 10);
        assert!((b - 21.0).abs() < 3.0, "ball {b}");
        assert_eq!(anf.ball_size(9999, 5), 0.0); // unknown vertex
    }

    #[test]
    fn empty_graph() {
        let anf = HyperAnf::new(12).unwrap();
        assert_eq!(anf.neighborhood_function(3), vec![0.0, 0.0, 0.0, 0.0]);
        assert_eq!(anf.num_vertices(), 0);
    }
}
