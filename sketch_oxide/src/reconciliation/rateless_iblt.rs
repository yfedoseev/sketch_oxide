//! Rateless IBLT — set reconciliation with no pre-agreed size (Yang, Gilad & Alizadeh, SIGCOMM 2024).
//!
//! A classic [IBLT](crate::reconciliation::Iblt) must be sized for an expected set-difference `d`: too
//! small and it cannot decode, too large and it wastes bandwidth. A **Rateless IBLT** instead defines
//! an *infinite* stream of coded symbols `s₀, s₁, s₂, …` for a set. Alice streams her symbols; Bob
//! subtracts his own and peeling-decodes the running difference, stopping once everything is recovered
//! — on average after only `≈ 1.35·d` coded symbols, with **no prior knowledge of `d`**.
//!
//! # Coded symbols
//!
//! Each [`CodedSymbol`] holds three accumulators: `sum` (XOR of the source symbols mapped to it),
//! `checksum` (XOR of their hashes), and a signed `count`. Two sets' symbol streams are *linear*:
//! `aᵢ − bᵢ` is exactly the coded-symbol sequence of the symmetric difference `A △ B`, so a source
//! symbol present in both cancels out. A coded symbol is **pure** (degree 1) when `|count| = 1` and
//! `checksum = hash(sum)` — then `sum` is the one source symbol it holds, on side `A` (`count = +1`)
//! or `B` (`count = −1`).
//!
//! # Mapping
//!
//! Source symbol `x` maps to coded-symbol index `i` with probability `ρ(i) = 1/(1 + i/2)` (so `ρ(0)=1`
//! — every symbol hits index 0, the completion indicator). Rather than testing every index, the next
//! mapped index is sampled directly in constant time from the closed-form inverse CDF
//! `next = i + ⌈(i + 1.5)·(1/√(1−r) − 1)⌉` with `r` drawn from a PRNG seeded by `x` (the paper's
//! `α = 0.5` instantiation). This `O(log m)` mapping density is what makes encoding and peeling cheap.

use crate::common::hash::xxhash;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

const CHECK_SEED: u64 = 0x5217_C0DE_1B17_2024;
const MAP_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// A single coded symbol: the XOR of the source symbols mapped to it, the XOR of their hashes, and a
/// signed count of how many were added (from side `A`) minus removed (from side `B`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CodedSymbol {
    /// XOR of mapped source symbols.
    pub sum: u64,
    /// XOR of the hashes of mapped source symbols (purity check).
    pub checksum: u64,
    /// Signed count: `+1` per symbol from side `A`, `−1` per symbol from side `B`.
    pub count: i64,
}

impl CodedSymbol {
    /// Adds (`direction = +1`) or removes (`direction = −1`) source symbol `sym`.
    #[inline]
    fn apply(&mut self, sym: u64, direction: i64) {
        self.sum ^= sym;
        self.checksum ^= checksum_hash(sym);
        self.count += direction;
    }

    /// Returns `self − other`, the coded symbol of the symmetric difference at this index.
    #[inline]
    pub fn subtract(&self, other: &CodedSymbol) -> CodedSymbol {
        CodedSymbol {
            sum: self.sum ^ other.sum,
            checksum: self.checksum ^ other.checksum,
            count: self.count - other.count,
        }
    }

    /// Whether this coded symbol is empty (holds no source symbols).
    #[inline]
    fn is_empty(&self) -> bool {
        self.count == 0 && self.sum == 0 && self.checksum == 0
    }

    /// Whether this coded symbol is *pure* — a degree-1 cell holding exactly one source symbol.
    #[inline]
    fn is_pure(&self) -> bool {
        self.count.abs() == 1 && self.checksum == checksum_hash(self.sum)
    }
}

/// Deterministic per-symbol index sequence realising the mapping probability `ρ(i) = 1/(1 + i/2)`.
#[derive(Debug, Clone)]
struct RandomMapping {
    state: u64,
    last_idx: u64,
}

impl RandomMapping {
    /// Starts the sequence for `sym`; the first mapped index is always `0`.
    fn new(sym: u64) -> Self {
        Self {
            state: xxhash(&sym.to_le_bytes(), MAP_SEED) | 1,
            last_idx: 0,
        }
    }

