//! Streaming and sliding window algorithms
//!
//! Data structures for time-bounded and windowed computations on streams.
//!
//! # Available Sketches
//!
//! - [`SlidingWindowCounter`]: Simple sliding window counter using exponential histogram
//! - [`ExponentialHistogram`]: Enhanced exponential histogram with formal error bounds
//! - [`SlidingHyperLogLog`]: Time-windowed cardinality estimation with HyperLogLog

mod ecm_sketch;
mod eh_core;
mod exponential_histogram;
mod forward_decay;
mod sliding_hll;
mod sliding_window;
mod windowed_aggregator;

pub use ecm_sketch::EcmSketch;
pub use exponential_histogram::ExponentialHistogram;
pub use forward_decay::{ForwardDecay, PolynomialForwardDecay};
pub use sliding_hll::{SlidingHLLStats, SlidingHyperLogLog};
pub use sliding_window::SlidingWindowCounter;
pub use windowed_aggregator::WindowedAggregator;

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_exists() {
        // This test ensures the module compiles successfully
    }
}
