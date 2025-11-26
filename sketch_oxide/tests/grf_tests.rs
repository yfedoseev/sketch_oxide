//! Comprehensive Test Suite for GRF (Gorilla Range Filter)
//!
//! This test suite provides 55+ tests covering all aspects of the GRF
//! implementation with a Test-Driven Development (TDD) approach.
//!
//! Test Categories:
//! 1. Construction (5 tests)
//! 2. Range Queries (10 tests)
//! 3. Shape Encoding (8 tests)
//! 4. FPR Bounds (8 tests)
//! 5. LSM-Tree Integration (7 tests)
//! 6. Memory Efficiency (8 tests)
//! 7. Edge Cases (8 tests)
//! 8. Property Tests (5 tests)

use sketch_oxide::common::RangeFilter;
use sketch_oxide::range_filters::GRF;
use sketch_oxide::SketchError;

// ============================================================================
// 1. Construction Tests (5 tests)
// ============================================================================

#[test]
fn test_construction_valid_parameters() {
    // Valid construction with typical parameters
    let keys = vec![10, 20, 30, 40, 50];
    let result = GRF::build(&keys, 6);
    assert!(result.is_ok());

    let filter = result.unwrap();
    assert_eq!(filter.key_count(), 5);
    assert_eq!(filter.bits_per_key(), 6);
    assert!(filter.segment_count() > 0);
}

#[test]
fn test_construction_empty_keys() {
    // Should fail with empty key set
    let keys: Vec<u64> = vec![];
    let result = GRF::build(&keys, 6);
    assert!(result.is_err());

    match result {
        Err(SketchError::InvalidParameter { param, .. }) => {
            assert_eq!(param, "keys");
        }
        _ => panic!("Expected InvalidParameter error for keys"),
    }
}

#[test]
fn test_construction_single_key() {
    // Should work with single key
    let keys = vec![42];
    let result = GRF::build(&keys, 6);
    assert!(result.is_ok());

    let filter = result.unwrap();
    assert_eq!(filter.key_count(), 1);
    assert!(filter.may_contain(42));
}

#[test]
fn test_construction_unsorted_keys() {
    // Should handle unsorted keys (will be sorted internally)
    let keys = vec![50, 10, 30, 20, 40];
    let result = GRF::build(&keys, 6);
    assert!(result.is_ok());

    let filter = result.unwrap();
    assert_eq!(filter.key_count(), 5);
    assert!(filter.may_contain(10));
    assert!(filter.may_contain(50));
}

#[test]
fn test_construction_large_keyset() {
    // Should handle very large key sets efficiently
    let keys: Vec<u64> = (0..1_000_000).map(|i| i * 10).collect();
    let result = GRF::build(&keys, 6);
    assert!(result.is_ok());

    let filter = result.unwrap();
    assert_eq!(filter.key_count(), 1_000_000);
    assert!(filter.segment_count() > 0);
}

// ============================================================================
// 2. Range Query Tests (10 tests)
// ============================================================================

#[test]
fn test_range_single_key_in_range() {
    // Range containing single key
    let keys = vec![10, 20, 30, 40, 50];
    let filter = GRF::build(&keys, 6).unwrap();

    // Range [15, 25] contains key 20
    assert!(filter.may_contain_range(15, 25));
}

#[test]
fn test_range_multiple_keys_in_range() {
    // Range containing multiple keys
    let keys = vec![10, 20, 30, 40, 50];
    let filter = GRF::build(&keys, 6).unwrap();

    // Range [15, 45] contains keys 20, 30, 40
    assert!(filter.may_contain_range(15, 45));
}

#[test]
fn test_range_no_keys_in_range() {
    // Range containing no keys
    let keys = vec![10, 20, 30, 40, 50];
    let filter = GRF::build(&keys, 8).unwrap(); // Higher bits for lower FPR

    // Range [60, 70] contains no keys
    assert!(!filter.may_contain_range(60, 70));
}

#[test]
fn test_range_boundaries() {
    // Test exact boundaries
    let keys = vec![10, 20, 30, 40, 50];
    let filter = GRF::build(&keys, 6).unwrap();

    // Exact key boundaries
    assert!(filter.may_contain_range(10, 10)); // Point query
    assert!(filter.may_contain_range(50, 50)); // Point query
    assert!(filter.may_contain_range(10, 50)); // Full range
}

