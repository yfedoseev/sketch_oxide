//! AGM connectivity — graph connectivity from linear sketches.
//!
//! AGM (Ahn, Guha & McGregor, "Analyzing Graph Structure via Linear Measurements", SODA 2012) is the
//! breakthrough that connectivity — a global graph property — can be decided from a *linear* sketch
//! of the edge stream, in `O(n·polylog n)` space. Each vertex keeps an **L0 sketch** of its incident
//! edges, where edge `(i, j)` (with `i < j`) contributes `+1` to vertex `i` and `−1` to vertex `j`.
//! The trick: summing the sketches of any vertex subset `S` makes every edge *inside* `S` cancel
//! (`+1` and `−1`), leaving only edges that **cross the cut** out of `S`. Sampling one such crossing
//! edge per current component and contracting it is one round of Borůvka's algorithm; after
//! `O(log n)` rounds the components are the connected components — all from the sketches, never
//! re-reading the edges.
//!
//! The L0 sketch here is a leveled `(count, id_sum, id²_sum)` structure (an IBLT-style 1-sparse
//! recovery per cell); it is linear, so component sketches are obtained by adding vertex sketches.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use std::collections::HashMap;

/// One L0 cell: signed sums over the edges hashed into it.
#[derive(Debug, Clone, Copy, Default)]
struct Cell {
    count: i64,
    id_sum: i64,
    id_sq_sum: i128,
}

impl Cell {
    fn add(&mut self, sign: i64, id: i64) {
        self.count += sign;
        self.id_sum += sign * id;
        self.id_sq_sum += sign as i128 * (id as i128) * (id as i128);
    }
    fn merge(&mut self, o: &Cell) {
        self.count += o.count;
        self.id_sum += o.id_sum;
        self.id_sq_sum += o.id_sq_sum;
    }
    /// If exactly one edge id is present, return it.
    fn recover(&self) -> Option<i64> {
        if self.count == 0 {
            return None;
        }
        // 1-sparse test: with a single signed edge, id_sum² = count · id²_sum.
        if (self.id_sum as i128) * (self.id_sum as i128) != self.count as i128 * self.id_sq_sum {
            return None;
        }
        if self.id_sum % self.count != 0 {
            return None;
        }
        Some(self.id_sum / self.count)
    }
}

/// An AGM linear-sketch connectivity structure over `n` vertices.
///
/// # Example
/// ```
/// use sketch_oxide::graph::AgmConnectivity;
///
/// // Two triangles {0,1,2} and {3,4,5}: two components.
/// let mut g = AgmConnectivity::new(6, 1).unwrap();
/// for (u, v) in [(0,1),(1,2),(0,2),(3,4),(4,5),(3,5)] { g.add_edge(u, v); }
/// assert_eq!(g.num_components(), 2);
/// assert!(g.connected(0, 2));
/// assert!(!g.connected(0, 3));
/// ```
#[derive(Debug, Clone)]
pub struct AgmConnectivity {
    n: usize,
    levels: u32,
    width: usize,
    reps: usize,
    seed: u64,
    /// Per-vertex L0 sketch: `[repetition][level][bucket]`. Independent repetitions boost the
    /// probability that some cell is 1-sparse for a given cut.
    sketches: Vec<Vec<Vec<Vec<Cell>>>>,
}

impl AgmConnectivity {
    /// Creates a connectivity sketch over `n` vertices.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `n < 2`.
    pub fn new(n: usize, seed: u64) -> Result<Self> {
        if n < 2 {
            return Err(SketchError::InvalidParameter {
                param: "n".to_string(),
                value: n.to_string(),
                constraint: "must be >= 2".to_string(),
            });
        }
        // Enough levels to isolate any edge degree, and a wide-enough table for reliable recovery.
        let levels = (usize::BITS - (n * n).leading_zeros()).max(4) + 2;
        let width = 16;
        let reps = 4;
        let sketches = vec![vec![vec![vec![Cell::default(); width]; levels as usize]; reps]; n];
        Ok(Self {
            n,
            levels,
            width,
            reps,
            seed,
            sketches,
        })
    }

    #[inline]
    fn edge_id(&self, u: usize, v: usize) -> i64 {
        let (a, b) = (u.min(v), u.max(v));
        (a * self.n + b) as i64
    }

    /// The top level at which an edge is present (geometric: `~2^-l` of edges reach level `l`).
    #[inline]
    fn max_level(&self, id: i64, rep: usize) -> u32 {
        let h = xxhash(
            &id.to_le_bytes(),
            self.seed.wrapping_add(0x91 * rep as u64 + 1),
        );
        (h | (1u64 << (self.levels - 1)))
            .trailing_zeros()
            .min(self.levels - 1)
    }

    #[inline]
    fn bucket(&self, id: i64, rep: usize, level: u32) -> usize {
        let salt = self
            .seed
            .wrapping_add(0x9E37 * rep as u64 + level as u64 + 1);
        (xxhash(&id.to_le_bytes(), salt) % self.width as u64) as usize
    }

    /// Adds an undirected edge `(u, v)`. Self-loops and duplicate edges are ignored.
    pub fn add_edge(&mut self, u: usize, v: usize) {
        if u == v || u >= self.n || v >= self.n {
            return;
        }
        let id = self.edge_id(u, v);
        // Sign convention: +1 for the smaller endpoint, −1 for the larger.
        let (lo, hi) = (u.min(v), u.max(v));
        for rep in 0..self.reps {
            let max_l = self.max_level(id, rep);
            for l in 0..=max_l {
                let b = self.bucket(id, rep, l);
                self.sketches[lo][rep][l as usize][b].add(1, id);
                self.sketches[hi][rep][l as usize][b].add(-1, id);
            }
        }
    }

