//! Proteus — a self-designing range filter (Knorr, Lemaire, Lim et al., SIGMOD 2022).
//!
//! Approximate range filters answer "does any key lie in `[lo, hi]`?" so a storage engine can skip
//! data pages that cannot contain the range. The two state-of-the-art families make opposite
//! trade-offs: **Rosetta**-style *prefix Bloom filters* hash key prefixes (great for small queries,
//! poor for large ones and sensitive to prefix-length choice), while **SuRF**-style *succinct tries*
//! encode key prefixes deterministically (rule out large empty ranges in constant time, but coarsely
//! encode sparse regions and have a key-distribution-dependent minimum size).
//!
//! **Proteus unifies both into one design space and *self-designs* its layout to the workload.** It
//! combines a **trie** over the top `t` prefix levels (which rules out large ranges) with a single
//! **prefix Bloom filter** at a chosen length `l ≥ t` (which catches queries that fall close to the
//! key set, inside the trie's non-empty regions). A query descends the trie over the prefixes
//! covering `[lo, hi]`; whenever a prefix is present, the length-`l` sub-prefixes within that region
//! are probed in the Bloom filter — a positive (or false positive) ends the query, otherwise it moves
//! to the next non-empty trie region. Proteus is free to spend its entire budget on either component,
//! becoming fully deterministic or fully probabilistic as the workload demands.
//!
//! The self-designing step (paper Algorithm 1, the *Contextual Prefix FPR* model) is given the key
//! set, a memory budget, and a sample of representative **empty** range queries, and chooses the
//! `(t, l)` that minimises the expected false-positive rate.
//!
//! # Implementation notes
//!
//! Behaviour-faithful reference layout: the trie is the **sorted set of distinct `t`-bit key
//! prefixes** (a binary search gives the same range-ruling membership as the paper's LOUDS-DS FST,
//! whose succinct bit-packing is a space optimisation over this contract); the Bloom component reuses
//! [`BloomFilter`]. The tuner selects `(t, l)` by **measuring** each candidate's false-positive rate
//! on the sample queries rather than evaluating the closed-form CPFPR model — the same selection the
//! paper validates its model against, at higher build cost. Keys are 64-bit; the SuRF/trie branch of
//! Proteus's design space beyond a single Bloom length is left as a follow-up.

use crate::common::{Result, SketchError};
use crate::membership::BloomFilter;

const KEY_BITS: u32 = 64;
/// Estimated bits per trie node (LOUDS-DS is ~2–3 bits/node); used only to apportion the budget.
const TRIE_BITS_PER_NODE: usize = 3;
/// Safety cap on Bloom probes per trie region; beyond it a region is conservatively reported present.
const REGION_PROBE_CAP: u64 = 1 << 16;

/// A self-designing range filter over `u64` keys.
///
/// # Example
/// ```
/// use sketch_oxide::range_filters::Proteus;
///
/// // Keys clustered in a sub-range of the space.
/// let keys: Vec<u64> = (0..5_000u64).map(|i| 1_000_000 + i * 37).collect();
/// // Sample of empty range queries representative of the workload (none contains a key).
/// let samples: Vec<(u64, u64)> = (0..1_000u64)
///     .map(|i| {
///         let lo = 5_000_000 + i * 911;
///         (lo, lo + 40)
///     })
///     .collect();
///
/// // 16 bits/key budget; Proteus tunes its (trie depth, Bloom length) to the workload.
/// let pf = Proteus::build(&keys, 16 * keys.len(), &samples).unwrap();
///
/// // No false negatives: every key is reported present.
/// for &k in &keys {
///     assert!(pf.range_query(k, k));
/// }
/// // Empty queries far from the key set are ruled out.
/// assert!(!pf.range_query(900_000_000, 900_001_000));
/// ```
#[derive(Debug, Clone)]
pub struct Proteus {
    trie_depth: u32,
    bloom_len: u32,
    /// Sorted, distinct `trie_depth`-bit prefixes of the key set (empty when `trie_depth == 0`).
    trie: Vec<u64>,
    bloom: BloomFilter,
}

impl Proteus {
    /// Sorted, distinct `depth`-bit prefixes of an ascending, deduplicated key slice. Empty for
    /// `depth == 0` (a single all-encompassing region).
    fn distinct_prefixes(keys_sorted: &[u64], depth: u32) -> Vec<u64> {
        if depth == 0 {
            return Vec::new();
        }
        let shift = KEY_BITS - depth;
        let mut out = Vec::new();
        let mut last: Option<u64> = None;
        for &k in keys_sorted {
            let p = k >> shift;
            if last != Some(p) {
                out.push(p);
                last = Some(p);
            }
        }
        out
    }

