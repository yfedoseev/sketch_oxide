//! Streaming statistics: moments, inner products, and related estimators.
//!
//! Sketches whose target quantity is a statistic of the stream's frequency vector rather
//! than the items themselves — second moment (F2), inner products / join sizes, and (in
//! later waves) entropy, Lp norms, and correlation.

mod ams;
mod p_stable;

pub use ams::AmsSketch;
pub use p_stable::PStableLpSketch;
