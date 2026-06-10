//! Shared Exponential Histogram bucket engine (Datar et al., SODA 2002).
//!
//! This is the single bucket-maintenance core behind every count-based sliding
//! window in the library. [`ExponentialHistogram`](super::ExponentialHistogram) and
//! [`SlidingWindowCounter`](super::SlidingWindowCounter) are thin wrappers over it,
//! and it is the substrate intended for windowed sketches built on top of windowed
//! counting (ECM-Sketch, APBF-adjacent constructions, EH-of-sketch aggregation).
//!
//! # Algorithm
//!
//! Events are recorded as buckets whose counts are powers of two, newest first.
//! For each bucket size at most `k + 1 = ceil(1/epsilon) + 1` buckets are kept; when
//! that is exceeded the two oldest buckets of that size are merged into one bucket of
//! the next size up (keeping the older timestamp). A query over the window sums every
//! bucket fully inside the window plus half of the single bucket that straddles the
//! window's trailing edge — so the only error comes from that straddling bucket, and
//! the `l`-canonical form bounds the relative error by `epsilon`.

use crate::common::{Result, SketchError};

/// A single Exponential Histogram bucket. `count` is always a power of two.
#[derive(Clone, Debug)]
pub(crate) struct EhBucket {
    /// Timestamp at which this bucket's events were recorded.
    pub timestamp: u64,
    /// Number of events represented by this bucket (a power of two).
    pub count: u64,
}

/// Shared Exponential Histogram bucket engine.
///
/// Maintains the buckets, window size, error bound, and the derived per-level bucket
/// cap `k`. Wrappers add their own bookkeeping (e.g. a running total, a last-seen
/// timestamp) and presentation (bare estimate vs. estimate-with-bounds).
#[derive(Clone, Debug)]
pub(crate) struct EhCore {
    /// Buckets ordered newest-first by timestamp.
    pub buckets: Vec<EhBucket>,
    /// Window size in time units.
    pub window_size: u64,
    /// Relative error bound (epsilon), in (0, 1).
    pub epsilon: f64,
    /// Maximum buckets per level: `k = ceil(1/epsilon)`.
    pub k: usize,
}

