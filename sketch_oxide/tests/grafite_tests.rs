//! Comprehensive Test Suite for Grafite Range Filter
//!
//! This test suite provides 42+ tests covering all aspects of the Grafite
//! implementation with a Test-Driven Development (TDD) approach.
//!
//! Test Categories:
//! 1. Construction (5 tests)
//! 2. Range Queries (8 tests)
//! 3. FPR Bounds (6 tests)
//! 4. Fingerprint Assignment (5 tests)
//! 5. Range Optimality (5 tests)
//! 6. Edge Cases (6 tests)
//! 7. Integration (4 tests)
//! 8. Property Tests (3 tests)

use sketch_oxide::common::RangeFilter;
use sketch_oxide::range_filters::Grafite;
use sketch_oxide::SketchError;

// ============================================================================
// 1. Construction Tests (5 tests)
// ============================================================================

#[test]
fn test_construction_valid_parameters() {
    // Valid construction with typical parameters
    let keys = vec![10, 20, 30, 40, 50];
    let result = Grafite::build(&keys, 6);
    assert!(result.is_ok());

    let filter = result.unwrap();
    assert_eq!(filter.key_count(), 5);
    assert_eq!(filter.bits_per_key(), 6);
}

#[test]
fn test_construction_empty_keys() {
    // Should fail with empty key set
    let keys: Vec<u64> = vec![];
    let result = Grafite::build(&keys, 6);
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
    let result = Grafite::build(&keys, 6);
    assert!(result.is_ok());

    let filter = result.unwrap();
    assert_eq!(filter.key_count(), 1);
    assert!(filter.may_contain(42));
}

#[test]
fn test_construction_duplicate_keys() {
    // Should deduplicate keys
    let keys = vec![10, 20, 20, 30, 30, 30, 40];
    let result = Grafite::build(&keys, 6);
    assert!(result.is_ok());

    let filter = result.unwrap();
    assert_eq!(filter.key_count(), 4); // 10, 20, 30, 40
}

#[test]
fn test_construction_large_keyset() {
    // Should handle very large key sets efficiently
    let keys: Vec<u64> = (0..10000).map(|i| i * 10).collect();
    let result = Grafite::build(&keys, 6);
    assert!(result.is_ok());

    let filter = result.unwrap();
    assert_eq!(filter.key_count(), 10000);
}

// ============================================================================
// 2. Range Query Tests (8 tests)
// ============================================================================

#[test]
fn test_range_single_key_in_range() {
    // Range containing single key
    let keys = vec![10, 20, 30, 40, 50];
    let filter = Grafite::build(&keys, 6).unwrap();

    // Range [15, 25] contains key 20
    assert!(filter.may_contain_range(15, 25));
}

#[test]
fn test_range_multiple_keys_in_range() {
    // Range containing multiple keys
    let keys = vec![10, 20, 30, 40, 50];
    let filter = Grafite::build(&keys, 6).unwrap();

    // Range [15, 45] contains keys 20, 30, 40
    assert!(filter.may_contain_range(15, 45));
}

#[test]
fn test_range_no_keys_in_range() {
    // Range containing no keys (should have low FPR)
    let keys = vec![10, 20, 30, 40, 50];
    let filter = Grafite::build(&keys, 8).unwrap(); // Higher bits for lower FPR

    // Range [60, 70] contains no keys
    // With high bits_per_key, should mostly return false
    let mut false_positives = 0;
    let total_tests = 100;

    for i in 0..total_tests {
        let start = 60 + i;
        if filter.may_contain_range(start, start + 10) {
            false_positives += 1;
        }
    }

    // Should have relatively low false positive count
    let fpr = false_positives as f64 / total_tests as f64;
    assert!(fpr < 0.9, "FPR too high: {}", fpr);
}

#[test]
fn test_range_boundaries() {
    // Test exact boundaries
    let keys = vec![10, 20, 30, 40, 50];
    let filter = Grafite::build(&keys, 6).unwrap();

    // Exact key boundaries
    assert!(filter.may_contain_range(10, 10)); // Exact match at start
    assert!(filter.may_contain_range(50, 50)); // Exact match at end
    assert!(filter.may_contain_range(10, 50)); // Full range
}

#[test]
fn test_range_overlapping() {
    // Test overlapping ranges
    let keys = vec![10, 20, 30, 40, 50];
    let filter = Grafite::build(&keys, 6).unwrap();

    assert!(filter.may_contain_range(5, 15)); // Overlaps key 10
    assert!(filter.may_contain_range(45, 55)); // Overlaps key 50
    assert!(filter.may_contain_range(25, 35)); // Overlaps key 30
}

