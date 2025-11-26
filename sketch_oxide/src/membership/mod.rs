//! Membership testing algorithms
//!
//! Probabilistic data structures for set membership queries.

mod binary_fuse;
mod blocked_bloom;
mod bloom;
mod counting_bloom;
mod cuckoo;
mod learned_bloom;
mod ribbon;
mod stable_bloom;
mod vacuum_filter;

pub use binary_fuse::BinaryFuseFilter;
pub use blocked_bloom::BlockedBloomFilter;
pub use bloom::BloomFilter;
pub use counting_bloom::CountingBloomFilter;
pub use cuckoo::CuckooFilter;
pub use learned_bloom::{LearnedBloomFilter, LearnedBloomStats};
pub use ribbon::RibbonFilter;
pub use stable_bloom::StableBloomFilter;
pub use vacuum_filter::{VacuumFilter, VacuumFilterStats};

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_exists() {
        // This test ensures the module compiles successfully
    }
}