    /// SplitMix64 step producing the next uniform draw.
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Advances to and returns the next mapped index (strictly greater than the current one).
    fn next_index(&mut self) -> u64 {
        let r = self.next_u64() as f64 / 18_446_744_073_709_551_616.0; // r in [0, 1)
        let i = self.last_idx as f64;
        let jump = ((i + 1.5) * (1.0 / (1.0 - r).sqrt() - 1.0)).ceil() as u64;
        self.last_idx = self.last_idx.saturating_add(jump.max(1));
        self.last_idx
    }
}

/// A Rateless IBLT encoder for one set: it streams coded symbols `s₀, s₁, s₂, …`.
///
/// # Example
/// ```
/// use sketch_oxide::reconciliation::RatelessIblt;
///
/// // Alice has {1..1000}, Bob has {1..1000} except he is missing 3 and has 2 extras.
/// let alice: Vec<u64> = (1..=1000).collect();
/// let mut bob: Vec<u64> = (1..=1000).filter(|&x| x != 17 && x != 42 && x != 900).collect();
/// bob.push(5000);
/// bob.push(6000);
///
/// // Reconcile without either side knowing the difference size up front.
/// let (a_only, b_only) = RatelessIblt::reconcile(&alice, &bob, 64).unwrap();
/// let mut a_only = a_only; a_only.sort();
/// let mut b_only = b_only; b_only.sort();
/// assert_eq!(a_only, vec![17, 42, 900]);   // in Alice, missing from Bob
/// assert_eq!(b_only, vec![5000, 6000]);    // in Bob, missing from Alice
/// ```
#[derive(Debug, Clone, Default)]
pub struct RatelessIblt {
    syms: Vec<u64>,
    mappings: Vec<RandomMapping>,
    heap: BinaryHeap<Reverse<(u64, usize)>>,
    produced: u64,
}

impl RatelessIblt {
    /// Creates an empty encoder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds source symbol `sym` to the set.
    pub fn insert(&mut self, sym: u64) {
        let sid = self.syms.len();
        let mapping = RandomMapping::new(sym);
        self.heap.push(Reverse((mapping.last_idx, sid))); // first mapped index is 0
        self.syms.push(sym);
        self.mappings.push(mapping);
    }

    /// Number of coded symbols produced so far.
    #[inline]
    pub fn produced(&self) -> u64 {
        self.produced
    }

    /// Produces the next coded symbol in the infinite stream.
    pub fn next_coded_symbol(&mut self) -> CodedSymbol {
        let i = self.produced;
        let mut c = CodedSymbol::default();
        while let Some(&Reverse((idx, sid))) = self.heap.peek() {
            if idx != i {
                break;
            }
            self.heap.pop();
            c.apply(self.syms[sid], 1);
            let next = self.mappings[sid].next_index();
            self.heap.push(Reverse((next, sid)));
        }
        self.produced += 1;
        c
    }

    /// Produces the first `n` coded symbols of this set's stream.
    pub fn coded_symbols(&mut self, n: usize) -> Vec<CodedSymbol> {
        (0..n).map(|_| self.next_coded_symbol()).collect()
    }

    /// Reconciles sets `a` and `b` using a budget of `num_coded` coded symbols. Returns
    /// `(a_only, b_only)` — the symbols in `a` but not `b`, and in `b` but not `a` — or `None` if the
    /// budget was too small to fully decode the difference (send more coded symbols and retry).
    pub fn reconcile(a: &[u64], b: &[u64], num_coded: usize) -> Option<(Vec<u64>, Vec<u64>)> {
        let mut ea = RatelessIblt::new();
        for &s in a {
            ea.insert(s);
        }
        let mut eb = RatelessIblt::new();
        for &s in b {
            eb.insert(s);
        }
        let coded: Vec<CodedSymbol> = (0..num_coded)
            .map(|_| ea.next_coded_symbol().subtract(&eb.next_coded_symbol()))
            .collect();
        Self::decode(&coded)
    }

