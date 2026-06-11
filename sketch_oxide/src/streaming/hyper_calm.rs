//! HyperCalm — one-pass mining of *periodic batches* in data streams (Liu et al., ICDE 2023).
//!
//! A **batch** is a group of identical items arriving close together (within a *batch threshold* `T`
//! of one another); a **periodic batch** is several batches of the same item arriving at a roughly
//! fixed period. HyperCalm reports the top-`k` items by batch *periodicity* in one pass with `O(1)`
//! per-item work, via three cooperating components:
//!
//! 1. **HyperBloomFilter (HyperBF)** — a *time-aware* Bloom filter that detects the **start** of a
//!    batch. Every Bloom bit is widened to a 2-bit cell holding a cyclic *time slice* `1..=3`
//!    (`s = ⌊t/T⌋ mod 3 + 1`) or `0` (empty). An item maps to `d` cells across `d` arrays; on each
//!    access the *outdated* slice (`s mod 3 + 1`, i.e. the one ~2–3·`T` old) is cleaned to `0` within
//!    the cell's cache-line-sized block. If any of the item's `d` cells reads `0` after cleaning, the
//!    item has not been seen within `T` → a new batch starts. An **asynchronous timeline** gives each
//!    array its own offset so a gap straddling the fuzzy `T..2T` zone still spans three slices in some
//!    array. (Cleaning relies on the stream's background traffic touching blocks, as the paper notes.)
//! 2. **TimeRecorder** — a bounded LRU table mapping each item to its last batch time. On a new batch
//!    of item `e` at time `t`, it emits the batch interval `V = t − t_last` (the candidate period) and
//!    updates `t_last`; unseen items are stored, evicting the least-recently-used when full.
//! 3. **CalmSS (Calm Space-Saving)** — a top-`k` finder over entries `⟨e, V⟩`. A short **LRU queue**
//!    guards a **Space-Saving** summary: an entry is counted in the LRU until its count reaches a
//!    promotion threshold `P`, then it is promoted into Space-Saving (displacing the smallest entry as
//!    `(e, f_min + P)` when full). This keeps one-off cold entries out of the precious Space-Saving
//!    bins, sharply cutting over-estimation.
//!
//! `top_k_periodic` reports the `k` `⟨item, period⟩` pairs with the largest batch frequencies.

use crate::common::hash::xxhash;
use crate::common::{Result, SketchError};
use std::collections::HashMap;

const HBF_SEED: u64 = 0x4CA1_8F00_0000_0001;
const ROW_STRIDE: u64 = 0x9E37_79B9_7F4A_7C15;

/// Time-aware Bloom filter detecting batch starts.
#[derive(Debug, Clone)]
struct HyperBf {
    d: usize,
    m: usize,
    l: usize,
    t: u64,
    /// `d × m` cells, each holding a 2-bit value in `0..=3`.
    cells: Vec<u8>,
    /// Per-array timeline offsets (asynchronous timeline), evenly spread over `[0, T)`.
    offsets: Vec<u64>,
}

impl HyperBf {
    fn new(d: usize, m: usize, l: usize, t: u64) -> Self {
        let offsets = (0..d).map(|i| i as u64 * t / d as u64).collect();
        Self {
            d,
            m,
            l,
            t,
            cells: vec![0u8; d * m],
            offsets,
        }
    }

    /// Cyclic time slice `1..=3` for array `i` at time `time`.
    fn slice(&self, time: u64, i: usize) -> u8 {
        (((time + self.offsets[i]) / self.t) % 3) as u8 + 1
    }

    fn cell_index(&self, item: &[u8], i: usize) -> usize {
        let seed = HBF_SEED.wrapping_add((i as u64).wrapping_mul(ROW_STRIDE));
        (xxhash(item, seed) % self.m as u64) as usize
    }