#[test]
fn test_range_full_range_query() {
    // Query entire range
    let keys = vec![10, 20, 30, 40, 50];
    let filter = GRF::build(&keys, 6).unwrap();

    assert!(filter.may_contain_range(0, 100)); // Contains all keys
}

#[test]
fn test_range_point_query() {
    // Point query (low == high)
    let keys = vec![10, 20, 30, 40, 50];
    let filter = GRF::build(&keys, 6).unwrap();

    assert!(filter.may_contain_range(20, 20)); // Exact match
    assert!(filter.may_contain_range(30, 30)); // Exact match
}

#[test]
fn test_range_inverted_range() {
    // Inverted range (low > high)
    let keys = vec![10, 20, 30, 40, 50];
    let filter = GRF::build(&keys, 6).unwrap();

    assert!(!filter.may_contain_range(50, 10)); // Invalid range
}

#[test]
fn test_range_overlapping_ranges() {
    // Test overlapping ranges
    let keys = vec![10, 20, 30, 40, 50];
    let filter = GRF::build(&keys, 6).unwrap();

    assert!(filter.may_contain_range(5, 15)); // Overlaps key 10
    assert!(filter.may_contain_range(45, 55)); // Overlaps key 50
}

#[test]
fn test_range_before_all_keys() {
    // Range before all keys
    let keys = vec![100, 200, 300, 400, 500];
    let filter = GRF::build(&keys, 6).unwrap();

    assert!(!filter.may_contain_range(10, 50)); // No keys in range
}

#[test]
fn test_range_after_all_keys() {
    // Range after all keys
    let keys = vec![100, 200, 300, 400, 500];
    let filter = GRF::build(&keys, 6).unwrap();

    assert!(!filter.may_contain_range(600, 700)); // No keys in range
}

// ============================================================================
// 3. Shape Encoding Tests (8 tests)
// ============================================================================

#[test]
fn test_shape_segment_creation() {
    // Verify segments are created
    let keys = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
    let filter = GRF::build(&keys, 6).unwrap();

    assert!(filter.segment_count() > 0);
    assert!(filter.segment_count() <= keys.len());
}

#[test]
fn test_shape_adaptive_segments_few_bits() {
    // Fewer bits should create more segments
    let keys: Vec<u64> = (0..100).map(|i| i * 10).collect();

    let filter_4bit = GRF::build(&keys, 4).unwrap();
    let filter_8bit = GRF::build(&keys, 8).unwrap();

    // Fewer bits = more segments (better precision)
    assert!(filter_4bit.segment_count() >= filter_8bit.segment_count());
}

#[test]
fn test_shape_uniform_distribution() {
    // Uniform distribution should create evenly sized segments
    let keys: Vec<u64> = (0..100).map(|i| i).collect();
    let filter = GRF::build(&keys, 6).unwrap();

    let stats = filter.stats();
    assert!(stats.avg_keys_per_segment > 0.0);
}

#[test]
fn test_shape_skewed_distribution() {
    // Skewed distribution (Zipf-like)
    let mut keys = Vec::new();
    keys.extend(vec![1; 100]); // Heavy key
    keys.extend(vec![2; 50]); // Medium key
    keys.extend(vec![3; 25]); // Light key
    keys.extend((4..20).collect::<Vec<u64>>()); // Tail

    let filter = GRF::build(&keys, 6).unwrap();
    assert!(filter.segment_count() > 0);

    // Should handle skewed data well
    assert!(filter.may_contain_range(1, 3));
}

#[test]
fn test_shape_large_gaps() {
    // Keys with large gaps
    let keys = vec![10, 100, 200, 1000, 10000];
    let filter = GRF::build(&keys, 6).unwrap();

    assert!(filter.segment_count() > 0);
    assert!(filter.may_contain_range(50, 150)); // Contains 100
}

#[test]
fn test_shape_dense_then_sparse() {
    // Dense region followed by sparse region
    let mut keys: Vec<u64> = (0..50).collect(); // Dense
    keys.extend((100..105).map(|i| i * 100)); // Sparse

    let filter = GRF::build(&keys, 6).unwrap();
    assert!(filter.segment_count() > 1);
}

#[test]
fn test_shape_fibonacci_sequence() {
    // Fibonacci sequence (naturally growing gaps)
    let keys = vec![1, 2, 3, 5, 8, 13, 21, 34, 55, 89];
    let filter = GRF::build(&keys, 6).unwrap();

    assert!(filter.segment_count() > 0);
    assert!(filter.may_contain_range(10, 25)); // Contains 13, 21
}

