//! Python bindings for person-level (user-level) differentially-private histograms.

use pyo3::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;
use sketch_oxide::privacy::PersonLevelDp as RustPersonLevelDp;

/// PersonLevelDp — builds an ε-differentially-private histogram under *user-level*
/// privacy: each user's total contribution is bounded (at most `max_keys_per_user`
/// keys, each capped at `max_per_key`) so adding/removing one user changes the
/// release by a bounded amount. Uses an OS-seeded discrete-noise CSPRNG.
///
/// Args:
///     max_keys_per_user (int): contribution bound on distinct keys per user.
///     max_per_key (int): per-key value cap.
///     epsilon (float): privacy parameter (> 0).
#[pyclass(module = "sketch_oxide")]
pub struct PersonLevelDp {
    inner: RustPersonLevelDp,
    rng: StdRng,
}

#[pymethods]
impl PersonLevelDp {
    #[new]
    fn new(max_keys_per_user: usize, max_per_key: i64, epsilon: f64) -> PyResult<Self> {
        RustPersonLevelDp::new(max_keys_per_user, max_per_key, epsilon)
            .map(|inner| Self {
                inner,
                rng: StdRng::from_os_rng(),
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Records that integer `user` contributed `value` to integer `key`.
    fn add(&mut self, user: u64, key: u64, value: i64) {
        self.inner.add(user, key, value);
    }

    /// The true (non-private) histogram as a list of `(key, count)` pairs.
    fn true_histogram(&self) -> Vec<(u64, i64)> {
        self.inner.true_histogram()
    }

    /// User-level sensitivity of the release.
    fn sensitivity(&self) -> u64 {
        self.inner.sensitivity()
    }

    /// Differentially-private histogram release as a list of `(key, noisy_count)` pairs.
    fn release(&mut self) -> PyResult<Vec<(u64, i64)>> {
        self.inner
            .release(&mut self.rng)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Number of distinct keys.
    fn num_keys(&self) -> usize {
        self.inner.num_keys()
    }

    fn __repr__(&self) -> String {
        format!("PersonLevelDp(num_keys={})", self.inner.num_keys())
    }
}