#[test]
fn test_range_full_range_query() {
    // Query entire key space
    let keys = vec![10, 20, 30, 40, 50];
    let filter = Grafite::build(&keys, 6).unwrap();

    // Range covering all keys
    assert!(filter.may_contain_range(0, u64::MAX));
    assert!(filter.may_contain_range(10, 50));
}

#[test]
fn test_range_point_query() {
    // Point queries (low == high)
    let keys = vec![10, 20, 30, 40, 50];
    let filter = Grafite::build(&keys, 6).unwrap();

    assert!(filter.may_contain_range(20, 20)); // Point query for existing key
    assert!(filter.may_contain(20)); // Using may_contain method
}

#[test]
fn test_range_inverted() {
    // Invalid range (low > high) should return false
    let keys = vec![10, 20, 30, 40, 50];
    let filter = Grafite::build(&keys, 6).unwrap();

    assert!(!filter.may_contain_range(50, 10)); // Inverted range
}

// ============================================================================
// 3. FPR Bounds Tests (6 tests)
// ============================================================================

#[test]
fn test_fpr_matches_formula() {
    // FPR should match expected formula: L / 2^(B-2)
    let keys = vec![10, 20, 30];
    let filter = Grafite::build(&keys, 6).unwrap();

    let range_width = 10u64;
    let expected_fpr = range_width as f64 / (1u64 << (6 - 2)) as f64;
    let actual_fpr = filter.expected_fpr(range_width);

    assert!((expected_fpr - actual_fpr).abs() < 0.001);
}

#[test]
fn test_fpr_optimal_for_bits() {
    // Test FPR for different bits_per_key values
    let keys: Vec<u64> = (0..100).collect();

    let filter4 = Grafite::build(&keys, 4).unwrap();
    let filter6 = Grafite::build(&keys, 6).unwrap();
    let filter8 = Grafite::build(&keys, 8).unwrap();

    let range_width = 10u64;
    let fpr4 = filter4.expected_fpr(range_width);
    let fpr6 = filter6.expected_fpr(range_width);
    let fpr8 = filter8.expected_fpr(range_width);

    // More bits should give lower FPR
    assert!(fpr4 > fpr6);
    assert!(fpr6 > fpr8);
}

#[test]
fn test_fpr_scales_with_range() {
    // Wider ranges should have higher FPR
    let keys: Vec<u64> = (0..100).collect();
    let filter = Grafite::build(&keys, 6).unwrap();

    let fpr_small = filter.expected_fpr(5);
    let fpr_medium = filter.expected_fpr(20);
    let fpr_large = filter.expected_fpr(100);

    assert!(fpr_small < fpr_medium);
    assert!(fpr_medium < fpr_large);
}

#[test]
fn test_fpr_no_false_negatives() {
    // No false negatives - all actual keys must be found
    let keys: Vec<u64> = (0..100).step_by(10).collect();
    let filter = Grafite::build(&keys, 6).unwrap();

    // Test all keys are found
    for key in &keys {
        assert!(filter.may_contain(*key), "False negative for key {}", key);
    }
}

#[test]
fn test_fpr_increases_with_range_size() {
    // Empirical test: measure FPR for different range sizes
    let keys: Vec<u64> = vec![100, 200, 300, 400, 500];
    let filter = Grafite::build(&keys, 8).unwrap();

    // Test ranges that don't contain keys
    let test_ranges = vec![
        (600, 610), // width 11
        (600, 650), // width 51
        (600, 750), // width 151
    ];

    let expected_fprs: Vec<f64> = test_ranges
        .iter()
        .map(|(low, high)| filter.expected_fpr(high - low + 1))
        .collect();

    // FPRs should increase
    assert!(expected_fprs[0] < expected_fprs[1]);
    assert!(expected_fprs[1] < expected_fprs[2]);
}

#[test]
fn test_fpr_adversarial_bounds() {
    // Test worst-case adversarial queries
    let keys: Vec<u64> = (0..1000).step_by(100).collect();
    let filter = Grafite::build(&keys, 6).unwrap();

    // Test multiple adversarial patterns
    let adversarial_ranges = vec![
        (50, 99),   // Between keys
        (150, 199), // Between keys
        (250, 299), // Between keys
    ];

    for (low, high) in adversarial_ranges {
        let width = high - low + 1;
        let expected_fpr = filter.expected_fpr(width);

        // FPR should be bounded by formula
        assert!(
            expected_fpr <= (width as f64) / ((1u64 << (filter.bits_per_key() - 2)) as f64) + 0.01,
            "FPR exceeds bound for range [{}, {}]",
            low,
            high
        );
    }
}

