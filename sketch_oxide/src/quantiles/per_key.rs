//! Per-key quantiles — quantile summaries for the heavy-hitter keys of a keyed stream.
//!
//! Many monitoring tasks need not one global quantile but a quantile *per key* — the p99 latency of
//! each API endpoint, the median packet size per flow. Tracking an exact summary for every key is
//! impossible at scale, so SQUAD (Shahout, Ben-Basat, ... "SQUAD: Combining Sketching and Sampling Is
//! Better than Either for Per-item Quantile Estimation", 2023) and its predecessors keep summaries
//! only for the **heavy hitters**: a capacity-bounded set of monitored keys (when full, the
//! least-frequently-used key is evicted) with a compact quantile summary per key.
//!
//! Here each monitored key carries a Greenwald–Khanna
//! [`GreenwaldKhanna`](crate::quantiles::GreenwaldKhanna) summary, giving deterministic `±εn`
//! per-key rank error. SQUAD's per-key sampling buffer (which lowers the constant for very skewed
//! keys) is a documented follow-up over this composition.

use crate::common::{Result, SketchError};
use crate::quantiles::GreenwaldKhanna;
use std::collections::HashMap;
use std::hash::Hash;

/// Per-key quantile estimation over keys of type `T`, tracking up to `capacity` heavy keys.
///
/// # Example
/// ```
/// use sketch_oxide::quantiles::PerKeyQuantiles;
///
/// let mut pkq = PerKeyQuantiles::new(8, 0.01).unwrap();
/// for v in 0..1000 { pkq.update("api/a", v as f64); }     // a's latencies 0..1000
/// for v in 2000..3000 { pkq.update("api/b", v as f64); }  // b's latencies 2000..3000
///
/// assert!((pkq.quantile(&"api/a", 0.5).unwrap() - 500.0).abs() < 50.0);
/// assert!((pkq.quantile(&"api/b", 0.5).unwrap() - 2500.0).abs() < 50.0);
/// ```
#[derive(Debug, Clone)]
pub struct PerKeyQuantiles<T: Hash + Eq + Clone> {
    capacity: usize,
    epsilon: f64,
    /// Monitored keys: each holds a quantile summary and an activity count.
    monitored: HashMap<T, (GreenwaldKhanna, u64)>,
}

impl<T: Hash + Eq + Clone> PerKeyQuantiles<T> {
    /// Creates a tracker for up to `capacity` keys, each with `epsilon`-accurate quantiles.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `capacity` is 0 or `epsilon` is not in `(0, 1)`.
    pub fn new(capacity: usize, epsilon: f64) -> Result<Self> {
        if capacity == 0 {
            return Err(SketchError::InvalidParameter {
                param: "capacity".to_string(),
                value: "0".to_string(),
                constraint: "must be > 0".to_string(),
            });
        }
        if !(epsilon > 0.0 && epsilon < 1.0) {
            return Err(SketchError::InvalidParameter {
                param: "epsilon".to_string(),
                value: epsilon.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }
        Ok(Self {
            capacity,
            epsilon,
            monitored: HashMap::new(),
        })
    }

    /// Records `value` for `key`.
    pub fn update(&mut self, key: T, value: f64) {
        if let Some((gk, count)) = self.monitored.get_mut(&key) {
            gk.insert(value);
            *count += 1;
            return;
        }
        if self.monitored.len() < self.capacity {
            let mut gk = GreenwaldKhanna::new(self.epsilon).expect("epsilon validated");
            gk.insert(value);
            self.monitored.insert(key, (gk, 1));
            return;
        }
        // Full: evict the least-frequently-used key so established heavy keys keep their summaries
        // under churn. (Space-Saving's count-inheritance is avoided here because it would constantly
        // reset a promoted key's quantile summary.)
        let min_key = self
            .monitored
            .iter()
            .min_by_key(|(_, (_, c))| *c)
            .map(|(k, _)| k.clone())
            .expect("non-empty when full");
        self.monitored.remove(&min_key);
        let mut gk = GreenwaldKhanna::new(self.epsilon).expect("epsilon validated");
        gk.insert(value);
        self.monitored.insert(key, (gk, 1));
    }

    /// Estimated `phi`-quantile of `key`'s values, or `None` if the key is not monitored.
    pub fn quantile(&self, key: &T, phi: f64) -> Option<f64> {
        self.monitored.get(key).and_then(|(gk, _)| gk.quantile(phi))
    }

    /// Activity count of `key` (0 if not monitored).
    pub fn count(&self, key: &T) -> u64 {
        self.monitored.get(key).map_or(0, |(_, c)| *c)
    }

    /// Whether `key` is currently monitored.
    pub fn is_monitored(&self, key: &T) -> bool {
        self.monitored.contains_key(key)
    }

    /// Number of monitored keys.
    #[inline]
    pub fn num_keys(&self) -> usize {
        self.monitored.len()
    }

    /// Maximum number of monitored keys.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_params() {
        assert!(PerKeyQuantiles::<u64>::new(0, 0.01).is_err());
        assert!(PerKeyQuantiles::<u64>::new(8, 0.0).is_err());
        assert!(PerKeyQuantiles::<u64>::new(8, 1.0).is_err());
        assert!(PerKeyQuantiles::<u64>::new(8, 0.01).is_ok());
    }

    #[test]
    fn tracks_per_key_medians_independently() {
        let mut pkq = PerKeyQuantiles::new(8, 0.01).unwrap();
        for v in 0..1000u64 {
            pkq.update("a", v as f64);
        }
        for v in 2000..3000u64 {
            pkq.update("b", v as f64);
        }
        assert!((pkq.quantile(&"a", 0.5).unwrap() - 500.0).abs() < 50.0);
        assert!((pkq.quantile(&"b", 0.5).unwrap() - 2500.0).abs() < 50.0);
    }

    #[test]
    fn per_key_percentiles() {
        let mut pkq = PerKeyQuantiles::new(4, 0.005).unwrap();
        for v in 0..10_000u64 {
            pkq.update("k", v as f64);
        }
        assert!((pkq.quantile(&"k", 0.95).unwrap() - 9500.0).abs() < 200.0);
        assert!((pkq.quantile(&"k", 0.1).unwrap() - 1000.0).abs() < 200.0);
        assert_eq!(pkq.count(&"k"), 10_000);
    }

    #[test]
    fn unmonitored_key_returns_none() {
        let pkq = PerKeyQuantiles::<&str>::new(4, 0.01).unwrap();
        assert!(pkq.quantile(&"missing", 0.5).is_none());
        assert!(!pkq.is_monitored(&"missing"));
    }

    #[test]
    fn capacity_evicts_least_active() {
        let mut pkq = PerKeyQuantiles::new(2, 0.05).unwrap();
        // Two heavy keys, then a flood of one-shot light keys.
        for v in 0..500u64 {
            pkq.update(0u64, v as f64);
            pkq.update(1u64, v as f64);
        }
        for k in 100..200u64 {
            pkq.update(k, 1.0); // light keys
        }
        assert!(pkq.num_keys() <= 2);
        // The heavy keys should survive the churn.
        assert!(pkq.is_monitored(&0) || pkq.is_monitored(&1));
    }
}
