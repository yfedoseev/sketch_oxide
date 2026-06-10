//! Differential-privacy noise mechanisms.
//!
//! These are the calibrated-noise primitives every DP wrapper in the library draws on:
//! the (discrete) Laplace and Gaussian mechanisms for numeric queries, and randomized
//! response for local DP. Composition accounting lives in
//! [`accountant`](super::accountant).
//!
//! # Why discrete mechanisms
//!
//! Naively sampling *continuous* Laplace/Gaussian noise with floating point is unsafe:
//! the gaps between representable f64 values leak information and break the formal
//! guarantee (Mironov, "On Significance of the Least Significant Bits", CCS 2012). We
//! therefore use the **discrete** Laplace and Gaussian distributions over the integers,
//! sampled with the exact Bernoulli/`exp` construction of Canonne, Kamath & Steinke,
//! "The Discrete Gaussian for Differential Privacy" (NeurIPS 2020). Queries are expected
//! to be integer-valued (counts, histogram cells, sketch registers); scale a real query
//! to a fixed-point integer grid before adding noise.
//!
//! # Randomness
//!
//! Every function takes the RNG explicitly. **For a real privacy guarantee the RNG must be
//! a CSPRNG** — pass [`secure_rng`] (seeded from the OS) or another cryptographic source.
//! Seedable PRNGs are for tests only; a predictable RNG voids DP. Note the fast
//! `SmallRng` used elsewhere in this crate for sampling is *not* acceptable here.
//!
//! # Limitations
//!
//! The exact-`exp` sampler runs in expected time proportional to the noise scale, so very
//! large scales (tiny epsilon) are slower. [`gaussian_sigma`] uses the classical
//! (continuous) Gaussian calibration, which is valid but slightly conservative for the
//! discrete Gaussian; tight analytic calibration is left for a later iteration. The
//! Bernoulli step uses `f64`; for the strongest guarantee an exact rational implementation
//! would be required, but the integer-valued output already removes the continuous-Laplace
//! attack surface.

use crate::error::{Result, SketchError};
use rand::Rng;

/// Returns a cryptographically secure RNG (ChaCha, seeded from the OS) suitable for DP.
///
/// Use this (or another CSPRNG) for any release that must actually be private.
pub fn secure_rng() -> impl Rng {
    use rand::SeedableRng;
    rand::rngs::StdRng::from_os_rng()
}

fn require_positive(name: &str, value: f64) -> Result<()> {
    if !value.is_finite() || value <= 0.0 {
        return Err(SketchError::InvalidParameter {
            param: name.to_string(),
            value: value.to_string(),
            constraint: "must be a finite value > 0".to_string(),
        });
    }
    Ok(())
}

/// Samples `Bernoulli(exp(-gamma))` for `gamma` in `[0, 1]` (Canonne–Kamath–Steinke).
fn bernoulli_exp_unit<R: Rng + ?Sized>(rng: &mut R, gamma: f64) -> bool {
    // gamma is assumed in [0, 1].
    let mut k: u64 = 1;
    loop {
        // Bernoulli(gamma / k); gamma/k is always in [0, 1] here.
        if rng.random_bool(gamma / k as f64) {
            k += 1;
        } else {
            // Accept iff the number of successful trials was even, i.e. k is odd.
            return k % 2 == 1;
        }
    }
}

/// Samples `Bernoulli(exp(-gamma))` for any `gamma >= 0` (CKS, extends to `gamma > 1`).
fn bernoulli_exp<R: Rng + ?Sized>(rng: &mut R, gamma: f64) -> bool {
    debug_assert!(gamma >= 0.0);
    if gamma <= 1.0 {
        return bernoulli_exp_unit(rng, gamma);
    }
    // exp(-gamma) = exp(-1)^floor(gamma) * exp(-(gamma - floor(gamma))).
    let whole = gamma.floor();
    let mut i = 0.0;
    while i < whole {
        if !bernoulli_exp_unit(rng, 1.0) {
            return false;
        }
        i += 1.0;
    }
    bernoulli_exp_unit(rng, gamma - whole)
}

/// Samples a `Geometric(1 - exp(-lambda))` variate on `{0, 1, 2, ...}` (number of
/// `Bernoulli(exp(-lambda))` successes before the first failure).
fn geometric_exp<R: Rng + ?Sized>(rng: &mut R, lambda: f64) -> u64 {
    let mut k = 0u64;
    while bernoulli_exp(rng, lambda) {
        k += 1;
    }
    k
}