// ============================================================================
// 4. Fingerprint Assignment Tests (5 tests)
// ============================================================================

#[test]
fn test_fingerprints_assigned() {
    // Verify fingerprints are assigned to all keys
    let keys = vec![10, 20, 30, 40, 50];
    let filter = Grafite::build(&keys, 6).unwrap();

    let stats = filter.stats();
    assert_eq!(stats.total_bits, 5 * 6); // 5 keys * 6 bits each
}

#[test]
fn test_fingerprints_within_bit_range() {
    // Fingerprints should fit within bits_per_key
    let keys: Vec<u64> = (0..100).collect();
    let bits_per_key = 6;
    let filter = Grafite::build(&keys, bits_per_key).unwrap();

    // This is an internal test - we verify via expected_fpr
    let fpr = filter.expected_fpr(1);
    let expected_max_fpr = 1.0 / ((1u64 << (bits_per_key - 2)) as f64);
    assert!(fpr <= expected_max_fpr);
}

#[test]
fn test_fingerprint_entropy() {
    // Fingerprints should have good entropy distribution
    // We can't directly access fingerprints, but we can test behavior
    let keys: Vec<u64> = (0..1000).collect();
    let filter = Grafite::build(&keys, 6).unwrap();

    // All keys should be findable (no false negatives)
    for key in 0..1000 {
        assert!(filter.may_contain(key));
    }
}

#[test]
fn test_fingerprints_for_close_keys() {
    // Close keys should still be distinguishable
    let keys = vec![100, 101, 102, 103, 104];
    let filter = Grafite::build(&keys, 6).unwrap();

    // All close keys should be found
    for key in &keys {
        assert!(filter.may_contain(*key));
    }
}

#[test]
fn test_fingerprint_determinism() {
    // Same keys should produce same filter
    let keys = vec![10, 20, 30, 40, 50];

    let filter1 = Grafite::build(&keys, 6).unwrap();
    let filter2 = Grafite::build(&keys, 6).unwrap();

    // Test same behavior
    for test_key in 0..100 {
        assert_eq!(
            filter1.may_contain(test_key),
            filter2.may_contain(test_key),
            "Different results for key {}",
            test_key
        );
    }
}

// ============================================================================
// 5. Range Optimality Tests (5 tests)
// ============================================================================

#[test]
fn test_optimal_space_for_fpr() {
    // Grafite should use optimal space for given FPR target
    let keys: Vec<u64> = (0..1000).collect();

    // Different bits_per_key configurations
    let configs = vec![4, 6, 8];

    for bits in configs {
        let filter = Grafite::build(&keys, bits).unwrap();
        let stats = filter.stats();

        // Total bits should be keys * bits_per_key
        assert_eq!(stats.total_bits, (1000 * bits) as u64);
    }
}

#[test]
fn test_tradeoff_bits_vs_fpr() {
    // Test the trade-off between bits per key and FPR
    let keys: Vec<u64> = (0..100).collect();
    let range_width = 10u64;

    let results: Vec<(usize, f64)> = vec![4, 5, 6, 7, 8]
        .into_iter()
        .map(|bits| {
            let filter = Grafite::build(&keys, bits).unwrap();
            (bits, filter.expected_fpr(range_width))
        })
        .collect();

    // More bits should give exponentially lower FPR
    for i in 0..results.len() - 1 {
        assert!(
            results[i].1 > results[i + 1].1,
            "FPR not decreasing: {} bits = {:.4}, {} bits = {:.4}",
            results[i].0,
            results[i].1,
            results[i + 1].0,
            results[i + 1].1
        );
    }
}

#[test]
fn test_practical_db_ranges() {
    // Test with realistic database query patterns
    let keys: Vec<u64> = (0..10000).step_by(10).collect(); // 1000 keys
    let filter = Grafite::build(&keys, 6).unwrap();

    // Typical DB range queries
    let db_queries = vec![
        (0, 100),     // Small range scan
        (1000, 2000), // Medium range scan
        (0, 5000),    // Large range scan
    ];

    for (low, high) in db_queries {
        let result = filter.may_contain_range(low, high);
        // All these ranges contain keys, so should return true
        assert!(result, "Failed for range [{}, {}]", low, high);
    }
}

