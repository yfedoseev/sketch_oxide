//! sketch_oxide: State-of-the-Art DataSketches Library (2026)
//!
//! This library implements modern probabilistic data structures based on
//! 2024-2026 research, offering 28-75% better space efficiency than classic algorithms.
//!
//! # A note on the two `ExponentialHistogram` types
//!
//! There are two distinct structures that share this name:
//! - [`streaming::ExponentialHistogram`] — the **DGIM** sliding-window counter
//!   (exponential bucketing over time), used for windowed aggregation.
//! - [`quantiles::otel_histogram`] — the **OpenTelemetry** base-2 exponential
//!   histogram (`ExponentialHistogramDataPoint`), used for latency distributions.
//!
//! They are unrelated; pick by domain (time-windowing vs. value distribution).

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod cardinality;
pub mod common;
pub mod frequency;
pub mod graph;
pub mod learned;
pub mod matrix;
pub mod membership;
pub mod net;
pub mod privacy;
pub mod quantiles;
pub mod range_filters;
pub mod reconciliation;
pub mod sampling;
pub mod similarity;
pub mod statistics;
pub mod streaming;
pub mod universal;
pub mod vector;

// Re-export core types for convenience
pub use common::{
    Mergeable, RangeFilter, Reconcilable, Result, SetDifference, Sketch, SketchError,
    WindowedSketch, hash,
};

/// Error types and result aliases for sketch operations
pub mod error {
    pub use crate::common::{Result, SketchError};
}

// Re-export commonly used sketches
pub use cardinality::{CpcSketch, HyperLogLog, QSketch, ThetaSketch, UltraLogLog};
pub use frequency::{
    ConservativeCountMin, CountMinSketch, CountSketch, ElasticSketch, FrequentItems, HeavyKeeper,
    NitroSketch, NitroSketchStats, RemovableUniversalSketch, SALSA, SpaceSaving,
};
pub use membership::{LearnedBloomFilter, LearnedBloomStats, VacuumFilter, VacuumFilterStats};
pub use quantiles::{DDSketch, KllSketch, ReqSketch, SplineSketch, TDigest};
pub use range_filters::{GRF, GRFStats, Grafite, GrafiteStats, MementoFilter, MementoStats};

// Re-export representative types from the 2026 modules (previously absent from
// the root re-exports — fable5 doc 01 F8).
pub use learned::{LearnedCountMin, SandwichedLearnedBloom};
pub use matrix::{FrequentDirections, JohnsonLindenstrauss};
pub use privacy::{DpContinualCounter, DpMisraGries, DpQuantile};
pub use reconciliation::{Iblt, IbltStats};
#[allow(deprecated)]
pub use reconciliation::{RatelessIBLT, RatelessIBLTStats};
pub use sampling::{ReservoirSampling, VarOptSampling};
pub use similarity::{MinHash, SimHash};
pub use statistics::{HllJointEstimator, MorrisCounter};
pub use streaming::{ExponentialHistogram, SlidingHyperLogLog, SlidingWindowCounter};
pub use universal::{UnivMon, UnivMonStats};
pub use vector::{PreparedQuery, RaBitQ, RaBitQCode};

#[cfg(test)]
mod tests {
    #[test]
    fn test_library_compiles() {
        // TDD: Start with simple compilation test
        // This test ensures the library compiles successfully
    }
}
