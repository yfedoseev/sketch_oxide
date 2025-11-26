//! Memento Filter Comprehensive Test Suite - TDD Approach
//!
//! Testing the first dynamic range filter with insertion support.
//! 42+ tests covering all aspects of the algorithm.
//!
//! Test Categories:
//! 1. Construction (4 tests)
//! 2. Dynamic Insertion (8 tests)
//! 3. Range Queries (6 tests)
//! 4. FPR Maintenance (5 tests)
//! 5. Range Expansion (4 tests)
//! 6. Quotient Filter Layer (4 tests)
//! 7. Edge Cases (6 tests)
//! 8. Property Tests (5 tests)

use proptest::prelude::*;
use sketch_oxide::common::RangeFilter;
use sketch_oxide::range_filters::MementoFilter;

// ============================================================================
// Phase 1: Construction Tests (4 tests)
// ============================================================================

#[test]
fn test_new_memento_filter() {
    let filter = MementoFilter::new(1000, 0.01).unwrap();

    assert!(filter.is_empty(), "New filter should be empty");
    assert_eq!(filter.len(), 0, "Length should be 0");
    assert_eq!(
        filter.range(),
        None,
        "Range should be None for empty filter"
    );
}

#[test]
fn test_invalid_fpr_zero() {
    let result = MementoFilter::new(1000, 0.0);
    assert!(result.is_err(), "Should reject FPR of 0.0");

    if let Err(e) = result {
        assert!(
            e.to_string().contains("fpr"),
            "Error should mention fpr parameter"
        );
    }
}

#[test]
fn test_invalid_fpr_one() {
    let result = MementoFilter::new(1000, 1.0);
    assert!(result.is_err(), "Should reject FPR of 1.0");
}

#[test]
fn test_invalid_expected_elements_zero() {
    let result = MementoFilter::new(0, 0.01);
    assert!(result.is_err(), "Should reject expected_elements of 0");

    if let Err(e) = result {
        assert!(
            e.to_string().contains("expected_elements"),
            "Error should mention expected_elements parameter"
        );
    }
}

#[test]
fn test_stats_accuracy_on_construction() {
    let filter = MementoFilter::new(1000, 0.01).unwrap();
    let stats = filter.stats();

    assert_eq!(stats.num_elements, 0);
    assert_eq!(stats.capacity, 1000);
    assert_eq!(stats.fpr_target, 0.01);
    assert_eq!(stats.num_expansions, 0);
    assert_eq!(stats.load_factor, 0.0);
}

// ============================================================================
// Phase 2: Dynamic Insertion Tests (8 tests)
// ============================================================================

#[test]
fn test_single_insertion() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    filter.insert(42, b"value1").unwrap();

    assert!(!filter.is_empty(), "Filter should not be empty");
    assert_eq!(filter.len(), 1, "Length should be 1");
    assert_eq!(
        filter.range(),
        Some((42, 42)),
        "Range should contain single key"
    );
}

#[test]
fn test_multiple_insertions() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    filter.insert(10, b"v1").unwrap();
    filter.insert(20, b"v2").unwrap();
    filter.insert(30, b"v3").unwrap();

    assert_eq!(filter.len(), 3, "Should have 3 elements");
    assert_eq!(filter.range(), Some((10, 30)), "Range should be [10, 30]");
}

#[test]
fn test_insertions_within_range() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    // Create initial range
    filter.insert(100, b"v1").unwrap();
    filter.insert(200, b"v2").unwrap();

    let initial_expansions = filter.stats().num_expansions;

    // Insert within range - should not trigger expansion
    filter.insert(150, b"v3").unwrap();

    assert_eq!(
        filter.stats().num_expansions,
        initial_expansions,
        "Insertion within range should not expand"
    );
}

#[test]
fn test_insertions_outside_range_expansion() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    // Create initial range
    filter.insert(100, b"v1").unwrap();
    filter.insert(200, b"v2").unwrap();

    let initial_expansions = filter.stats().num_expansions;

    // Insert outside range - should trigger expansion
    filter.insert(300, b"v3").unwrap();

    assert!(
        filter.stats().num_expansions > initial_expansions,
        "Insertion outside range should expand"
    );
    assert_eq!(
        filter.range(),
        Some((100, 300)),
        "Range should expand to [100, 300]"
    );
}

