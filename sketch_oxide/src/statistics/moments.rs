//! Moments sketch — mergeable streaming central moments.
//!
//! A constant-size summary of a numeric stream's first four central moments — enough to report the
//! **mean, variance, skewness and (excess) kurtosis** — maintained in a numerically stable, fully
//! **mergeable** form (Pébay, "Formulas for Robust, One-Pass Parallel Computation of Covariances and
//! Arbitrary-Order Statistical Moments", Sandia 2008). Two sketches combine in O(1) with no loss of
//! accuracy, so it parallelizes and distributes trivially, and it underpins the moment-based
//! quantile reconstruction of the Moments Sketch (Gan et al., SIGMOD 2018) — the maximum-entropy
//! solver for quantiles over these moments is a documented follow-up query layer.

use crate::common::Result;

/// A streaming central-moments sketch over `f64` values.
///
/// # Example
/// ```
/// use sketch_oxide::statistics::MomentsSketch;
///
/// let mut m = MomentsSketch::new();
/// for i in 0..1000 { m.update(i as f64); }
/// assert!((m.mean() - 499.5).abs() < 1e-6);
/// assert!(m.skewness().abs() < 1e-6); // symmetric data
/// ```
#[derive(Debug, Clone, Default)]
pub struct MomentsSketch {
    n: u64,
    mean: f64,
    /// Sum of squared deviations (M2).
    m2: f64,
    /// Sum of cubed deviations (M3).
    m3: f64,
    /// Sum of fourth-power deviations (M4).
    m4: f64,
    min: f64,
    max: f64,
}

impl MomentsSketch {
    /// Creates an empty sketch.
    pub fn new() -> Self {
        Self {
            n: 0,
            mean: 0.0,
            m2: 0.0,
            m3: 0.0,
            m4: 0.0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        }
    }

    /// Adds a value (Pébay's stable one-pass update).
    pub fn update(&mut self, x: f64) {
        let n1 = self.n as f64;
        self.n += 1;
        let n = self.n as f64;
        let delta = x - self.mean;
        let delta_n = delta / n;
        let delta_n2 = delta_n * delta_n;
        let term1 = delta * delta_n * n1;
        self.mean += delta_n;
        self.m4 += term1 * delta_n2 * (n * n - 3.0 * n + 3.0) + 6.0 * delta_n2 * self.m2
            - 4.0 * delta_n * self.m3;
        self.m3 += term1 * delta_n * (n - 2.0) - 3.0 * delta_n * self.m2;
        self.m2 += term1;
        self.min = self.min.min(x);
        self.max = self.max.max(x);
    }

    /// Number of values.
    #[inline]
    pub fn count(&self) -> u64 {
        self.n
    }

    /// Whether nothing has been added.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// The mean (0 if empty).
    pub fn mean(&self) -> f64 {
        self.mean
    }

    /// Sample variance `M2 / (n − 1)` (`NaN` if fewer than 2 values).
    pub fn variance(&self) -> f64 {
        if self.n < 2 {
            f64::NAN
        } else {
            self.m2 / (self.n as f64 - 1.0)
        }
    }

    /// Sample standard deviation.
    pub fn std_dev(&self) -> f64 {
        self.variance().sqrt()
    }

    /// Population skewness `g1` (`NaN` if no spread).
    pub fn skewness(&self) -> f64 {
        if self.n < 2 || self.m2 == 0.0 {
            return f64::NAN;
        }
        let n = self.n as f64;
        (n.sqrt() * self.m3) / self.m2.powf(1.5)
    }

    /// Excess kurtosis `g2` (`NaN` if no spread); 0 for a normal distribution.
    pub fn kurtosis(&self) -> f64 {
        if self.n < 2 || self.m2 == 0.0 {
            return f64::NAN;
        }
        let n = self.n as f64;
        (n * self.m4) / (self.m2 * self.m2) - 3.0
    }

    /// Smallest value seen (`None` if empty).
    pub fn min(&self) -> Option<f64> {
        if self.n == 0 { None } else { Some(self.min) }
    }

    /// Largest value seen (`None` if empty).
    pub fn max(&self) -> Option<f64> {
        if self.n == 0 { None } else { Some(self.max) }
    }

