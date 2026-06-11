//! Hidden Sketch — a reversible frequency sketch that recovers heavy keys *and* their counts.
//!
//! Standard frequency sketches ([`CountMinSketch`](crate::frequency::CountMinSketch),
//! [`HeavyKeeper`](crate::frequency::HeavyKeeper)) can answer "what is the frequency of key `X`?" but
//! cannot *list* the heavy keys — the keys themselves are lost. The Hidden Sketch (2025) makes the
//! sketch **invertible**: alongside a Count-Min-style count it stores, in each cell, the running sum
//! `Σ key·c` and a verification sum `Σ H(key)·c`. A cell touched by a single distinct key `k` is
//! *pure* — `key_sum / count = k` and `hash_sum = count · H(k)` — so `k` and its frequency can be read
//! straight out, then peeled from its other cells. Iterating recovers every `(key, frequency)` pair
//! whenever the distinct keys fit the table, and the heavy hitters even when they do not.
//!
//! Each key maps to one cell per `depth` blocks (so its cells are always distinct, as in an
//! Invertible Bloom Lookup Table). [`estimate`](HiddenSketch::estimate) gives the Count-Min upper
//! bound for a known key; [`decode`](HiddenSketch::decode) lists the recoverable `(key, frequency)`
//! pairs.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};

/// Verification-hash seed.
const VERIFY_SEED: u64 = 0x1D34_5333_C347_57E6;
/// Base seed for the per-block position hashes.
const POS_SEED_BASE: u64 = 0xB10C_5EED_0000_0001;

#[derive(Debug, Clone, Default)]
struct Cell {
    count: i64,
    key_sum: i128,
    hash_sum: i128,
}

/// A reversible frequency sketch over `u64` keys.
///
/// # Example
/// ```
/// use sketch_oxide::frequency::HiddenSketch;
///
/// let mut hs = HiddenSketch::new(400, 3).unwrap();
/// // 100 distinct keys; key k has frequency k+1.
/// for k in 0..100u64 {
///     hs.insert_many(k, k + 1);
/// }
/// let mut recovered = hs.decode();
/// recovered.sort();
/// assert_eq!(recovered.len(), 100);
/// assert_eq!(recovered[7], (7, 8)); // key 7 occurred 8 times
/// ```
#[derive(Debug, Clone)]
pub struct HiddenSketch {
    /// Number of blocks (one cell per block per key).
    depth: usize,
    /// Cells per block.
    block_size: usize,
    cells: Vec<Cell>,
}