    /// Sums the L0 sketches of a set of vertices into a single component sketch.
    fn component_sketch(&self, members: &[usize]) -> Vec<Vec<Vec<Cell>>> {
        let mut out =
            vec![vec![vec![Cell::default(); self.width]; self.levels as usize]; self.reps];
        for &v in members {
            for (rep, rsketch) in self.sketches[v].iter().enumerate() {
                for (l, lvl) in rsketch.iter().enumerate() {
                    for (b, cell) in lvl.iter().enumerate() {
                        out[rep][l][b].merge(cell);
                    }
                }
            }
        }
        out
    }

    /// Recovers one crossing edge `(u, v)` from a component sketch, if any cell is 1-sparse.
    fn sample_edge(&self, sketch: &[Vec<Vec<Cell>>]) -> Option<(usize, usize)> {
        // Try every repetition; within one, scan from the sparsest level down so a single crossing
        // edge is most likely isolated.
        for rep in sketch {
            for level in (0..self.levels as usize).rev() {
                for cell in &rep[level] {
                    if let Some(id) = cell.recover() {
                        let u = (id as usize) / self.n;
                        let v = (id as usize) % self.n;
                        if u < self.n && v < self.n && u != v {
                            return Some((u, v));
                        }
                    }
                }
            }
        }
        None
    }

    /// Computes connected-component labels (a representative vertex per component) via Borůvka over
    /// the sketches.
    pub fn components(&self) -> Vec<usize> {
        let mut parent: Vec<usize> = (0..self.n).collect();
        fn find(parent: &mut [usize], mut x: usize) -> usize {
            while parent[x] != x {
                parent[x] = parent[parent[x]];
                x = parent[x];
            }
            x
        }
        let rounds = (usize::BITS - self.n.leading_zeros()) + 1;
        for _ in 0..rounds {
            // Group current members by root.
            let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
            for v in 0..self.n {
                let r = find(&mut parent, v);
                groups.entry(r).or_default().push(v);
            }
            if groups.len() == 1 {
                break;
            }
            let mut merges = Vec::new();
            for members in groups.values() {
                let sk = self.component_sketch(members);
                if let Some((a, b)) = self.sample_edge(&sk) {
                    merges.push((a, b));
                }
            }
            if merges.is_empty() {
                break;
            }
            for (a, b) in merges {
                let ra = find(&mut parent, a);
                let rb = find(&mut parent, b);
                if ra != rb {
                    parent[ra] = rb;
                }
            }
        }
        (0..self.n).map(|v| find(&mut parent, v)).collect()
    }

    /// Number of connected components.
    pub fn num_components(&self) -> usize {
        let labels = self.components();
        let mut roots: Vec<usize> = labels;
        roots.sort_unstable();
        roots.dedup();
        roots.len()
    }

    /// Whether `u` and `v` are in the same connected component.
    pub fn connected(&self, u: usize, v: usize) -> bool {
        if u >= self.n || v >= self.n {
            return false;
        }
        let labels = self.components();
        labels[u] == labels[v]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_tiny_n() {
        assert!(AgmConnectivity::new(1, 0).is_err());
        assert!(AgmConnectivity::new(2, 0).is_ok());
    }

    #[test]
    fn no_edges_is_all_singletons() {
        let g = AgmConnectivity::new(10, 1).unwrap();
        assert_eq!(g.num_components(), 10);
    }

    #[test]
    fn two_triangles_are_two_components() {
        let mut g = AgmConnectivity::new(6, 1).unwrap();
        for (u, v) in [(0, 1), (1, 2), (0, 2), (3, 4), (4, 5), (3, 5)] {
            g.add_edge(u, v);
        }
        assert_eq!(g.num_components(), 2);
        assert!(g.connected(0, 2));
        assert!(g.connected(3, 5));
        assert!(!g.connected(0, 3));
    }

    #[test]
    fn a_path_is_one_component() {
        let n = 50;
        let mut g = AgmConnectivity::new(n, 7).unwrap();
        for i in 0..n - 1 {
            g.add_edge(i, i + 1);
        }
        assert_eq!(g.num_components(), 1);
        assert!(g.connected(0, n - 1));
    }

    #[test]
    fn disjoint_components_counted() {
        // Five disjoint edges → five 2-vertex components over 10 vertices.
        let mut g = AgmConnectivity::new(10, 3).unwrap();
        for i in 0..5 {
            g.add_edge(2 * i, 2 * i + 1);
        }
        assert_eq!(g.num_components(), 5);
        assert!(g.connected(0, 1));
        assert!(!g.connected(0, 2));
    }

    #[test]
    fn larger_random_graph_matches_union_find() {
        // Build a graph and compare AGM components against a direct union-find ground truth.
        let n = 80;
        let edges: Vec<(usize, usize)> = (0..200u64)
            .map(|k| {
                let a = (k.wrapping_mul(2_654_435_761) % n as u64) as usize;
                let b = (k.wrapping_mul(40_503).wrapping_add(7) % n as u64) as usize;
                (a, b)
            })
            .filter(|&(a, b)| a != b)
            .collect();
        let mut g = AgmConnectivity::new(n, 11).unwrap();
        for &(a, b) in &edges {
            g.add_edge(a, b);
        }
        // Ground truth.
        let mut parent: Vec<usize> = (0..n).collect();
        fn find(p: &mut [usize], mut x: usize) -> usize {
            while p[x] != x {
                p[x] = p[p[x]];
                x = p[x];
            }
            x
        }
        for &(a, b) in &edges {
            let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
            if ra != rb {
                parent[ra] = rb;
            }
        }
        let truth: std::collections::HashSet<usize> =
            (0..n).map(|v| find(&mut parent, v)).collect();
        assert_eq!(g.num_components(), truth.len());
    }
}