#[test]
fn test_shape_power_of_two_keys() {
    // Powers of 2 (exponentially growing gaps)
    let keys = vec![1, 2, 4, 8, 16, 32, 64, 128, 256, 512];
    let filter = GRF::build(&keys, 6).unwrap();

    assert!(filter.segment_count() > 0);
    assert!(filter.may_contain_range(60, 130)); // Contains 64, 128
}

// ============================================================================
// 4. FPR Bounds Tests (8 tests)
// ============================================================================

#[test]
fn test_fpr_calculation_basic() {
    // Basic FPR calculation
    let keys = vec![10, 20, 30, 40, 50];
    let filter = GRF::build(&keys, 6).unwrap();

    let fpr = filter.expected_fpr(10);
    assert!(fpr >= 0.0 && fpr <= 1.0);
}

#[test]
fn test_fpr_increases_with_range_width() {
    // Larger ranges should have higher FPR
    let keys = vec![10, 20, 30, 40, 50];
    let filter = GRF::build(&keys, 6).unwrap();

    let fpr_small = filter.expected_fpr(10);
    let fpr_large = filter.expected_fpr(100);

    assert!(fpr_large >= fpr_small);
}

#[test]
fn test_fpr_decreases_with_more_bits() {
    // More bits per key should decrease FPR
    let keys = vec![10, 20, 30, 40, 50];

    let filter_4bit = GRF::build(&keys, 4).unwrap();
    let filter_8bit = GRF::build(&keys, 8).unwrap();

    let fpr_4bit = filter_4bit.expected_fpr(10);
    let fpr_8bit = filter_8bit.expected_fpr(10);

    assert!(fpr_8bit <= fpr_4bit);
}

#[test]
fn test_fpr_zero_range() {
    // Zero range width should have zero FPR
    let keys = vec![10, 20, 30, 40, 50];
    let filter = GRF::build(&keys, 6).unwrap();

    let fpr = filter.expected_fpr(0);
    assert_eq!(fpr, 0.0);
}

#[test]
fn test_fpr_skewed_vs_uniform() {
    // GRF should perform better on skewed data
    // Uniform distribution
    let uniform_keys: Vec<u64> = (0..100).map(|i| i * 10).collect();
    let uniform_filter = GRF::build(&uniform_keys, 6).unwrap();

    // Skewed distribution
    let mut skewed_keys = Vec::new();
    skewed_keys.extend(vec![1; 50]);
    skewed_keys.extend(vec![2; 25]);
    skewed_keys.extend((3..30).collect::<Vec<u64>>());

    let skewed_filter = GRF::build(&skewed_keys, 6).unwrap();

    // Both should have reasonable FPR
    let uniform_fpr = uniform_filter.expected_fpr(10);
    let skewed_fpr = skewed_filter.expected_fpr(10);

    assert!(uniform_fpr < 1.0);
    assert!(skewed_fpr < 1.0);
}

#[test]
fn test_fpr_no_false_negatives() {
    // Verify no false negatives (if key exists, must return true)
    let keys = vec![10, 20, 30, 40, 50];
    let filter = GRF::build(&keys, 6).unwrap();

    // All existing keys must be found
    for key in keys {
        assert!(filter.may_contain(key), "False negative for key {}", key);
    }
}

#[test]
fn test_fpr_range_no_false_negatives() {
    // Verify no false negatives for ranges
    let keys = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
    let filter = GRF::build(&keys, 6).unwrap();

    // Any range containing actual keys must return true
    assert!(filter.may_contain_range(15, 25)); // Contains 20
    assert!(filter.may_contain_range(35, 65)); // Contains 40, 50, 60
    assert!(filter.may_contain_range(85, 105)); // Contains 90, 100
}

#[test]
fn test_fpr_empirical_measurement() {
    // Empirical FPR measurement
    let keys: Vec<u64> = (0..1000).map(|i| i * 100).collect();
    let filter = GRF::build(&keys, 8).unwrap();

    let mut false_positives = 0;
    let total_queries = 1000;

    // Query ranges that don't contain keys
    for i in 0..total_queries {
        let start = 50 + i * 100; // Between keys
        if filter.may_contain_range(start, start + 10) {
            false_positives += 1;
        }
    }

    let empirical_fpr = false_positives as f64 / total_queries as f64;
    assert!(empirical_fpr < 0.5); // Should be reasonably low
}

