//! Deterministic Wave — `ε`-approximate Basic Counting over a sliding window (Gibbons & Tirthapura,
//! "Distributed Streams Algorithms for Sliding Windows", SPAA 2002).
//!
//! *Basic Counting* asks for the number of 1-bits among the last `n` items of a stream. The
//! Exponential Histogram (Datar et al.) solves it in `O((1/ε)·log N)` space but with `O(log N)`
//! worst-case per-item time (a cascade of bucket merges). The **wave** synopsis matches EH's space and
//! `O(1)` query while avoiding the cascade — and underlies distributed and distinct-count extensions.
//!
//! A wave keeps the positions of recent 1-bits at `⌈log₂(2εN)⌉` **levels**: level `i` stores the
//! `1/ε + 1` most recent 1-bits whose **1-rank** (their index among all 1-bits) is a multiple of `2^i`.
//! Coarser levels reach exponentially further back with the same `1/ε + 1` budget, so the wave always
//! brackets any window of size `≤ N`.
//!
//! * **Update**: on a 0-bit, just advance the position. On a 1-bit (new rank `r`), append `(pos, r)`
//!   to every level `i` for which `r ≡ 0 (mod 2^i)`, keeping each level to its `1/ε + 1` most recent
//!   entries (the cap alone bounds the space).
//! * **Query** `n`: with window start `s = pos − n + 1`, let `r₁` be the 1-rank of the latest stored
//!   position before `s` and `r₂` the earliest stored position at or after `s`; estimate
//!   `rank − r̄ + 1` where `r̄ = r₂` if `r₂ − r₁ = 1` (exact boundary) else `(r₁ + r₂)/2`. The relative
//!   error is `< ε`.
//!
//! This is the *basic* wave (clear, `O((1/ε)·log εN)` space); the paper's §3.2 refinement (store each
//! position only at its top level, with modulo-`N` counters) attains `O(1)` worst-case update.

use crate::common::{Result, SketchError};
use std::collections::VecDeque;

/// A deterministic wave for `ε`-approximate Basic Counting over windows up to `N`.
///
/// # Example
/// ```
/// use sketch_oxide::streaming::DeterministicWave;
///
/// // Max window 1024, relative error 10%.
/// let mut w = DeterministicWave::new(1024, 0.1).unwrap();
/// // Feed a stream: a 1 every 3rd position for 600 positions (≈200 ones).
/// for i in 0..600u64 {
///     w.update(i % 3 == 0);
/// }
/// // Estimate the ones in the most recent 300 positions (true ≈ 100).
/// let est = w.estimate(300);
/// assert!((est as f64 - 100.0).abs() <= 0.1 * 100.0 + 2.0, "est {est}");
/// ```
#[derive(Debug, Clone)]
pub struct DeterministicWave {
    n: u64,                            // maximum window size N
    cap: usize,                        // 1/ε + 1 entries per level
    levels: Vec<VecDeque<(u64, u64)>>, // levels[i]: FIFO of (position, 1-rank)
    pos: u64,                          // current stream length
    rank: u64,                         // current number of 1-bits
}

