//! Membership testing algorithms
//!
//! Probabilistic data structures for set membership queries.

mod adaptive_quotient_filter;
mod aleph_filter;
mod binary_fuse;
mod blocked_bloom;
mod bloom;
mod bloomier_filter;
mod burr;
mod counting_bloom;
mod counting_quotient_filter;
mod cuckoo;
mod learned_bloom;
mod morton_filter;
mod prefix_filter;
mod ribbon;
mod scalable_bloom;
mod stable_bloom;
mod stacked_filter;
mod taffy_cuckoo_filter;
mod telescoping_filter;
mod vacuum_filter;
mod vector_quotient_filter;
mod xor_filter;

pub use adaptive_quotient_filter::AdaptiveQuotientFilter;
pub use aleph_filter::AlephFilter;
pub use binary_fuse::BinaryFuseFilter;
pub use blocked_bloom::BlockedBloomFilter;
pub use bloom::BloomFilter;
pub use bloomier_filter::BloomierFilter;
pub use burr::BurrFilter;
pub use counting_bloom::CountingBloomFilter;
pub use counting_quotient_filter::CountingQuotientFilter;
pub use cuckoo::CuckooFilter;
pub use learned_bloom::{LearnedBloomFilter, LearnedBloomStats};
pub use morton_filter::MortonFilter;
pub use prefix_filter::PrefixFilter;
pub use ribbon::RibbonFilter;
pub use scalable_bloom::ScalableBloomFilter;
pub use stable_bloom::StableBloomFilter;
pub use stacked_filter::StackedFilter;
pub use taffy_cuckoo_filter::TaffyCuckooFilter;
pub use telescoping_filter::TelescopingFilter;
pub use vacuum_filter::{VacuumFilter, VacuumFilterStats};
pub use vector_quotient_filter::VectorQuotientFilter;
pub use xor_filter::XorFilter;

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_exists() {
        // This test ensures the module compiles successfully
    }
}

#[cfg(test)]
mod capability_smoke_tests {
    use crate::common::capabilities::{Filter, Update};
    use crate::membership::{CountingBloomFilter, ScalableBloomFilter, StableBloomFilter};

    // Exercises the capability traits generically: any adopted filter that is
    // both `Update<[u8]>` and `Filter<[u8]>` can be driven through this one
    // pipeline, proving the traits are usable across concrete types.
    fn update_then_query<F: Update<[u8]> + Filter<[u8]>>(filter: &mut F) {
        let key: &[u8] = b"capability-smoke-key";
        assert!(
            !Filter::contains(filter, key),
            "fresh filter should not contain the key"
        );
        Update::update(filter, key);
        assert!(
            Filter::contains(filter, key),
            "filter should contain the key after update"
        );
    }

    #[test]
    fn membership_filters_adopt_update_and_filter_generically() {
        let mut counting = CountingBloomFilter::new(1000, 0.01);
        update_then_query(&mut counting);

        let mut scalable = ScalableBloomFilter::new(1000, 0.01).unwrap();
        update_then_query(&mut scalable);

        let mut stable = StableBloomFilter::new(1000, 0.01).unwrap();
        update_then_query(&mut stable);
    }
}
