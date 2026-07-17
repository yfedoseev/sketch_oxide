//! FLEET — butterfly (bipartite 4-cycle) estimation from a graph stream (Sanei-Mehri, Zhang, Sariyüce
//! & Tirthapura, "FLEET: Butterfly Estimation from a Bipartite Graph Stream", CIKM 2019).
//!
//! A **butterfly** is the bipartite analog of a triangle: four vertices `{a, b} ⊆ L`, `{x, y} ⊆ R` with
//! all four edges `(a,x), (a,y), (b,x), (b,y)` present (a 2×2 biclique / 4-cycle). The butterfly count
//! is a core density measure for bipartite networks (user–product, author–paper, fraud rings). Exact
//! counting over a stream needs `Ω(n²)` space, so FLEET estimates it in **bounded memory** by sampling.
//!
//! This implements **FLEET1** (adaptive sampling): keep a reservoir of at most `M` edges sampled with
//! probability `p` (initially 1). When the reservoir fills, **halve** the sampling rate (`p ← γp`,
//! default `γ = 1/2`) and **sub-sample** the reservoir, retaining each edge with probability `γ` — so
//! every edge in the reservoir is always a uniform `p`-sample. The exact number of butterflies among
//! the reservoir edges, `ξ(R)`, is maintained incrementally, and since each of a butterfly's four edges
//! is in the reservoir with probability `p`, the estimate `ξ(R)/p⁴` is **unbiased** for the true count.

use crate::common::{Result, SketchError};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::{HashMap, HashSet};

/// A FLEET (FLEET1) streaming butterfly-count estimator over a bipartite edge stream.
///
/// Edges are `(l, r)` with `l` a left-partition vertex id and `r` a right-partition vertex id.
///
/// # Example
/// ```
/// use sketch_oxide::graph::Fleet;
///
/// // A complete bipartite K(3,3) has C(3,2)·C(3,2) = 9 butterflies. With a reservoir large enough to
/// // hold every edge, FLEET reports the exact count.
/// let mut f = Fleet::new(64, 0.5, 7).unwrap();
/// for l in 0..3u64 {
///     for r in 100..103u64 {
///         f.add_edge(l, r);
///     }
/// }
/// assert_eq!(f.estimate(), 9.0);
/// ```
#[derive(Debug, Clone)]
pub struct Fleet {
    max_reservoir: usize,
    gamma: f64,
    p: f64,                            // current sampling probability
    edges: Vec<(u64, u64)>,            // sampled edges (the reservoir)
    left: HashMap<u64, HashSet<u64>>,  // l -> its right-neighbours in the reservoir
    right: HashMap<u64, HashSet<u64>>, // r -> its left-neighbours in the reservoir
    butterflies: u64,                  // exact butterfly count among reservoir edges
    rng: SmallRng,
}

