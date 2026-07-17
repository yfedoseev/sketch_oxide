//! DOULION — triangle counting by edge sparsification.
//!
//! DOULION (Tsourakakis, Kang, Miller & Faloutsos, "DOULION: Counting Triangles in Massive Graphs
//! with a Coin", KDD 2009) estimates the number of triangles in a huge graph by keeping each edge
//! independently with probability `p` and counting triangles only in the sparsified subgraph. Since
//! a given triangle survives exactly when all three of its edges are kept — probability `p³` — the
//! exact count on the sample, divided by `p³`, is an **unbiased** estimate of the true triangle
//! count, computed on a graph with only a `p` fraction of the edges.
//!
//! Smaller `p` means less memory and faster counting at the cost of higher variance; `p = 1`
//! degrades to exact counting.

use crate::common::cursor::{Framing, SketchId, WriteBuf};
use crate::common::hash::xxhash;
use crate::common::{Result, Serializable, SketchError};
use std::collections::{HashMap, HashSet};

/// A DOULION triangle-count estimator over a sparsified edge sample.
///
/// # Example
/// ```
/// use sketch_oxide::graph::Doulion;
///
/// // A triangle plus a dangling edge: exactly 1 triangle.
/// let mut d = Doulion::new(1.0, 1).unwrap(); // p = 1 → exact
/// for (u, v) in [(1, 2), (2, 3), (1, 3), (3, 4)] { d.add_edge(u, v); }
/// assert_eq!(d.estimate_triangles(), 1.0);
/// ```
#[derive(Debug, Clone)]
pub struct Doulion {
    p: f64,
    seed: u64,
    /// Adjacency of the kept subgraph.
    adj: HashMap<u64, HashSet<u64>>,
    /// Canonical kept edges `(min, max)`, to avoid re-rolling duplicates.
    edges: HashSet<(u64, u64)>,
}

impl Doulion {
    /// Creates an estimator keeping each edge with probability `keep_prob` (`(0, 1]`).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `keep_prob` is not in `(0, 1]`.
    pub fn new(keep_prob: f64, seed: u64) -> Result<Self> {
        if !(keep_prob > 0.0 && keep_prob <= 1.0) {
            return Err(SketchError::InvalidParameter {
                param: "keep_prob".to_string(),
                value: keep_prob.to_string(),
                constraint: "must be in (0, 1]".to_string(),
            });
        }
        Ok(Self {
            p: keep_prob,
            seed,
            adj: HashMap::new(),
            edges: HashSet::new(),
        })
    }

    /// Adds an undirected edge `(u, v)`. Self-loops and duplicate edges are ignored; each distinct
    /// edge is kept (deterministically in the edge) with probability `p`.
    pub fn add_edge(&mut self, u: u64, v: u64) {
        if u == v {
            return;
        }
        let key = (u.min(v), u.max(v));
        if self.edges.contains(&key) {
            return;
        }
        // Deterministic coin per edge: keep iff hash < p.
        let mut buf = [0u8; 16];
        buf[..8].copy_from_slice(&key.0.to_le_bytes());
        buf[8..].copy_from_slice(&key.1.to_le_bytes());
        let coin = (xxhash(&buf, self.seed) as f64) / (u64::MAX as f64);
        if coin >= self.p {
            self.edges.insert(key); // record so a repeat isn't re-rolled, but don't add to adjacency
            return;
        }
        self.edges.insert(key);
        self.adj.entry(u).or_default().insert(v);
        self.adj.entry(v).or_default().insert(u);
    }

    /// Exact triangle count of the kept subgraph.
    fn kept_triangles(&self) -> u64 {
        let mut total = 0u64;
        for (&u, nbrs) in &self.adj {
            for &v in nbrs {
                if v <= u {
                    continue; // each undirected edge once
                }
                // Common neighbors of u and v form triangles; count w > v to count each once.
                if let Some(vn) = self.adj.get(&v) {
                    let (small, large) = if nbrs.len() <= vn.len() {
                        (nbrs, vn)
                    } else {
                        (vn, nbrs)
                    };
                    for &w in small {
                        if w > v && large.contains(&w) {
                            total += 1;
                        }
                    }
                }
            }
        }
        total
    }

    /// Unbiased estimate of the true triangle count: kept triangles divided by `p³`.
    pub fn estimate_triangles(&self) -> f64 {
        self.kept_triangles() as f64 / self.p.powi(3)
    }

