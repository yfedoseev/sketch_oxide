//! Python bindings for the differentially-private continual-observation counter.

use pyo3::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;
use sketch_oxide::privacy::DpContinualCounter as RustDpContinualCounter;

/// DpContinualCounter — the binary-tree (Dwork et al. / Chan-Shi-Song) mechanism
/// for releasing a running count under ε-differential privacy at every step,
/// adding only `O(log T)` noise across `max_steps` updates. Uses an OS-seeded
/// discrete-noise CSPRNG.
///
/// Args:
///     epsilon (float): total privacy budget across the stream (> 0).
///     max_steps (int): maximum number of updates.
#[pyclass(module = "sketch_oxide")]
pub struct DpContinualCounter {
    inner: RustDpContinualCounter,
    rng: StdRng,
}

#[pymethods]
impl DpContinualCounter {
    #[new]
    fn new(epsilon: f64, max_steps: u64) -> PyResult<Self> {
        RustDpContinualCounter::new(epsilon, max_steps)
            .map(|inner| Self {
                inner,
                rng: StdRng::from_os_rng(),
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Adds `value` at the next time step (advances the released count).
    fn insert(&mut self, value: i64) -> PyResult<()> {
        self.inner
            .insert(value, &mut self.rng)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Current noisy running count.
    fn count(&self) -> i64 {
        self.inner.count()
    }

    /// Current noisy running count, clamped to be non-negative.
    fn count_clamped(&self) -> i64 {
        self.inner.count_clamped()
    }

    /// Number of steps taken.
    fn steps(&self) -> u64 {
        self.inner.steps()
    }

    /// Total privacy budget.
    fn epsilon(&self) -> f64 {
        self.inner.epsilon()
    }

    fn __repr__(&self) -> String {
        format!(
            "DpContinualCounter(steps={}, count={})",
            self.inner.steps(),
            self.inner.count()
        )
    }
}