    /// Records `item` at `time`; returns `true` if this is the start of a new batch.
    fn observe(&mut self, item: &[u8], time: u64) -> bool {
        let mut batch_start = false;
        // Per array: clean the cell's block of outdated slices, then test for a gap.
        for i in 0..self.d {
            let s = self.slice(time, i);
            let outdated = s % 3 + 1;
            let cell = self.cell_index(item, i);
            let block = cell / self.l;
            let start = i * self.m + block * self.l;
            let end = (start + self.l).min(i * self.m + self.m);
            for g in start..end {
                if self.cells[g] == outdated {
                    self.cells[g] = 0;
                }
            }
            if self.cells[i * self.m + cell] == 0 {
                batch_start = true;
            }
        }
        // Update all d hashed cells to the current slice.
        for i in 0..self.d {
            let s = self.slice(time, i);
            let cell = self.cell_index(item, i);
            self.cells[i * self.m + cell] = s;
        }
        batch_start
    }
}

/// A bounded LRU table: item → (value, access tick). Used both as TimeRecorder (value = last batch
/// time) and as the CalmSS guard queue (value = running count).
#[derive(Debug, Clone)]
struct LruTable<K> {
    cap: usize,
    map: HashMap<K, (u64, u64)>,
    tick: u64,
}

impl<K: std::hash::Hash + Eq + Clone> LruTable<K> {
    fn new(cap: usize) -> Self {
        Self {
            cap,
            map: HashMap::new(),
            tick: 0,
        }
    }

    fn evict_lru(&mut self) {
        if self.map.len() < self.cap {
            return;
        }
        if let Some(victim) = self
            .map
            .iter()
            .min_by_key(|(_, &(_, tk))| tk)
            .map(|(k, _)| k.clone())
        {
            self.map.remove(&victim);
        }
    }
}

/// Calm Space-Saving: an LRU guard queue feeding a Space-Saving summary.
#[derive(Debug, Clone)]
struct CalmSs {
    promotion: u64,
    ss_size: usize,
    lru: LruTable<(Vec<u8>, u64)>,
    /// Space-Saving summary: entry → estimated count.
    ss: HashMap<(Vec<u8>, u64), u64>,
}

impl CalmSs {
    fn new(lru_w: usize, promotion: u64, ss_size: usize) -> Self {
        Self {
            promotion,
            ss_size,
            lru: LruTable::new(lru_w),
            ss: HashMap::new(),
        }
    }

    fn insert(&mut self, entry: (Vec<u8>, u64)) {
        // 1) Already promoted: just increment.
        if let Some(c) = self.ss.get_mut(&entry) {
            *c += 1;
            return;
        }
        // 2) In the LRU guard: increment, and promote on reaching the threshold.
        if self.lru.map.contains_key(&entry) {
            self.lru.tick += 1;
            let promote = {
                let v = self.lru.map.get_mut(&entry).unwrap();
                v.0 += 1;
                v.1 = self.lru.tick;
                v.0 >= self.promotion
            };
            if promote {
                self.lru.map.remove(&entry);
                self.promote(entry, self.promotion);
            }
            return;
        }
        // 3) New entry: insert into the LRU guard (evicting the coldest if full).
        self.lru.evict_lru();
        self.lru.tick += 1;
        let tick = self.lru.tick;
        self.lru.map.insert(entry, (1, tick));
    }

    fn promote(&mut self, entry: (Vec<u8>, u64), add: u64) {
        if let Some(c) = self.ss.get_mut(&entry) {
            *c += add;
            return;
        }
        if self.ss.len() < self.ss_size {
            self.ss.insert(entry, add);
            return;
        }
        // Space-Saving replacement: the smallest entry yields its slot, keeping its count as the bias.
        if let Some((victim, f_min)) = self
            .ss
            .iter()
            .min_by_key(|(_, &c)| c)
            .map(|(k, &c)| (k.clone(), c))
        {
            self.ss.remove(&victim);
            self.ss.insert(entry, f_min + add);
        }
    }