// ============================================================================
// 5. LSM-Tree Integration Tests (7 tests)
// ============================================================================

#[test]
fn test_lsm_single_level() {
    // Simulate single LSM level
    let keys: Vec<u64> = (0..100).map(|i| i * 10).collect();
    let filter = GRF::build(&keys, 6).unwrap();

    // Range query for SSTable
    assert!(filter.may_contain_range(200, 300)); // Contains multiple keys
}

#[test]
fn test_lsm_multiple_levels() {
    // Simulate multiple LSM levels
    let level0_keys: Vec<u64> = (0..100).collect();
    let level1_keys: Vec<u64> = (1000..1100).collect();
    let level2_keys: Vec<u64> = (2000..2100).collect();

    let filter0 = GRF::build(&level0_keys, 6).unwrap();
    let filter1 = GRF::build(&level1_keys, 6).unwrap();
    let filter2 = GRF::build(&level2_keys, 6).unwrap();

    // Query should check all levels
    assert!(filter0.may_contain_range(50, 60));
    assert!(!filter0.may_contain_range(1000, 1100));

    assert!(filter1.may_contain_range(1050, 1060));
    assert!(!filter1.may_contain_range(0, 100));

    assert!(filter2.may_contain_range(2050, 2060));
}

#[test]
fn test_lsm_compaction_scenario() {
    // Simulate compaction (merging sorted runs)
    let run1: Vec<u64> = (0..50).map(|i| i * 2).collect();
    let run2: Vec<u64> = (0..50).map(|i| i * 2 + 1).collect();

    let mut merged = run1;
    merged.extend(run2);
    merged.sort_unstable();

    let filter = GRF::build(&merged, 6).unwrap();
    assert_eq!(filter.key_count(), 100);
}

#[test]
fn test_lsm_bloom_replacement() {
    // GRF as bloom filter replacement for LSM
    let keys: Vec<u64> = (0..10000).map(|i| i).collect();
    let filter = GRF::build(&keys, 6).unwrap();

    // Point queries (like bloom filter)
    assert!(filter.may_contain(5000));
    assert!(filter.may_contain(9999));

    // Range queries (advantage over bloom)
    assert!(filter.may_contain_range(5000, 6000));
}

#[test]
fn test_lsm_sstable_footer_size() {
    // Verify filter fits in SSTable footer
    let keys: Vec<u64> = (0..1000).collect();
    let filter = GRF::build(&keys, 6).unwrap();

    let stats = filter.stats();
    // Should be compact enough for SSTable metadata
    assert!(stats.memory_bytes < 100_000); // <100KB
}

#[test]
fn test_lsm_range_delete_optimization() {
    // Range deletes in LSM can benefit from range filters
    let keys: Vec<u64> = (0..1000).collect();
    let filter = GRF::build(&keys, 6).unwrap();

    // Check if range delete [500, 600] affects this SSTable
    assert!(filter.may_contain_range(500, 600));
}

#[test]
fn test_lsm_prefix_scan() {
    // Prefix scans common in LSM trees
    let keys: Vec<u64> = (0..1000).map(|i| i * 1000).collect();
    let filter = GRF::build(&keys, 6).unwrap();

    // Prefix scan from 500000
    assert!(filter.may_contain_range(500000, 600000));
}

// ============================================================================
// 6. Memory Efficiency Tests (8 tests)
// ============================================================================

#[test]
fn test_memory_stats_basic() {
    // Basic memory stats
    let keys = vec![10, 20, 30, 40, 50];
    let filter = GRF::build(&keys, 6).unwrap();

    let stats = filter.stats();
    assert!(stats.memory_bytes > 0);
    assert_eq!(stats.key_count, 5);
}

#[test]
fn test_memory_bits_per_key() {
    // Verify bits per key calculation
    let keys = vec![10, 20, 30, 40, 50];
    let filter = GRF::build(&keys, 6).unwrap();

    let stats = filter.stats();
    assert_eq!(stats.bits_per_key, 6);
    assert_eq!(stats.total_bits, 30); // 5 keys * 6 bits
}

#[test]
fn test_memory_segment_overhead() {
    // Measure segment overhead
    let keys: Vec<u64> = (0..1000).collect();
    let filter = GRF::build(&keys, 6).unwrap();

    let stats = filter.stats();
    assert!(stats.segment_count > 0);
    assert!(stats.avg_keys_per_segment > 0.0);
}

