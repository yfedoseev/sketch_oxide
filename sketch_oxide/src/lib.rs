//! sketch_oxide: State-of-the-Art DataSketches Library (2025)
//!
//! This library implements modern probabilistic data structures based on
//! 2024-2025 research, offering 28-75% better space efficiency than classic algorithms.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod cardinality;
pub mod common;
pub mod frequency;
pub mod membership;
pub mod quantiles;
pub mod range_filters;
pub mod reconciliation;
pub mod sampling;
pub mod similarity;
pub mod streaming;
pub mod universal;

// Re-export core types for convenience
pub use common::{
    hash, Mergeable, RangeFilter, Reconcilable, Result, SetDifference, Sketch, SketchError,
    WindowedSketch,
};

/// Error types and result aliases for sketch operations
pub mod error {
    pub use crate::common::{Result, SketchError};
}

// Re-export commonly used sketches
pub use cardinality::{CpcSketch, HyperLogLog, QSketch, ThetaSketch, UltraLogLog};
pub use frequency::{
    ConservativeCountMin, CountMinSketch, CountSketch, ElasticSketch, FrequentItems, HeavyKeeper,
    NitroSketch, NitroSketchStats, RemovableUniversalSketch, SpaceSaving, SALSA,
};
pub use membership::{LearnedBloomFilter, LearnedBloomStats, VacuumFilter, VacuumFilterStats};
pub use quantiles::{KllSketch, SplineSketch, TDigest};
pub use range_filters::{GRFStats, Grafite, GrafiteStats, MementoFilter, MementoStats, GRF};
pub use reconciliation::{RatelessIBLT, RatelessIBLTStats};
pub use sampling::{ReservoirSampling, VarOptSampling};
pub use similarity::{MinHash, SimHash};
pub use streaming::{ExponentialHistogram, SlidingHyperLogLog, SlidingWindowCounter};
pub use universal::{UnivMon, UnivMonStats};

#[cfg(test)]
mod tests {
    #[test]
    fn test_library_compiles() {
        // TDD: Start with simple compilation test
        // This test ensures the library compiles successfully
    }
}