/// Samples integer noise from the **discrete Laplace** distribution with scale `t`:
/// `P(X = x) ∝ exp(-|x| / t)`.
///
/// Sampled exactly as the difference of two i.i.d. geometrics (CKS), so it is free of the
/// floating-point rounding attack on continuous Laplace.
///
/// # Errors
/// [`SketchError::InvalidParameter`] if `t` is not finite and positive.
pub fn discrete_laplace<R: Rng + ?Sized>(rng: &mut R, t: f64) -> Result<i64> {
    require_positive("scale", t)?;
    let lambda = 1.0 / t;
    let a = geometric_exp(rng, lambda) as i64;
    let b = geometric_exp(rng, lambda) as i64;
    Ok(a - b)
}

/// The **discrete Laplace mechanism**: returns `value` plus integer noise calibrated for
/// `epsilon`-DP given an L1 `sensitivity`.
///
/// Noise scale is `sensitivity / epsilon`.
///
/// # Errors
/// [`SketchError::InvalidParameter`] if `epsilon <= 0` or `sensitivity == 0`.
pub fn laplace_mechanism<R: Rng + ?Sized>(
    rng: &mut R,
    value: i64,
    sensitivity: u64,
    epsilon: f64,
) -> Result<i64> {
    require_positive("epsilon", epsilon)?;
    if sensitivity == 0 {
        return Err(SketchError::InvalidParameter {
            param: "sensitivity".to_string(),
            value: "0".to_string(),
            constraint: "must be > 0".to_string(),
        });
    }
    let t = sensitivity as f64 / epsilon;
    Ok(value.saturating_add(discrete_laplace(rng, t)?))
}

/// Samples integer noise from the **discrete Gaussian** distribution with standard
/// deviation parameter `sigma` (CKS rejection sampler).
///
/// # Errors
/// [`SketchError::InvalidParameter`] if `sigma` is not finite and positive.
pub fn discrete_gaussian<R: Rng + ?Sized>(rng: &mut R, sigma: f64) -> Result<i64> {
    require_positive("sigma", sigma)?;
    // Proposal scale for the discrete Laplace from which we reject.
    let t = sigma.floor() + 1.0;
    let sigma_sq = sigma * sigma;
    let bias = sigma_sq / t;
    loop {
        let y = discrete_laplace(rng, t)?;
        let arg = ((y as f64).abs() - bias).powi(2) / (2.0 * sigma_sq);
        if bernoulli_exp(rng, arg) {
            return Ok(y);
        }
    }
}

/// The **discrete Gaussian mechanism**: returns `value` plus discrete-Gaussian noise with
/// the given `sigma`. Use [`gaussian_sigma`] to pick `sigma` from `(epsilon, delta)`.
///
/// # Errors
/// [`SketchError::InvalidParameter`] if `sigma <= 0`.
pub fn gaussian_mechanism<R: Rng + ?Sized>(rng: &mut R, value: i64, sigma: f64) -> Result<i64> {
    Ok(value.saturating_add(discrete_gaussian(rng, sigma)?))
}

/// Classical calibration of the Gaussian-mechanism `sigma` for `(epsilon, delta)`-DP given
/// an L2 `sensitivity`: `sigma = sqrt(2 ln(1.25/delta)) * sensitivity / epsilon`.
///
/// Valid for `epsilon < 1`; slightly conservative for the discrete Gaussian. For tight
/// calibration use the analytic Gaussian mechanism (future work).
///
/// # Errors
/// [`SketchError::InvalidParameter`] if `epsilon <= 0`, or `delta` is not in `(0, 1)`.
pub fn gaussian_sigma(sensitivity: f64, epsilon: f64, delta: f64) -> Result<f64> {
    require_positive("epsilon", epsilon)?;
    require_positive("sensitivity", sensitivity)?;
    if !(delta > 0.0 && delta < 1.0) {
        return Err(SketchError::InvalidParameter {
            param: "delta".to_string(),
            value: delta.to_string(),
            constraint: "must be in (0, 1)".to_string(),
        });
    }
    Ok((2.0 * (1.25 / delta).ln()).sqrt() * sensitivity / epsilon)
}

/// The probability that [`randomized_response`] reports the true bit: `e^ε / (e^ε + 1)`.
///
/// Aggregators use this to debias counts of randomized responses.
///
/// # Errors
/// [`SketchError::InvalidParameter`] if `epsilon <= 0`.
pub fn randomized_response_truth_prob(epsilon: f64) -> Result<f64> {
    require_positive("epsilon", epsilon)?;
    let e = epsilon.exp();
    Ok(e / (e + 1.0))
}