#[test]
fn test_memory_large_dataset() {
    // Memory efficiency for large dataset
    let keys: Vec<u64> = (0..100_000).map(|i| i).collect();
    let filter = GRF::build(&keys, 6).unwrap();

    let stats = filter.stats();
    // Should be reasonable memory usage
    assert!(stats.memory_bytes < 10_000_000); // <10MB
}

#[test]
fn test_memory_varying_bits_per_key() {
    // Memory scales with bits per key
    let keys: Vec<u64> = (0..1000).collect();

    let filter_4bit = GRF::build(&keys, 4).unwrap();
    let filter_8bit = GRF::build(&keys, 8).unwrap();

    let stats_4bit = filter_4bit.stats();
    let stats_8bit = filter_8bit.stats();

    assert!(stats_8bit.total_bits > stats_4bit.total_bits);
}

#[test]
fn test_memory_deduplication() {
    // Verify deduplication saves memory
    let keys_with_dups = vec![10, 10, 20, 20, 30, 30, 40, 40, 50, 50];
    let filter = GRF::build(&keys_with_dups, 6).unwrap();

    assert_eq!(filter.key_count(), 5); // Deduplicated
    let stats = filter.stats();
    assert_eq!(stats.total_bits, 30); // 5 * 6, not 10 * 6
}

#[test]
fn test_memory_sparse_keys() {
    // Sparse keys shouldn't waste memory
    let keys = vec![1, 1000, 1_000_000, 1_000_000_000];
    let filter = GRF::build(&keys, 6).unwrap();

    let stats = filter.stats();
    assert_eq!(stats.key_count, 4);
    assert_eq!(stats.total_bits, 24); // 4 * 6
}

#[test]
fn test_memory_comparison_with_theoretical() {
    // Compare actual memory with theoretical minimum
    let keys: Vec<u64> = (0..1000).collect();
    let filter = GRF::build(&keys, 6).unwrap();

    let stats = filter.stats();
    let theoretical_bits = 1000 * 6; // Minimum for fingerprints
    let actual_bits = stats.total_bits;

    assert_eq!(actual_bits, theoretical_bits);
}

// ============================================================================
// 7. Edge Cases Tests (8 tests)
// ============================================================================

#[test]
fn test_edge_empty_range() {
    // Empty range (low == high)
    let keys = vec![10, 20, 30, 40, 50];
    let filter = GRF::build(&keys, 6).unwrap();

    assert!(filter.may_contain_range(20, 20)); // Point query
}

#[test]
fn test_edge_single_key_database() {
    // Database with single key
    let keys = vec![42];
    let filter = GRF::build(&keys, 6).unwrap();

    assert!(filter.may_contain(42));
    assert!(filter.may_contain_range(42, 42));
    assert!(!filter.may_contain_range(100, 200));
}

#[test]
fn test_edge_very_large_range() {
    // Very large range query
    let keys = vec![10, 20, 30, 40, 50];
    let filter = GRF::build(&keys, 6).unwrap();

    assert!(filter.may_contain_range(0, u64::MAX)); // Contains all
}

#[test]
fn test_edge_boundary_values() {
    // Boundary values (0, u64::MAX)
    let keys = vec![0, u64::MAX];
    let filter = GRF::build(&keys, 6).unwrap();

    assert!(filter.may_contain(0));
    assert!(filter.may_contain(u64::MAX));
    assert!(filter.may_contain_range(0, u64::MAX));
}

#[test]
fn test_edge_consecutive_keys() {
    // All consecutive keys
    let keys: Vec<u64> = (0..100).collect();
    let filter = GRF::build(&keys, 6).unwrap();

    assert!(filter.may_contain_range(50, 60));
}

#[test]
fn test_edge_extreme_skew() {
    // Extreme skew (99% one key, 1% others)
    let mut keys = vec![1; 9900];
    keys.extend((2..102).collect::<Vec<u64>>());

    let filter = GRF::build(&keys, 6).unwrap();
    assert!(filter.may_contain(1));
    assert!(filter.may_contain_range(50, 60));
}

