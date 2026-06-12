use pyo3::prelude::*;

mod adaptive_quotient_filter;
mod aleph_filter;
mod bbit_minhash;
mod binary_fuse;
mod blocked_bloom;
mod bloom;
mod bloomier_filter;
mod bubble_sketch;
mod burr;
mod c_minhash;
mod c_oph;
mod common;
mod conservative_count_min;
mod count_min;
mod count_min_log;
mod count_sketch;
mod counting_bloom;
mod counting_quotient_filter;
mod cpc;
mod cuckoo;
mod cuckoo_heavy_keeper;
mod cvm;
mod ddsketch;
mod double_anonymous;
mod dyadic_count_sketch;
mod elastic_sketch;
mod exa_log_log;
mod exponential_histogram;
mod fcm_sketch;
mod filtered_space_saving;
mod fm_sketch;
mod frequent;
mod gk;
mod grafite;
mod grf;
mod heavy_keeper;
mod heavy_locker;
mod hidden_sketch;
mod hll_plus;
mod hyperbitbit;
mod hyperloglog;
mod kll;
mod kmv;
mod learned_bloom;
mod linear_counting;
mod lossy_counting;
mod lsh_ensemble;
mod memento_filter;
mod minhash;
mod minhash_lsh;
mod moments_sketch;
mod morton_filter;
mod mv_sketch;
mod nitrosketch;
mod odd_sketch;
mod on_off_sketch;
mod oph;
mod otel_histogram;
mod per_flow_quantiles;
mod per_key;
mod prefix_filter;
mod prob_min_hash;
mod q_digest;
mod qsketch;
mod rateless_iblt;
mod recordinality;
mod removable_sketch;
mod req;
mod reservoir;
mod rhhh;
mod ribbon;
mod salsa;
mod scalable_bloom;
mod set_sketch;
mod simhash;
mod simhash_lsh;
mod sketch_polymer;
mod sliding_hll;
mod sliding_window;
mod space_saving;
mod space_saving_pm;
mod spline_sketch;
mod spread_sketch;
mod stable_bloom;
mod stable_sketch;
mod stacked_filter;
mod sticky_sampling;
mod super_min_hash;
mod taffy_cuckoo_filter;
mod tdigest;
mod telescoping_filter;
mod theta;
mod tower_sketch;
mod udd_sketch;
mod ultraloglog;
mod unbiased_space_saving;
mod univmon;
mod vacuum_filter;
mod varopt;
mod vector_quotient_filter;
mod waving_sketch;
mod weighted_minhash;
mod xor_filter;

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
    m.add_class::<hll_plus::HyperLogLogPlus>()?;
    m.add_class::<cpc::CpcSketch>()?;
    m.add_class::<theta::ThetaSketch>()?;
    m.add_class::<qsketch::QSketch>()?;
    m.add_class::<cvm::CvmSketch>()?;
    m.add_class::<exa_log_log::ExaLogLog>()?;
    m.add_class::<fm_sketch::FmSketch>()?;
    m.add_class::<hyperbitbit::HyperBitBit>()?;
    m.add_class::<kmv::KmvSketch>()?;
    m.add_class::<linear_counting::LinearCounting>()?;
    m.add_class::<recordinality::Recordinality>()?;
    m.add_class::<set_sketch::SetSketch>()?;

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
    m.add_class::<scalable_bloom::ScalableBloomFilter>()?;
    m.add_class::<prefix_filter::PrefixFilter>()?;
    m.add_class::<aleph_filter::AlephFilter>()?;
    m.add_class::<counting_quotient_filter::CountingQuotientFilter>()?;
    m.add_class::<vector_quotient_filter::VectorQuotientFilter>()?;
    m.add_class::<adaptive_quotient_filter::AdaptiveQuotientFilter>()?;
    m.add_class::<telescoping_filter::TelescopingFilter>()?;
    m.add_class::<morton_filter::MortonFilter>()?;
    m.add_class::<taffy_cuckoo_filter::TaffyCuckooFilter>()?;
    m.add_class::<burr::BurrFilter>()?;
    m.add_class::<stacked_filter::StackedFilter>()?;
    m.add_class::<xor_filter::XorFilter>()?;
    m.add_class::<bloomier_filter::BloomierFilter>()?;

    // Quantile estimation
    m.add_class::<ddsketch::DDSketch>()?;
    m.add_class::<req::ReqSketch>()?;
    m.add_class::<tdigest::TDigest>()?;
    m.add_class::<kll::KllSketch>()?;
    m.add_class::<spline_sketch::SplineSketch>()?;
    m.add_class::<gk::GreenwaldKhanna>()?;
    m.add_class::<moments_sketch::MomentsSketch>()?;
    m.add_class::<otel_histogram::OtelExponentialHistogram>()?;
    m.add_class::<udd_sketch::UddSketch>()?;
    m.add_class::<q_digest::QDigest>()?;
    m.add_class::<dyadic_count_sketch::DyadicCountSketch>()?;
    m.add_class::<per_flow_quantiles::PerFlowQuantiles>()?;
    m.add_class::<per_key::PerKeyQuantiles>()?;
    m.add_class::<sketch_polymer::SketchPolymer>()?;

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
    m.add_class::<count_min_log::CountMinLog>()?;
    m.add_class::<tower_sketch::TowerSketch>()?;
    m.add_class::<fcm_sketch::FcmSketch>()?;
    m.add_class::<hidden_sketch::HiddenSketch>()?;
    m.add_class::<bubble_sketch::BubbleSketch>()?;
    m.add_class::<mv_sketch::MvSketch>()?;
    m.add_class::<stable_sketch::StableSketch>()?;
    m.add_class::<waving_sketch::WavingSketch>()?;
    m.add_class::<heavy_locker::HeavyLocker>()?;
    m.add_class::<lossy_counting::LossyCounting>()?;
    m.add_class::<sticky_sampling::StickySampling>()?;
    m.add_class::<space_saving_pm::SpaceSavingPlusMinus>()?;
    m.add_class::<filtered_space_saving::FilteredSpaceSaving>()?;
    m.add_class::<unbiased_space_saving::UnbiasedSpaceSaving>()?;
    m.add_class::<double_anonymous::DoubleAnonymousSketch>()?;
    m.add_class::<rhhh::Rhhh>()?;
    m.add_class::<spread_sketch::SpreadSketch>()?;
    m.add_class::<on_off_sketch::OnOffSketch>()?;
    m.add_class::<cuckoo_heavy_keeper::CuckooHeavyKeeper>()?;

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
    m.add_class::<super_min_hash::SuperMinHash>()?;
    m.add_class::<oph::OnePermutationHash>()?;
    m.add_class::<c_minhash::CMinHash>()?;
    m.add_class::<c_oph::COph>()?;
    m.add_class::<odd_sketch::OddSketch>()?;
    m.add_class::<prob_min_hash::ProbMinHash>()?;
    m.add_class::<weighted_minhash::WeightedMinHash>()?;
    m.add_class::<bbit_minhash::BBitMinHash>()?;
    m.add_class::<minhash_lsh::MinHashLsh>()?;
    m.add_class::<simhash_lsh::SimHashLsh>()?;
    m.add_class::<lsh_ensemble::LshEnsemble>()?;

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
