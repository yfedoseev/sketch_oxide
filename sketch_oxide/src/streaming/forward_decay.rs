//! Forward decay — time-weighted counts and averages that emphasise recent data.
//!
//! Backward decay (decay weights measured back from *now*) forces every stored item to be
//! re-aged on each query. **Forward decay** (Cormode, Shkapenyuk, Srivastava & Xu, ICDE
//! 2009) instead measures age forward from a fixed *landmark* `L`: an item seen at time
//! `t_i` gets weight `g(t_i − L)`, and a query at time `t` normalises by `g(t − L)`, so the
//! effective weight is `g(t_i − L) / g(t − L)`. Only running sums are kept; nothing is
//! re-aged per item.
//!
//! This implements **exponential** forward decay, `g(x) = e^{rate·x}`. With the landmark
//! pinned to the most recent timestamp, the accumulators stay bounded (≤ the item count),
//! so there is no overflow even on unbounded streams. Aggregates retrofit any value stream:
//! a decayed count, a decayed sum, and a decay-invariant average (the landmark cancels).

use crate::common::{Result, SketchError};

/// Exponential forward-decay aggregator over a `(value, timestamp)` stream.
///
/// # Example
/// ```
/// use sketch_oxide::streaming::ForwardDecay;
///
/// // Decay rate 0.1 per time unit.
/// let mut d = ForwardDecay::new(0.1).unwrap();
/// for t in 0..100u64 {
///     d.update(1.0, t);
/// }
/// // Decayed count right after the last event ≈ geometric sum ≈ 1/(1-e^-0.1) ≈ 10.5.
/// let c = d.decayed_count(99);
/// assert!(c > 8.0 && c < 12.0, "decayed count {c}");
/// ```
#[derive(Debug, Clone)]
pub struct ForwardDecay {
    rate: f64,
    /// The current landmark = the most recent timestamp folded in.
    landmark: u64,
    /// Σ of decayed weights relative to `landmark` (≤ number of items).
    weight: f64,
    /// Σ of value·weight relative to `landmark`.
    weighted_sum: f64,
    /// Raw number of updates.
    count: u64,
}

impl ForwardDecay {
    /// Creates an aggregator with exponential decay `rate` (per timestamp unit). Larger
    /// `rate` forgets faster.
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `rate` is not finite and positive.
    pub fn new(rate: f64) -> Result<Self> {
        if !rate.is_finite() || rate <= 0.0 {
            return Err(SketchError::InvalidParameter {
                param: "rate".to_string(),
                value: rate.to_string(),
                constraint: "must be a finite value > 0".to_string(),
            });
        }
        Ok(Self {
            rate,
            landmark: 0,
            weight: 0.0,
            weighted_sum: 0.0,
            count: 0,
        })
    }

    /// Records `value` observed at `timestamp`.
    ///
    /// In-order timestamps advance the landmark (rescaling existing sums down so they stay
    /// bounded); an out-of-order (older) timestamp is folded in at its reduced weight without
    /// moving the landmark.
    pub fn update(&mut self, value: f64, timestamp: u64) {
        self.count += 1;
        if self.weight == 0.0 {
            // First item becomes the landmark with weight 1.
            self.landmark = timestamp;
            self.weight = 1.0;
            self.weighted_sum = value;
            return;
        }

        if timestamp >= self.landmark {
            // Rebase existing sums to the new landmark, then add the new item (weight 1).
            let decay = (-self.rate * (timestamp - self.landmark) as f64).exp();
            self.weight = self.weight * decay + 1.0;
            self.weighted_sum = self.weighted_sum * decay + value;
            self.landmark = timestamp;
        } else {
            // Older item: weight relative to the current landmark, landmark unchanged.
            let w = (-self.rate * (self.landmark - timestamp) as f64).exp();
            self.weight += w;
            self.weighted_sum += value * w;
        }
    }

