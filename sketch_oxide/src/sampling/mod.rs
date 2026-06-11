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

pub mod distinct_sampling;
pub mod ebpps;
pub mod l0_sampler;
pub mod priority_sampling;
pub mod reservoir;
pub mod reservoir_l;
pub mod signed_update_sampler;
pub mod sliding_window_sample;
pub mod stratified_reservoir;
pub mod varopt;
pub mod weighted_reservoir;

pub use distinct_sampling::DistinctSampling;
pub use ebpps::EbppsSketch;
pub use l0_sampler::L0Sampler;
pub use priority_sampling::PrioritySampling;
pub use reservoir::ReservoirSampling;
pub use reservoir_l::ReservoirSamplingL;
pub use signed_update_sampler::SignedUpdateSampler;
pub use sliding_window_sample::SlidingWindowSample;
pub use stratified_reservoir::StratifiedReservoir;
pub use varopt::VarOptSampling;
pub use weighted_reservoir::WeightedReservoirSampling;