#[test]
fn test_edge_duplicate_handling() {
    // Many duplicates
    let keys = vec![10, 10, 10, 20, 20, 20, 30, 30, 30];
    let filter = GRF::build(&keys, 6).unwrap();

    assert_eq!(filter.key_count(), 3);
    assert!(filter.may_contain(10));
    assert!(filter.may_contain(20));
    assert!(filter.may_contain(30));
}

#[test]
fn test_edge_minimum_bits_per_key() {
    // Minimum valid bits per key
    let keys = vec![10, 20, 30, 40, 50];
    let filter = GRF::build(&keys, 2).unwrap();

    assert_eq!(filter.bits_per_key(), 2);
}

// ============================================================================
// 8. Property Tests (5 tests)
// ============================================================================

#[test]
fn test_property_no_false_negatives() {
    // Property: No false negatives
    let keys: Vec<u64> = (0..100).map(|i| i * 7).collect();
    let filter = GRF::build(&keys, 6).unwrap();

    // All existing keys must return true
    for key in keys {
        assert!(filter.may_contain(key), "False negative for key {}", key);
    }
}

#[test]
fn test_property_consistency() {
    // Property: Consistent results for same query
    let keys = vec![10, 20, 30, 40, 50];
    let filter = GRF::build(&keys, 6).unwrap();

    // Same query should always return same result
    let result1 = filter.may_contain_range(15, 25);
    let result2 = filter.may_contain_range(15, 25);
    let result3 = filter.may_contain_range(15, 25);

    assert_eq!(result1, result2);
    assert_eq!(result2, result3);
}

#[test]
fn test_property_monotonicity() {
    // Property: Larger ranges include smaller ranges
    let keys = vec![10, 20, 30, 40, 50];
    let filter = GRF::build(&keys, 6).unwrap();

    // If small range has keys, larger range should too
    if filter.may_contain_range(20, 30) {
        assert!(filter.may_contain_range(10, 40));
    }
}

#[test]
fn test_property_commutativity_of_build() {
    // Property: Build result independent of input order
    let keys1 = vec![10, 20, 30, 40, 50];
    let keys2 = vec![50, 40, 30, 20, 10];

    let filter1 = GRF::build(&keys1, 6).unwrap();
    let filter2 = GRF::build(&keys2, 6).unwrap();

    assert_eq!(filter1.key_count(), filter2.key_count());

    // Same queries should return same results
    assert_eq!(
        filter1.may_contain_range(15, 25),
        filter2.may_contain_range(15, 25)
    );
}

#[test]
fn test_property_range_subdivision() {
    // Property: If range [a,b] returns true, at least one of [a,m] or [m,b] should too
    let keys: Vec<u64> = (0..100).map(|i| i * 10).collect();
    let filter = GRF::build(&keys, 6).unwrap();

    let low = 100u64;
    let high = 500u64;
    let mid = (low + high) / 2;

    if filter.may_contain_range(low, high) {
        let left = filter.may_contain_range(low, mid);
        let right = filter.may_contain_range(mid, high);
        // At least one half should contain keys (or be false positive)
        assert!(left || right);
    }
}

// ============================================================================
// Additional Integration Tests
// ============================================================================

#[test]
fn test_stats_comprehensive() {
    // Comprehensive stats test
    let keys: Vec<u64> = (0..1000).collect();
    let filter = GRF::build(&keys, 6).unwrap();

    let stats = filter.stats();

    assert_eq!(stats.key_count, 1000);
    assert!(stats.segment_count > 0);
    assert!(stats.avg_keys_per_segment > 0.0);
    assert_eq!(stats.bits_per_key, 6);
    assert_eq!(stats.total_bits, 6000);
    assert!(stats.memory_bytes > 0);
}

#[test]
fn test_multiple_builds() {
    // Multiple builds with same data should work
    let keys = vec![10, 20, 30, 40, 50];

    let filter1 = GRF::build(&keys, 6).unwrap();
    let filter2 = GRF::build(&keys, 6).unwrap();

    assert_eq!(filter1.key_count(), filter2.key_count());
}

#[test]
fn test_bits_per_key_validation() {
    let keys = vec![10, 20, 30];

    // Valid bits per key
    assert!(GRF::build(&keys, 2).is_ok());
    assert!(GRF::build(&keys, 8).is_ok());
    assert!(GRF::build(&keys, 16).is_ok());

    // Invalid bits per key
    assert!(GRF::build(&keys, 1).is_err());
    assert!(GRF::build(&keys, 17).is_err());
    assert!(GRF::build(&keys, 100).is_err());
}