impl HiddenSketch {
    /// Creates a sketch of `num_cells` cells across `depth` blocks.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `depth == 0` or `num_cells < depth`.
    pub fn new(num_cells: usize, depth: usize) -> Result<Self> {
        if depth == 0 {
            return Err(SketchError::InvalidParameter {
                param: "depth".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        if num_cells < depth {
            return Err(SketchError::InvalidParameter {
                param: "num_cells".to_string(),
                value: num_cells.to_string(),
                constraint: "must be >= depth".to_string(),
            });
        }
        let block_size = num_cells / depth;
        Ok(Self {
            depth,
            block_size,
            cells: vec![Cell::default(); block_size * depth],
        })
    }

    /// Verification hash of a key.
    #[inline]
    fn verify(key: u64) -> i128 {
        xxhash(&key.to_le_bytes(), VERIFY_SEED) as i128
    }

    /// The `depth` cell indices of `key` (one per block).
    #[inline]
    fn cells_of(&self, key: u64) -> impl Iterator<Item = usize> + '_ {
        (0..self.depth).map(move |j| {
            let h = xxhash(&key.to_le_bytes(), POS_SEED_BASE.wrapping_add(j as u64));
            j * self.block_size + (h % self.block_size as u64) as usize
        })
    }

    /// Records one occurrence of `key`.
    pub fn insert(&mut self, key: u64) {
        self.insert_many(key, 1);
    }

    /// Records `count` occurrences of `key`.
    pub fn insert_many(&mut self, key: u64, count: u64) {
        let c = count as i128;
        let h = Self::verify(key);
        let positions: Vec<usize> = self.cells_of(key).collect();
        for idx in positions {
            let cell = &mut self.cells[idx];
            cell.count += count as i64;
            cell.key_sum += c * key as i128;
            cell.hash_sum += c * h;
        }
    }

    /// Count-Min upper-bound estimate of `key`'s frequency (the minimum cell count).
    pub fn estimate(&self, key: u64) -> u64 {
        self.cells_of(key)
            .map(|idx| self.cells[idx].count.max(0) as u64)
            .min()
            .unwrap_or(0)
    }

    /// Lists the recoverable `(key, frequency)` pairs by peeling pure cells. Best-effort: returns
    /// every pair it can invert (all of them when the distinct keys fit the table).
    pub fn decode(&self) -> Vec<(u64, u64)> {
        let mut cells = self.cells.clone();
        let mut recovered = Vec::new();
        loop {
            // Find a pure cell: a single distinct key explains its count, key_sum and hash_sum.
            let mut found = None;
            for cell in cells.iter() {
                if cell.count <= 0 {
                    continue;
                }
                let count = cell.count as i128;
                if cell.key_sum % count != 0 {
                    continue;
                }
                let k = cell.key_sum / count;
                if !(0..=u64::MAX as i128).contains(&k) {
                    continue;
                }
                let key = k as u64;
                if cell.hash_sum == count * Self::verify(key) {
                    found = Some((key, cell.count as u64));
                    break;
                }
            }
            match found {
                None => break,
                Some((key, freq)) => {
                    recovered.push((key, freq));
                    // Peel this key (with its full frequency) from all of its cells.
                    let c = freq as i128;
                    let h = Self::verify(key);
                    let positions: Vec<usize> = {
                        (0..self.depth)
                            .map(|j| {
                                let hh = xxhash(
                                    &key.to_le_bytes(),
                                    POS_SEED_BASE.wrapping_add(j as u64),
                                );
                                j * self.block_size + (hh % self.block_size as u64) as usize
                            })
                            .collect()
                    };
                    for idx in positions {
                        cells[idx].count -= freq as i64;
                        cells[idx].key_sum -= c * key as i128;
                        cells[idx].hash_sum -= c * h;
                    }
                }
            }
        }
        recovered
    }

    /// Number of blocks `depth`.
    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Total number of cells.
    #[inline]
    pub fn num_cells(&self) -> usize {
        self.cells.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn rejects_bad_params() {
        assert!(HiddenSketch::new(10, 0).is_err());
        assert!(HiddenSketch::new(2, 3).is_err());
        assert!(HiddenSketch::new(300, 3).is_ok());
    }

    #[test]
    fn recovers_all_keys_and_counts_at_low_load() {
        let mut hs = HiddenSketch::new(400, 3).unwrap();
        let mut truth: HashMap<u64, u64> = HashMap::new();
        for k in 0..100u64 {
            let freq = k + 1;
            hs.insert_many(k, freq);
            truth.insert(k, freq);
        }
        let recovered: HashMap<u64, u64> = hs.decode().into_iter().collect();
        assert_eq!(recovered, truth);
    }

    #[test]
    fn estimate_is_count_min_upper_bound() {
        let mut hs = HiddenSketch::new(400, 4).unwrap();
        for k in 0..50u64 {
            hs.insert_many(k, 10 + k);
        }
        for k in 0..50u64 {
            let est = hs.estimate(k);
            assert!(est >= 10 + k, "estimate {est} < truth {}", 10 + k);
        }
    }

    #[test]
    fn recovered_keys_are_always_correct_even_when_overloaded() {
        // Far more distinct keys than the table can fully peel: decode must never invent a wrong pair.
        let mut hs = HiddenSketch::new(200, 3).unwrap();
        let mut truth: HashMap<u64, u64> = HashMap::new();
        for k in 0..2000u64 {
            hs.insert_many(k, 1);
            truth.insert(k, 1);
        }
        for (key, freq) in hs.decode() {
            assert_eq!(
                truth.get(&key),
                Some(&freq),
                "bogus recovery ({key}, {freq})"
            );
        }
    }

    #[test]
    fn empty_decodes_to_nothing() {
        let hs = HiddenSketch::new(100, 4).unwrap();
        assert!(hs.decode().is_empty());
        assert_eq!(hs.estimate(7), 0);
    }
}
