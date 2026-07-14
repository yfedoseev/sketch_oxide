//! JoinSketch — accurate, unbiased inner-product (join-size) estimation (Wang et al., SIGMOD 2023).
//!
//! The **inner product** `J = Σ_e f(e)·g(e)` of two streams' frequency vectors is exactly the size of
//! their equi-join, and underlies cosine similarity and query-optimizer cardinality estimates.
//! Classic AGMS / Fast-AGMS sketches estimate it with a single noisy structure, but collisions
//! between *frequent* items inject large variance. JoinSketch's insight: **separate items by
//! frequency** so the heavy hitters — which dominate `J` — are recorded exactly, and only the long
//! tail goes through the noisy sketch. It is ~10× more accurate than Fast-AGMS in the paper.
//!
//! Three components (paper §3.2):
//! - **Frequent part (FP):** a hash table of `k` buckets × `c` entries holding `(key, count)` for
//!   items whose frequency has passed a threshold `T` — recorded exactly.
//! - **Medium part (MP):** an array of `l` buckets × `m` entries holding `(key, count)`; items live
//!   here until they either cross `T` (promoted to FP) or are evicted as the bucket fills.
//! - **Infrequent part (IFP):** a **Fast-AGMS** sketch (`d` rows × `w` signed counters, each row with
//!   a hash `h_i` and a `±1` sign `ξ_i`) absorbing evicted light items — unbiased by construction.
//!
//! Insertion routes an item to FP if already there; otherwise into its MP bucket, where it is
//! incremented (and **promoted** to FP on crossing `T`), placed in a free entry, or — if the bucket is
//! full — triggers eviction of the bucket's **smallest** item into the IFP. A frequency lookup adds
//! the keyed count (FP or MP) to the IFP estimate (median of `ξ_i·IFP[i][h_i]`), since some instances
//! may have been evicted before the item grew heavy. The inner product of two JoinSketches sums the
//! **nine cross-pieces** (FP/MP/IFP × FP/MP/IFP): keyed×keyed and keyed×IFP terms by exact key match,
//! and IFP×IFP by the Fast-AGMS inner product (median over rows).
//!
//! # Implementation note
//!
//! When the FP is full on a promotion, this reference evicts the FP bucket's smallest item into the
//! IFP (still unbiased), in place of the paper's dynamic FP doubling (Optimization 3.4.1).

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

const SEED_FP: u64 = 0x101A_0000_0000_0001;
const SEED_MP: u64 = 0x101A_0000_0000_0002;
const SEED_IFP_H: u64 = 0x101A_0000_0000_0003;
const SEED_IFP_XI: u64 = 0x101A_0000_0000_0004;
const ROW_STRIDE: u64 = 0x9E37_79B9_7F4A_7C15;

/// A JoinSketch for one stream. Two JoinSketches with identical parameters can be joined for an
/// inner-product (join-size) estimate.
///
/// # Example
/// ```
/// use sketch_oxide::statistics::JoinSketch;
///
/// let mut f = JoinSketch::new(64, 4, 128, 4, 5, 512, 100).unwrap();
/// let mut g = JoinSketch::new(64, 4, 128, 4, 5, 512, 100).unwrap();
/// // Overlapping heavy item plus disjoint tails.
/// for _ in 0..1000 { f.insert(b"shared"); }
/// for _ in 0..800 { g.insert(b"shared"); }
/// for i in 0..2000u32 { f.insert(&i.to_le_bytes()); }
/// for i in 5000..7000u32 { g.insert(&i.to_le_bytes()); }
///
/// // True inner product is dominated by 1000·800 = 800_000 from "shared".
/// let j = f.inner_product(&g).unwrap();
/// assert!((j - 800_000).abs() < 80_000, "estimate {j}");
/// ```
#[derive(Debug, Clone)]
pub struct JoinSketch {
    fp_buckets: usize,
    fp_entries: usize,
    mp_buckets: usize,
    mp_entries: usize,
    ifp_depth: usize,
    ifp_width: usize,
    threshold: u64,
    fp: Vec<Vec<(Vec<u8>, u64)>>,
    mp: Vec<Vec<(Vec<u8>, u64)>>,
    ifp: Vec<i64>,
}