    /// Decayed count as of `now` (`now ≥` the landmark): the sum of item weights measured
    /// forward to `now`. Equals the raw weight at the landmark and decays as `now` advances.
    pub fn decayed_count(&self, now: u64) -> f64 {
        self.weight * self.normalizer(now)
    }

    /// Decayed sum of values as of `now`.
    pub fn decayed_sum(&self, now: u64) -> f64 {
        self.weighted_sum * self.normalizer(now)
    }

    /// Decay-weighted average value. Independent of `now` and of the landmark (the
    /// normaliser cancels). `None` if no items have been recorded.
    pub fn average(&self) -> Option<f64> {
        if self.weight == 0.0 {
            None
        } else {
            Some(self.weighted_sum / self.weight)
        }
    }

    /// Total number of updates (undecayed).
    pub fn count(&self) -> u64 {
        self.count
    }

    /// `g(landmark − now) = e^{-rate·(now − landmark)}` for `now ≥ landmark` (else 1).
    fn normalizer(&self, now: u64) -> f64 {
        if now >= self.landmark {
            (-self.rate * (now - self.landmark) as f64).exp()
        } else {
            1.0
        }
    }
}

/// Polynomial forward-decay aggregator, `g(x) = (x − landmark)^β`.
///
/// Unlike the exponential variant, polynomial decay uses a **fixed** landmark (e.g. the
/// stream's start), so weights grow only polynomially in time — recent data is emphasised
/// gently and old data never fully vanishes. Good for "weight by recency but keep history"
/// aggregates. Accumulators grow as `t^β`; for very long streams with large `β` prefer the
/// exponential variant to avoid overflow.
#[derive(Debug, Clone)]
pub struct PolynomialForwardDecay {
    beta: f64,
    landmark: u64,
    weight: f64,
    weighted_sum: f64,
    count: u64,
}

impl PolynomialForwardDecay {
    /// Creates a polynomial-decay aggregator with exponent `beta` and origin `landmark`
    /// (the time from which age is measured; usually the stream start).
    ///
    /// # Errors
    /// [`SketchError::InvalidParameter`] if `beta` is not finite and positive.
    pub fn new(beta: f64, landmark: u64) -> Result<Self> {
        if !beta.is_finite() || beta <= 0.0 {
            return Err(SketchError::InvalidParameter {
                param: "beta".to_string(),
                value: beta.to_string(),
                constraint: "must be a finite value > 0".to_string(),
            });
        }
        Ok(Self {
            beta,
            landmark,
            weight: 0.0,
            weighted_sum: 0.0,
            count: 0,
        })
    }

    /// Records `value` observed at `timestamp` (must be `>=` the landmark; earlier
    /// timestamps are recorded at the landmark, weight `0`).
    pub fn update(&mut self, value: f64, timestamp: u64) {
        self.count += 1;
        let age = timestamp.saturating_sub(self.landmark) as f64;
        let w = age.powf(self.beta);
        self.weight += w;
        self.weighted_sum += value * w;
    }

    /// Decayed count as of `now`: `Σ g(t_i − L) / g(now − L)`.
    pub fn decayed_count(&self, now: u64) -> f64 {
        let denom = (now.saturating_sub(self.landmark) as f64).powf(self.beta);
        if denom == 0.0 {
            0.0
        } else {
            self.weight / denom
        }
    }

    /// Decay-weighted average value (landmark-independent). `None` if empty or all weights
    /// are zero (e.g. only the landmark instant was observed).
    pub fn average(&self) -> Option<f64> {
        if self.weight == 0.0 {
            None
        } else {
            Some(self.weighted_sum / self.weight)
        }
    }

