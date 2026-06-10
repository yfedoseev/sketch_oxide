//! L0 sampling — a uniform sample from the support of a turnstile stream.
//!
//! Reservoir sampling handles insert-only streams; an **L0 sampler** (Cormode & Firmani,
//! "A unifying framework for ℓ0-sampling algorithms", 2014) returns a near-uniform random
//! element from the *set of items with non-zero net weight* under a fully dynamic
//! (insertions **and deletions**) stream — the canonical deletion-capable sampler and the
//! workhorse primitive behind graph sketching (connectivity, spanning forests) and dynamic
//! duplicate detection.
//!
//! # How it works
//!
//! Items are sub-sampled into geometric levels: an item is present at level `j` iff its hash
//! has at least `j` trailing zeros (so level `j` keeps roughly a `2^-j` fraction). Each level
//! is a **1-sparse recovery** structure — running sums `(Σw, Σ key·w, Σ w·H(key))` — that can
//! exactly recover the single surviving item once a level holds just one, and a fingerprint
//! check rejects levels that secretly hold several. To sample, scan from the sparsest level
//! down and return the first recoverable singleton.

use crate::common::hash::xxhash;

/// Seed for the level-assignment hash.
const LEVEL_SEED: u64 = 0x4C30_4C56_4C30; // "L0LVL0"
/// Seed for the fingerprint hash used to validate singletons.
const FP_SEED: u64 = 0x4650_4C30_5346; // "FPL0SF"
/// Number of geometric levels (covers all 64 possible trailing-zero counts).
const LEVELS: usize = 64;

#[inline]
fn key_hash(key: u64, seed: u64) -> u64 {
    xxhash(&key.to_le_bytes(), seed)
}

/// A 1-sparse recovery cell over signed-weight updates.
#[derive(Debug, Clone, Default)]
struct OneSparse {
    /// Σ weight.
    w: i64,
    /// Σ key·weight (wide to avoid overflow).
    kw: i128,
    /// Σ weight·H(key) — a fingerprint that detects non-singletons.
    fp: i64,
}

impl OneSparse {
    fn add(&mut self, key: u64, delta: i64) {
        self.w += delta;
        self.kw += key as i128 * delta as i128;
        self.fp = self
            .fp
            .wrapping_add(delta.wrapping_mul(key_hash(key, FP_SEED) as i64));
    }

    /// Recovers the unique key if this cell holds exactly one item with non-zero weight.
    fn recover(&self) -> Option<u64> {
        if self.w == 0 {
            return None; // empty, or a balanced collision
        }
        let w = self.w as i128;
        if self.kw % w != 0 {
            return None; // not an integer key => collision
        }
        let k = self.kw / w;
        if k < 0 || k > u64::MAX as i128 {
            return None;
        }
        let key = k as u64;
        let expected = self.w.wrapping_mul(key_hash(key, FP_SEED) as i64);
        (expected == self.fp).then_some(key)
    }
}

/// An L0 sampler over a turnstile stream of `(key, ±weight)` updates.
///
/// # Example
/// ```
/// use sketch_oxide::sampling::L0Sampler;
///
/// let mut s = L0Sampler::new();
/// for k in 0..1000u64 { s.insert(k); }
/// // Returns some element of the (non-empty) support.
/// let sample = s.sample().unwrap();
/// assert!(sample < 1000);
///
/// // Delete everything except key 7; the sampler must return 7.
/// for k in 0..1000u64 { if k != 7 { s.delete(k); } }
/// assert_eq!(s.sample(), Some(7));
/// ```
#[derive(Debug, Clone)]
pub struct L0Sampler {
    levels: Vec<OneSparse>,
}

impl Default for L0Sampler {
    fn default() -> Self {
        Self::new()
    }
}

impl L0Sampler {
    /// Creates an empty L0 sampler.
    pub fn new() -> Self {
        Self {
            levels: vec![OneSparse::default(); LEVELS],
        }
    }

    /// Applies a signed update: `key`'s net weight changes by `delta`.
    pub fn update(&mut self, key: u64, delta: i64) {
        if delta == 0 {
            return;
        }
        // The item belongs to levels 0..=tz, where tz = trailing zeros of its level hash.
        let tz = (key_hash(key, LEVEL_SEED).trailing_zeros() as usize).min(LEVELS - 1);
        for level in &mut self.levels[..=tz] {
            level.add(key, delta);
        }
    }

    /// Inserts one occurrence of `key` (`delta = +1`).
    pub fn insert(&mut self, key: u64) {
        self.update(key, 1);
    }

    /// Deletes one occurrence of `key` (`delta = -1`).
    pub fn delete(&mut self, key: u64) {
        self.update(key, -1);
    }

    /// Returns a near-uniform sample from the current support (keys with non-zero net
    /// weight), or `None` if the support is empty or no level is currently recoverable.
    pub fn sample(&self) -> Option<u64> {
        // Scan from the sparsest level down; the first recoverable singleton is the sample.
        for level in self.levels.iter().rev() {
            if let Some(k) = level.recover() {
                return Some(k);
            }
        }
        None
    }

    /// Whether the stream is currently empty (level 0 carries no net weight).
    pub fn is_empty(&self) -> bool {
        let l0 = &self.levels[0];
        l0.w == 0 && l0.kw == 0 && l0.fp == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_has_no_sample() {
        let s = L0Sampler::new();
        assert!(s.is_empty());
        assert_eq!(s.sample(), None);
    }

    #[test]
    fn single_item_recovered() {
        let mut s = L0Sampler::new();
        s.insert(42);
        assert_eq!(s.sample(), Some(42));
    }

    #[test]
    fn sample_is_a_support_member() {
        let mut s = L0Sampler::new();
        for k in 0..2000u64 {
            s.insert(k);
        }
        let sample = s.sample().expect("non-empty support");
        assert!(sample < 2000, "sample {sample} not in support");
    }

    #[test]
    fn deletions_remove_from_support() {
        let mut s = L0Sampler::new();
        for k in 0..1000u64 {
            s.insert(k);
        }
        // Delete all but key 7.
        for k in 0..1000u64 {
            if k != 7 {
                s.delete(k);
            }
        }
        assert!(!s.is_empty());
        assert_eq!(s.sample(), Some(7), "only key 7 should remain");
    }

    #[test]
    fn net_zero_is_empty() {
        let mut s = L0Sampler::new();
        for k in 0..100u64 {
            s.insert(k);
            s.delete(k);
        }
        assert!(s.is_empty());
        assert_eq!(s.sample(), None);
    }

    #[test]
    fn duplicate_inserts_then_full_deletion() {
        let mut s = L0Sampler::new();
        s.update(5, 3); // weight 3
        s.update(9, 2);
        s.update(5, -3); // remove 5 entirely
        assert_eq!(s.sample(), Some(9), "only key 9 remains");
    }

    #[test]
    fn samples_vary_across_support() {
        // Different supports should be able to yield different samples (sanity, not a
        // distribution test).
        let mut a = L0Sampler::new();
        let mut b = L0Sampler::new();
        for k in 0..500u64 {
            a.insert(k);
        }
        for k in 500..1000u64 {
            b.insert(k);
        }
        assert!(a.sample().unwrap() < 500);
        assert!(b.sample().unwrap() >= 500);
    }
}
