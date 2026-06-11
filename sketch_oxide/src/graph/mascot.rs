//! MASCOT — triangle counting in graph streams via fixed-probability edge sampling.
//!
//! MASCOT (Lim & Kang, "MASCOT: Memory-efficient and Accurate Sampling for Counting Local Triangles
//! in Graph Streams", KDD 2015) estimates the number of triangles in a graph given as an edge stream.
//! Where [`Triest`](crate::graph::Triest) keeps a fixed-size reservoir, MASCOT keeps each edge
//! independently with a fixed probability `p`. For every arriving edge `(u, v)` it first counts the
//! triangles that edge closes against the **already-sampled** graph — each common neighbor `w` of `u`
//! and `v` in the sample is a triangle whose other two edges `(u,w)`, `(v,w)` were each retained with
//! probability `p` — and credits `1/p²` per such triangle. It then keeps `(u, v)` with probability
//! `p`. Because a triangle is detected only when both companion edges happen to be in the sample
//! (probability `p²`), the `1/p²` correction makes the global count an **unbiased** estimator.
//!
//! This is the MASCOT-C variant (global count). At `p = 1` it samples every edge and counts exactly.

use crate::common::{Result, SketchError};
use rand::Rng;
use std::collections::{HashMap, HashSet};

/// Triangle-count estimator over an edge stream, sampling each edge with probability `p`.
///
/// Vertices are `u64` ids; edges are undirected, de-duplicated (re-adding an existing edge is
/// ignored), and self-loops are dropped.
///
/// # Example
/// ```
/// use sketch_oxide::graph::Mascot;
///
/// // K4 has 4 triangles; with p = 1 every edge is sampled, so counting is exact.
/// let mut m = Mascot::with_seed(1.0, 1).unwrap();
/// let edges = [(0u64, 1u64), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
/// for &(u, v) in &edges { m.add_edge(u, v); }
/// assert!((m.estimate() - 4.0).abs() < 1e-9);
/// ```
#[derive(Debug, Clone)]
pub struct Mascot {
    /// Edge-sampling probability `p`.
    p: f64,
    /// Running unbiased global triangle-count estimate.
    estimate: f64,
    /// Adjacency of the *sampled* subgraph.
    adj: HashMap<u64, HashSet<u64>>,
    /// Edges already observed (for de-duplication), normalized so `u < v`.
    seen: HashSet<(u64, u64)>,
    rng: rand::rngs::SmallRng,
}

impl Mascot {
    /// Creates an estimator with edge-sampling probability `p`, seeded from the OS.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `p` is not in `(0, 1]`.
    pub fn new(p: f64) -> Result<Self> {
        use rand::SeedableRng;
        Self::from_rng(p, rand::rngs::SmallRng::from_os_rng())
    }

    /// Creates an estimator with a fixed RNG seed (reproducible).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `p` is not in `(0, 1]`.
    pub fn with_seed(p: f64, seed: u64) -> Result<Self> {
        use rand::SeedableRng;
        Self::from_rng(p, rand::rngs::SmallRng::seed_from_u64(seed))
    }

    fn from_rng(p: f64, rng: rand::rngs::SmallRng) -> Result<Self> {
        if !(p > 0.0 && p <= 1.0) {
            return Err(SketchError::InvalidParameter {
                param: "p".to_string(),
                value: p.to_string(),
                constraint: "must be in (0, 1]".to_string(),
            });
        }
        Ok(Self {
            p,
            estimate: 0.0,
            adj: HashMap::new(),
            seen: HashSet::new(),
            rng,
        })
    }

    /// Number of common neighbors of `u` and `v` in the sampled subgraph.
    fn common_neighbors(&self, u: u64, v: u64) -> usize {
        match (self.adj.get(&u), self.adj.get(&v)) {
            (Some(nu), Some(nv)) => {
                // Iterate the smaller set for efficiency.
                let (small, large) = if nu.len() <= nv.len() {
                    (nu, nv)
                } else {
                    (nv, nu)
                };
                small.iter().filter(|w| large.contains(w)).count()
            }
            _ => 0,
        }
    }

