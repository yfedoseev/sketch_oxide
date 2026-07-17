//! TRIÈST — triangle counting in graph streams with bounded memory.
//!
//! TRIÈST (Stefani, Epasto, Riondato & Upfal, "TRIÈST: Counting Local and Global Triangles
//! in Fully-Dynamic Streams with Fixed Memory Size", KDD 2016) estimates the number of
//! triangles in a graph given as a stream of edges, keeping only a reservoir of `m` edges.
//! Each arriving edge is offered to the reservoir by standard reservoir sampling; when an
//! edge enters or leaves the sample, the running triangle count is adjusted by the number of
//! triangles that edge closes with the *sampled* graph. The unbiased global estimate scales
//! the sample's triangle count by the sampling-probability correction `ξ(t)`.
//!
//! This is the TRIÈST-BASE variant: exact while the whole stream fits in the reservoir,
//! and an unbiased estimate thereafter.

use crate::common::cursor::{Framing, SketchId, WriteBuf};
use crate::common::{Result, Serializable, SketchError};
use rand::Rng;
use std::collections::{HashMap, HashSet};

/// Triangle-count estimator over an edge stream, using a reservoir of `m` edges.
///
/// Vertices are `u64` ids; edges are undirected and de-duplicated (re-adding an existing edge
/// is ignored).
///
/// # Example
/// ```
/// use sketch_oxide::graph::Triest;
///
/// // K4 has 4 triangles; a reservoir large enough to hold all 6 edges is exact.
/// let mut t = Triest::with_seed(100, 1).unwrap();
/// let edges = [(0,1),(0,2),(0,3),(1,2),(1,3),(2,3)];
/// for &(u, v) in &edges { t.add_edge(u, v); }
/// assert!((t.estimate() - 4.0).abs() < 1e-9);
/// ```
#[derive(Debug, Clone)]
pub struct Triest {
    m: usize,
    t: u64,
    /// Triangles in the current sample.
    tau: i64,
    edges: Vec<(u64, u64)>,
    edge_set: HashSet<(u64, u64)>,
    adj: HashMap<u64, HashSet<u64>>,
    rng: rand::rngs::SmallRng,
}

impl Triest {
    /// Creates an estimator with reservoir size `m`, seeded from the OS.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `m < 6` (too small to ever hold a triangle's
    /// edges plus context).
    pub fn new(m: usize) -> Result<Self> {
        use rand::SeedableRng;
        Self::from_rng(m, rand::rngs::SmallRng::from_os_rng())
    }

    /// Creates an estimator with a fixed RNG seed for reproducibility.
    pub fn with_seed(m: usize, seed: u64) -> Result<Self> {
        use rand::SeedableRng;
        Self::from_rng(m, rand::rngs::SmallRng::seed_from_u64(seed))
    }

    fn from_rng(m: usize, rng: rand::rngs::SmallRng) -> Result<Self> {
        if m < 6 {
            return Err(SketchError::InvalidParameter {
                param: "m".to_string(),
                value: m.to_string(),
                constraint: "must be >= 6".to_string(),
            });
        }
        Ok(Self {
            m,
            t: 0,
            tau: 0,
            edges: Vec::with_capacity(m),
            edge_set: HashSet::new(),
            adj: HashMap::new(),
            rng,
        })
    }

    #[inline]
    fn canon(u: u64, v: u64) -> (u64, u64) {
        if u <= v { (u, v) } else { (v, u) }
    }

    /// Common neighbours of `a` and `b` in the sampled graph.
    fn shared_neighbors(&self, a: u64, b: u64) -> usize {
        match (self.adj.get(&a), self.adj.get(&b)) {
            (Some(na), Some(nb)) => {
                let (small, big) = if na.len() <= nb.len() {
                    (na, nb)
                } else {
                    (nb, na)
                };
                small
                    .iter()
                    .filter(|&&c| c != a && c != b && big.contains(&c))
                    .count()
            }
            _ => 0,
        }
    }

    /// Adjusts `tau` by the triangles edge `(a,b)` closes with the current sample (the edge
    /// must already be in the adjacency for `+1`, still present for `-1`).
    fn update_counters(&mut self, sign: i64, a: u64, b: u64) {
        let common = self.shared_neighbors(a, b) as i64;
        self.tau += sign * common;
    }

    fn add_to_sample(&mut self, e: (u64, u64)) {
        self.adj.entry(e.0).or_default().insert(e.1);
        self.adj.entry(e.1).or_default().insert(e.0);
        self.edges.push(e);
        self.edge_set.insert(e);
    }

    fn remove_from_sample(&mut self, idx: usize) -> (u64, u64) {
        let e = self.edges.swap_remove(idx);
        self.edge_set.remove(&e);
        if let Some(s) = self.adj.get_mut(&e.0) {
            s.remove(&e.1);
        }
        if let Some(s) = self.adj.get_mut(&e.1) {
            s.remove(&e.0);
        }
        e
    }

    /// Adds an undirected edge `(u, v)` to the stream. Self-loops and duplicate edges are
    /// ignored.
    pub fn add_edge(&mut self, u: u64, v: u64) {
        if u == v {
            return;
        }
        let e = Self::canon(u, v);
        if self.edge_set.contains(&e) {
            return;
        }
        self.t += 1;

        if self.edges.len() < self.m {
            self.add_to_sample(e);
            self.update_counters(1, e.0, e.1);
        } else if self.rng.random::<f64>() < self.m as f64 / self.t as f64 {
            let victim = self.rng.random_range(0..self.edges.len());
            let (x, y) = self.edges[victim];
            self.update_counters(-1, x, y); // count before removal
            self.remove_from_sample(victim);
            self.add_to_sample(e);
            self.update_counters(1, e.0, e.1);
        }
        // else: edge not sampled; counters unchanged.
    }

