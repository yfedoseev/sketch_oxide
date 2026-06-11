//! Membership testing algorithms
//!
//! Probabilistic data structures for set membership queries.

mod adaptive_quotient_filter;
mod binary_fuse;
mod blocked_bloom;
mod bloom;
mod burr;
mod counting_bloom;
mod counting_quotient_filter;
mod cuckoo;
mod learned_bloom;
mod prefix_filter;
mod ribbon;
mod scalable_bloom;
mod stable_bloom;
mod stacked_filter;
mod vacuum_filter;
mod vector_quotient_filter;

pub use adaptive_quotient_filter::AdaptiveQuotientFilter;
pub use binary_fuse::BinaryFuseFilter;
pub use blocked_bloom::BlockedBloomFilter;
pub use bloom::BloomFilter;
pub use burr::BurrFilter;
pub use counting_bloom::CountingBloomFilter;
pub use counting_quotient_filter::CountingQuotientFilter;
pub use cuckoo::CuckooFilter;
pub use learned_bloom::{LearnedBloomFilter, LearnedBloomStats};
pub use prefix_filter::PrefixFilter;
pub use ribbon::RibbonFilter;
pub use scalable_bloom::ScalableBloomFilter;
pub use stable_bloom::StableBloomFilter;
pub use stacked_filter::StackedFilter;
pub use vacuum_filter::{VacuumFilter, VacuumFilterStats};
pub use vector_quotient_filter::VectorQuotientFilter;

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_exists() {
        // This test ensures the module compiles successfully
    }
}
