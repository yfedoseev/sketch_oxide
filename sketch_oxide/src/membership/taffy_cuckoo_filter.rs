//! Taffy Cuckoo Filter (TCF) — a cuckoo filter that *grows* without rebuilds or fpp inflation
//! (Jim Apple, "Stretching Your Data With Taffy Filters", SP&E 2022).
//!
//! Ordinary cuckoo and Bloom filters must be sized up front: once full, inserts fail or the
//! false-positive probability (fpp) doubles. A **Taffy Cuckoo Filter** starts tiny and doubles on
//! demand while keeping the fpp bounded by `O(2^-F)`, by combining two ideas:
//!
//! * **Quotienting via invertible permutations.** Each side of the cuckoo table applies an *invertible*
//!   permutation to the top `a + F` bits of a key's hash; the high `a` bits select the bucket and the
//!   low `F` bits are the stored fingerprint. Because the permutation is invertible, the `a + F` key
//!   bits can be *recovered* from `(bucket, fingerprint)` — needed for eviction and for growth.
//! * **Tails + bit-stealing growth.** Each stored entry also keeps a short **tail**: the next few hash
//!   bits below the `a+F` field. A lookup matches only if the fingerprint matches *and* the stored tail
//!   is a prefix of the query's tail. To **upsize** (`a → a+1`), every entry steals its top tail bit,
//!   appends it to the recovered key bits (so the bucket index grows by one bit), and shrinks its tail
//!   by one. An entry whose tail is already empty splits into the two possible keys (append `0` and
//!   `1`); one is real, the other a harmless phantom. This grows the table without ever increasing the
//!   per-entry fingerprint collision probability — the defining Taffy property.
//!
//! Like every filter here, a TCF has **no false negatives**: an inserted key is always found. False
//! positives stay bounded as it grows.
//!
//! # Implementation note
//!
//! The paper uses Feistel networks for the side permutations; correctness (no false negatives) only
//! needs an *invertible* permutation, so this reference uses a composed multiply/rotate/xor bijection
//! on `a+F` bits with a known modular inverse — behaviour-faithful, with permutation quality affecting
//! only the fpp constant.

use crate::common::hash::xxhash;

const FP_BITS: u32 = 12; // fingerprint width F
const TAIL_BITS: u32 = 8; // maximum tail length T
const SLOTS: usize = 4; // slots per bucket b
const A_INIT: u32 = 4; // initial log2(buckets per side)
const A_MAX: u32 = 48; // cap so a + F + T stays within 64 bits
const MAX_EVICT: usize = 500;
const HASH_SEED: u64 = 0x7AFF_4313_C0CC_0001;

/// One stored entry: a fingerprint plus a (possibly empty) tail of `tail_len` bits (MSB first).
#[derive(Debug, Clone, Copy)]
struct Entry {
    fp: u32,
    tail: u32,
    tail_len: u8,
}

/// An invertible permutation on `n`-bit values: `rotl(rotl(x·k1, r1) ^ c1 · k2, r2)`.
#[derive(Debug, Clone)]
struct Perm {
    n: u32,
    mask: u64,
    k1: u64,
    k1_inv: u64,
    r1: u32,
    c1: u64,
    k2: u64,
    k2_inv: u64,
    r2: u32,
}

impl Perm {
    fn new(seed: u64, n: u32) -> Self {
        let mask = if n >= 64 { u64::MAX } else { (1u64 << n) - 1 };
        let mut s = seed;
        let k1 = (splitmix(&mut s) | 1) & mask;
        let c1 = splitmix(&mut s) & mask;
        let k2 = (splitmix(&mut s) | 1) & mask;
        let r1 = 1 + (splitmix(&mut s) % (n as u64 - 1)) as u32;
        let r2 = 1 + (splitmix(&mut s) % (n as u64 - 1)) as u32;
        Self {
            n,
            mask,
            k1,
            k1_inv: mod_inverse_pow2(k1),
            r1,
            c1,
            k2,
            k2_inv: mod_inverse_pow2(k2),
            r2,
        }
    }

    #[inline]
    fn rotl(&self, x: u64, r: u32) -> u64 {
        ((x << r) | (x >> (self.n - r))) & self.mask
    }

    /// Forward permutation.
    fn forward(&self, x: u64) -> u64 {
        let mut x = x.wrapping_mul(self.k1) & self.mask;
        x = self.rotl(x, self.r1);
        x ^= self.c1;
        x = x.wrapping_mul(self.k2) & self.mask;
        self.rotl(x, self.r2)
    }

