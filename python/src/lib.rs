use pyo3::prelude::*;

mod binary_fuse;
mod blocked_bloom;
mod bloom;
mod common;
mod conservative_count_min;
mod count_min;
mod count_sketch;
mod counting_bloom;
mod cpc;
mod cuckoo;
mod ddsketch;
mod elastic_sketch;
mod exponential_histogram;
mod frequent;
mod grafite;
mod grf;
mod heavy_keeper;
mod hyperloglog;
mod kll;
mod learned_bloom;
mod memento_filter;
mod minhash;
mod nitrosketch;
mod qsketch;
mod rateless_iblt;
mod removable_sketch;
mod req;
mod reservoir;
mod ribbon;
mod salsa;
mod simhash;
mod sliding_hll;
mod sliding_window;
mod space_saving;
mod spline_sketch;
mod stable_bloom;
mod tdigest;
mod theta;
mod ultraloglog;
mod univmon;
mod vacuum_filter;
mod varopt;

/// sketch_oxide: State-of-the-Art DataSketches Library (2025)
///
/// This module provides Python bindings for the sketch_oxide Rust library,
/// offering 28-75% better space efficiency than classic algorithms.
///
/// ## Algorithms
///
/// ### Cardinality Estimation
/// - **UltraLogLog**: 28% better than HyperLogLog (VLDB 2024)
/// - **CpcSketch**: 30-40% better than HyperLogLog (Apache DataSketches)
/// - **ThetaSketch**: Supports set operations (union, intersection, difference)
///
/// ### Membership Testing
/// - **BinaryFuseFilter**: 75% better than Bloom filters (ACM JEA 2022)
/// - **BloomFilter**: Classic dynamic membership filter (~10 bits/key @ 1% FPR)
/// - **BlockedBloomFilter**: Cache-efficient Bloom (1 cache miss per query)
/// - **CountingBloomFilter**: Bloom with deletions (~40 bits/key)
/// - **CuckooFilter**: Space-efficient deletable filter (~12 bits/key, Fan 2014)
/// - **RibbonFilter**: Space-efficient (~7 bits/key @ 1% FPR, RocksDB 2021+)
/// - **StableBloomFilter**: Bounded FPR for unbounded streams (Deng 2006)
///
/// ### Quantile Estimation
/// - **DDSketch**: Relative error guarantees (VLDB 2019, Datadog/ClickHouse)
/// - **ReqSketch**: Zero error at tail quantiles (PODS 2021, Google BigQuery)
///
/// ### Frequency Estimation
/// - **CountMinSketch**: Standard frequency estimation (Redis, monitoring)
/// - **CountSketch**: Unbiased estimation, L2 error bounds (Charikar 2002)
/// - **ConservativeCountMin**: Up to 10x more accurate than standard CM (Estan 2002)
/// - **SpaceSaving**: Heavy hitter detection, deterministic error bounds (Metwally 2005)
/// - **FrequentItems**: Top-K heavy hitters with deterministic bounds
///
/// ### Streaming
/// - **SlidingWindowCounter**: Time-bounded counting with O(log²N) space (Datar 2002)
///
/// ### Similarity Estimation
/// - **MinHash**: Jaccard similarity (Broder 1997, LSH, deduplication)
/// - **SimHash**: Near-duplicate detection (Charikar 2002, Google web crawling)
///
/// ### Sampling
/// - **ReservoirSampling**: Uniform random sampling (Vitter 1985)
/// - **VarOptSampling**: Variance-optimal weighted sampling (Cohen 2014)
///
/// ## Quick Start
///
/// ```python
/// from sketch_oxide import UltraLogLog, DDSketch, BinaryFuseFilter
///
/// # Cardinality estimation
/// ull = UltraLogLog(precision=12)
/// for item in data:
///     ull.update(item)
/// print(f"Unique items: {ull.estimate():.0f}")
///
/// # Quantile estimation
/// dd = DDSketch(relative_accuracy=0.01)
/// for latency in latencies:
///     dd.update(latency)
/// print(f"p99 latency: {dd.quantile(0.99):.2f}ms")
///
/// # Membership testing
/// filter = BinaryFuseFilter([1, 2, 3, 4, 5], bits_per_entry=9)
/// print(filter.contains(3))  # True
/// ```
#[pymodule]
fn sketch_oxide(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add(
        "__doc__",
        "State-of-the-art DataSketches library (2025) - 28-75% better space efficiency",
    )?;

    // Cardinality estimation
    m.add_class::<ultraloglog::UltraLogLog>()?;
    m.add_class::<hyperloglog::HyperLogLog>()?;
    m.add_class::<cpc::CpcSketch>()?;
    m.add_class::<theta::ThetaSketch>()?;
    m.add_class::<qsketch::QSketch>()?;

    // Membership testing
    m.add_class::<binary_fuse::BinaryFuseFilter>()?;
    m.add_class::<bloom::BloomFilter>()?;
    m.add_class::<blocked_bloom::BlockedBloomFilter>()?;
    m.add_class::<counting_bloom::CountingBloomFilter>()?;
    m.add_class::<cuckoo::CuckooFilter>()?;
    m.add_class::<ribbon::RibbonFilter>()?;
    m.add_class::<stable_bloom::StableBloomFilter>()?;
    m.add_class::<vacuum_filter::VacuumFilter>()?;
    m.add_class::<learned_bloom::LearnedBloomFilter>()?;

    // Quantile estimation
    m.add_class::<ddsketch::DDSketch>()?;
    m.add_class::<req::ReqSketch>()?;
    m.add_class::<tdigest::TDigest>()?;
    m.add_class::<kll::KllSketch>()?;
    m.add_class::<spline_sketch::SplineSketch>()?;

    // Frequency estimation
    m.add_class::<count_min::CountMinSketch>()?;
    m.add_class::<count_sketch::CountSketch>()?;
    m.add_class::<conservative_count_min::ConservativeCountMin>()?;
    m.add_class::<space_saving::SpaceSaving>()?;
    m.add_class::<elastic_sketch::ElasticSketch>()?;
    m.add_class::<salsa::SALSA>()?;
    m.add_class::<removable_sketch::RemovableUniversalSketch>()?;
    m.add_class::<frequent::FrequentItems>()?;
    m.add_class::<heavy_keeper::HeavyKeeper>()?;
    m.add_class::<nitrosketch::NitroSketch>()?;

    // Streaming
    m.add_class::<sliding_window::SlidingWindowCounter>()?;
    m.add_class::<exponential_histogram::ExponentialHistogram>()?;
    m.add_class::<sliding_hll::SlidingHyperLogLog>()?;

    // Range filters
    m.add_class::<grafite::Grafite>()?;
    m.add_class::<grf::GRF>()?;
    m.add_class::<memento_filter::MementoFilter>()?;

    // Set reconciliation
    m.add_class::<rateless_iblt::RatelessIBLT>()?;

    // Similarity estimation
    m.add_class::<minhash::MinHash>()?;
    m.add_class::<simhash::SimHash>()?;

    // Sampling
    m.add_class::<reservoir::ReservoirSampling>()?;
    m.add_class::<varopt::VarOptSampling>()?;

    // Universal monitoring
    m.add_class::<univmon::UnivMon>()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_compiles() {
        // TDD: Ensure PyO3 module compiles
        // This test passes if the module compiles successfully
    }
}