impl JoinSketch {
    /// Creates a JoinSketch with frequent part `fp_buckets × fp_entries`, medium part
    /// `mp_buckets × mp_entries`, infrequent Fast-AGMS sketch `ifp_depth × ifp_width`, and promotion
    /// threshold `threshold`. All sizes must be `≥ 1`.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if any argument is 0.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        fp_buckets: usize,
        fp_entries: usize,
        mp_buckets: usize,
        mp_entries: usize,
        ifp_depth: usize,
        ifp_width: usize,
        threshold: u64,
    ) -> Result<Self> {
        let check = |v: usize, name: &str| {
            if v == 0 {
                Err(SketchError::InvalidParameter {
                    param: name.to_string(),
                    value: "0".to_string(),
                    constraint: "must be >= 1".to_string(),
                })
            } else {
                Ok(())
            }
        };
        check(fp_buckets, "fp_buckets")?;
        check(fp_entries, "fp_entries")?;
        check(mp_buckets, "mp_buckets")?;
        check(mp_entries, "mp_entries")?;
        check(ifp_depth, "ifp_depth")?;
        check(ifp_width, "ifp_width")?;
        if threshold == 0 {
            return Err(SketchError::InvalidParameter {
                param: "threshold".to_string(),
                value: "0".to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        Ok(Self {
            fp_buckets,
            fp_entries,
            mp_buckets,
            mp_entries,
            ifp_depth,
            ifp_width,
            threshold,
            fp: vec![Vec::new(); fp_buckets],
            mp: vec![Vec::new(); mp_buckets],
            ifp: vec![0; ifp_depth * ifp_width],
        })
    }

    fn fp_bucket(&self, item: &[u8]) -> usize {
        (xxhash(item, SEED_FP) % self.fp_buckets as u64) as usize
    }
    fn mp_bucket(&self, item: &[u8]) -> usize {
        (xxhash(item, SEED_MP) % self.mp_buckets as u64) as usize
    }
    fn ifp_col(&self, item: &[u8], row: usize) -> usize {
        let seed = SEED_IFP_H.wrapping_add((row as u64).wrapping_mul(ROW_STRIDE));
        (xxhash(item, seed) % self.ifp_width as u64) as usize
    }
    fn ifp_sign(&self, item: &[u8], row: usize) -> i64 {
        let seed = SEED_IFP_XI.wrapping_add((row as u64).wrapping_mul(ROW_STRIDE));
        if xxhash(item, seed) & 1 == 0 { 1 } else { -1 }
    }

    /// Adds an item with weight `weight` into the infrequent Fast-AGMS sketch.
    fn ifp_add(&mut self, item: &[u8], weight: i64) {
        for r in 0..self.ifp_depth {
            let idx = r * self.ifp_width + self.ifp_col(item, r);
            self.ifp[idx] += self.ifp_sign(item, r) * weight;
        }
    }

    /// Fast-AGMS frequency estimate for `item` (median over rows of `ξ_i · IFP[i][h_i]`).
    fn ifp_estimate(&self, item: &[u8]) -> i64 {
        let mut vals: Vec<i64> = (0..self.ifp_depth)
            .map(|r| {
                let idx = r * self.ifp_width + self.ifp_col(item, r);
                self.ifp_sign(item, r) * self.ifp[idx]
            })
            .collect();
        vals.sort_unstable();
        vals[vals.len() / 2]
    }

    /// Inserts a frequent-part entry, evicting the bucket's smallest item to the IFP if it is full.
    fn fp_insert(&mut self, item: &[u8], count: u64) {
        let b = self.fp_bucket(item);
        if self.fp[b].len() < self.fp_entries {
            self.fp[b].push((item.to_vec(), count));
            return;
        }
        // Full: evict the smallest entry to the infrequent part, then take its slot.
        let min_idx = self.fp[b]
            .iter()
            .enumerate()
            .min_by_key(|(_, (_, c))| *c)
            .map(|(i, _)| i)
            .unwrap();
        let (ekey, ecnt) = self.fp[b][min_idx].clone();
        self.ifp_add(&ekey, ecnt as i64);
        self.fp[b][min_idx] = (item.to_vec(), count);
    }

    /// Inserts one occurrence of `item` (paper Algorithm 1).
    pub fn insert(&mut self, item: &[u8]) {
        // 1) Already a frequent item: increment exactly.
        let fb = self.fp_bucket(item);
        if let Some(entry) = self.fp[fb].iter_mut().find(|(k, _)| k.as_slice() == item) {
            entry.1 += 1;
            return;
        }
        // 2) Medium part.
        let mb = self.mp_bucket(item);
        if let Some(pos) = self.mp[mb].iter().position(|(k, _)| k.as_slice() == item) {
            // Case 1: present — increment, promote on crossing the threshold.
            self.mp[mb][pos].1 += 1;
            if self.mp[mb][pos].1 >= self.threshold {
                let (k, c) = self.mp[mb].remove(pos);
                self.fp_insert(&k, c);
            }
            return;
        }
        if self.mp[mb].len() < self.mp_entries {
            // Case 2: free entry.
            self.mp[mb].push((item.to_vec(), 1));
            return;
        }
        // Case 3: full — evict the smallest item to the IFP and take its slot.
        let min_idx = self.mp[mb]
            .iter()
            .enumerate()
            .min_by_key(|(_, (_, c))| *c)
            .map(|(i, _)| i)
            .unwrap();
        let (ykey, ycnt) = self.mp[mb][min_idx].clone();
        self.ifp_add(&ykey, ycnt as i64);
        self.mp[mb][min_idx] = (item.to_vec(), 1);
    }

    /// Keyed count of `item` in the frequent or medium part (0 if it is not stored there).
    fn keyed_count(&self, item: &[u8]) -> i64 {
        if let Some((_, c)) = self.fp[self.fp_bucket(item)]
            .iter()
            .find(|(k, _)| k.as_slice() == item)
        {
            return *c as i64;
        }
        if let Some((_, c)) = self.mp[self.mp_bucket(item)]
            .iter()
            .find(|(k, _)| k.as_slice() == item)
        {
            return *c as i64;
        }
        0
    }

    /// Iterates every keyed `(item, count)` stored in the frequent and medium parts.
    fn keyed_items(&self) -> impl Iterator<Item = (&[u8], i64)> {
        self.fp
            .iter()
            .chain(self.mp.iter())
            .flat_map(|bucket| bucket.iter().map(|(k, c)| (k.as_slice(), *c as i64)))
    }

    /// Estimated frequency of `item` (Algorithm 2): keyed count plus the Fast-AGMS estimate. Always
    /// queries the IFP, since some early instances may have been evicted before `item` grew heavy.
    pub fn estimate(&self, item: &[u8]) -> i64 {
        (self.keyed_count(item) + self.ifp_estimate(item)).max(0)
    }

    /// Estimates the inner product `Σ_e f(e)·g(e)` of this stream and `other` — equivalently the size
    /// of their equi-join. Both sketches must share the same parameters.
    ///
    /// # Errors
    /// [`SketchError::IncompatibleSketches`] if the two sketches' shapes differ.
    pub fn inner_product(&self, other: &Self) -> Result<i64> {
        if self.fp_buckets != other.fp_buckets
            || self.mp_buckets != other.mp_buckets
            || self.ifp_depth != other.ifp_depth
            || self.ifp_width != other.ifp_width
        {
            return Err(SketchError::IncompatibleSketches {
                reason: "JoinSketches must share the same dimensions".to_string(),
            });
        }
        let mut j: i64 = 0;
        // Pieces (1)(2)(4)(5) keyed×keyed and (3)(6) keyed_F × IFP_G.
        for (e, fc) in self.keyed_items() {
            j += fc * other.keyed_count(e);
            j += fc * other.ifp_estimate(e);
        }
        // Pieces (7)(8): IFP_F × keyed_G.
        for (e, gc) in other.keyed_items() {
            j += gc * self.ifp_estimate(e);
        }
        // Piece (9): IFP × IFP (Fast-AGMS inner product, median over rows).
        let mut rows: Vec<i64> = (0..self.ifp_depth)
            .map(|r| {
                let base = r * self.ifp_width;
                (0..self.ifp_width)
                    .map(|c| self.ifp[base + c] * other.ifp[base + c])
                    .sum()
            })
            .collect();
        rows.sort_unstable();
        j += rows[rows.len() / 2];
        Ok(j.max(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(JoinSketch::new(0, 4, 128, 4, 5, 512, 100).is_err());
        assert!(JoinSketch::new(64, 4, 128, 4, 5, 512, 0).is_err());
        assert!(JoinSketch::new(64, 4, 128, 4, 5, 512, 100).is_ok());
    }

    #[test]
    fn empty_estimates_zero() {
        let s = JoinSketch::new(64, 4, 128, 4, 5, 512, 100).unwrap();
        assert_eq!(s.estimate(b"x"), 0);
        let g = JoinSketch::new(64, 4, 128, 4, 5, 512, 100).unwrap();
        assert_eq!(s.inner_product(&g).unwrap(), 0);
    }

    #[test]
    fn heavy_item_frequency_is_exact() {
        let mut s = JoinSketch::new(64, 4, 128, 4, 5, 1024, 100).unwrap();
        for _ in 0..5000 {
            s.insert(b"heavy");
        }
        for i in 0..3000u32 {
            s.insert(&i.to_le_bytes());
        }
        // The heavy item lives in the frequent part: exact count plus a near-zero IFP term.
        let est = s.estimate(b"heavy");
        assert!((est - 5000).abs() < 100, "estimate {est}");
    }

    fn build(freqs: &[(u32, u64)]) -> JoinSketch {
        let mut s = JoinSketch::new(128, 4, 256, 4, 7, 2048, 100).unwrap();
        for &(item, f) in freqs {
            for _ in 0..f {
                s.insert(&item.to_le_bytes());
            }
        }
        s
    }

    #[test]
    fn inner_product_is_accurate() {
        // Two Zipf-like streams over a shared key space.
        let f_freqs: Vec<(u32, u64)> = (1..=200).map(|i| (i, (2000 / i as u64).max(1))).collect();
        let g_freqs: Vec<(u32, u64)> = (1..=200).map(|i| (i, (1500 / i as u64).max(1))).collect();
        let true_j: i64 = f_freqs
            .iter()
            .zip(g_freqs.iter())
            .map(|(&(_, f), &(_, g))| (f * g) as i64)
            .sum();
        let sf = build(&f_freqs);
        let sg = build(&g_freqs);
        let est = sf.inner_product(&sg).unwrap();
        let rel = (est - true_j).abs() as f64 / true_j as f64;
        assert!(rel < 0.10, "inner product {est} vs {true_j} (rel {rel:.3})");
    }

    #[test]
    fn disjoint_streams_have_small_inner_product() {
        let f_freqs: Vec<(u32, u64)> = (1..=200).map(|i| (i, (2000 / i as u64).max(1))).collect();
        let g_freqs: Vec<(u32, u64)> = (1..=200)
            .map(|i| (i + 100_000, (2000 / i as u64).max(1)))
            .collect();
        let sf = build(&f_freqs);
        let sg = build(&g_freqs);
        let est = sf.inner_product(&sg).unwrap();
        // No shared keys ⇒ true inner product is 0; the estimate should be small relative to each
        // stream's self-join (~Σ f_i² ≈ 6.6M).
        assert!(est < 200_000, "disjoint inner product {est} too large");
    }

    #[test]
    fn rejects_mismatched_dims() {
        let a = JoinSketch::new(64, 4, 128, 4, 5, 512, 100).unwrap();
        let b = JoinSketch::new(64, 4, 128, 4, 5, 256, 100).unwrap();
        assert!(a.inner_product(&b).is_err());
    }
}