    /// Peeling-decodes a prefix of difference coded symbols (`aᵢ − bᵢ`). Returns `(a_only, b_only)` if
    /// the prefix fully decodes (every cell peels to empty), else `None` (send more and retry).
    pub fn decode(prefix: &[CodedSymbol]) -> Option<(Vec<u64>, Vec<u64>)> {
        let m = prefix.len();
        let mut cells = prefix.to_vec();
        let mut a_only = Vec::new();
        let mut b_only = Vec::new();

        loop {
            let mut progressed = false;
            for idx in 0..m {
                if !cells[idx].is_pure() {
                    continue;
                }
                let sym = cells[idx].sum;
                let side = cells[idx].count; // +1 ⇒ A, −1 ⇒ B
                if side > 0 {
                    a_only.push(sym);
                } else {
                    b_only.push(sym);
                }
                // Remove the decoded symbol from every coded symbol it maps to within the prefix.
                let mut map = RandomMapping::new(sym);
                let mut j = map.last_idx; // 0
                while (j as usize) < m {
                    cells[j as usize].apply(sym, -side);
                    j = map.next_index();
                }
                progressed = true;
            }
            if !progressed {
                break;
            }
        }

        if cells.iter().all(CodedSymbol::is_empty) {
            Some((a_only, b_only))
        } else {
            None
        }
    }
}

/// Hash of a source symbol for the `checksum` field.
#[inline]
fn checksum_hash(sym: u64) -> u64 {
    xxhash(&sym.to_le_bytes(), CHECK_SEED)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconciles_small_difference() {
        let alice: Vec<u64> = (1..=1000).collect();
        let mut bob: Vec<u64> = (1..=1000)
            .filter(|&x| x != 17 && x != 42 && x != 900)
            .collect();
        bob.push(5000);
        bob.push(6000);
        let (mut a_only, mut b_only) = RatelessIblt::reconcile(&alice, &bob, 64).unwrap();
        a_only.sort_unstable();
        b_only.sort_unstable();
        assert_eq!(a_only, vec![17, 42, 900]);
        assert_eq!(b_only, vec![5000, 6000]);
    }

    #[test]
    fn identical_sets_decode_to_nothing() {
        let s: Vec<u64> = (0..500).map(|i| i * 7 + 3).collect();
        // Even a handful of coded symbols suffice when the difference is empty.
        let (a_only, b_only) = RatelessIblt::reconcile(&s, &s, 8).unwrap();
        assert!(a_only.is_empty() && b_only.is_empty());
    }

    #[test]
    fn undersized_budget_fails_to_decode() {
        // 200 differences cannot be recovered from far too few coded symbols.
        let a: Vec<u64> = (0..1000).collect();
        let b: Vec<u64> = (200..1000).collect(); // a_only = 0..200 (200 differences)
        assert!(RatelessIblt::reconcile(&a, &b, 16).is_none());
        // A budget above ~1.35·d decodes successfully.
        let (a_only, b_only) = RatelessIblt::reconcile(&a, &b, 400).unwrap();
        assert_eq!(a_only.len(), 200);
        assert!(b_only.is_empty());
    }

    #[test]
    fn rateless_overhead_is_near_optimal() {
        // A difference of d should decode from ~1.35·d coded symbols once d is moderately large.
        let d = 300usize;
        let a: Vec<u64> = (0..2000).collect();
        let b: Vec<u64> = (0..2000)
            .filter(|&x| (x as usize) % (2000 / d) != 0)
            .collect();
        let true_d = a.len() - b.len();
        // Generous-but-bounded budget (2·d) must always succeed.
        let res = RatelessIblt::reconcile(&a, &b, 2 * true_d + 10);
        assert!(res.is_some(), "should decode within 2·d coded symbols");
        let (a_only, _b_only) = res.unwrap();
        assert_eq!(a_only.len(), true_d);
    }

    #[test]
    fn streaming_encoder_is_consistent() {
        // Index 0 is hit by every symbol: its count equals the set size.
        let mut e = RatelessIblt::new();
        for x in 0..50u64 {
            e.insert(x);
        }
        let c0 = e.next_coded_symbol();
        assert_eq!(c0.count, 50);
        assert_eq!(e.produced(), 1);
    }

    #[test]
    fn coded_symbol_subtract_cancels_common_elements() {
        let mut ea = RatelessIblt::new();
        let mut eb = RatelessIblt::new();
        for x in [1u64, 2, 3, 4] {
            ea.insert(x);
        }
        for x in [2u64, 3, 4, 5] {
            eb.insert(x);
        }
        // Difference is {1 (A), 5 (B)}; decode a generous prefix.
        let coded: Vec<CodedSymbol> = (0..16)
            .map(|_| ea.next_coded_symbol().subtract(&eb.next_coded_symbol()))
            .collect();
        let (mut a_only, mut b_only) = RatelessIblt::decode(&coded).unwrap();
        a_only.sort_unstable();
        b_only.sort_unstable();
        assert_eq!(a_only, vec![1]);
        assert_eq!(b_only, vec![5]);
    }
}
