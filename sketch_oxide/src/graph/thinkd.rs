//! ThinkD — triangle counting in *fully dynamic* graph streams with deletions (Shin et al., PKDD
//! 2018).
//!
//! [`Mascot`](crate::graph::Mascot) and [`Triest`](crate::graph::Triest) count triangles in
//! insertion-only streams. ThinkD ("Think before you Discard") handles **edge deletions** too, and —
//! unlike approaches that simply drop unsampled edges — it uses *every* edge to update the estimate
//! before discarding it. This is the ThinkD-fast variant (Algorithm 1): each edge is kept in the
//! sample with a fixed probability `r`, but on *every* arriving edge `(u, v)` (insertion or deletion)
//! the algorithm counts the triangles `(u, v, w)` it closes against the sampled subgraph and moves the
//! global/local estimates by `±1/r²` (`+` for insertions, `−` for deletions). Because a triangle is
//! discovered exactly when both companion edges are sampled (probability `r²`), the `1/r²` correction
//! makes the estimate **unbiased** for the current graph's triangle count. At `r = 1` it counts
//! exactly.
//!
//! The stream must be valid (an edge is deleted only while present, no duplicate insertions), as
//! assumed by the paper.

use crate::common::{Result, SketchError};
use rand::Rng;
use std::collections::{HashMap, HashSet};

/// Fully-dynamic triangle-count estimator over an edge stream, sampling each edge with probability
/// `r`.
///
/// Vertices are `u64`; edges are undirected and self-loops are ignored.
///
/// # Example
/// ```
/// use sketch_oxide::graph::ThinkD;
///
/// // r = 1 ⇒ exact counting. Build K4 (4 triangles), then delete one edge (→ 2 triangles).
/// let mut t = ThinkD::with_seed(1.0, 1).unwrap();
/// for &(u, v) in &[(0u64, 1u64), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)] {
///     t.add_edge(u, v);
/// }
/// assert!((t.global_count() - 4.0).abs() < 1e-9);
/// t.remove_edge(0, 1);
/// assert!((t.global_count() - 2.0).abs() < 1e-9);
/// ```
#[derive(Debug, Clone)]
pub struct ThinkD {
    r: f64,
    /// Global triangle-count estimate.
    global: f64,
    /// Per-node local triangle-count estimates.
    local: HashMap<u64, f64>,
    /// Adjacency of the *sampled* subgraph.
    adj: HashMap<u64, HashSet<u64>>,
    rng: rand::rngs::SmallRng,
}

impl ThinkD {
    /// Creates an estimator with edge-sampling probability `r`, seeded from the OS.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `r` is not in `(0, 1]`.
    pub fn new(r: f64) -> Result<Self> {
        use rand::SeedableRng;
        Self::from_rng(r, rand::rngs::SmallRng::from_os_rng())
    }

    /// Creates an estimator with a fixed RNG seed (reproducible).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `r` is not in `(0, 1]`.
    pub fn with_seed(r: f64, seed: u64) -> Result<Self> {
        use rand::SeedableRng;
        Self::from_rng(r, rand::rngs::SmallRng::seed_from_u64(seed))
    }

    fn from_rng(r: f64, rng: rand::rngs::SmallRng) -> Result<Self> {
        if !(r > 0.0 && r <= 1.0) {
            return Err(SketchError::InvalidParameter {
                param: "r".to_string(),
                value: r.to_string(),
                constraint: "must be in (0, 1]".to_string(),
            });
        }
        Ok(Self {
            r,
            global: 0.0,
            local: HashMap::new(),
            adj: HashMap::new(),
            rng,
        })
    }

    /// Updates the estimates for the triangles that edge `(u, v)` closes against the sampled subgraph,
    /// moving them by `sign · 1/r²` per discovered triangle.
    fn update(&mut self, u: u64, v: u64, sign: f64) {
        let common: Vec<u64> = match (self.adj.get(&u), self.adj.get(&v)) {
            (Some(nu), Some(nv)) => {
                let (small, large) = if nu.len() <= nv.len() {
                    (nu, nv)
                } else {
                    (nv, nu)
                };
                small
                    .iter()
                    .copied()
                    .filter(|w| large.contains(w))
                    .collect()
            }
            _ => Vec::new(),
        };
        if common.is_empty() {
            return;
        }
        let delta = sign / (self.r * self.r);
        for w in common {
            self.global += delta;
            *self.local.entry(u).or_insert(0.0) += delta;
            *self.local.entry(v).or_insert(0.0) += delta;
            *self.local.entry(w).or_insert(0.0) += delta;
        }
    }