    /// Total number of updates (undecayed).
    pub fn count(&self) -> u64 {
        self.count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_rate() {
        assert!(ForwardDecay::new(0.0).is_err());
        assert!(ForwardDecay::new(-1.0).is_err());
        assert!(ForwardDecay::new(f64::NAN).is_err());
        assert!(ForwardDecay::new(0.1).is_ok());
    }

    #[test]
    fn decayed_count_is_geometric_sum() {
        let mut d = ForwardDecay::new(0.1).unwrap();
        for t in 0..1000u64 {
            d.update(1.0, t);
        }
        // Σ e^{-0.1 k} for k=0.. ≈ 1/(1 - e^-0.1) ≈ 10.508.
        let c = d.decayed_count(999);
        assert!((c - 10.508).abs() < 0.5, "decayed count {c}");
    }

    #[test]
    fn count_decays_as_time_advances() {
        let mut d = ForwardDecay::new(0.5).unwrap();
        d.update(1.0, 0);
        let now0 = d.decayed_count(0);
        let now10 = d.decayed_count(10);
        assert!(now10 < now0, "count should decay: {now10} < {now0}");
        assert!((now10 - (-5.0_f64).exp()).abs() < 1e-9, "e^{{-0.5*10}}");
    }

    #[test]
    fn average_is_decay_invariant() {
        let mut d = ForwardDecay::new(0.2).unwrap();
        // Recent values dominate the average.
        d.update(0.0, 0);
        d.update(100.0, 100);
        let avg = d.average().unwrap();
        // The value 100 at the landmark dominates the heavily-decayed 0.
        assert!(avg > 99.0, "recent-weighted average {avg}");
    }

    #[test]
    fn recent_items_weigh_more_than_old() {
        let mut d = ForwardDecay::new(0.3).unwrap();
        d.update(1.0, 0); // old
        d.update(1.0, 100); // recent (landmark)
                            // At the landmark, the recent item has weight 1, the old item ~e^{-30} ≈ 0.
        let c = d.decayed_count(100);
        assert!((c - 1.0).abs() < 0.01, "recent dominates: count {c}");
    }

    #[test]
    fn out_of_order_older_item_contributes_less() {
        let mut d = ForwardDecay::new(0.3).unwrap();
        d.update(1.0, 100); // landmark at 100
        let before = d.decayed_count(100);
        d.update(1.0, 50); // older, out of order
        let after = d.decayed_count(100);
        // The older item adds a small (< 1) weight without moving the landmark.
        assert!(
            after > before && after < before + 1.0,
            "before {before} after {after}"
        );
    }

    #[test]
    fn polynomial_rejects_bad_beta() {
        assert!(PolynomialForwardDecay::new(0.0, 0).is_err());
        assert!(PolynomialForwardDecay::new(-1.0, 0).is_err());
        assert!(PolynomialForwardDecay::new(2.0, 0).is_ok());
    }

    #[test]
    fn polynomial_recent_weighs_more() {
        let mut d = PolynomialForwardDecay::new(2.0, 0).unwrap();
        // Value 1 at time 10 (age 10, weight 100) vs value 1 at time 100 (age 100, w 10000).
        d.update(1.0, 10);
        d.update(1.0, 100);
        // Decayed count at now=100: (100 + 10000) / 100^2 = 10100/10000 = 1.01.
        let c = d.decayed_count(100);
        assert!((c - 1.01).abs() < 1e-6, "decayed count {c}");
    }

    #[test]
    fn polynomial_average_recency_weighted() {
        let mut d = PolynomialForwardDecay::new(2.0, 0).unwrap();
        d.update(0.0, 10); // small weight
        d.update(100.0, 100); // large weight
        let avg = d.average().unwrap();
        assert!(avg > 99.0, "recency-weighted average {avg}");
    }

    #[test]
    fn polynomial_grows_slower_than_exponential() {
        // Polynomial keeps more history: decayed count of many old items stays substantial.
        let mut d = PolynomialForwardDecay::new(1.0, 0).unwrap();
        for t in 1..=1000u64 {
            d.update(1.0, t);
        }
        let c = d.decayed_count(1000);
        // Σ t for t=1..1000 = 500500; / 1000 = 500.5 -> retains ~half the linear-weighted mass.
        assert!(c > 400.0, "polynomial retains history: {c}");
    }
}