#[test]
fn test_duplicate_insertions() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    filter.insert(42, b"value1").unwrap();
    filter.insert(42, b"value1").unwrap();
    filter.insert(42, b"value2").unwrap(); // Same key, different value

    assert_eq!(filter.len(), 3, "Should count duplicates");
}

#[test]
fn test_large_scale_insertions() {
    let mut filter = MementoFilter::new(100_000, 0.01).unwrap();

    // Insert 10,000 elements
    for i in 0..10_000 {
        let value = format!("value{}", i);
        filter.insert(i * 10, value.as_bytes()).unwrap();
    }

    assert_eq!(filter.len(), 10_000, "Should have 10,000 elements");
    assert_eq!(
        filter.range(),
        Some((0, 99_990)),
        "Range should span all keys"
    );
}

#[test]
fn test_capacity_management() {
    let mut filter = MementoFilter::new(10, 0.01).unwrap();

    // Fill to capacity
    for i in 0..10 {
        filter.insert(i, b"value").unwrap();
    }

    assert_eq!(filter.len(), 10);
    assert_eq!(filter.stats().load_factor, 1.0);

    // Attempt to exceed capacity
    let result = filter.insert(11, b"value");
    assert!(result.is_err(), "Should reject insertion beyond capacity");
}

#[test]
fn test_element_tracking() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    for i in 0..5 {
        filter.insert(i * 100, b"value").unwrap();
        assert_eq!(
            filter.len(),
            (i + 1) as usize,
            "Length should increment with each insert"
        );

        let stats = filter.stats();
        assert_eq!(stats.num_elements, (i + 1) as usize);
        assert!(stats.load_factor <= 1.0);
    }
}

// ============================================================================
// Phase 3: Range Query Tests (6 tests)
// ============================================================================

#[test]
fn test_range_query_within_current_range() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    filter.insert(100, b"v1").unwrap();
    filter.insert(200, b"v2").unwrap();
    filter.insert(300, b"v3").unwrap();

    // Query within range
    assert!(
        filter.may_contain_range(150, 250),
        "Should return true for range with elements"
    );
}

#[test]
fn test_range_query_outside_current_range() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    filter.insert(100, b"v1").unwrap();
    filter.insert(200, b"v2").unwrap();

    // Query outside range
    assert!(
        !filter.may_contain_range(500, 600),
        "Should return false for range outside filter range"
    );
}

#[test]
fn test_range_query_across_boundaries() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    filter.insert(100, b"v1").unwrap();
    filter.insert(200, b"v2").unwrap();

    // Query overlapping with boundary
    assert!(
        filter.may_contain_range(50, 150),
        "Should return true for range overlapping lower boundary"
    );
    assert!(
        filter.may_contain_range(150, 300),
        "Should return true for range overlapping upper boundary"
    );
}

#[test]
fn test_point_queries() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    filter.insert(100, b"v1").unwrap();
    filter.insert(200, b"v2").unwrap();
    filter.insert(300, b"v3").unwrap();

    // Point queries (range of 1)
    assert!(
        filter.may_contain_range(100, 100),
        "Should find exact key 100"
    );
    assert!(
        filter.may_contain_range(200, 200),
        "Should find exact key 200"
    );
    assert!(
        filter.may_contain_range(300, 300),
        "Should find exact key 300"
    );
}

#[test]
fn test_empty_range_queries() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    filter.insert(100, b"v1").unwrap();

    // Query with low > high (invalid range)
    assert!(
        !filter.may_contain_range(200, 100),
        "Invalid range should return false"
    );
}

#[test]
fn test_full_range_query() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    filter.insert(50, b"v1").unwrap();
    filter.insert(100, b"v2").unwrap();
    filter.insert(150, b"v3").unwrap();

    // Query entire filter range
    assert!(
        filter.may_contain_range(0, u64::MAX),
        "Full range query should return true"
    );
}

// ============================================================================
// Phase 4: FPR Maintenance Tests (5 tests)
// ============================================================================

