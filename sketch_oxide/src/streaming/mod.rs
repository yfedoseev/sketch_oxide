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
mod deterministic_wave;
mod ecm_sketch;
mod eh_core;
mod exponential_histogram;
mod fiba;
mod forward_decay;
mod hokusai;
mod hyper_calm;
mod periodic_sketch;
mod persistent_bloom;
mod persistent_count_min;
mod sliding_hll;
mod sliding_sketch;
mod sliding_window;
mod sliding_window_quantiles;
mod sliding_window_universal;
mod smooth_histogram;
mod windowed_aggregator;

pub use ada_sketch::AdaSketch;
pub use apbf::Apbf;
pub use deterministic_wave::DeterministicWave;
pub use ecm_sketch::EcmSketch;
pub use exponential_histogram::ExponentialHistogram;
pub use fiba::FibaAggregator;
pub use forward_decay::{ForwardDecay, PolynomialForwardDecay};
pub use hokusai::Hokusai;
pub use hyper_calm::HyperCalm;
pub use periodic_sketch::PeriodicSketch;
pub use persistent_bloom::PersistentBloomFilter;
pub use persistent_count_min::PersistentCountMin;
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

/// Smoke test for the capability-trait adoptions (fable5 doc 01 F3): drive
/// several concrete sketches purely through the generic `Update` /
/// `PointQuery` / `CardinalityEstimate` traits, proving the split composes.
#[cfg(test)]
mod capability_smoke {
    use crate::common::{CardinalityEstimate, PointQuery, Temporal, Update};
    use crate::similarity::OddSketch;
    use crate::streaming::{Hokusai, SlidingSketch};

    // Generic over ANY byte-ingesting point-query sketch.
    fn ingest_then_query<S: Update<[u8]> + PointQuery<[u8]>>(
        sketch: &mut S,
        item: &[u8],
        n: usize,
    ) -> u64 {
        for _ in 0..n {
            sketch.update(item);
        }
        sketch.query(item)
    }

    // Generic over ANY byte-ingesting cardinality estimator.
    fn distinct<S: Update<[u8]> + CardinalityEstimate>(sketch: &mut S, items: &[&[u8]]) -> f64 {
        for it in items {
            sketch.update(it);
        }
        sketch.estimate_cardinality()
    }

    #[test]
    fn generic_point_query_over_capability_traits() {
        // SlidingSketch (needs its clock primed) and Hokusai both satisfy
        // Update<[u8]> + PointQuery<[u8]>; neither should underestimate.
        let mut s = SlidingSketch::new(1000, 10, 4, 1024).unwrap();
        s.advance(0);
        assert!(ingest_then_query(&mut s, b"alpha", 40) >= 40);

        let mut h = Hokusai::new(4, 256, 4).unwrap();
        assert!(ingest_then_query(&mut h, b"alpha", 40) >= 40);
    }

    #[test]
    fn generic_cardinality_over_capability_traits() {
        let items: Vec<[u8; 4]> = (0u32..200).map(u32::to_le_bytes).collect();
        let refs: Vec<&[u8]> = items.iter().map(|b| b.as_slice()).collect();
        let mut odd = OddSketch::new(4096).unwrap();
        let est = distinct(&mut odd, &refs);
        assert!(
            est.is_finite() && est > 40.0,
            "OddSketch cardinality off: {est}"
        );
    }
}