impl Fleet {
    /// Creates an estimator with reservoir capacity `max_reservoir` (`≥ 4`) and resampling rate `gamma`
    /// (`0 < gamma < 1`, default `0.5`), seeded by `seed`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `max_reservoir < 4` or `gamma` is not in `(0, 1)`.
    pub fn new(max_reservoir: usize, gamma: f64, seed: u64) -> Result<Self> {
        if max_reservoir < 4 {
            return Err(SketchError::InvalidParameter {
                param: "max_reservoir".to_string(),
                value: max_reservoir.to_string(),
                constraint: "must be >= 4 (a butterfly has 4 edges)".to_string(),
            });
        }
        if !gamma.is_finite() || gamma <= 0.0 || gamma >= 1.0 {
            return Err(SketchError::InvalidParameter {
                param: "gamma".to_string(),
                value: gamma.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }
        Ok(Self {
            max_reservoir,
            gamma,
            p: 1.0,
            edges: Vec::new(),
            left: HashMap::new(),
            right: HashMap::new(),
            butterflies: 0,
            rng: SmallRng::seed_from_u64(seed),
        })
    }

    /// Number of butterflies the edge `(l, r)` would complete with the edges already in the reservoir.
    fn new_butterflies(&self, l: u64, r: u64) -> u64 {
        let (Some(r_neighbors), Some(l_neighbors)) = (self.left.get(&l), self.right.get(&r)) else {
            return 0; // l or r is new to the reservoir ⇒ no butterfly closes
        };
        // Butterflies through (l, r): pick y ∈ N(l), b ∈ N(r) with edge (b, y) present.
        let mut count = 0u64;
        for &y in r_neighbors {
            if let Some(y_left) = self.right.get(&y) {
                // common left-neighbours of r and y
                let (small, large) = if l_neighbors.len() <= y_left.len() {
                    (l_neighbors, y_left)
                } else {
                    (y_left, l_neighbors)
                };
                count += small.iter().filter(|b| large.contains(b)).count() as u64;
            }
        }
        count
    }

    /// Exact number of butterflies among the current reservoir edges (wedge counting on the left side).
    fn count_all_butterflies(&self) -> u64 {
        // For each unordered pair of right-vertices, count common left-neighbours c; butterflies share
        // C(c, 2). Accumulate c per right-pair via each left-vertex's right-neighbour pairs.
        let mut pair_count: HashMap<(u64, u64), u64> = HashMap::new();
        for neighbors in self.left.values() {
            let ns: Vec<u64> = neighbors.iter().copied().collect();
            for i in 0..ns.len() {
                for j in (i + 1)..ns.len() {
                    let key = if ns[i] < ns[j] {
                        (ns[i], ns[j])
                    } else {
                        (ns[j], ns[i])
                    };
                    *pair_count.entry(key).or_insert(0) += 1;
                }
            }
        }
        pair_count.values().map(|&c| c * (c - 1) / 2).sum()
    }

    /// Sub-samples the reservoir: retains each edge with probability `gamma`, then rebuilds the
    /// adjacency and recomputes the exact butterfly count.
    fn subsample(&mut self) {
        let gamma = self.gamma;
        let kept: Vec<(u64, u64)> = self
            .edges
            .drain(..)
            .filter(|_| self.rng.random::<f64>() < gamma)
            .collect();
        self.left.clear();
        self.right.clear();
        for &(l, r) in &kept {
            self.left.entry(l).or_default().insert(r);
            self.right.entry(r).or_default().insert(l);
        }
        self.edges = kept;
        self.butterflies = self.count_all_butterflies();
    }

    /// Processes a stream edge `(l, r)`.
    pub fn add_edge(&mut self, l: u64, r: u64) {
        while self.edges.len() >= self.max_reservoir {
            self.p *= self.gamma;
            self.subsample();
        }
        if self.rng.random::<f64>() < self.p {
            self.butterflies += self.new_butterflies(l, r);
            self.left.entry(l).or_default().insert(r);
            self.right.entry(r).or_default().insert(l);
            self.edges.push((l, r));
        }
    }

    /// Unbiased estimate of the total number of butterflies in the stream so far (`ξ(R)/p⁴`).
    pub fn estimate(&self) -> f64 {
        self.butterflies as f64 / self.p.powi(4)
    }

    /// Current number of edges held in the reservoir.
    #[inline]
    pub fn reservoir_size(&self) -> usize {
        self.edges.len()
    }

    /// Current sampling probability `p`.
    #[inline]
    pub fn sampling_probability(&self) -> f64 {
        self.p
    }
}

impl crate::common::Serializable for Fleet {
    /// Framed layout: `max_reservoir:u64`, `gamma:f64`, `p:f64`,
    /// `butterflies:u64`, `num_edges:u64` + reservoir edges `(l:u64, r:u64)`.
    /// The left/right adjacency is rebuilt from the reservoir on decode; the
    /// RNG stream is not preserved (future coin flips stay independent).
    fn to_bytes(&self) -> Result<Vec<u8>> {
        use crate::common::cursor::{Framing, SketchId, WriteBuf};
        let mut b = WriteBuf::with_capacity(40 + self.edges.len() * 16);
        b.write_u64_le(self.max_reservoir as u64);
        b.write_f64_le(self.gamma);
        b.write_f64_le(self.p);
        b.write_u64_le(self.butterflies);
        b.write_u64_le(self.edges.len() as u64);
        for &(l, r) in &self.edges {
            b.write_u64_le(l);
            b.write_u64_le(r);
        }
        Ok(Framing::new(SketchId::FLEET, 1).frame(&b.into_bytes()))
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        use crate::common::cursor::{Framing, SketchId};
        let (_, mut cur) = Framing::parse(bytes, SketchId::FLEET)?;
        let max_reservoir = cur.read_u64_le()? as usize;
        let gamma = cur.read_f64_le()?;
        let p = cur.read_f64_le()?;
        let butterflies = cur.read_u64_le()?;
        let mut out = Self::new(max_reservoir, gamma, 0)?;
        if !(p.is_finite() && p > 0.0 && p <= 1.0) {
            return Err(SketchError::DeserializationError(format!(
                "sampling probability {p} outside (0, 1]"
            )));
        }
        out.p = p;
        out.butterflies = butterflies;
        out.rng = SmallRng::from_os_rng();
        let num_edges = cur.read_len_prefixed_count(16)?;
        if num_edges > max_reservoir {
            return Err(SketchError::DeserializationError(format!(
                "reservoir holds {num_edges} edges but capacity is {max_reservoir}"
            )));
        }
        for _ in 0..num_edges {
            let l = cur.read_u64_le()?;
            let r = cur.read_u64_le()?;
            out.left.entry(l).or_default().insert(r);
            out.right.entry(r).or_default().insert(l);
            out.edges.push((l, r));
        }
        if cur.remaining() != 0 {
            return Err(SketchError::DeserializationError(
                "trailing bytes after Fleet payload".to_string(),
            ));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feeds a complete bipartite graph K(a, b) and returns the estimator.
    fn complete_bipartite(a: u64, b: u64, m: usize, seed: u64) -> Fleet {
        let mut f = Fleet::new(m, 0.5, seed).unwrap();
        for l in 0..a {
            for r in 1000..(1000 + b) {
                f.add_edge(l, r);
            }
        }
        f
    }

    #[test]
    fn rejects_bad_params() {
        assert!(Fleet::new(3, 0.5, 1).is_err());
        assert!(Fleet::new(64, 0.0, 1).is_err());
        assert!(Fleet::new(64, 1.0, 1).is_err());
        assert!(Fleet::new(64, 0.5, 1).is_ok());
    }

    #[test]
    fn exact_when_reservoir_holds_everything() {
        // Under capacity, p stays 1 and the estimate is the exact butterfly count.
        // K(2,2) = 1, K(2,3) = 3, K(3,3) = 9, K(4,3) = C(4,2)·C(3,2) = 6·3 = 18.
        assert_eq!(complete_bipartite(2, 2, 64, 1).estimate(), 1.0);
        assert_eq!(complete_bipartite(2, 3, 64, 1).estimate(), 3.0);
        assert_eq!(complete_bipartite(3, 3, 64, 1).estimate(), 9.0);
        assert_eq!(complete_bipartite(4, 3, 64, 1).estimate(), 18.0);
    }

    #[test]
    fn no_butterflies_in_a_tree() {
        // A star / path has no 4-cycle.
        let mut f = Fleet::new(64, 0.5, 1).unwrap();
        for r in 100..110u64 {
            f.add_edge(0, r); // a single left-vertex connected to many right-vertices
        }
        assert_eq!(f.estimate(), 0.0);
    }

    #[test]
    fn incremental_count_matches_exact_under_capacity() {
        // Build K(5,4) (= C(5,2)·C(4,2) = 10·6 = 60 butterflies) within capacity.
        let f = complete_bipartite(5, 4, 100, 3);
        assert_eq!(f.estimate(), 60.0);
        assert_eq!(f.sampling_probability(), 1.0);
        assert_eq!(f.reservoir_size(), 20);
    }

    #[test]
    fn unbiased_under_subsampling() {
        // K(12,12) has C(12,2)^2 = 66^2 = 4356 butterflies and 144 edges; a reservoir of 70 forces
        // sub-sampling. A single estimate is high-variance, but the mean over many seeds — FLEET is
        // unbiased — converges to the truth.
        let truth = 4356.0;
        let runs = 80u64;
        let mut sum = 0.0;
        let mut subsampled = false;
        for seed in 0..runs {
            let f = complete_bipartite(12, 12, 70, seed * 2654435761);
            sum += f.estimate();
            subsampled |= f.sampling_probability() < 1.0;
        }
        assert!(subsampled, "sub-sampling should have occurred");
        let mean = sum / runs as f64;
        assert!(
            (mean - truth).abs() < 0.25 * truth,
            "mean estimate {mean} far from {truth}"
        );
    }
}