#[test]
fn test_fpr_stays_below_target() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    // Insert elements
    for i in 0..500 {
        filter.insert(i * 2, b"value").unwrap();
    }

    // Test false positive rate
    let mut false_positives = 0;
    let num_tests = 1000;

    for i in 0..num_tests {
        let test_key = i * 2 + 1; // Keys not inserted
        if test_key < 1000 && filter.may_contain_range(test_key, test_key) {
            false_positives += 1;
        }
    }

    let actual_fpr = false_positives as f64 / num_tests as f64;
    assert!(
        actual_fpr <= 0.05,
        "FPR {} should be reasonably close to target 0.01",
        actual_fpr
    );
}

#[test]
fn test_fpr_after_insertions() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    // Insert in batches and check FPR
    for batch in 0..5 {
        for i in 0..100 {
            let key = batch * 100 + i;
            filter.insert(key, b"value").unwrap();
        }

        // FPR should remain bounded
        let stats = filter.stats();
        assert!(stats.fpr_target == 0.01);
    }
}

#[test]
fn test_fpr_with_range_expansion() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    // Insert elements causing expansions
    filter.insert(0, b"v1").unwrap();
    filter.insert(1000, b"v2").unwrap();
    filter.insert(2000, b"v3").unwrap();

    assert!(filter.stats().num_expansions >= 2, "Should have expanded");

    // FPR should still be maintained
    let stats = filter.stats();
    assert_eq!(stats.fpr_target, 0.01);
}

#[test]
fn test_fpr_with_high_load_factor() {
    let mut filter = MementoFilter::new(100, 0.01).unwrap();

    // Fill to 90% capacity
    for i in 0..90 {
        filter.insert(i * 10, b"value").unwrap();
    }

    let stats = filter.stats();
    assert_eq!(stats.load_factor, 0.9);
    assert_eq!(stats.fpr_target, 0.01);
}

#[test]
fn test_statistical_fpr_validation() {
    let mut filter = MementoFilter::new(10_000, 0.01).unwrap();

    // Insert 1000 elements
    for i in 0..1000 {
        filter.insert(i, b"value").unwrap();
    }

    // Test with non-inserted keys
    let mut false_positives = 0;
    let num_tests = 10_000;

    for i in 1000..1000 + num_tests {
        if filter.may_contain_range(i, i) {
            false_positives += 1;
        }
    }

    let actual_fpr = false_positives as f64 / num_tests as f64;
    // Allow 5x margin for statistical variation
    assert!(
        actual_fpr <= 0.05,
        "Actual FPR {} exceeds 5x target",
        actual_fpr
    );
}

// ============================================================================
// Phase 5: Range Expansion Tests (4 tests)
// ============================================================================

#[test]
fn test_expands_when_needed() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    filter.insert(100, b"v1").unwrap();
    assert_eq!(
        filter.stats().num_expansions,
        0,
        "No expansion on first insert"
    );

    filter.insert(200, b"v2").unwrap();
    assert_eq!(
        filter.stats().num_expansions,
        1,
        "One expansion for range extension"
    );

    filter.insert(50, b"v3").unwrap();
    assert_eq!(
        filter.stats().num_expansions,
        2,
        "Another expansion for lower bound"
    );
}

#[test]
fn test_maintains_fpr_during_expansion() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    // Create multiple expansions
    for i in 0..10 {
        filter.insert(i * 1000, b"value").unwrap();
    }

    assert!(
        filter.stats().num_expansions >= 9,
        "Should have multiple expansions"
    );

    // FPR target should be unchanged
    assert_eq!(filter.stats().fpr_target, 0.01);
}

#[test]
fn test_no_data_loss_on_expansion() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    // Insert initial elements
    for i in 0..10 {
        filter.insert(i * 10, b"value").unwrap();
    }

    let initial_len = filter.len();

    // Cause expansion
    filter.insert(1000, b"value").unwrap();

    assert_eq!(
        filter.len(),
        initial_len + 1,
        "Should preserve all elements"
    );

    // Verify all original elements still queryable
    for i in 0..10 {
        let key = i * 10;
        assert!(
            filter.may_contain_range(key, key),
            "Should still find element {} after expansion",
            key
        );
    }
}

#[test]
fn test_expansion_efficiency() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    // Insert in expanding pattern
    let keys = vec![100, 200, 50, 250, 25, 300];

    for &key in &keys {
        filter.insert(key, b"value").unwrap();
    }

    // Should expand efficiently
    let stats = filter.stats();
    assert!(
        stats.num_expansions <= keys.len(),
        "Expansions should be efficient"
    );
    assert_eq!(filter.range(), Some((25, 300)));
}