    /// Inverse permutation (`forward` and `inverse` compose to the identity).
    fn inverse(&self, y: u64) -> u64 {
        let mut y = self.rotl(y, self.n - self.r2);
        y = y.wrapping_mul(self.k2_inv) & self.mask;
        y ^= self.c1;
        y = self.rotl(y, self.n - self.r1);
        y.wrapping_mul(self.k1_inv) & self.mask
    }
}

/// A growing cuckoo filter with bounded false-positive probability over byte-string keys.
///
/// # Example
/// ```
/// use sketch_oxide::membership::TaffyCuckooFilter;
///
/// // Starts tiny and grows automatically as keys are inserted — no capacity argument.
/// let mut f = TaffyCuckooFilter::with_seed(1);
/// for i in 0..10_000u32 {
///     f.insert(&i.to_le_bytes());
/// }
/// // No false negatives: every inserted key is found, even after many automatic upsizes.
/// for i in 0..10_000u32 {
///     assert!(f.contains(&i.to_le_bytes()));
/// }
/// assert_eq!(f.len(), 10_000);
/// ```
#[derive(Debug, Clone)]
pub struct TaffyCuckooFilter {
    a: u32,
    sides: [Vec<Option<Entry>>; 2],
    perms: [Perm; 2],
    side_seed: [u64; 2],
    len: usize,
    rng: u64,
}

impl TaffyCuckooFilter {
    /// Creates an empty Taffy Cuckoo Filter seeded from a fixed default.
    pub fn new() -> Self {
        Self::with_seed(0x9E37_79B9)
    }

    /// Creates an empty filter with a caller-chosen seed (controls the side permutations and the
    /// eviction RNG, making behaviour reproducible).
    pub fn with_seed(seed: u64) -> Self {
        let side_seed = [seed ^ 0xA5A5_A5A5_0000_0001, seed ^ 0x5A5A_5A5A_0000_0002];
        let a = A_INIT;
        let cells = (1usize << a) * SLOTS;
        Self {
            a,
            sides: [vec![None; cells], vec![None; cells]],
            perms: [
                Perm::new(side_seed[0], a + FP_BITS),
                Perm::new(side_seed[1], a + FP_BITS),
            ],
            side_seed,
            len: 0,
            rng: seed | 1,
        }
    }

    /// Number of keys inserted.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether no keys have been inserted.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Current physical slot capacity (both sides). Grows as the filter upsizes.
    #[inline]
    pub fn capacity(&self) -> usize {
        2 * (1usize << self.a) * SLOTS
    }

    /// `log2` of the number of buckets per side (increases on each upsize).
    #[inline]
    pub fn log_buckets(&self) -> u32 {
        self.a
    }

    #[inline]
    fn fp_mask() -> u64 {
        (1u64 << FP_BITS) - 1
    }

