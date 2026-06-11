//! Morton Filter — a faster, more space-efficient cuckoo filter (Breslow & Jayasena, VLDB 2018).
//!
//! A cuckoo filter probes two candidate buckets per lookup (two cache lines) and sizes its buckets
//! for a high load factor, wasting bits on rarely-used slots. The Morton Filter (MF) re-engineers the
//! cuckoo filter around three ideas — **compression**, **biasing**, and **decoupled logical
//! sparsity** — so that lookups, inserts, and deletes typically touch a single cache line:
//!
//! - **Block store / decoupled sparsity.** Fingerprints are grouped into fixed-size *blocks*, each
//!   storing the fingerprints of `B` logical buckets in a compact **Fingerprint Storage Array (FSA)**
//!   whose capacity is *smaller* than `B·S` logical slots. A **Fullness Counter Array (FCA)** records
//!   each bucket's load, recovering the logical layout in-situ. Because the per-block FSA pools many
//!   sparsely-filled buckets, the filter is logically underloaded (small, mostly-empty buckets → few
//!   comparisons) yet physically dense (the FSA runs ~95% full → little wasted space).
//! - **Biasing.** Insertions always try the **primary** bucket (`H1`) first, only falling back to the
//!   **secondary** (`H2`) on overflow — so most items live in a single bucket/cache line.
//! - **Overflow Tracking Array (OTA).** A small per-block bit vector: whenever a fingerprint is
//!   relocated out of a bucket, the bit for that bucket is set. A negative lookup whose primary bucket
//!   maps to an unset OTA bit can skip the secondary bucket entirely.
//!
//! The alternate bucket is `H2(β) = β ± offset(fp)` with the sign chosen by `β`'s parity and an
//! **odd** `offset(fp) = (B + fp mod B) | 1 ≥ B`; oddness flips parity so this is an *involution*
//! (recoverable from any bucket without the key, as cuckoo displacement requires), and `offset ≥ B`
//! guarantees the two candidates lie in different blocks (so relocating always unloads the origin
//! block). The OTA is set monotonically on every relocation `β → H2(β)`, which is exactly the bit a
//! lookup of a now-secondary item checks — so there are **never false negatives** (bits are not
//! cleared; stale sets only cost an extra probe).
//!
//! # Layout note
//!
//! Behaviour-faithful reference layout: each block holds explicit per-bucket fingerprint lists plus
//! its OTA, with the FSA capacity enforced as a per-block fingerprint budget — reproducing the same
//! membership semantics, biasing, OTA filtering, and block-vs-bucket overflow dynamics. The literal
//! bit-packed FSA/FCA with `popcount` rank-select (a space/speed optimisation) is left as a follow-up,
//! matching this crate's [`CountingQuotientFilter`] approach.
//!
//! [`CountingQuotientFilter`]: crate::membership::CountingQuotientFilter

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

const HASH_SEED: u64 = 0x309D_0000_0000_0001;
const FP_SEED: u64 = 0x309D_0000_0000_0002;
const KICK_SEED: u64 = 0x309D_0000_0000_0003;
const MAX_KICKS: usize = 500;

/// One block: `B` logical buckets (explicit fingerprint lists) sharing an FSA budget, plus its OTA.
#[derive(Debug, Clone)]
struct Block {
    /// One fingerprint list per logical bucket (`len == B`); list length is the FCA counter.
    buckets: Vec<Vec<u16>>,
    /// Overflow Tracking Array bits.
    ota: Vec<bool>,
}

/// A Morton Filter: an array of compressed blocks behaving as a biased cuckoo filter.
///
/// # Example
/// ```
/// use sketch_oxide::membership::MortonFilter;
///
/// // 2048 blocks × 64 buckets × 3 slots, FSA budget 40 fingerprints/block, 12-bit fingerprints,
/// // 16-bit OTA. Capacity ≈ 2048 · 40 ≈ 81k items.
/// let mut mf = MortonFilter::new(2048, 64, 3, 40, 12, 16).unwrap();
/// for i in 0..60_000u32 {
///     assert!(mf.insert(&i.to_le_bytes()));
/// }
/// // No false negatives.
/// for i in 0..60_000u32 {
///     assert!(mf.contains(&i.to_le_bytes()));
/// }
/// // Deletions are supported for inserted items.
/// assert!(mf.remove(&0u32.to_le_bytes()));
/// ```
#[derive(Debug, Clone)]
pub struct MortonFilter {
    b: usize,
    s: usize,
    fsa_cap: usize,
    fp_bits: u32,
    ota_bits: usize,
    n: usize, // total buckets = num_blocks * b
    blocks: Vec<Block>,
    count: usize,
}

