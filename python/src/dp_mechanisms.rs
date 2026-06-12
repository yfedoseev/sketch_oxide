//! Python bindings for the core differential-privacy noise mechanisms.
//!
//! These are stateless helpers; each call that adds noise draws from a fresh
//! OS-seeded CSPRNG (`StdRng::from_os_rng`), matching the library's `secure_rng`.

use pyo3::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;
use sketch_oxide::privacy::mechanisms;

fn map_err(e: impl std::fmt::Display) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string())
}

/// Draws a discrete-Laplace sample with scale `t` (mean 0).
#[pyfunction]
pub fn discrete_laplace(t: f64) -> PyResult<i64> {
    let mut rng = StdRng::from_os_rng();
    mechanisms::discrete_laplace(&mut rng, t).map_err(map_err)
}

/// Adds discrete-Laplace noise to `value` for `epsilon`-DP at the given `sensitivity`.
#[pyfunction]
pub fn laplace_mechanism(value: i64, sensitivity: u64, epsilon: f64) -> PyResult<i64> {
    let mut rng = StdRng::from_os_rng();
    mechanisms::laplace_mechanism(&mut rng, value, sensitivity, epsilon).map_err(map_err)
}

/// Draws a discrete-Gaussian sample with standard deviation `sigma` (mean 0).
#[pyfunction]
pub fn discrete_gaussian(sigma: f64) -> PyResult<i64> {
    let mut rng = StdRng::from_os_rng();
    mechanisms::discrete_gaussian(&mut rng, sigma).map_err(map_err)
}

/// Adds discrete-Gaussian noise (standard deviation `sigma`) to `value`.
#[pyfunction]
pub fn gaussian_mechanism(value: i64, sigma: f64) -> PyResult<i64> {
    let mut rng = StdRng::from_os_rng();
    mechanisms::gaussian_mechanism(&mut rng, value, sigma).map_err(map_err)
}

/// The Gaussian-mechanism standard deviation for `(epsilon, delta)`-DP at the
/// given `sensitivity`.
#[pyfunction]
pub fn gaussian_sigma(sensitivity: f64, epsilon: f64, delta: f64) -> PyResult<f64> {
    mechanisms::gaussian_sigma(sensitivity, epsilon, delta).map_err(map_err)
}

/// The probability of reporting the true bit under `epsilon`-randomized-response.
#[pyfunction]
pub fn randomized_response_truth_prob(epsilon: f64) -> PyResult<f64> {
    mechanisms::randomized_response_truth_prob(epsilon).map_err(map_err)
}

/// `epsilon`-local-DP randomized response for a single `bit`.
#[pyfunction]
pub fn randomized_response(bit: bool, epsilon: f64) -> PyResult<bool> {
    let mut rng = StdRng::from_os_rng();
    mechanisms::randomized_response(&mut rng, bit, epsilon).map_err(map_err)
}