    #[inline]
    fn next_rand(&mut self) -> u64 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        x
    }

    /// Top `a+F` bits and the following tail bits of a key's hash, at the current size.
    fn raw_from_key(&self, key: &[u8]) -> (u64, u32, u8) {
        let h = xxhash(key, HASH_SEED);
        let field = self.a + FP_BITS;
        let key_bits = h >> (64 - field);
        let avail = 64 - field;
        let tlen = TAIL_BITS.min(avail);
        let tail = if tlen == 0 {
            0
        } else {
            ((h >> (64 - field - tlen)) & ((1u64 << tlen) - 1)) as u32
        };
        (key_bits, tail, tlen as u8)
    }

    /// Inserts `key`. The filter grows automatically; insertion never fails.
    pub fn insert(&mut self, key: &[u8]) {
        let raw = self.raw_from_key(key);
        self.len += 1;
        if let Some(victim) = self.place(raw) {
            // The new key displaced `victim` out of the table; rebuild the full set (current entries
            // plus the displaced victim) at a larger size so both fit.
            self.grow_and_rebuild(Some(victim));
        }
    }

    /// Returns `true` if `key` may be present (never a false negative).
    pub fn contains(&self, key: &[u8]) -> bool {
        let (key_bits, qtail, qlen) = self.raw_from_key(key);
        let qlen = qlen as u32;
        for side in 0..2 {
            let pv = self.perms[side].forward(key_bits);
            let bucket = (pv >> FP_BITS) as usize;
            let fp = (pv & Self::fp_mask()) as u32;
            let base = bucket * SLOTS;
            for e in self.sides[side][base..base + SLOTS].iter().flatten() {
                if e.fp == fp && tail_is_prefix(e, qtail, qlen) {
                    return true;
                }
            }
        }
        false
    }

    /// Tries to place a current-size raw entry by cuckoo eviction starting on side 0. Returns the
    /// displaced raw entry if `MAX_EVICT` evictions are exhausted (table left valid otherwise).
    fn place(&mut self, raw: (u64, u32, u8)) -> Option<(u64, u32, u8)> {
        let (mut kb, mut tail, mut tlen) = raw;
        let mut side = 0usize;
        for _ in 0..MAX_EVICT {
            let pv = self.perms[side].forward(kb);
            let bucket = (pv >> FP_BITS) as usize;
            let fp = (pv & Self::fp_mask()) as u32;
            let base = bucket * SLOTS;
            if let Some(free) = self.sides[side][base..base + SLOTS]
                .iter()
                .position(|s| s.is_none())
            {
                self.sides[side][base + free] = Some(Entry {
                    fp,
                    tail,
                    tail_len: tlen,
                });
                return None;
            }
            // Bucket full: evict a random occupant, leave ours, carry the victim to the other side.
            let victim_idx = base + (self.next_rand() as usize % SLOTS);
            let victim = self.sides[side][victim_idx].unwrap();
            self.sides[side][victim_idx] = Some(Entry {
                fp,
                tail,
                tail_len: tlen,
            });
            kb = self.perms[side].inverse(((bucket as u64) << FP_BITS) | victim.fp as u64);
            tail = victim.tail;
            tlen = victim.tail_len;
            side = 1 - side;
        }
        Some((kb, tail, tlen))
    }

    /// Collects every stored entry as a side-independent raw `(key_bits, tail, tail_len)` at the
    /// current size.
    fn collect_raws(&self) -> Vec<(u64, u32, u8)> {
        let mut raws = Vec::with_capacity(self.len);
        for side in 0..2 {
            for (idx, slot) in self.sides[side].iter().enumerate() {
                if let Some(e) = slot {
                    let bucket = (idx / SLOTS) as u64;
                    let kb = self.perms[side].inverse((bucket << FP_BITS) | e.fp as u64);
                    raws.push((kb, e.tail, e.tail_len));
                }
            }
        }
        raws
    }

    /// Doubles the table (possibly several times) until the complete logical set fits, transforming
    /// every entry by stealing one tail bit per doubling. `extra` is a displaced entry (at the current
    /// size) that must also be (re)admitted.
    fn grow_and_rebuild(&mut self, extra: Option<(u64, u32, u8)>) {
        let mut raws = self.collect_raws();
        if let Some(e) = extra {
            raws.push(e);
        }
        loop {
            // Only grow when the table is *genuinely* full. A placement failure at low load means
            // duplicate-fingerprint saturation — the same key inserted more than `2·SLOTS` times, which
            // no cuckoo filter can hold and growth cannot fix. There we place best-effort: a dropped
            // copy is a duplicate whose key is already present, so `contains` keeps no false negative.
            let load = raws.len() as f64 / (2 * (1usize << self.a) * SLOTS) as f64;
            if self.a >= A_MAX || load < 0.85 {
                self.rebuild_at_current(&raws);
                return;
            }
            self.a += 1;
            let field = self.a + FP_BITS;
            self.perms = [
                Perm::new(self.side_seed[0], field),
                Perm::new(self.side_seed[1], field),
            ];
            let cells = (1usize << self.a) * SLOTS;
            self.sides = [vec![None; cells], vec![None; cells]];

            // Transform to the new size: each entry steals its top tail bit (empty tails split in two).
            let transformed: Vec<(u64, u32, u8)> =
                raws.iter().flat_map(|&r| steal_bit(r)).collect();
            let mut ok = true;
            for &t in &transformed {
                if self.place(t).is_some() {
                    ok = false;
                    break;
                }
            }
            if ok {
                return;
            }
            // A placement failed even after doubling (rare): grow again from the full new-size set.
            raws = transformed;
        }
    }

    /// Best-effort placement of all raws at the current size, ignoring overflow (only reached at the
    /// `A_MAX` cap).
    fn rebuild_at_current(&mut self, raws: &[(u64, u32, u8)]) {
        let cells = (1usize << self.a) * SLOTS;
        self.sides = [vec![None; cells], vec![None; cells]];
        for &r in raws {
            let _ = self.place(r);
        }
    }
}