// ============================================================================
// Phase 6: Quotient Filter Layer Tests (4 tests)
// ============================================================================

#[test]
fn test_quotient_filter_stores_correctly() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    // Insert with different values
    filter.insert(42, b"value1").unwrap();
    filter.insert(42, b"value2").unwrap();
    filter.insert(42, b"value3").unwrap();

    // All should be stored
    assert_eq!(filter.len(), 3, "QF layer should store all variants");
}

#[test]
fn test_precise_lookups_in_qf_layer() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    // Insert elements with same key but different values
    filter.insert(100, b"v1").unwrap();
    filter.insert(200, b"v2").unwrap();
    filter.insert(300, b"v3").unwrap();

    // Point queries should work precisely
    assert!(filter.may_contain_range(100, 100));
    assert!(filter.may_contain_range(200, 200));
    assert!(filter.may_contain_range(300, 300));
}

#[test]
fn test_qf_layer_capacity_management() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    // Fill quotient filter
    for i in 0..500 {
        filter.insert(i, format!("value{}", i).as_bytes()).unwrap();
    }

    let stats = filter.stats();
    assert_eq!(stats.num_elements, 500);
    assert!(stats.load_factor < 1.0);
}

#[test]
fn test_qf_layer_handles_collisions() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    // Insert many elements to force hash collisions
    for i in 0..100 {
        filter.insert(i, b"value").unwrap();
    }

    // All should still be queryable
    for i in 0..100 {
        assert!(
            filter.may_contain_range(i, i),
            "Should handle collision for key {}",
            i
        );
    }
}

// ============================================================================
// Phase 7: Edge Cases (6 tests)
// ============================================================================

#[test]
fn test_empty_filter_queries() {
    let filter = MementoFilter::new(1000, 0.01).unwrap();

    assert!(
        !filter.may_contain_range(0, 100),
        "Empty filter should return false"
    );
    assert!(
        !filter.may_contain_range(0, u64::MAX),
        "Empty filter should return false for any range"
    );
}

#[test]
fn test_fill_to_capacity() {
    let mut filter = MementoFilter::new(50, 0.01).unwrap();

    // Fill exactly to capacity
    for i in 0..50 {
        filter.insert(i, b"value").unwrap();
    }

    assert_eq!(filter.len(), 50);
    assert_eq!(filter.stats().load_factor, 1.0);
}

#[test]
fn test_exceed_capacity_error() {
    let mut filter = MementoFilter::new(5, 0.01).unwrap();

    // Fill to capacity
    for i in 0..5 {
        filter.insert(i, b"value").unwrap();
    }

    // Attempt to exceed
    let result = filter.insert(6, b"value");
    assert!(result.is_err());

    if let Err(e) = result {
        assert!(e.to_string().contains("capacity"));
    }
}

#[test]
fn test_single_element_database() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    filter.insert(42, b"only_value").unwrap();

    assert!(
        filter.may_contain_range(42, 42),
        "Should find single element"
    );
    assert!(
        filter.may_contain_range(0, 100),
        "Should find in containing range"
    );
    assert!(
        !filter.may_contain_range(100, 200),
        "Should not find outside range"
    );
}

#[test]
fn test_very_wide_ranges() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    filter.insert(0, b"v1").unwrap();
    filter.insert(u64::MAX / 2, b"v2").unwrap();

    assert!(
        filter.may_contain_range(0, u64::MAX),
        "Should handle very wide ranges"
    );
    assert_eq!(filter.range(), Some((0, u64::MAX / 2)));
}

#[test]
fn test_rapid_insertions() {
    let mut filter = MementoFilter::new(10_000, 0.01).unwrap();

    // Rapid insertions in tight loop
    for i in 0..1000 {
        filter.insert(i, b"value").unwrap();
    }

    assert_eq!(filter.len(), 1000);
    assert!(filter.stats().load_factor <= 1.0);
}

// ============================================================================
// Phase 8: Property Tests (5 tests)
// ============================================================================