/// **Randomized response** for a single bit with `epsilon`-local-DP: reports the true `bit`
/// with probability `e^ε / (e^ε + 1)`, and the flipped bit otherwise.
///
/// # Errors
/// [`SketchError::InvalidParameter`] if `epsilon <= 0`.
pub fn randomized_response<R: Rng + ?Sized>(rng: &mut R, bit: bool, epsilon: f64) -> Result<bool> {
    let p = randomized_response_truth_prob(epsilon)?;
    Ok(if rng.random_bool(p) { bit } else { !bit })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{rngs::StdRng, SeedableRng};

    fn rng() -> StdRng {
        StdRng::seed_from_u64(0xD1FF_0BEE)
    }

    #[test]
    fn bernoulli_exp_matches_rate() {
        let mut r = rng();
        for &lambda in &[0.25_f64, 0.75, 1.5, 3.0] {
            let n = 40_000;
            let hits = (0..n).filter(|_| bernoulli_exp(&mut r, lambda)).count();
            let observed = hits as f64 / n as f64;
            let expected = (-lambda).exp();
            assert!(
                (observed - expected).abs() < 0.02,
                "lambda={lambda}: observed {observed}, expected {expected}"
            );
        }
    }

    #[test]
    fn discrete_laplace_is_centered() {
        let mut r = rng();
        let t = 5.0;
        let n = 50_000;
        let sum: i64 = (0..n).map(|_| discrete_laplace(&mut r, t).unwrap()).sum();
        let mean = sum as f64 / n as f64;
        assert!(mean.abs() < 0.2, "mean {mean} not ~0");
        // Variance of discrete Laplace = 2p/(1-p)^2 with p = e^{-1/t}.
        // Just sanity-check the spread is in the right ballpark (a few * t).
    }

    #[test]
    fn discrete_laplace_produces_both_signs() {
        let mut r = rng();
        let mut neg = false;
        let mut pos = false;
        for _ in 0..1000 {
            match discrete_laplace(&mut r, 3.0).unwrap().signum() {
                -1 => neg = true,
                1 => pos = true,
                _ => {}
            }
        }
        assert!(neg && pos, "expected both positive and negative noise");
    }

    #[test]
    fn laplace_mechanism_validates() {
        let mut r = rng();
        assert!(laplace_mechanism(&mut r, 10, 1, 0.0).is_err(), "epsilon=0");
        assert!(
            laplace_mechanism(&mut r, 10, 0, 1.0).is_err(),
            "sensitivity=0"
        );
        assert!(laplace_mechanism(&mut r, 10, 1, 1.0).is_ok());
    }

    #[test]
    fn discrete_gaussian_centered_and_spread() {
        let mut r = rng();
        let sigma = 8.0;
        let n = 40_000;
        let samples: Vec<i64> = (0..n)
            .map(|_| discrete_gaussian(&mut r, sigma).unwrap())
            .collect();
        let mean = samples.iter().sum::<i64>() as f64 / n as f64;
        let var = samples
            .iter()
            .map(|&x| (x as f64 - mean).powi(2))
            .sum::<f64>()
            / n as f64;
        assert!(mean.abs() < 0.5, "mean {mean} not ~0");
        // Discrete Gaussian variance ≈ sigma^2 = 64; allow generous tolerance.
        assert!(
            var > 45.0 && var < 85.0,
            "variance {var} not ~{}",
            sigma * sigma
        );
    }

    #[test]
    fn gaussian_sigma_calibration() {
        let s = gaussian_sigma(1.0, 0.5, 1e-5).unwrap();
        // sqrt(2 ln(1.25e5)) / 0.5 ≈ 9.7
        assert!(s > 8.0 && s < 12.0, "sigma {s}");
        assert!(gaussian_sigma(1.0, 0.5, 0.0).is_err());
        assert!(gaussian_sigma(1.0, 0.0, 0.5).is_err());
    }

    #[test]
    fn randomized_response_honours_rate() {
        let mut r = rng();
        let epsilon = 1.5_f64;
        let p = randomized_response_truth_prob(epsilon).unwrap();
        let n = 50_000;
        let truthful = (0..n)
            .filter(|_| randomized_response(&mut r, true, epsilon).unwrap())
            .count();
        let observed = truthful as f64 / n as f64;
        assert!(
            (observed - p).abs() < 0.02,
            "observed {observed}, expected {p}"
        );
    }

    #[test]
    fn randomized_response_validates() {
        let mut r = rng();
        assert!(randomized_response(&mut r, true, -1.0).is_err());
    }
}