    /// The sampling-probability correction `ξ(t)`.
    fn xi(&self) -> f64 {
        if self.t <= self.m as u64 {
            1.0
        } else {
            let t = self.t as f64;
            let m = self.m as f64;
            (t / m) * ((t - 1.0) / (m - 1.0)) * ((t - 2.0) / (m - 2.0))
        }
    }

    /// Unbiased estimate of the total number of triangles in the graph so far.
    pub fn estimate(&self) -> f64 {
        self.xi() * self.tau as f64
    }

    /// Number of distinct edges seen so far.
    #[inline]
    pub fn edges_seen(&self) -> u64 {
        self.t
    }

    /// Number of edges currently in the reservoir.
    #[inline]
    pub fn sample_size(&self) -> usize {
        self.edges.len()
    }
}

impl Serializable for Triest {
    /// Framed layout: `m:u64`, `t:u64`, `tau:i64`, `num_edges:u64`, then the
    /// reservoir edges as `(u:u64, v:u64)` pairs. The adjacency and edge-set
    /// are rebuilt from the reservoir on decode.
    ///
    /// The RNG stream is **not** preserved: the restored sketch draws fresh
    /// randomness. Reservoir sampling only requires future coin flips to be
    /// independent, so the estimator's guarantees are unaffected.
    fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut b = WriteBuf::with_capacity(32 + self.edges.len() * 16);
        b.write_u64_le(self.m as u64);
        b.write_u64_le(self.t);
        b.write_i64_le(self.tau);
        b.write_u64_le(self.edges.len() as u64);
        for &(u, v) in &self.edges {
            b.write_u64_le(u);
            b.write_u64_le(v);
        }
        Ok(Framing::new(SketchId::TRIEST, 1).frame(&b.into_bytes()))
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        use rand::SeedableRng;
        let (_, mut cur) = Framing::parse(bytes, SketchId::TRIEST)?;
        let m = cur.read_u64_le()? as usize;
        let t = cur.read_u64_le()?;
        let tau = cur.read_i64_le()?;
        let mut out = Self::from_rng(m, rand::rngs::SmallRng::from_os_rng())?;
        out.t = t;
        out.tau = tau;
        let num_edges = cur.read_len_prefixed_count(16)?;
        if num_edges > m {
            return Err(SketchError::DeserializationError(format!(
                "reservoir holds {num_edges} edges but capacity is {m}"
            )));
        }
        for _ in 0..num_edges {
            let u = cur.read_u64_le()?;
            let v = cur.read_u64_le()?;
            let e = Self::canon(u, v);
            if u == v || !out.edge_set.insert(e) {
                return Err(SketchError::DeserializationError(
                    "invalid reservoir edge (self-loop or duplicate)".to_string(),
                ));
            }
            out.adj.entry(e.0).or_default().insert(e.1);
            out.adj.entry(e.1).or_default().insert(e.0);
            out.edges.push(e);
        }
        if cur.remaining() != 0 {
            return Err(SketchError::DeserializationError(
                "trailing bytes after Triest payload".to_string(),
            ));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_tiny_reservoir() {
        assert!(Triest::with_seed(5, 1).is_err());
        assert!(Triest::with_seed(6, 1).is_ok());
    }

    #[test]
    fn exact_single_triangle() {
        let mut t = Triest::with_seed(100, 1).unwrap();
        t.add_edge(0, 1);
        t.add_edge(1, 2);
        t.add_edge(0, 2);
        assert!(
            (t.estimate() - 1.0).abs() < 1e-9,
            "estimate {}",
            t.estimate()
        );
    }

    #[test]
    fn exact_k4_four_triangles() {
        let mut t = Triest::with_seed(100, 7).unwrap();
        for &(u, v) in &[(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)] {
            t.add_edge(u, v);
        }
        assert!(
            (t.estimate() - 4.0).abs() < 1e-9,
            "estimate {}",
            t.estimate()
        );
    }

    #[test]
    fn ignores_duplicates_and_self_loops() {
        let mut t = Triest::with_seed(100, 1).unwrap();
        t.add_edge(0, 1);
        t.add_edge(1, 0); // duplicate (undirected)
        t.add_edge(2, 2); // self-loop
        assert_eq!(t.edges_seen(), 1);
    }

    #[test]
    fn estimates_under_sampling() {
        // Complete graph K8 has C(8,3) = 56 triangles, 28 edges. Reservoir 20 < 28 forces
        // sampling; the estimate should be in the right ballpark.
        let mut t = Triest::with_seed(20, 42).unwrap();
        for u in 0..8u64 {
            for v in (u + 1)..8u64 {
                t.add_edge(u, v);
            }
        }
        assert!(t.sample_size() <= 20);
        let est = t.estimate();
        assert!(
            est > 28.0 && est < 90.0,
            "K8 triangle estimate {est} (true 56)"
        );
    }

    #[test]
    fn no_triangles_in_a_path() {
        let mut t = Triest::with_seed(100, 1).unwrap();
        for i in 0..50u64 {
            t.add_edge(i, i + 1); // a path has no triangles
        }
        assert_eq!(t.estimate(), 0.0);
    }
}