    /// Merges another sketch (Pébay's parallel combination); exact, not approximate.
    ///
    /// # Errors
    /// Never fails; returns `Result` for API symmetry with other mergeable sketches.
    pub fn merge(&mut self, other: &Self) -> Result<()> {
        if other.n == 0 {
            return Ok(());
        }
        if self.n == 0 {
            *self = other.clone();
            return Ok(());
        }
        let na = self.n as f64;
        let nb = other.n as f64;
        let n = na + nb;
        let delta = other.mean - self.mean;
        let delta2 = delta * delta;
        let delta3 = delta2 * delta;
        let delta4 = delta2 * delta2;

        let m2 = self.m2 + other.m2 + delta2 * na * nb / n;
        let m3 = self.m3
            + other.m3
            + delta3 * na * nb * (na - nb) / (n * n)
            + 3.0 * delta * (na * other.m2 - nb * self.m2) / n;
        let m4 = self.m4
            + other.m4
            + delta4 * na * nb * (na * na - na * nb + nb * nb) / (n * n * n)
            + 6.0 * delta2 * (na * na * other.m2 + nb * nb * self.m2) / (n * n)
            + 4.0 * delta * (na * other.m3 - nb * self.m3) / n;

        self.mean += delta * nb / n;
        self.m2 = m2;
        self.m3 = m3;
        self.m4 = m4;
        self.n += other.n;
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_well_defined() {
        let m = MomentsSketch::new();
        assert!(m.is_empty());
        assert_eq!(m.count(), 0);
        assert!(m.min().is_none());
        assert!(m.variance().is_nan());
    }

    #[test]
    fn mean_and_variance_of_uniform() {
        let mut m = MomentsSketch::new();
        let n = 1000;
        for i in 0..n {
            m.update(i as f64);
        }
        assert!((m.mean() - 499.5).abs() < 1e-6, "mean {}", m.mean());
        // Sample variance of 0..999.
        let var = m.variance();
        assert!((var - 83_416.6).abs() < 1.0, "variance {var}");
        assert_eq!(m.min(), Some(0.0));
        assert_eq!(m.max(), Some(999.0));
    }

    #[test]
    fn symmetric_data_has_zero_skew() {
        let mut m = MomentsSketch::new();
        for i in -500..=500 {
            m.update(i as f64);
        }
        assert!(m.skewness().abs() < 1e-6, "skewness {}", m.skewness());
    }

    #[test]
    fn skewed_data_has_positive_skew() {
        // A right-skewed set (a few large values).
        let mut m = MomentsSketch::new();
        for _ in 0..1000 {
            m.update(1.0);
        }
        for _ in 0..50 {
            m.update(100.0);
        }
        assert!(
            m.skewness() > 1.0,
            "skewness {} should be strongly positive",
            m.skewness()
        );
    }

    #[test]
    fn merge_matches_single_pass() {
        let mut whole = MomentsSketch::new();
        let mut a = MomentsSketch::new();
        let mut b = MomentsSketch::new();
        for i in 0..1500u64 {
            let x = ((i * 2_654_435_761) % 9973) as f64;
            whole.update(x);
            if i % 2 == 0 {
                a.update(x);
            } else {
                b.update(x);
            }
        }
        a.merge(&b).unwrap();
        assert_eq!(a.count(), whole.count());
        assert!((a.mean() - whole.mean()).abs() < 1e-6, "mean");
        assert!((a.variance() - whole.variance()).abs() < 1e-3, "variance");
        assert!((a.skewness() - whole.skewness()).abs() < 1e-6, "skewness");
        assert!((a.kurtosis() - whole.kurtosis()).abs() < 1e-6, "kurtosis");
    }

    #[test]
    fn merge_with_empty_is_identity() {
        let mut a = MomentsSketch::new();
        for i in 0..100 {
            a.update(i as f64);
        }
        let mean_before = a.mean();
        a.merge(&MomentsSketch::new()).unwrap();
        assert_eq!(a.count(), 100);
        assert!((a.mean() - mean_before).abs() < 1e-12);
    }
}
