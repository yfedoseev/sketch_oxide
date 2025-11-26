//! Sampling algorithms for streams
//!
//! This module provides algorithms for maintaining random samples from data streams:
//!
//! - [`ReservoirSampling`]: Uniform random sampling without replacement (Vitter 1985)
//! - [`VarOptSampling`]: Variance-optimal weighted sampling (Cohen 2014)
//!
//! # When to Use Sampling vs Sketching
//!
//! | Feature | Sampling | Sketching |
//! |---------|----------|-----------|
//! | **Output** | Actual items | Statistics only |
//! | **Memory** | O(k) items | O(sketch size) |
//! | **Best for** | Debugging, auditing | Aggregate statistics |
//! | **Examples** | Log sampling, A/B tests | Cardinality, percentiles |
//!
//! # Choosing Between Reservoir and VarOpt
//!
//! - **Reservoir**: Uniform sampling, all items equally likely
//! - **VarOpt**: Weighted sampling, higher-weight items more likely

pub mod reservoir;
pub mod varopt;

pub use reservoir::ReservoirSampling;
pub use varopt::VarOptSampling;