    fn top_k(&self, k: usize) -> Vec<((Vec<u8>, u64), u64)> {
        let mut v: Vec<((Vec<u8>, u64), u64)> =
            self.ss.iter().map(|(e, &c)| (e.clone(), c)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        v.truncate(k);
        v
    }
}

/// A HyperCalm sketch reporting the top-`k` items by periodic-batch frequency.
///
/// # Example
/// ```
/// use sketch_oxide::streaming::HyperCalm;
///
/// // batch threshold T=6; HyperBF 4 arrays × 512 cells, blocks of 8; recorder 2048; CalmSS LRU 512,
/// // promotion 2, Space-Saving 256; report top-8.
/// let mut hc = HyperCalm::new(6, 4, 512, 8, 2048, 512, 2, 256, 8).unwrap();
///
/// // Time advances continuously; "A" forms a batch every 50 ticks, amid steady background traffic
/// // (which HyperBF relies on to clean its cells in time).
/// for t in 0..3000u64 {
///     if t % 50 == 0 {
///         hc.insert(b"A", t);
///     }
///     for j in 0..30u64 {
///         hc.insert(format!("noise-{t}-{j}").as_bytes(), t);
///     }
/// }
/// // "A" should surface as a periodic batch with period 50.
/// let top = hc.top_k_periodic();
/// assert!(top.iter().any(|((item, period), _)| item == b"A" && *period == 50));
/// ```
#[derive(Debug, Clone)]
pub struct HyperCalm {
    hbf: HyperBf,
    recorder: LruTable<Vec<u8>>,
    calmss: CalmSs,
    k: usize,
}

impl HyperCalm {
    /// Creates a HyperCalm sketch.
    ///
    /// * `t_threshold` — batch threshold `T` (same time unit as timestamps); two occurrences more
    ///   than `T` apart belong to different batches.
    /// * `d`, `m`, `l` — HyperBF arrays, 2-bit cells per array, and cells per (cache-line) block.
    /// * `recorder_cap` — TimeRecorder LRU capacity (distinct items whose last batch time is kept).
    /// * `lru_w`, `promotion`, `ss_size` — CalmSS guard-queue capacity, promotion threshold `P`, and
    ///   Space-Saving bins.
    /// * `k` — number of top periodic batches to report.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if any size is zero, `l > m`, or `t_threshold == 0`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        t_threshold: u64,
        d: usize,
        m: usize,
        l: usize,
        recorder_cap: usize,
        lru_w: usize,
        promotion: u64,
        ss_size: usize,
        k: usize,
    ) -> Result<Self> {
        let check = |cond: bool, param: &str, value: String, constraint: &str| {
            if cond {
                Ok(())
            } else {
                Err(SketchError::InvalidParameter {
                    param: param.to_string(),
                    value,
                    constraint: constraint.to_string(),
                })
            }
        };
        check(
            t_threshold >= 1,
            "t_threshold",
            t_threshold.to_string(),
            "must be >= 1",
        )?;
        check(d >= 1, "d", d.to_string(), "must be >= 1")?;
        check(m >= 1, "m", m.to_string(), "must be >= 1")?;
        check(l >= 1 && l <= m, "l", l.to_string(), "must be in 1..=m")?;
        check(
            recorder_cap >= 1,
            "recorder_cap",
            recorder_cap.to_string(),
            "must be >= 1",
        )?;
        check(lru_w >= 1, "lru_w", lru_w.to_string(), "must be >= 1")?;
        check(
            promotion >= 1,
            "promotion",
            promotion.to_string(),
            "must be >= 1",
        )?;
        check(ss_size >= 1, "ss_size", ss_size.to_string(), "must be >= 1")?;
        check(k >= 1, "k", k.to_string(), "must be >= 1")?;
        Ok(Self {
            hbf: HyperBf::new(d, m, l, t_threshold),
            recorder: LruTable::new(recorder_cap),
            calmss: CalmSs::new(lru_w, promotion, ss_size),
            k,
        })
    }

    /// Processes one occurrence of `item` at `timestamp`. On a detected batch start with a known
    /// previous batch, the resulting `⟨item, period⟩` entry is fed to CalmSS.
    pub fn insert(&mut self, item: &[u8], timestamp: u64) {
        if !self.hbf.observe(item, timestamp) {
            return;
        }
        // Batch start: record its time and, if a previous batch exists, emit the period to CalmSS.
        self.recorder.tick += 1;
        let tick = self.recorder.tick;
        if let Some(prev) = self.recorder.map.get_mut(item) {
            let period = timestamp.saturating_sub(prev.0);
            prev.0 = timestamp;
            prev.1 = tick;
            self.calmss.insert((item.to_vec(), period));
        } else {
            self.recorder.evict_lru();
            self.recorder.map.insert(item.to_vec(), (timestamp, tick));
        }
    }

