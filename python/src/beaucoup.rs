//! Python bindings for the BeauCoup super-spreader detector.

use pyo3::prelude::*;
use sketch_oxide::net::BeauCoup as RustBeauCoup;

/// BeauCoup — coupon-collector-based per-key distinct counting for network
/// super-spreader detection: each `(key, value)` pair activates a coupon with a
/// small probability, and the number of distinct values per key is inverted from
/// how many coupons it has collected.
///
/// Args:
///     num_coupons (int): coupons per key (accuracy/space trade-off).
///     activation_prob (float): per-pair coupon activation probability.
#[pyclass(module = "sketch_oxide")]
pub struct BeauCoup {
    inner: RustBeauCoup,
}

#[pymethods]
impl BeauCoup {
    #[new]
    fn new(num_coupons: u32, activation_prob: f64) -> PyResult<Self> {
        RustBeauCoup::new(num_coupons, activation_prob)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Records that integer `key` was seen together with integer `value`
    /// (idempotent in the pair, so repeated pairs do not double-count).
    fn record(&mut self, key: u64, value: u64) {
        self.inner.record(key, value);
    }

    /// Estimated number of distinct values seen for `key`.
    fn estimate_distinct(&self, key: u64) -> f64 {
        self.inner.estimate_distinct(key)
    }

    /// `(key, estimate)` for keys whose estimated distinct count meets `threshold`.
    fn super_spreaders(&self, threshold: f64) -> Vec<(u64, f64)> {
        self.inner.super_spreaders(threshold)
    }

    /// Number of distinct keys tracked.
    fn num_keys(&self) -> usize {
        self.inner.num_keys()
    }

    /// Coupons per key.
    fn num_coupons(&self) -> u32 {
        self.inner.num_coupons()
    }

    fn __repr__(&self) -> String {
        format!(
            "BeauCoup(num_keys={}, num_coupons={})",
            self.inner.num_keys(),
            self.inner.num_coupons()
        )
    }
}
