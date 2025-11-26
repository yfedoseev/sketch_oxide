//! Streaming and sliding window algorithms
//!
//! Data structures for time-bounded and windowed computations on streams.
//!
//! # Available Sketches
//!
//! - [`SlidingWindowCounter`]: Simple sliding window counter using exponential histogram
//! - [`ExponentialHistogram`]: Enhanced exponential histogram with formal error bounds
//! - [`SlidingHyperLogLog`]: Time-windowed cardinality estimation with HyperLogLog

mod exponential_histogram;
mod sliding_hll;
mod sliding_window;

pub use exponential_histogram::ExponentialHistogram;
pub use sliding_hll::{SlidingHLLStats, SlidingHyperLogLog};
pub use sliding_window::SlidingWindowCounter;

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_exists() {
        // This test ensures the module compiles successfully
    }
}