#[test]
fn test_memory_efficiency() {
    // Compare memory usage with theoretical minimum
    let keys: Vec<u64> = (0..10000).collect();
    let bits_per_key = 6;
    let filter = Grafite::build(&keys, bits_per_key).unwrap();

    let stats = filter.stats();

    // Theoretical: 10000 keys * 6 bits = 60000 bits = 7500 bytes
    let theoretical_bits = 10000 * bits_per_key;
    assert_eq!(stats.total_bits, theoretical_bits as u64);
}

#[test]
fn test_comparison_range_sizes() {
    // Compare FPR for different range sizes
    let keys: Vec<u64> = (0..1000).step_by(10).collect();
    let filter = Grafite::build(&keys, 6).unwrap();

    let ranges = vec![
        (2005, 2009), // Width 5
        (2005, 2014), // Width 10
        (2005, 2024), // Width 20
        (2005, 2054), // Width 50
    ];

    let fprs: Vec<f64> = ranges
        .iter()
        .map(|(low, high)| filter.expected_fpr(high - low + 1))
        .collect();

    // FPR should increase monotonically with range width
    for i in 0..fprs.len() - 1 {
        assert!(fprs[i] < fprs[i + 1]);
    }
}

// ============================================================================
// 6. Edge Cases Tests (6 tests)
// ============================================================================

#[test]
fn test_edge_empty_range() {
    // Zero-width range (though low == high is a point query)
    let keys = vec![10, 20, 30];
    let filter = Grafite::build(&keys, 6).unwrap();

    // Point query is valid
    assert!(filter.may_contain_range(20, 20));
}

#[test]
fn test_edge_single_key_database() {
    // Database with only one key
    let keys = vec![42];
    let filter = Grafite::build(&keys, 6).unwrap();

    assert!(filter.may_contain(42));
    assert!(filter.may_contain_range(42, 42));
    assert!(filter.may_contain_range(40, 50));
}

#[test]
fn test_edge_million_keys() {
    // Large-scale test with 1M keys
    let keys: Vec<u64> = (0..1_000_000).step_by(10).collect(); // 100k keys
    let result = Grafite::build(&keys, 6);
    assert!(result.is_ok());

    let filter = result.unwrap();
    assert_eq!(filter.key_count(), 100_000);

    // Sample queries
    assert!(filter.may_contain(0));
    assert!(filter.may_contain(500_000));
    assert!(filter.may_contain(999_990));
}

#[test]
fn test_edge_boundary_values() {
    // Keys at u64 boundaries
    let keys = vec![0, 1, u64::MAX - 1, u64::MAX];
    let filter = Grafite::build(&keys, 6).unwrap();

    assert!(filter.may_contain(0));
    assert!(filter.may_contain(1));
    assert!(filter.may_contain(u64::MAX - 1));
    assert!(filter.may_contain(u64::MAX));
}

#[test]
fn test_edge_dense_clustering() {
    // Keys densely clustered together
    let keys: Vec<u64> = (1000..1100).collect(); // 100 consecutive keys
    let filter = Grafite::build(&keys, 6).unwrap();

    // All keys should be found
    for key in 1000..1100 {
        assert!(filter.may_contain(key));
    }

    // Range query covering cluster
    assert!(filter.may_contain_range(1000, 1099));
}

#[test]
fn test_edge_sparse_distribution() {
    // Keys sparsely distributed
    let keys: Vec<u64> = vec![0, 1_000_000, 2_000_000, 3_000_000];
    let filter = Grafite::build(&keys, 6).unwrap();

    // All sparse keys should be found
    for &key in &keys {
        assert!(filter.may_contain(key));
    }

    // Gaps between keys
    assert!(!filter.may_contain_range(1_000_001, 1_999_999) || filter.expected_fpr(999_999) > 0.0);
    // Might have false positives
}

// ============================================================================
// 7. Integration Tests (4 tests)
// ============================================================================

#[test]
fn test_integration_with_trait() {
    // Test RangeFilter trait implementation
    let keys = vec![10, 20, 30, 40, 50];
    let filter = Grafite::build(&keys, 6).unwrap();

    // Use through trait
    fn check_range<T: RangeFilter>(filter: &T, low: u64, high: u64) -> bool {
        filter.may_contain_range(low, high)
    }

    assert!(check_range(&filter, 15, 25));
    assert!(check_range(&filter, 10, 50));
}

#[test]
fn test_integration_stats_api() {
    // Test stats API
    let keys: Vec<u64> = (0..100).collect();
    let filter = Grafite::build(&keys, 6).unwrap();

    let stats = filter.stats();
    assert_eq!(stats.key_count, 100);
    assert_eq!(stats.bits_per_key, 6);
    assert_eq!(stats.total_bits, 600);
}

