//! Cardinality estimation algorithms
//!
//! This module provides probabilistic algorithms for counting unique items
//! in large data streams.
//!
//! # Algorithm Comparison
//!
//! | Algorithm | Space Efficiency | Accuracy | Use Case |
//! |-----------|------------------|----------|----------|
//! | UltraLogLog | Best (28% better than HLL) | ~1.04/√m | New applications |
//! | HyperLogLog | Good | ~1.04/√m | Ecosystem interop (Redis, Druid) |
//! | CpcSketch | Better than HLL | ~1/√m | Apache DataSketches compat |
//! | ThetaSketch | Good | ~1/√k | Set operations (union, intersection) |

mod cpc;
mod cvm;
mod fm_sketch;
mod hyperloglog;
mod kmv;
mod linear_counting;
mod qsketch;
mod recordinality;
mod theta;
mod theta_core;
mod tuple_sketch;
mod ultraloglog;

pub use cpc::CpcSketch;
pub use cvm::CvmSketch;
pub use fm_sketch::FmSketch;
pub use hyperloglog::HyperLogLog;
pub use kmv::KmvSketch;
pub use linear_counting::LinearCounting;
pub use qsketch::QSketch;
pub use recordinality::Recordinality;
pub use theta::ThetaSketch;
pub use theta_core::{NoSummary, SumDoubles, Summary, ThetaCore};
pub use tuple_sketch::TupleSketch;
pub use ultraloglog::UltraLogLog;

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_exists() {
        // This test ensures the module compiles successfully
    }
}