    /// Reports up to `k` `⟨item, period⟩` pairs with the largest periodic-batch frequencies, sorted
    /// by descending frequency (ties broken by entry). One item may appear under several periods.
    pub fn top_k_periodic(&self) -> Vec<((Vec<u8>, u64), u64)> {
        self.calmss.top_k(self.k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(HyperCalm::new(0, 4, 64, 8, 64, 64, 2, 32, 4).is_err());
        assert!(HyperCalm::new(4, 0, 64, 8, 64, 64, 2, 32, 4).is_err());
        assert!(HyperCalm::new(4, 4, 64, 128, 64, 64, 2, 32, 4).is_err()); // l > m
        assert!(HyperCalm::new(4, 4, 64, 8, 64, 64, 2, 32, 0).is_err()); // k = 0
        assert!(HyperCalm::new(4, 4, 64, 8, 64, 64, 2, 32, 4).is_ok());
    }

    #[test]
    fn empty_reports_nothing() {
        let hc = HyperCalm::new(4, 4, 64, 8, 64, 64, 2, 32, 4).unwrap();
        assert!(hc.top_k_periodic().is_empty());
    }

    #[test]
    fn hyperbf_detects_gaps_with_background_traffic() {
        // With steady background traffic cleaning the cells, a return after a long gap is a new batch,
        // while a quick re-appearance within T is not.
        let mut hbf = HyperBf::new(4, 512, 8, 6);
        // First sighting → batch start.
        assert!(hbf.observe(b"x", 0));
        // Background traffic over the next interval (touches blocks so cleaning happens).
        for t in 0..60u64 {
            for j in 0..40u64 {
                hbf.observe(format!("bg-{t}-{j}").as_bytes(), t);
            }
        }
        // "x" returns far later → its cells were cleaned → batch start again.
        assert!(hbf.observe(b"x", 60));
        // Immediate re-appearance within T → same batch, not a start.
        assert!(!hbf.observe(b"x", 61));
    }

    fn run_periodic(period_a: u64, period_b: u64) -> Vec<((Vec<u8>, u64), u64)> {
        let mut hc = HyperCalm::new(6, 4, 512, 8, 2048, 512, 2, 256, 8).unwrap();
        let rounds = 60u64;
        let horizon = rounds * period_a.max(period_b);
        let mut t = 0u64;
        while t <= horizon {
            // Periodic batches (a short burst each, all within T).
            if t % period_a == 0 {
                hc.insert(b"A", t);
                hc.insert(b"A", t + 1);
            }
            if t % period_b == 0 {
                hc.insert(b"B", t);
                hc.insert(b"B", t + 1);
            }
            // Heavy background traffic so HyperBF cleans cells in time.
            for j in 0..30u64 {
                hc.insert(format!("n-{t}-{j}").as_bytes(), t);
            }
            t += 1;
        }
        hc.top_k_periodic()
    }

    #[test]
    fn finds_periodic_batches() {
        let top = run_periodic(40, 56);
        assert!(
            top.iter()
                .any(|((item, period), _)| item == b"A" && *period == 40),
            "expected (A, 40) in {top:?}"
        );
        assert!(
            top.iter()
                .any(|((item, period), _)| item == b"B" && *period == 56),
            "expected (B, 56) in {top:?}"
        );
    }

    #[test]
    fn periodic_items_outrank_noise() {
        // The two periodic flows should be the highest-frequency reported entries.
        let top = run_periodic(40, 56);
        assert!(top.len() >= 2);
        let top_items: std::collections::HashSet<&[u8]> = top
            .iter()
            .take(2)
            .map(|((item, _), _)| item.as_slice())
            .collect();
        assert!(
            top_items.contains(b"A".as_slice()),
            "A not in top-2: {top:?}"
        );
        assert!(
            top_items.contains(b"B".as_slice()),
            "B not in top-2: {top:?}"
        );
    }
}