impl MortonFilter {
    /// Creates a Morton Filter.
    ///
    /// * `num_blocks` — number of blocks (`≥ 2`).
    /// * `buckets_per_block` (`B`) — logical buckets per block; must be even and `≥ 2`.
    /// * `slots_per_bucket` (`S`) — logical capacity of each bucket (`≥ 1`).
    /// * `fsa_capacity` — fingerprints a block can physically hold (`1 ≤ fsa_capacity ≤ B·S`);
    ///   choosing it below `B·S` is the compression that gives the MF its space edge.
    /// * `fingerprint_bits` (`f`) — fingerprint width (`1..=16`); FPR ≈ `2^-f` per probed slot.
    /// * `ota_bits` — Overflow Tracking Array width per block (`≥ 1`).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if any constraint above is violated.
    pub fn new(
        num_blocks: usize,
        buckets_per_block: usize,
        slots_per_bucket: usize,
        fsa_capacity: usize,
        fingerprint_bits: u32,
        ota_bits: usize,
    ) -> Result<Self> {
        let err = |param: &str, value: String, constraint: &str| {
            Err(SketchError::InvalidParameter {
                param: param.to_string(),
                value,
                constraint: constraint.to_string(),
            })
        };
        if num_blocks < 2 {
            return err("num_blocks", num_blocks.to_string(), "must be >= 2");
        }
        if buckets_per_block < 2 || !buckets_per_block.is_multiple_of(2) {
            return err(
                "buckets_per_block",
                buckets_per_block.to_string(),
                "must be even and >= 2",
            );
        }
        if slots_per_bucket < 1 {
            return err(
                "slots_per_bucket",
                slots_per_bucket.to_string(),
                "must be >= 1",
            );
        }
        if fsa_capacity < 1 || fsa_capacity > buckets_per_block * slots_per_bucket {
            return err(
                "fsa_capacity",
                fsa_capacity.to_string(),
                "must be in 1..=buckets_per_block*slots_per_bucket",
            );
        }
        if !(1..=16).contains(&fingerprint_bits) {
            return err(
                "fingerprint_bits",
                fingerprint_bits.to_string(),
                "must be in 1..=16",
            );
        }
        if ota_bits < 1 {
            return err("ota_bits", ota_bits.to_string(), "must be >= 1");
        }
        let blocks = vec![
            Block {
                buckets: vec![Vec::new(); buckets_per_block],
                ota: vec![false; ota_bits],
            };
            num_blocks
        ];
        Ok(Self {
            b: buckets_per_block,
            s: slots_per_bucket,
            fsa_cap: fsa_capacity,
            fp_bits: fingerprint_bits,
            ota_bits,
            n: num_blocks * buckets_per_block,
            blocks,
            count: 0,
        })
    }

    /// Number of items inserted.
    #[inline]
    pub fn len(&self) -> usize {
        self.count
    }

    /// Whether nothing has been inserted.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Fingerprint of `item` (nonzero, `fp_bits` wide).
    fn fingerprint(&self, item: &[u8]) -> u16 {
        let h = xxhash(item, FP_SEED);
        ((h % ((1u64 << self.fp_bits) - 1)) + 1) as u16
    }

    /// Primary global bucket index of `item` (Lemire reduction, unbiased over `0..n`).
    fn primary(&self, item: &[u8]) -> usize {
        let h = xxhash(item, HASH_SEED);
        ((h as u128 * self.n as u128) >> 64) as usize
    }

    /// Alternate (secondary ↔ primary) bucket of bucket `beta` holding fingerprint `fp`. Involution.
    fn alternate(&self, beta: usize, fp: u16) -> usize {
        // Odd offset in [B, 2B): flips parity (→ involution) and lands in a different block.
        let offset = ((self.b + (fp as usize % self.b)) | 1) as i64;
        let x = if beta.is_multiple_of(2) {
            beta as i64 + offset
        } else {
            beta as i64 - offset
        };
        let n = self.n as i64;
        // offset < n, so a single wrap suffices.
        let wrapped = if x < 0 {
            x + n
        } else if x >= n {
            x - n
        } else {
            x
        };
        wrapped as usize
    }

    /// OTA bit index for a block-local bucket index.
    #[inline]
    fn ota_index(&self, lbi: usize) -> usize {
        lbi % self.ota_bits
    }