    /// Processes an edge **insertion** `(u, v)`.
    pub fn add_edge(&mut self, u: u64, v: u64) {
        if u == v {
            return;
        }
        self.update(u, v, 1.0);
        // Keep the edge in the sample with probability r.
        if self.rng.random::<f64>() < self.r {
            self.adj.entry(u).or_default().insert(v);
            self.adj.entry(v).or_default().insert(u);
        }
    }

    /// Processes an edge **deletion** `(u, v)`.
    pub fn remove_edge(&mut self, u: u64, v: u64) {
        if u == v {
            return;
        }
        self.update(u, v, -1.0);
        // Remove the edge from the sample if it is there.
        if let Some(nu) = self.adj.get_mut(&u) {
            nu.remove(&v);
        }
        if let Some(nv) = self.adj.get_mut(&v) {
            nv.remove(&u);
        }
    }

    /// Current unbiased estimate of the global triangle count.
    #[inline]
    pub fn global_count(&self) -> f64 {
        self.global
    }

    /// Current estimate of the local triangle count of `node`.
    pub fn local_count(&self, node: u64) -> f64 {
        self.local.get(&node).copied().unwrap_or(0.0)
    }

    /// Edge-sampling probability `r`.
    #[inline]
    pub fn probability(&self) -> f64 {
        self.r
    }

    /// Number of edges currently retained in the sample.
    pub fn sampled_edges(&self) -> usize {
        self.adj.values().map(|s| s.len()).sum::<usize>() / 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add_clique(t: &mut ThinkD, n: u64) {
        for a in 0..n {
            for b in (a + 1)..n {
                t.add_edge(a, b);
            }
        }
    }

    #[test]
    fn rejects_bad_probability() {
        assert!(ThinkD::new(0.0).is_err());
        assert!(ThinkD::new(-0.1).is_err());
        assert!(ThinkD::new(1.5).is_err());
        assert!(ThinkD::new(1.0).is_ok());
    }

    #[test]
    fn exact_dynamic_at_r_one() {
        let mut t = ThinkD::with_seed(1.0, 1).unwrap();
        add_clique(&mut t, 5); // K5 has 10 triangles
        assert!(
            (t.global_count() - 10.0).abs() < 1e-9,
            "after build {}",
            t.global_count()
        );
        // Remove edge (0,1): kills triangles {0,1,w} for w in {2,3,4} ⇒ 3 fewer.
        t.remove_edge(0, 1);
        assert!(
            (t.global_count() - 7.0).abs() < 1e-9,
            "after delete {}",
            t.global_count()
        );
        // Re-add it: back to 10.
        t.add_edge(0, 1);
        assert!(
            (t.global_count() - 10.0).abs() < 1e-9,
            "after re-add {}",
            t.global_count()
        );
    }

    #[test]
    fn deleting_all_edges_returns_to_zero() {
        let mut t = ThinkD::with_seed(1.0, 2).unwrap();
        let edges = [(0u64, 1u64), (0, 2), (1, 2)]; // one triangle
        for &(u, v) in &edges {
            t.add_edge(u, v);
        }
        assert!((t.global_count() - 1.0).abs() < 1e-9);
        for &(u, v) in &edges {
            t.remove_edge(u, v);
        }
        assert!(
            t.global_count().abs() < 1e-9,
            "after deletions {}",
            t.global_count()
        );
    }

    #[test]
    fn local_counts_at_r_one() {
        let mut t = ThinkD::with_seed(1.0, 3).unwrap();
        add_clique(&mut t, 4); // K4: each node is in C(3,2) = 3 triangles
        for n in 0..4u64 {
            assert!(
                (t.local_count(n) - 3.0).abs() < 1e-9,
                "node {n}: {}",
                t.local_count(n)
            );
        }
    }

    #[test]
    fn triangle_free_is_zero() {
        let mut t = ThinkD::with_seed(0.5, 4).unwrap();
        for leaf in 1..30u64 {
            t.add_edge(0, leaf); // star: no triangles
        }
        assert_eq!(t.global_count(), 0.0);
    }

    #[test]
    fn unbiased_over_many_seeds_with_deletions() {
        // Average over independent runs converges to the true count, even after deletions.
        let runs = 200u64;
        let mut sum = 0.0;
        for seed in 0..runs {
            let mut t = ThinkD::with_seed(0.5, seed).unwrap();
            add_clique(&mut t, 12); // K12 has C(12,3) = 220 triangles
                                    // Delete every edge incident to node 0 (removes C(11,2) = 55 triangles ⇒ 165 remain).
            for w in 1..12u64 {
                t.remove_edge(0, w);
            }
            sum += t.global_count();
        }
        let mean = sum / runs as f64;
        assert!((mean - 165.0).abs() < 0.1 * 165.0, "mean {mean}");
    }
}