    /// Builds the filter for a fixed `(trie_depth, bloom_len)` and Bloom bit budget.
    fn with_parts(keys_sorted: &[u64], trie_depth: u32, bloom_len: u32, bloom_bits: usize) -> Self {
        let trie = Self::distinct_prefixes(keys_sorted, trie_depth);
        let prefixes = Self::distinct_prefixes(keys_sorted, bloom_len);
        let n = prefixes.len().max(1);
        let m = bloom_bits.max(1);
        let k = (((m as f64 / n as f64) * std::f64::consts::LN_2).round() as usize).clamp(1, 32);
        let mut bloom = BloomFilter::with_params(n, m, k);
        for &p in &prefixes {
            bloom.insert(&p.to_le_bytes());
        }
        Self {
            trie_depth,
            bloom_len,
            trie,
            bloom,
        }
    }

    /// Self-designs a Proteus filter for `keys`, a `mem_bits` budget, and a sample of representative
    /// **empty** range queries, choosing the `(trie depth, Bloom length)` with the lowest measured
    /// false-positive rate (paper Algorithm 1).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `keys` is empty, `mem_bits` is 0, or `sample_queries` is
    /// empty (the self-designing step needs a workload).
    pub fn build(keys: &[u64], mem_bits: usize, sample_queries: &[(u64, u64)]) -> Result<Self> {
        if keys.is_empty() {
            return Err(SketchError::InvalidParameter {
                param: "keys".to_string(),
                value: "0".to_string(),
                constraint: "must be non-empty".to_string(),
            });
        }
        if mem_bits == 0 {
            return Err(SketchError::InvalidParameter {
                param: "mem_bits".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        if sample_queries.is_empty() {
            return Err(SketchError::InvalidParameter {
                param: "sample_queries".to_string(),
                value: "0".to_string(),
                constraint: "must be non-empty (self-designing needs a workload)".to_string(),
            });
        }
        let mut keys_sorted = keys.to_vec();
        keys_sorted.sort_unstable();
        keys_sorted.dedup();

        let mut best: Option<(u32, u32, usize)> = None;
        let mut best_fpr = f64::INFINITY;
        for trie_depth in (0..=40u32).step_by(8) {
            let trie_nodes = Self::distinct_prefixes(&keys_sorted, trie_depth).len();
            let trie_mem = trie_nodes * TRIE_BITS_PER_NODE;
            if trie_mem >= mem_bits {
                continue; // trie alone exceeds the budget
            }
            let bloom_bits = mem_bits - trie_mem;
            let mut lengths: Vec<u32> = (trie_depth.max(1)..=KEY_BITS).step_by(8).collect();
            if lengths.last() != Some(&KEY_BITS) {
                lengths.push(KEY_BITS);
            }
            for &bloom_len in &lengths {
                let cand = Self::with_parts(&keys_sorted, trie_depth, bloom_len, bloom_bits);
                let fpr = cand.measure_fpr(sample_queries);
                if fpr < best_fpr {
                    best_fpr = fpr;
                    best = Some((trie_depth, bloom_len, bloom_bits));
                }
            }
        }
        let (trie_depth, bloom_len, bloom_bits) =
            best.expect("at least the trie_depth=0 configurations are always feasible");
        Ok(Self::with_parts(
            &keys_sorted,
            trie_depth,
            bloom_len,
            bloom_bits,
        ))
    }

    /// The selected trie depth (top prefix levels encoded deterministically).
    #[inline]
    pub fn trie_depth(&self) -> u32 {
        self.trie_depth
    }

    /// The selected Bloom-filter prefix length.
    #[inline]
    pub fn bloom_len(&self) -> u32 {
        self.bloom_len
    }

    /// Probes the Bloom filter for any length-`bloom_len` prefix overlapping `[qlo, qhi]`.
    fn bloom_region(&self, qlo: u64, qhi: u64) -> bool {
        let shift = KEY_BITS - self.bloom_len; // bloom_len ≥ 1
        let lo = qlo >> shift;
        let hi = qhi >> shift;
        if hi - lo >= REGION_PROBE_CAP {
            return true; // too many sub-prefixes to enumerate: conservatively report present
        }
        (lo..=hi).any(|p| self.bloom.contains(&p.to_le_bytes()))
    }

    /// Returns `true` if a key may lie in `[lo, hi]` (no false negatives), `false` if the range is
    /// certainly empty.
    pub fn range_query(&self, lo: u64, hi: u64) -> bool {
        if lo > hi {
            return false;
        }
        if self.trie_depth == 0 {
            // No trie: the whole query range is one region handled by the Bloom filter.
            return self.bloom_region(lo, hi);
        }
        let shift = KEY_BITS - self.trie_depth; // trie_depth ≥ 1 here
        let lo_p = lo >> shift;
        let hi_p = hi >> shift;
        // Present trie prefixes overlapping [lo_p, hi_p].
        let start = self.trie.partition_point(|&p| p < lo_p);
        for &p in &self.trie[start..] {
            if p > hi_p {
                break;
            }
            // The portion of [lo, hi] inside region p.
            let region_lo = p << shift;
            let region_hi = region_lo | ((1u64 << shift) - 1);
            let qlo = lo.max(region_lo);
            let qhi = hi.min(region_hi);
            if self.bloom_region(qlo, qhi) {
                return true;
            }
        }
        false
    }

    /// Fraction of the (empty) sample queries that return a positive — the measured FPR.
    fn measure_fpr(&self, samples: &[(u64, u64)]) -> f64 {
        let pos = samples
            .iter()
            .filter(|&&(lo, hi)| self.range_query(lo, hi))
            .count();
        pos as f64 / samples.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic linear-congruential pseudo-random sequence (reproducible tests, no rng dep).
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            self.0
        }
    }

    fn workload() -> (Vec<u64>, Vec<(u64, u64)>, Vec<(u64, u64)>) {
        let mut rng = Lcg(0x1234_5678);
        // 5000 keys spread across a 40-bit space.
        let mut keys: Vec<u64> = (0..5_000)
            .map(|_| rng.next() & ((1u64 << 40) - 1))
            .collect();
        keys.sort_unstable();
        keys.dedup();
        let key_set: std::collections::HashSet<u64> = keys.iter().copied().collect();
        // Empty range queries (width 64) that contain no key.
        let mut make_empty = |rng: &mut Lcg, count: usize| -> Vec<(u64, u64)> {
            let mut out = Vec::new();
            while out.len() < count {
                let lo = rng.next() & ((1u64 << 40) - 1);
                let hi = lo + 64;
                if (lo..=hi).all(|x| !key_set.contains(&x)) {
                    out.push((lo, hi));
                }
            }
            out
        };
        let samples = make_empty(&mut rng, 1_000);
        let test = make_empty(&mut rng, 2_000);
        (keys, samples, test)
    }

    #[test]
    fn rejects_bad_params() {
        let s = vec![(10u64, 20u64)];
        assert!(Proteus::build(&[], 1000, &s).is_err());
        assert!(Proteus::build(&[1, 2, 3], 0, &s).is_err());
        assert!(Proteus::build(&[1, 2, 3], 1000, &[]).is_err());
        assert!(Proteus::build(&[1, 2, 3], 1000, &s).is_ok());
    }

    #[test]
    fn no_false_negatives() {
        let (keys, samples, _) = workload();
        let pf = Proteus::build(&keys, 16 * keys.len(), &samples).unwrap();
        // Every key must be reported present, both as a point and inside a containing range.
        for &k in &keys {
            assert!(pf.range_query(k, k), "false negative at point {k}");
            assert!(
                pf.range_query(k.saturating_sub(10), k + 10),
                "false negative around {k}"
            );
        }
    }

    #[test]
    fn low_fpr_on_held_out_queries() {
        let (keys, samples, test) = workload();
        let pf = Proteus::build(&keys, 16 * keys.len(), &samples).unwrap();
        let positives = test
            .iter()
            .filter(|&&(lo, hi)| pf.range_query(lo, hi))
            .count();
        let fpr = positives as f64 / test.len() as f64;
        assert!(
            fpr < 0.1,
            "held-out FPR {fpr} too high (t={} l={})",
            pf.trie_depth(),
            pf.bloom_len()
        );
    }

    #[test]
    fn rules_out_far_ranges() {
        let (keys, samples, _) = workload();
        let pf = Proteus::build(&keys, 16 * keys.len(), &samples).unwrap();
        // Queries well outside the 40-bit key space. When the tuner keeps a trie deep enough to
        // separate this region they are ruled out exactly; otherwise they fall through to the Bloom
        // filter and may false-positive at its (low) rate — so we require a low FPR, not zero.
        let positives = (0..1_000u64)
            .filter(|i| {
                let lo = (1u64 << 50) + i * 7919;
                pf.range_query(lo, lo + 100)
            })
            .count();
        let fpr = positives as f64 / 1_000.0;
        assert!(
            fpr < 0.05,
            "far-range FPR {fpr} too high (t={} l={})",
            pf.trie_depth(),
            pf.bloom_len()
        );
    }

    #[test]
    fn larger_budget_does_not_increase_fpr() {
        let (keys, samples, test) = workload();
        let measure = |bits_per_key: usize| {
            let pf = Proteus::build(&keys, bits_per_key * keys.len(), &samples).unwrap();
            let p = test
                .iter()
                .filter(|&&(lo, hi)| pf.range_query(lo, hi))
                .count();
            p as f64 / test.len() as f64
        };
        let small = measure(8);
        let large = measure(20);
        assert!(
            large <= small + 0.02,
            "more memory worsened FPR: {small} -> {large}"
        );
    }
}