impl EhCore {
    /// Creates a new engine, validating `window_size > 0` and `epsilon` in (0, 1).
    pub fn new(window_size: u64, epsilon: f64) -> Result<Self> {
        if window_size == 0 {
            return Err(SketchError::InvalidParameter {
                param: "window_size".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        if epsilon <= 0.0 || epsilon >= 1.0 {
            return Err(SketchError::InvalidParameter {
                param: "epsilon".to_string(),
                value: epsilon.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }

        let k = (1.0_f64 / epsilon).ceil() as usize;
        Ok(Self {
            buckets: Vec::new(),
            window_size,
            epsilon,
            k,
        })
    }

    /// Reconstructs an engine from already-validated parts (used by deserialization).
    ///
    /// `k` is recomputed from `epsilon` so it always matches the invariant, regardless
    /// of what a serialized blob claimed.
    pub fn from_parts(window_size: u64, epsilon: f64, buckets: Vec<EhBucket>) -> Self {
        let k = (1.0_f64 / epsilon).ceil() as usize;
        Self {
            buckets,
            window_size,
            epsilon,
            k,
        }
    }

    /// Records `count` events at `timestamp`, decomposing `count` into powers of two.
    pub fn insert(&mut self, timestamp: u64, count: u64) {
        if count == 0 {
            return;
        }

        let mut remaining = count;
        while remaining > 0 {
            let power = 63 - remaining.leading_zeros();
            let bucket_count = 1u64 << power;
            self.buckets.insert(
                0,
                EhBucket {
                    timestamp,
                    count: bucket_count,
                },
            );
            remaining -= bucket_count;
        }

        self.compress();
    }

    /// Compresses buckets to the `l`-canonical form: at most `k + 1` buckets per size.
    ///
    /// Robust to buckets not being grouped by size (e.g. after a merge), which is why
    /// it scans all buckets of a given size rather than only consecutive runs.
    pub fn compress(&mut self) {
        if self.buckets.len() < 2 {
            return;
        }

        let mut changed = true;
        while changed {
            changed = false;

            let mut i = 0;
            while i < self.buckets.len() {
                let current_count = self.buckets[i].count;

                // All buckets of this exact size (they may be scattered).
                let same_count_indices: Vec<usize> = (0..self.buckets.len())
                    .filter(|&j| self.buckets[j].count == current_count)
                    .collect();

                if same_count_indices.len() > self.k + 1 {
                    // Merge the two oldest buckets of this size into the next size up.
                    let mut indices_by_time: Vec<(usize, u64)> = same_count_indices
                        .iter()
                        .map(|&idx| (idx, self.buckets[idx].timestamp))
                        .collect();
                    indices_by_time.sort_by_key(|&(_, ts)| ts);

                    let oldest_idx = indices_by_time[0].0;
                    let second_oldest_idx = indices_by_time[1].0;

                    let older_timestamp = self.buckets[oldest_idx]
                        .timestamp
                        .min(self.buckets[second_oldest_idx].timestamp);
                    let merged_count = current_count * 2;

                    let (keep_idx, remove_idx) = if oldest_idx < second_oldest_idx {
                        (oldest_idx, second_oldest_idx)
                    } else {
                        (second_oldest_idx, oldest_idx)
                    };

                    self.buckets[keep_idx] = EhBucket {
                        timestamp: older_timestamp,
                        count: merged_count,
                    };
                    self.buckets.remove(remove_idx);

                    changed = true;
                    break; // Restart the scan; sizes shifted.
                }

                i += 1;
                while i < self.buckets.len() && self.buckets[i].count == current_count {
                    i += 1;
                }
            }
        }
    }

    /// Returns `(estimate, lower, upper)` for the window ending at `current_time`.
    ///
    /// `lower` counts only buckets fully inside the window; `upper` additionally counts
    /// the whole straddling bucket; `estimate` counts half of it.
    pub fn count_with_bounds(&self, current_time: u64) -> (u64, u64, u64) {
        if self.buckets.is_empty() {
            return (0, 0, 0);
        }

        let window_start = current_time.saturating_sub(self.window_size);

        let mut total = 0u64;
        let mut oldest_partial_count = 0u64;

        for bucket in &self.buckets {
            if bucket.timestamp > current_time {
                continue; // Future bucket.
            }
            if bucket.timestamp >= window_start {
                total += bucket.count;
            } else {
                // Oldest bucket straddling the trailing edge.
                oldest_partial_count = bucket.count;
                break;
            }
        }

        let estimate = total + oldest_partial_count / 2;
        let lower = total;
        let upper = total + oldest_partial_count;
        (estimate, lower, upper)
    }

    /// Approximate count of events with timestamp in `[start, end]`.
    ///
    /// Counts buckets fully inside the range plus half of the bucket straddling the
    /// `start` edge — the range-query analogue of [`count_with_bounds`](Self::count_with_bounds).
    pub fn count_range(&self, start: u64, end: u64) -> u64 {
        let mut total = 0u64;

        for bucket in &self.buckets {
            if bucket.timestamp > end {
                continue;
            }
            if bucket.timestamp < start {
                total += bucket.count / 2;
                break;
            }
            total += bucket.count;
        }

        total
    }

    /// Drops buckets entirely outside the window, keeping one straddling bucket.
    pub fn expire(&mut self, current_time: u64) {
        let window_start = current_time.saturating_sub(self.window_size);

        let mut found_outside = false;
        self.buckets.retain(|bucket| {
            if bucket.timestamp >= window_start {
                true
            } else if !found_outside {
                found_outside = true;
                true
            } else {
                false
            }
        });
    }

    /// Removes all buckets.
    pub fn clear(&mut self) {
        self.buckets.clear();
    }

    /// Number of buckets currently retained.
    #[inline]
    pub fn num_buckets(&self) -> usize {
        self.buckets.len()
    }

    /// Merges another engine's buckets into this one, then re-compresses.
    ///
    /// Requires identical `window_size` and `epsilon`. Merge is the union of the two
    /// multisets of buckets followed by canonical-form compression, so it is
    /// associative and commutative up to the histogram's `epsilon` guarantee.
    pub fn merge_from(&mut self, other: &EhCore) -> Result<()> {
        if self.window_size != other.window_size {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "Window size mismatch: {} vs {}",
                    self.window_size, other.window_size
                ),
            });
        }
        if (self.epsilon - other.epsilon).abs() > 1e-10 {
            return Err(SketchError::IncompatibleSketches {
                reason: format!("Epsilon mismatch: {} vs {}", self.epsilon, other.epsilon),
            });
        }

        self.buckets.extend(other.buckets.iter().cloned());
        // Keep newest-first ordering before compressing.
        self.buckets
            .sort_by_key(|bucket| std::cmp::Reverse(bucket.timestamp));
        self.compress();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_params_rejected() {
        assert!(EhCore::new(0, 0.1).is_err());
        assert!(EhCore::new(1000, 0.0).is_err());
        assert!(EhCore::new(1000, 1.0).is_err());
        assert!(EhCore::new(1000, -0.1).is_err());
        assert!(EhCore::new(1000, 0.1).is_ok());
    }

    #[test]
    fn bounds_bracket_estimate() {
        let mut core = EhCore::new(1000, 0.1).unwrap();
        for i in 0..100 {
            core.insert(i * 10, 1);
        }
        let (est, lo, hi) = core.count_with_bounds(1000);
        assert!(lo <= est && est <= hi);
    }

    #[test]
    fn compress_bounds_bucket_count() {
        let mut core = EhCore::new(10_000, 0.5).unwrap(); // k = 2
        for i in 0..100 {
            core.insert(i * 10, 1);
        }
        // O(k * log(count)) buckets, far fewer than 100.
        assert!(core.num_buckets() < 30, "got {}", core.num_buckets());
    }

    #[test]
    fn merge_requires_matching_config() {
        let mut a = EhCore::new(1000, 0.1).unwrap();
        let b = EhCore::new(500, 0.1).unwrap();
        assert!(a.merge_from(&b).is_err());
    }

    #[test]
    fn merge_is_union() {
        let mut a = EhCore::new(1000, 0.1).unwrap();
        a.insert(100, 1);
        a.insert(200, 1);
        let mut b = EhCore::new(1000, 0.1).unwrap();
        b.insert(300, 1);
        b.insert(400, 1);
        a.merge_from(&b).unwrap();
        let (est, _, _) = a.count_with_bounds(500);
        assert!(est >= 3, "got {est}");
    }
}