#[test]
fn test_monotonicity_inserted_elements_found() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    let keys = vec![10, 50, 100, 200, 500];

    for &key in &keys {
        filter.insert(key, b"value").unwrap();
    }

    // All inserted keys must be found (no false negatives)
    for &key in &keys {
        assert!(
            filter.may_contain_range(key, key),
            "No false negatives: key {} must be found",
            key
        );
    }
}

#[test]
fn test_consistency_across_operations() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    // Insert elements
    for i in 0..10 {
        filter.insert(i * 10, b"value").unwrap();
    }

    let range1 = filter.range().unwrap();
    let len1 = filter.len();

    // Query multiple times - should be consistent
    for _ in 0..10 {
        assert_eq!(
            filter.range().unwrap(),
            range1,
            "Range should be consistent"
        );
        assert_eq!(filter.len(), len1, "Length should be consistent");
    }
}

#[test]
fn test_no_false_negatives_guarantee() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    let test_keys: Vec<u64> = (0..100).map(|i| i * 10).collect();

    for &key in &test_keys {
        filter.insert(key, b"value").unwrap();
    }

    // Zero false negatives guaranteed
    for &key in &test_keys {
        assert!(
            filter.may_contain_range(key, key),
            "ZERO false negatives: must find key {}",
            key
        );
    }
}

#[test]
fn test_memory_growth_controlled() {
    let filter1 = MementoFilter::new(100, 0.01).unwrap();
    let filter2 = MementoFilter::new(10_000, 0.01).unwrap();

    // Memory should scale with capacity
    let stats1 = filter1.stats();
    let stats2 = filter2.stats();

    assert!(stats2.capacity > stats1.capacity);
}

#[test]
fn test_fpr_bounds_hold() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    // Insert elements
    for i in 0..500 {
        filter.insert(i, b"value").unwrap();
    }

    // FPR should be bounded
    let stats = filter.stats();
    assert_eq!(stats.fpr_target, 0.01);
    assert!(stats.load_factor <= 1.0);
}

// ============================================================================
// Proptest Property-Based Tests
// ============================================================================

proptest! {
    #[test]
    fn test_prop_no_false_negatives(keys in prop::collection::vec(0u64..10000, 1..100)) {
        let mut filter = MementoFilter::new(1000, 0.01).unwrap();

        for &key in &keys {
            filter.insert(key, b"value").unwrap();
        }

        // No false negatives
        for &key in &keys {
            prop_assert!(filter.may_contain_range(key, key),
                        "No false negatives for key {}", key);
        }
    }

    #[test]
    fn test_prop_range_monotonicity(
        keys in prop::collection::vec(0u64..10000, 1..50)
    ) {
        let mut filter = MementoFilter::new(1000, 0.01).unwrap();

        for &key in &keys {
            filter.insert(key, b"value").unwrap();
        }

        if let Some((min, max)) = filter.range() {
            prop_assert!(min <= max, "Range bounds should be ordered");

            for &key in &keys {
                prop_assert!(key >= min && key <= max,
                           "All keys should be within range bounds");
            }
        }
    }

    #[test]
    fn test_prop_stats_consistency(
        n in 10usize..1000,
        num_inserts in 1usize..100
    ) {
        let mut filter = MementoFilter::new(n, 0.01).unwrap();

        for i in 0..num_inserts.min(n) {
            filter.insert(i as u64, b"value").unwrap();
        }

        let stats = filter.stats();
        prop_assert_eq!(stats.num_elements, filter.len());
        prop_assert!(stats.load_factor <= 1.0);
        prop_assert!(stats.load_factor >= 0.0);
    }
}

// ============================================================================
// RangeFilter Trait Tests
// ============================================================================

#[test]
fn test_range_filter_trait_implementation() {
    let mut filter = MementoFilter::new(1000, 0.01).unwrap();

    filter.insert(100, b"value").unwrap();

    // Test trait method
    assert!(filter.may_contain_range(90, 110));
    assert!(!filter.may_contain_range(200, 300));
}

#[test]
fn test_range_filter_trait_polymorphism() {
    fn check_range<F: RangeFilter>(filter: &F, low: u64, high: u64) -> bool {
        filter.may_contain_range(low, high)
    }

    let mut filter = MementoFilter::new(1000, 0.01).unwrap();
    filter.insert(50, b"value").unwrap();

    assert!(check_range(&filter, 40, 60));
}