impl DeterministicWave {
    /// Creates a wave supporting windows up to `max_window` (`N`) with relative error `epsilon`
    /// (`0 < epsilon < 1`).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `max_window == 0` or `epsilon` is not in `(0, 1)`.
    pub fn new(max_window: u64, epsilon: f64) -> Result<Self> {
        if max_window == 0 {
            return Err(SketchError::InvalidParameter {
                param: "max_window".to_string(),
                value: "0".to_string(),
                constraint: "must be >= 1".to_string(),
            });
        }
        if !epsilon.is_finite() || epsilon <= 0.0 || epsilon >= 1.0 {
            return Err(SketchError::InvalidParameter {
                param: "epsilon".to_string(),
                value: epsilon.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }
        let cap = (1.0 / epsilon).ceil() as usize + 1;
        let num_levels = ((2.0 * epsilon * max_window as f64).log2().ceil() as i64).max(1) as usize;
        Ok(Self {
            n: max_window,
            cap,
            levels: vec![VecDeque::new(); num_levels],
            pos: 0,
            rank: 0,
        })
    }

    /// Processes the next stream item: a 1-bit (`true`) or 0-bit (`false`).
    pub fn update(&mut self, bit: bool) {
        self.pos += 1;
        if bit {
            self.rank += 1;
            // The new 1-bit belongs to every level i with rank ≡ 0 (mod 2^i): i = 0..=trailing_zeros.
            let top = (self.rank.trailing_zeros() as usize).min(self.levels.len() - 1);
            for level in self.levels.iter_mut().take(top + 1) {
                level.push_back((self.pos, self.rank));
                if level.len() > self.cap {
                    level.pop_front();
                }
            }
        }
        // The per-level `1/ε + 1` cap already bounds the entry count; the coarsest level reaches back
        // ≥ N ranks (hence ≥ N positions), so it always brackets any window ≤ N. (Position-value
        // expiry + modulo-N counters is the paper's §3.2 bit-bounding refinement, omitted here.)
    }

    /// Estimates the number of 1-bits among the last `window` items (`window` is clamped to `N`).
    pub fn estimate(&self, window: u64) -> u64 {
        if self.rank == 0 || window == 0 {
            return 0;
        }
        let window = window.min(self.n);
        if window >= self.pos {
            return self.rank; // the whole stream is inside the window
        }
        let s = self.pos - window + 1; // window is positions [s, pos]

        // Bracket s: r1 = rank of the latest stored position < s; r2 = rank of the earliest stored
        // position >= s.
        let mut p1: Option<(u64, u64)> = None; // (position, rank), maximal position < s
        let mut p2: Option<(u64, u64)> = None; // (position, rank), minimal position >= s
        for level in &self.levels {
            for &(p, r) in level {
                if p < s {
                    if p1.is_none_or(|(bp, _)| p > bp) {
                        p1 = Some((p, r));
                    }
                } else if p2.is_none_or(|(bp, _)| p < bp) {
                    p2 = Some((p, r));
                }
            }
        }

        match (p1, p2) {
            // No stored 1-bit lies inside the window.
            (_, None) => 0,
            // Every stored 1-bit is inside the window (initial transient): exact count from r2.
            (None, Some((_, r2))) => self.rank - r2 + 1,
            (Some((_, r1)), Some((_, r2))) => {
                let r_bar = if r2 - r1 == 1 { r2 } else { (r1 + r2) / 2 };
                self.rank + 1 - r_bar
            }
        }
    }

    /// Current stream length (number of items processed).
    #[inline]
    pub fn position(&self) -> u64 {
        self.pos
    }

    /// Total number of 1-bits seen so far.
    #[inline]
    pub fn ones(&self) -> u64 {
        self.rank
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Brute-force count of 1-bits among the last `window` of `bits`.
    fn true_count(bits: &[bool], window: usize) -> u64 {
        let start = bits.len().saturating_sub(window);
        bits[start..].iter().filter(|&&b| b).count() as u64
    }

    #[test]
    fn rejects_bad_params() {
        assert!(DeterministicWave::new(0, 0.1).is_err());
        assert!(DeterministicWave::new(1024, 0.0).is_err());
        assert!(DeterministicWave::new(1024, 1.0).is_err());
        assert!(DeterministicWave::new(1024, -0.1).is_err());
        assert!(DeterministicWave::new(1024, 0.1).is_ok());
    }

    #[test]
    fn exact_on_small_transient_stream() {
        // Few 1-bits ⇒ the wave stores them all ⇒ estimates are exact.
        let bits = [
            true, false, true, true, false, false, true, false, true, true,
        ];
        let mut w = DeterministicWave::new(64, 0.25).unwrap();
        for &b in &bits {
            w.update(b);
        }
        for window in 1..=bits.len() {
            assert_eq!(
                w.estimate(window as u64),
                true_count(&bits, window),
                "window {window}"
            );
        }
    }

    #[test]
    fn within_relative_error_on_long_stream() {
        let n = 1024u64;
        let eps = 0.1;
        let mut w = DeterministicWave::new(n, eps).unwrap();
        let mut bits = Vec::new();
        // A deterministic pseudo-random bit pattern.
        let mut state = 0x2545_F491_4F6C_DD1Du64;
        for _ in 0..5000u64 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let b = state & 7 != 0; // ~7/8 density
            bits.push(b);
            w.update(b);
        }
        for &window in &[16u64, 64, 256, 512, 1024] {
            let est = w.estimate(window) as f64;
            let truth = true_count(&bits, window as usize) as f64;
            assert!(
                (est - truth).abs() <= eps * truth + 2.0,
                "window {window}: est {est}, truth {truth}, error {}",
                (est - truth).abs()
            );
        }
    }

    #[test]
    fn all_zeros_and_all_ones() {
        let mut z = DeterministicWave::new(256, 0.1).unwrap();
        for _ in 0..500 {
            z.update(false);
        }
        assert_eq!(z.estimate(256), 0);

        let mut o = DeterministicWave::new(256, 0.1).unwrap();
        for _ in 0..500 {
            o.update(true);
        }
        // Window of 100 over an all-ones stream ⇒ ~100 ones.
        let est = o.estimate(100) as f64;
        assert!((est - 100.0).abs() <= 0.1 * 100.0 + 2.0, "est {est}");
    }

    #[test]
    fn window_covering_whole_stream_is_exact() {
        let mut w = DeterministicWave::new(1024, 0.1).unwrap();
        let mut ones = 0u64;
        for i in 0..200u64 {
            let b = i % 2 == 0;
            if b {
                ones += 1;
            }
            w.update(b);
        }
        // A window larger than the stream length returns the exact total.
        assert_eq!(w.estimate(1024), ones);
        assert_eq!(w.ones(), ones);
        assert_eq!(w.position(), 200);
    }
}

// ---------------------------------------------------------------------------
// Capability-trait adoptions (fable5 doc 01 F3 "split the `Sketch` trait").
// DeterministicWave ingests a stream of bits via `update(bool)`, so it
// satisfies `Update<bool>`. Its `estimate(window)` requires a window argument
// (it is a windowed bit-counter, not a per-key point query or set cardinality),
// so `PointQuery`/`CardinalityEstimate` are skipped.
// ---------------------------------------------------------------------------
use crate::common::Update;

impl Update<bool> for DeterministicWave {
    fn update(&mut self, item: &bool) {
        DeterministicWave::update(self, *item);
    }
}