    /// Scans bucket `beta` for `fp`.
    fn bucket_has(&self, beta: usize, fp: u16) -> bool {
        self.blocks[beta / self.b].buckets[beta % self.b].contains(&fp)
    }

    /// Total fingerprints currently stored in block `blk`.
    fn block_load(&self, blk: usize) -> usize {
        self.blocks[blk].buckets.iter().map(Vec::len).sum()
    }

    /// Tests membership. Never a false negative; false positives occur with bounded probability.
    pub fn contains(&self, item: &[u8]) -> bool {
        let fp = self.fingerprint(item);
        let b1 = self.primary(item);
        if self.bucket_has(b1, fp) {
            return true;
        }
        // Biasing: skip the secondary bucket unless this bucket's OTA bit says it may have overflowed.
        let blk1 = b1 / self.b;
        if !self.blocks[blk1].ota[self.ota_index(b1 % self.b)] {
            return false;
        }
        let b2 = self.alternate(b1, fp);
        self.bucket_has(b2, fp)
    }

    /// Inserts `item`. Returns `false` if the filter is too full to place it (after cuckoo kicks).
    pub fn insert(&mut self, item: &[u8]) -> bool {
        let fp = self.fingerprint(item);
        let b1 = self.primary(item);
        if self.try_store(b1, fp) {
            self.count += 1;
            return true;
        }
        // Biasing: mark the primary bucket as having overflowed, then try the secondary.
        let lbi1 = b1 % self.b;
        let idx = self.ota_index(lbi1);
        self.blocks[b1 / self.b].ota[idx] = true;
        let b2 = self.alternate(b1, fp);
        if self.try_store(b2, fp) {
            self.count += 1;
            return true;
        }
        if self.resolve_conflict(b2, fp) {
            self.count += 1;
            return true;
        }
        false
    }

    /// Stores `fp` in bucket `beta` if neither its bucket nor its block has overflowed.
    fn try_store(&mut self, beta: usize, fp: u16) -> bool {
        let blk = beta / self.b;
        let lbi = beta % self.b;
        if self.blocks[blk].buckets[lbi].len() < self.s && self.block_load(blk) < self.fsa_cap {
            self.blocks[blk].buckets[lbi].push(fp);
            true
        } else {
            false
        }
    }

    /// Cuckoo displacement: evict a fingerprint to make room, re-home it, repeat.
    fn resolve_conflict(&mut self, mut beta: usize, mut fp: u16) -> bool {
        for kick in 0..MAX_KICKS {
            let blk = beta / self.b;
            let lbi = beta % self.b;
            if self.blocks[blk].buckets[lbi].len() < self.s && self.block_load(blk) < self.fsa_cap {
                self.blocks[blk].buckets[lbi].push(fp);
                return true;
            }
            // Choose a victim: on a bucket overflow it must come from `beta`; on a (pure) block
            // overflow any non-empty bucket in the block can yield, since it unloads the block.
            let r = xxhash(
                &[&fp.to_le_bytes()[..], &(kick as u64).to_le_bytes()[..]].concat(),
                KICK_SEED,
            );
            let victim_lbi = if self.blocks[blk].buckets[lbi].len() >= self.s {
                lbi
            } else {
                let nonempty: Vec<usize> = (0..self.b)
                    .filter(|&i| !self.blocks[blk].buckets[i].is_empty())
                    .collect();
                nonempty[(r as usize) % nonempty.len()]
            };
            let vlen = self.blocks[blk].buckets[victim_lbi].len();
            let slot = ((r >> 20) as usize) % vlen;
            let evicted = self.blocks[blk].buckets[victim_lbi].remove(slot);
            // Relocation β→H2(β): set the victim bucket's OTA bit (covers every primary→secondary).
            let idx = self.ota_index(victim_lbi);
            self.blocks[blk].ota[idx] = true;
            // Place the incoming fingerprint, now that a slot is free.
            self.blocks[blk].buckets[lbi].push(fp);
            // Continue displacing the evicted fingerprint into its alternate bucket.
            let victim_global = blk * self.b + victim_lbi;
            beta = self.alternate(victim_global, evicted);
            fp = evicted;
        }
        false
    }

    /// Removes one occurrence of `item` if present (valid for previously-inserted items). Returns
    /// whether a fingerprint was removed.
    pub fn remove(&mut self, item: &[u8]) -> bool {
        let fp = self.fingerprint(item);
        let b1 = self.primary(item);
        if self.remove_from(b1, fp) {
            self.count -= 1;
            return true;
        }
        let blk1 = b1 / self.b;
        if self.blocks[blk1].ota[self.ota_index(b1 % self.b)] {
            let b2 = self.alternate(b1, fp);
            if self.remove_from(b2, fp) {
                self.count -= 1;
                return true;
            }
        }
        false
    }