    /// Number of edges kept in the sample.
    #[inline]
    pub fn kept_edges(&self) -> usize {
        self.adj.values().map(|n| n.len()).sum::<usize>() / 2
    }

    /// The keep probability `p`.
    #[inline]
    pub fn keep_prob(&self) -> f64 {
        self.p
    }
}

impl Serializable for Doulion {
    /// Framed layout: `p:f64`, `seed:u64`, `num_edges:u64`, then every *seen*
    /// canonical edge as `(u:u64, v:u64)`. The per-edge coin is a deterministic
    /// hash of `(edge, seed)`, so decoding replays the stream and rebuilds the
    /// identical kept subgraph.
    fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut b = WriteBuf::with_capacity(24 + self.edges.len() * 16);
        b.write_f64_le(self.p);
        b.write_u64_le(self.seed);
        b.write_u64_le(self.edges.len() as u64);
        for &(u, v) in &self.edges {
            b.write_u64_le(u);
            b.write_u64_le(v);
        }
        Ok(Framing::new(SketchId::DOULION, 1).frame(&b.into_bytes()))
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let (_, mut cur) = Framing::parse(bytes, SketchId::DOULION)?;
        let p = cur.read_f64_le()?;
        let seed = cur.read_u64_le()?;
        let mut out = Self::new(p, seed)?;
        let num_edges = cur.read_len_prefixed_count(16)?;
        for _ in 0..num_edges {
            let u = cur.read_u64_le()?;
            let v = cur.read_u64_le()?;
            out.add_edge(u, v); // deterministic coin: rebuilds the same sample
        }
        if cur.remaining() != 0 {
            return Err(SketchError::DeserializationError(
                "trailing bytes after Doulion payload".to_string(),
            ));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Adds all edges of a clique on `n` vertices.
    fn add_clique(d: &mut Doulion, n: u64) {
        for u in 0..n {
            for v in (u + 1)..n {
                d.add_edge(u, v);
            }
        }
    }

    #[test]
    fn rejects_bad_prob() {
        assert!(Doulion::new(0.0, 1).is_err());
        assert!(Doulion::new(1.5, 1).is_err());
        assert!(Doulion::new(0.5, 1).is_ok());
        assert!(Doulion::new(1.0, 1).is_ok());
    }

    #[test]
    fn exact_at_p_one() {
        let mut d = Doulion::new(1.0, 7).unwrap();
        add_clique(&mut d, 10); // K_10 has C(10,3) = 120 triangles
        assert_eq!(d.estimate_triangles(), 120.0);
    }

    #[test]
    fn no_triangles_in_a_tree() {
        let mut d = Doulion::new(1.0, 1).unwrap();
        for v in 1..100u64 {
            d.add_edge(0, v); // a star: no triangles
        }
        assert_eq!(d.estimate_triangles(), 0.0);
    }

    #[test]
    fn duplicate_and_self_edges_ignored() {
        let mut d = Doulion::new(1.0, 1).unwrap();
        d.add_edge(1, 1); // self-loop
        d.add_edge(1, 2);
        d.add_edge(2, 1); // duplicate (canonical)
        d.add_edge(2, 3);
        d.add_edge(1, 3);
        assert_eq!(d.kept_edges(), 3);
        assert_eq!(d.estimate_triangles(), 1.0);
    }

    #[test]
    fn sparsified_estimate_is_close() {
        // K_30 has C(30,3) = 4060 triangles; estimate from a p-sample should be in the ballpark.
        let truth = 4060.0;
        let mut d = Doulion::new(0.7, 12345).unwrap();
        add_clique(&mut d, 30);
        let est = d.estimate_triangles();
        assert!(
            (est - truth).abs() < 0.35 * truth,
            "estimate {est} vs {truth}"
        );
        assert!(d.kept_edges() < 435, "kept {} of 435 edges", d.kept_edges());
    }

    #[test]
    fn averages_to_truth_over_seeds() {
        // The estimator is unbiased: averaging many independent sparsifications converges.
        let truth = 120.0; // K_10
        let mut sum = 0.0;
        let trials = 40;
        for seed in 0..trials {
            let mut d = Doulion::new(0.6, seed).unwrap();
            add_clique(&mut d, 10);
            sum += d.estimate_triangles();
        }
        let mean = sum / trials as f64;
        assert!(
            (mean - truth).abs() < 0.2 * truth,
            "mean estimate {mean} vs {truth}"
        );
    }
}