    /// Processes one undirected edge `(u, v)` from the stream.
    ///
    /// Self-loops and duplicate edges are ignored.
    pub fn add_edge(&mut self, u: u64, v: u64) {
        if u == v {
            return;
        }
        let key = if u < v { (u, v) } else { (v, u) };
        if !self.seen.insert(key) {
            return; // duplicate
        }
        // Count triangles this edge closes against the sampled graph, with the 1/p² correction.
        let closed = self.common_neighbors(u, v);
        if closed > 0 {
            self.estimate += closed as f64 / (self.p * self.p);
        }
        // Keep the edge with probability p.
        if self.rng.random::<f64>() < self.p {
            self.adj.entry(u).or_default().insert(v);
            self.adj.entry(v).or_default().insert(u);
        }
    }

    /// Current unbiased estimate of the global triangle count.
    #[inline]
    pub fn estimate(&self) -> f64 {
        self.estimate
    }

    /// Edge-sampling probability `p`.
    #[inline]
    pub fn probability(&self) -> f64 {
        self.p
    }

    /// Number of edges currently retained in the sample.
    pub fn sampled_edges(&self) -> usize {
        self.adj.values().map(|s| s.len()).sum::<usize>() / 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Adds every edge of the complete graph on `n` vertices.
    fn add_clique(m: &mut Mascot, n: u64) {
        for a in 0..n {
            for b in (a + 1)..n {
                m.add_edge(a, b);
            }
        }
    }

    #[test]
    fn rejects_bad_probability() {
        assert!(Mascot::new(0.0).is_err());
        assert!(Mascot::new(-0.1).is_err());
        assert!(Mascot::new(1.5).is_err());
        assert!(Mascot::new(1.0).is_ok());
        assert!(Mascot::new(0.5).is_ok());
    }

    #[test]
    fn exact_at_p_one() {
        // p = 1 ⇒ every edge sampled ⇒ exact counting.
        let mut m = Mascot::with_seed(1.0, 1).unwrap();
        add_clique(&mut m, 5); // K5 has C(5,3) = 10 triangles
        assert!((m.estimate() - 10.0).abs() < 1e-9, "got {}", m.estimate());

        let mut m2 = Mascot::with_seed(1.0, 2).unwrap();
        add_clique(&mut m2, 7); // K7 has C(7,3) = 35 triangles
        assert!((m2.estimate() - 35.0).abs() < 1e-9, "got {}", m2.estimate());
    }

    #[test]
    fn triangle_free_is_zero() {
        // A star graph has no triangles, regardless of sampling.
        let mut m = Mascot::with_seed(0.5, 3).unwrap();
        for leaf in 1..50u64 {
            m.add_edge(0, leaf);
        }
        assert_eq!(m.estimate(), 0.0);
    }

    #[test]
    fn duplicates_and_self_loops_ignored() {
        let mut m = Mascot::with_seed(1.0, 4).unwrap();
        m.add_edge(0, 0); // self-loop
        m.add_edge(0, 1);
        m.add_edge(1, 0); // duplicate (undirected)
        m.add_edge(0, 2);
        m.add_edge(1, 2);
        // Triangle {0,1,2} counted exactly once.
        assert!((m.estimate() - 1.0).abs() < 1e-9, "got {}", m.estimate());
    }

    #[test]
    fn approximate_on_large_clique() {
        // K20 has C(20,3) = 1140 triangles. With p = 0.6 and many triangles the relative error is
        // small; assert within 30% for a fixed seed.
        let truth = 1140.0;
        let mut m = Mascot::with_seed(0.6, 12345).unwrap();
        add_clique(&mut m, 20);
        let est = m.estimate();
        assert!(
            (est - truth).abs() < 0.30 * truth,
            "estimate {est}, truth {truth}"
        );
    }

    #[test]
    fn averages_to_truth_over_many_seeds() {
        // The estimator is unbiased: averaging over independent runs converges to the truth.
        let truth = 120.0; // K10 has C(10,3) = 120 triangles
        let runs = 200;
        let mut sum = 0.0;
        for seed in 0..runs {
            let mut m = Mascot::with_seed(0.5, seed).unwrap();
            add_clique(&mut m, 10);
            sum += m.estimate();
        }
        let mean = sum / runs as f64;
        assert!(
            (mean - truth).abs() < 0.10 * truth,
            "mean {mean}, truth {truth}"
        );
    }
}