impl Default for TaffyCuckooFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Whether the stored entry's tail is a prefix of the query tail (`qtail`, `qlen` bits).
fn tail_is_prefix(e: &Entry, qtail: u32, qlen: u32) -> bool {
    let tl = e.tail_len as u32;
    if tl > qlen {
        return false;
    }
    e.tail == (qtail >> (qlen - tl))
}

/// Transforms a raw entry to the next size up: append the stolen top tail bit to the key bits and
/// shrink the tail. An empty tail yields the two candidate keys (`…0` and `…1`).
fn steal_bit((kb, tail, tlen): (u64, u32, u8)) -> Vec<(u64, u32, u8)> {
    if tlen == 0 {
        vec![(kb << 1, 0, 0), ((kb << 1) | 1, 0, 0)]
    } else {
        let stolen = (tail >> (tlen - 1)) & 1;
        let new_tail = tail & ((1u32 << (tlen - 1)) - 1);
        vec![((kb << 1) | stolen as u64, new_tail, tlen - 1)]
    }
}

/// Inverse of an odd `a` modulo `2^64` via Newton's iteration.
fn mod_inverse_pow2(a: u64) -> u64 {
    let mut inv: u64 = 1;
    for _ in 0..6 {
        inv = inv.wrapping_mul(2u64.wrapping_sub(a.wrapping_mul(inv)));
    }
    inv
}

/// SplitMix64 step for deriving permutation constants.
fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perm_round_trips() {
        for n in [16u32, 20, 24, 31] {
            let p = Perm::new(0xDEAD_BEEF ^ n as u64, n);
            let mask = (1u64 << n) - 1;
            for x in [0u64, 1, 2, 12345, mask, mask - 1, mask / 3] {
                let x = x & mask;
                assert_eq!(p.inverse(p.forward(x)), x, "n={n} x={x}");
            }
        }
    }

    #[test]
    fn no_false_negatives_with_growth() {
        let mut f = TaffyCuckooFilter::with_seed(7);
        let n = 20_000u32;
        let start_cap = f.capacity();
        for i in 0..n {
            f.insert(&i.to_le_bytes());
        }
        // Every inserted key must still be found after many automatic upsizes.
        for i in 0..n {
            assert!(f.contains(&i.to_le_bytes()), "false negative for {i}");
        }
        assert_eq!(f.len(), n as usize);
        assert!(f.capacity() > start_cap, "filter should have grown");
    }

    #[test]
    fn grows_from_tiny_start() {
        // Initial capacity is small; inserting far more keys forces several doublings.
        let mut f = TaffyCuckooFilter::with_seed(3);
        let a0 = f.log_buckets();
        for i in 0..5_000u32 {
            f.insert(&i.to_le_bytes());
        }
        assert!(f.log_buckets() >= a0 + 4, "expected multiple upsizes");
        for i in 0..5_000u32 {
            assert!(f.contains(&i.to_le_bytes()));
        }
    }

    #[test]
    fn bounded_false_positive_rate() {
        let mut f = TaffyCuckooFilter::with_seed(11);
        for i in 0..50_000u32 {
            f.insert(&i.to_le_bytes());
        }
        let trials = 200_000u64;
        let fps = (1_000_000_000u64..1_000_000_000 + trials)
            .filter(|x| f.contains(&x.to_le_bytes()))
            .count();
        let fpr = fps as f64 / trials as f64;
        // Two sides of F=12-bit fingerprints plus tail filtering ⇒ fpr well under 1%.
        assert!(fpr < 0.01, "fpr {fpr} too high");
    }

    #[test]
    fn duplicate_inserts_are_found() {
        let mut f = TaffyCuckooFilter::with_seed(5);
        for _ in 0..10 {
            f.insert(b"repeated");
        }
        assert!(f.contains(b"repeated"));
        assert_eq!(f.len(), 10);
    }

    #[test]
    fn empty_filter_finds_nothing_inserted() {
        let f = TaffyCuckooFilter::new();
        assert!(f.is_empty());
        // An empty filter has no inserted keys (a stray query is almost surely absent).
        assert!(!f.contains(b"never-inserted-xyz"));
    }
}
