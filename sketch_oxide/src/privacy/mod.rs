//! Privacy-preserving sketching: differential-privacy mechanisms and accounting.
//!
//! This module is the privacy substrate the rest of the library builds on. It provides:
//!
//! - [`mechanisms`] — calibrated-noise primitives (discrete Laplace/Gaussian mechanisms,
//!   randomized response), implemented with the attack-resistant *discrete* distributions
//!   so floating-point rounding cannot break the guarantee.
//! - [`accountant`] — `(ε, δ)` budget tracking under sequential composition.
//!
//! DP wrappers over concrete sketches (DP cardinality release, DP-Count-Min, continual
//! counting, LDP frequency oracles, Sketch-Flip-Merge) build on these and land in later
//! roadmap waves.
//!
//! # Using it safely
//!
//! - Pass a **CSPRNG** to every mechanism ([`mechanisms::secure_rng`]). A predictable RNG
//!   voids differential privacy.
//! - Queries should be **integer-valued** (counts, histogram cells, sketch registers);
//!   the noise is integer noise on the discrete Laplace/Gaussian.
//! - Track every release through an [`Accountant`] so cumulative privacy loss stays within
//!   budget.

pub mod accountant;
pub mod dp_continual;
pub mod dp_count_min;
pub mod dp_quantile;
pub mod ldp;
pub mod mechanisms;
pub mod olh;

pub use accountant::{Accountant, PrivacyParams};
pub use dp_continual::DpContinualCounter;
pub use dp_count_min::{DpCountMin, PrivateCountMin};
pub use dp_quantile::DpQuantile;
pub use ldp::GrrFrequencyOracle;
pub use mechanisms::{
    discrete_gaussian, discrete_laplace, gaussian_mechanism, gaussian_sigma, laplace_mechanism,
    randomized_response, randomized_response_truth_prob, secure_rng,
};
pub use olh::OlhFrequencyOracle;
