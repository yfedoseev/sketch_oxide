//! Learned and ML-augmented sketches.
//!
//! Structures that improve a classical sketch with a user-supplied model — predicting
//! which keys are heavy, which are members, or what a key's frequency is. The shared
//! foundation is the [`Oracle`] interface: the caller's model scores keys, the sketch
//! consumes those scores, and the model never crosses the FFI boundary (see
//! [`oracle`] for the design).
//!
//! # Status
//!
//! This module currently provides the oracle substrate. The learned sketches that build on
//! it — PLBF / Fast-PLBF, Sandwiched and Ada-BF learned Bloom filters, learned Count-Min /
//! Count-Sketch, learning-augmented Misra-Gries — land in later roadmap waves.

mod learned_count_min;
pub mod oracle;

pub use learned_count_min::LearnedCountMin;
pub use oracle::{ClosureOracle, Oracle, PrecomputedOracle, Score};