    fn remove_from(&mut self, beta: usize, fp: u16) -> bool {
        let bucket = &mut self.blocks[beta / self.b].buckets[beta % self.b];
        if let Some(pos) = bucket.iter().position(|&x| x == fp) {
            bucket.remove(pos);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(MortonFilter::new(1, 16, 3, 40, 12, 16).is_err()); // num_blocks < 2
        assert!(MortonFilter::new(64, 15, 3, 40, 12, 16).is_err()); // B odd
        assert!(MortonFilter::new(64, 16, 0, 40, 12, 16).is_err()); // S = 0
        assert!(MortonFilter::new(64, 16, 3, 49, 12, 16).is_err()); // fsa_cap > B*S
        assert!(MortonFilter::new(64, 16, 3, 40, 17, 16).is_err()); // f > 16
        assert!(MortonFilter::new(64, 16, 3, 40, 12, 0).is_err()); // ota_bits = 0
        assert!(MortonFilter::new(64, 16, 3, 40, 12, 16).is_ok());
    }

    #[test]
    fn empty_contains_nothing() {
        let mf = MortonFilter::new(64, 16, 3, 40, 12, 16).unwrap();
        assert!(mf.is_empty());
        assert!(!mf.contains(b"nothing"));
    }

    #[test]
    fn alternate_is_an_involution_in_different_block() {
        let mf = MortonFilter::new(64, 16, 3, 40, 12, 16).unwrap();
        for beta in [0usize, 1, 7, 16, 17, 100, 1023] {
            for fp in [1u16, 2, 5, 255, 4095] {
                let alt = mf.alternate(beta, fp);
                assert_eq!(mf.alternate(alt, fp), beta, "not involution at {beta},{fp}");
                assert_ne!(beta / mf.b, alt / mf.b, "same block at {beta},{fp}");
            }
        }
    }

    #[test]
    fn no_false_negatives_at_high_load() {
        // B=64 pools variance across many buckets, the regime the MF is designed for.
        let mut mf = MortonFilter::new(2048, 64, 3, 44, 12, 16).unwrap();
        let n = 72_000u32; // ~80% of the 2048·44 ≈ 90k FSA capacity
        let mut accepted = Vec::new();
        for i in 0..n {
            if mf.insert(&i.to_le_bytes()) {
                accepted.push(i);
            }
        }
        assert!(
            accepted.len() as f64 > 0.95 * n as f64,
            "only inserted {}/{n}",
            accepted.len()
        );
        // The defining invariant: every accepted item is found (no false negatives).
        for &i in &accepted {
            assert!(mf.contains(&i.to_le_bytes()), "missing {i}");
        }
    }

    #[test]
    fn false_positive_rate_is_bounded() {
        let mut mf = MortonFilter::new(4096, 16, 3, 40, 12, 16).unwrap();
        let n = 100_000u32;
        for i in 0..n {
            mf.insert(&i.to_le_bytes());
        }
        let trials = 200_000u32;
        let fps = (1_000_000..1_000_000 + trials)
            .filter(|i| mf.contains(&i.to_le_bytes()))
            .count();
        let fpr = fps as f64 / trials as f64;
        // 12-bit fingerprints, ≤ ~2 probed buckets of ≤3 slots ⇒ FPR well under 1%.
        assert!(fpr < 0.01, "FPR {fpr} too high ({fps}/{trials})");
    }

    #[test]
    fn deletion_round_trips() {
        let mut mf = MortonFilter::new(1024, 16, 3, 40, 12, 16).unwrap();
        for i in 0..20_000u32 {
            mf.insert(&i.to_le_bytes());
        }
        let before = mf.len();
        // Remove a subset; each removed item's count drops and it is (very likely) absent after.
        let mut removed = 0;
        for i in 0..1_000u32 {
            if mf.remove(&i.to_le_bytes()) {
                removed += 1;
            }
        }
        assert_eq!(removed, 1_000, "all inserted items should be removable");
        assert_eq!(mf.len(), before - 1_000);
        // The other items are untouched.
        for i in 1_000..20_000u32 {
            assert!(mf.contains(&i.to_le_bytes()), "removal corrupted {i}");
        }
    }
}
