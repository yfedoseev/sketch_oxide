//! Streaming and sliding window algorithms
//!
//! Data structures for time-bounded and windowed computations on streams.
//!
//! # Available Sketches
//!
//! - [`SlidingWindowCounter`]: Simple sliding window counter using exponential histogram
//! - [`ExponentialHistogram`]: Enhanced exponential histogram with formal error bounds
//! - [`SlidingHyperLogLog`]: Time-windowed cardinality estimation with HyperLogLog

mod ada_sketch;
mod apbf;
mod ecm_sketch;
mod eh_core;
mod exponential_histogram;
mod fiba;
mod forward_decay;
mod hokusai;
mod hyper_calm;
mod periodic_sketch;
mod persistent_bloom;
mod sliding_hll;
mod sliding_sketch;
mod sliding_window;
mod sliding_window_quantiles;
mod sliding_window_universal;
mod smooth_histogram;
mod windowed_aggregator;

pub use ada_sketch::AdaSketch;
pub use apbf::Apbf;
pub use ecm_sketch::EcmSketch;
pub use exponential_histogram::ExponentialHistogram;
pub use fiba::FibaAggregator;
pub use forward_decay::{ForwardDecay, PolynomialForwardDecay};
pub use hokusai::Hokusai;
pub use hyper_calm::HyperCalm;
pub use periodic_sketch::PeriodicSketch;
pub use persistent_bloom::PersistentBloomFilter;
pub use sliding_hll::{SlidingHLLStats, SlidingHyperLogLog};
pub use sliding_sketch::SlidingSketch;
pub use sliding_window::SlidingWindowCounter;
pub use sliding_window_quantiles::SlidingWindowQuantiles;
pub use sliding_window_universal::SlidingWindowUniversal;
pub use smooth_histogram::SmoothHistogramSum;
pub use windowed_aggregator::WindowedAggregator;

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_exists() {
        // This test ensures the module compiles successfully
    }
}