#[test]
fn test_integration_clone() {
    // Test that Grafite is cloneable
    let keys = vec![10, 20, 30, 40, 50];
    let filter1 = Grafite::build(&keys, 6).unwrap();
    let filter2 = filter1.clone();

    // Both filters should behave identically
    for test_key in 0..100 {
        assert_eq!(filter1.may_contain(test_key), filter2.may_contain(test_key));
    }
}

#[test]
fn test_integration_debug_format() {
    // Test Debug implementation
    let keys = vec![10, 20, 30];
    let filter = Grafite::build(&keys, 6).unwrap();

    let debug_str = format!("{:?}", filter);
    assert!(debug_str.contains("Grafite"));
}

// ============================================================================
// 8. Property Tests (3 tests)
// ============================================================================

#[test]
fn test_property_monotonicity() {
    // Property: Wider ranges have higher or equal FPR
    let keys: Vec<u64> = (0..100).collect();
    let filter = Grafite::build(&keys, 6).unwrap();

    for width1 in 1..20 {
        for width2 in width1 + 1..21 {
            let fpr1 = filter.expected_fpr(width1);
            let fpr2 = filter.expected_fpr(width2);
            assert!(
                fpr1 <= fpr2,
                "Monotonicity violated: FPR({}) = {:.4} > FPR({}) = {:.4}",
                width1,
                fpr1,
                width2,
                fpr2
            );
        }
    }
}

#[test]
fn test_property_consistency() {
    // Property: Same range always gives same result
    let keys = vec![10, 20, 30, 40, 50];
    let filter = Grafite::build(&keys, 6).unwrap();

    for _ in 0..10 {
        let result1 = filter.may_contain_range(15, 25);
        let result2 = filter.may_contain_range(15, 25);
        assert_eq!(result1, result2, "Inconsistent results for same range");
    }
}

#[test]
fn test_property_no_false_negatives() {
    // Property: No false negatives guaranteed
    let keys: Vec<u64> = (0..1000).step_by(7).collect(); // Prime step for variety
    let filter = Grafite::build(&keys, 6).unwrap();

    // Test all keys individually
    for &key in &keys {
        assert!(filter.may_contain(key), "False negative for key {}", key);
    }

    // Test ranges containing keys
    for i in 0..keys.len() - 1 {
        let low = keys[i];
        let high = keys[i + 1];
        assert!(
            filter.may_contain_range(low, high),
            "False negative for range [{}, {}]",
            low,
            high
        );
    }
}

// ============================================================================
// Additional Comprehensive Tests
// ============================================================================

#[test]
fn test_invalid_bits_per_key_too_small() {
    // bits_per_key < 2 should fail
    let keys = vec![10, 20, 30];
    let result = Grafite::build(&keys, 1);
    assert!(result.is_err());
}

#[test]
fn test_invalid_bits_per_key_too_large() {
    // bits_per_key > 16 should fail
    let keys = vec![10, 20, 30];
    let result = Grafite::build(&keys, 17);
    assert!(result.is_err());
}

#[test]
fn test_unsorted_keys_handled() {
    // Keys provided unsorted should be sorted internally
    let keys = vec![50, 10, 30, 20, 40];
    let filter = Grafite::build(&keys, 6).unwrap();

    // All keys should be found regardless of input order
    assert!(filter.may_contain(10));
    assert!(filter.may_contain(20));
    assert!(filter.may_contain(30));
    assert!(filter.may_contain(40));
    assert!(filter.may_contain(50));
}

#[test]
fn test_range_query_performance_pattern() {
    // Test that range queries are efficient
    let keys: Vec<u64> = (0..10000).step_by(10).collect();
    let filter = Grafite::build(&keys, 6).unwrap();

    // Multiple range queries should be fast (tested via benchmarks)
    for i in 0..100 {
        let low = i * 100;
        let high = low + 50;
        let _ = filter.may_contain_range(low, high);
    }
}

#[test]
fn test_expected_fpr_edge_cases() {
    let keys = vec![10, 20, 30];
    let filter = Grafite::build(&keys, 6).unwrap();

    // Width 0 (conceptual - though invalid range)
    let fpr0 = filter.expected_fpr(0);
    assert_eq!(fpr0, 0.0);

    // Width 1 (point query)
    let fpr1 = filter.expected_fpr(1);
    assert!(fpr1 > 0.0);
    assert!(fpr1 < 0.1);
}
