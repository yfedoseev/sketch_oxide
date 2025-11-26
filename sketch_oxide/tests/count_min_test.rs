//! Comprehensive tests for Count-Min Sketch frequency estimation
//!
//! Following TDD methodology: These tests are written BEFORE implementation.
//! They define the expected behavior and API of Count-Min Sketch.
//!
//! Count-Min Sketch (Cormode & Muthukrishnan, 2003) is the standard algorithm
//! for frequency estimation with no better alternative as of 2024.

use proptest::prelude::*;
use sketch_oxide::frequency::CountMinSketch;
use sketch_oxide::{Mergeable, SketchError};
use std::collections::HashMap;

// ============================================================================
// PHASE 1: Basic Functionality Tests
// ============================================================================

#[test]
fn test_new_count_min_sketch() {
    // Test creation with valid epsilon and delta
    let cms = CountMinSketch::new(0.01, 0.01);
    assert!(
        cms.is_ok(),
        "Should create CountMinSketch with epsilon=0.01, delta=0.01"
    );

    // Test with different valid parameters
    let cms2 = CountMinSketch::new(0.001, 0.001);
    assert!(
        cms2.is_ok(),
        "Should create CountMinSketch with epsilon=0.001, delta=0.001"
    );
}

#[test]
fn test_invalid_epsilon_zero() {
    let result = CountMinSketch::new(0.0, 0.01);
    assert!(result.is_err(), "Should reject epsilon = 0");
    match result {
        Err(SketchError::InvalidParameter { .. }) => {}
        _ => panic!("Expected InvalidParameter error"),
    }
}

#[test]
fn test_invalid_epsilon_negative() {
    let result = CountMinSketch::new(-0.01, 0.01);
    assert!(result.is_err(), "Should reject negative epsilon");
    match result {
        Err(SketchError::InvalidParameter { .. }) => {}
        _ => panic!("Expected InvalidParameter error"),
    }
}

#[test]
fn test_invalid_epsilon_too_large() {
    let result = CountMinSketch::new(1.0, 0.01);
    assert!(result.is_err(), "Should reject epsilon >= 1");
    match result {
        Err(SketchError::InvalidParameter { .. }) => {}
        _ => panic!("Expected InvalidParameter error"),
    }
}

#[test]
fn test_invalid_delta_zero() {
    let result = CountMinSketch::new(0.01, 0.0);
    assert!(result.is_err(), "Should reject delta = 0");
    match result {
        Err(SketchError::InvalidParameter { .. }) => {}
        _ => panic!("Expected InvalidParameter error"),
    }
}

#[test]
fn test_invalid_delta_negative() {
    let result = CountMinSketch::new(0.01, -0.01);
    assert!(result.is_err(), "Should reject negative delta");
    match result {
        Err(SketchError::InvalidParameter { .. }) => {}
        _ => panic!("Expected InvalidParameter error"),
    }
}

#[test]
fn test_invalid_delta_too_large() {
    let result = CountMinSketch::new(0.01, 1.0);
    assert!(result.is_err(), "Should reject delta >= 1");
    match result {
        Err(SketchError::InvalidParameter { .. }) => {}
        _ => panic!("Expected InvalidParameter error"),
    }
}

#[test]
fn test_update_single_item() {
    let mut cms = CountMinSketch::new(0.01, 0.01).unwrap();
    cms.update(&"test");

    let estimate = cms.estimate(&"test");
    assert_eq!(
        estimate, 1,
        "Single item should have exact count of 1 (never underestimates)"
    );
}

#[test]
fn test_update_multiple_same_item() {
    let mut cms = CountMinSketch::new(0.01, 0.01).unwrap();

    // Add same item 100 times
    for _ in 0..100 {
        cms.update(&"test");
    }

    let estimate = cms.estimate(&"test");
    assert!(
        estimate >= 100,
        "Should never underestimate: estimate {} should be >= 100",
        estimate
    );
}

#[test]
fn test_estimate_unseen_item() {
    let mut cms = CountMinSketch::new(0.01, 0.01).unwrap();
    cms.update(&"item1");
    cms.update(&"item2");

    let estimate = cms.estimate(&"unseen_item");
    assert_eq!(
        estimate, 0,
        "Unseen item should have estimate of 0 (or small overestimate)"
    );
}

#[test]
fn test_empty_sketch() {
    let cms = CountMinSketch::new(0.01, 0.01).unwrap();

    // Empty sketch should return 0 for any query
    assert_eq!(cms.estimate(&"any_item"), 0);
    assert_eq!(cms.estimate(&42u64), 0);
    assert_eq!(cms.estimate(&"another"), 0);
}

// ============================================================================
// PHASE 2: Accuracy Tests
// ============================================================================

#[test]
fn test_accuracy_single_heavy_hitter() {
    let mut cms = CountMinSketch::new(0.01, 0.01).unwrap();

    // Add one heavy hitter
    for _ in 0..1000 {
        cms.update(&"heavy_hitter");
    }

    let estimate = cms.estimate(&"heavy_hitter");
    assert!(
        estimate >= 1000,
        "Should never underestimate: got {}",
        estimate
    );

    // Error should be within epsilon * total
    // Since total = 1000 and epsilon = 0.01, max error = 10
    assert!(
        estimate <= 1010,
        "Estimate {} should be within error bounds (1000-1010)",
        estimate
    );
}

#[test]
fn test_accuracy_multiple_items() {
    let mut cms = CountMinSketch::new(0.01, 0.01).unwrap();

    // Add multiple items with different frequencies
    for _ in 0..100 {
        cms.update(&"item_100");
    }
    for _ in 0..50 {
        cms.update(&"item_50");
    }
    for _ in 0..10 {
        cms.update(&"item_10");
    }

    // Verify estimates
    let est_100 = cms.estimate(&"item_100");
    let est_50 = cms.estimate(&"item_50");
    let est_10 = cms.estimate(&"item_10");

    // Never underestimates
    assert!(est_100 >= 100, "Should not underestimate item_100");
    assert!(est_50 >= 50, "Should not underestimate item_50");
    assert!(est_10 >= 10, "Should not underestimate item_10");

    // Total stream size = 160
    // Max error per item = epsilon * total = 0.01 * 160 = 1.6
    assert!(
        est_100 <= 105,
        "item_100 estimate {} should be within bounds",
        est_100
    );
    assert!(
        est_50 <= 55,
        "item_50 estimate {} should be within bounds",
        est_50
    );
    assert!(
        est_10 <= 15,
        "item_10 estimate {} should be within bounds",
        est_10
    );
}

#[test]
fn test_never_underestimates_property() {
    let mut cms = CountMinSketch::new(0.001, 0.001).unwrap();

    // Track actual counts
    let mut actual_counts: HashMap<String, u64> = HashMap::new();

    // Add random items
    for i in 0..1000 {
        let item = format!("item_{}", i % 50); // 50 unique items
        cms.update(&item);
        *actual_counts.entry(item).or_insert(0) += 1;
    }

    // Verify no underestimates
    for (item, actual_count) in actual_counts.iter() {
        let estimate = cms.estimate(item);
        assert!(
            estimate >= *actual_count,
            "Item '{}' underestimated: actual={}, estimate={}",
            item,
            actual_count,
            estimate
        );
    }
}

#[test]
fn test_error_bound_guarantee() {
    // Test that error is bounded by epsilon with high probability
    let epsilon = 0.01;
    let delta = 0.01;
    let mut cms = CountMinSketch::new(epsilon, delta).unwrap();

    let total_count = 10000u64;

    // Add heavy hitter with known count
    for _ in 0..1000 {
        cms.update(&"target");
    }

    // Add noise from other items
    for i in 0..(total_count - 1000) {
        cms.update(&format!("noise_{}", i));
    }

    let estimate = cms.estimate(&"target");
    let actual = 1000u64;

    // Should never underestimate
    assert!(estimate >= actual);

    // Error should be bounded by epsilon * N with probability 1-delta
    // epsilon * 10000 = 100
    let error = estimate - actual;
    let max_error = (epsilon * total_count as f64) as u64;

    assert!(
        error <= max_error,
        "Error {} exceeds bound {} (epsilon * N)",
        error,
        max_error
    );
}

#[test]
fn test_overestimate_only() {
    let mut cms = CountMinSketch::new(0.01, 0.01).unwrap();

    // Add items
    for i in 0..100 {
        cms.update(&i);
    }

    // Query each item - all should be >= 1
    for i in 0..100 {
        let estimate = cms.estimate(&i);
        assert!(
            estimate >= 1,
            "Item {} should have estimate >= 1, got {}",
            i,
            estimate
        );
    }
}

// ============================================================================
// PHASE 3: Multiple Items Tests
// ============================================================================

#[test]
fn test_frequent_vs_rare_items() {
    let mut cms = CountMinSketch::new(0.01, 0.01).unwrap();

    // Add 1 very frequent item
    for _ in 0..1000 {
        cms.update(&"frequent");
    }

    // Add 100 rare items (1 each)
    for i in 0..100 {
        cms.update(&format!("rare_{}", i));
    }

    let freq_estimate = cms.estimate(&"frequent");
    assert!(
        freq_estimate >= 1000,
        "Frequent item should not be underestimated"
    );

    // Rare items should have small estimates
    for i in 0..100 {
        let rare_estimate = cms.estimate(&format!("rare_{}", i));
        assert!(
            rare_estimate >= 1,
            "Rare items should have estimate >= actual count (1)"
        );
        // With epsilon=0.01 and total=1100, max error is ~11
        assert!(
            rare_estimate <= 15,
            "Rare item estimate {} too high",
            rare_estimate
        );
    }
}

#[test]
fn test_many_unique_items() {
    let mut cms = CountMinSketch::new(0.001, 0.001).unwrap(); // Tighter bounds

    // Add 1000 unique items
    for i in 0..1000 {
        cms.update(&i);
    }

    // Each should have estimate >= 1
    for i in 0..1000 {
        let estimate = cms.estimate(&i);
        assert!(estimate >= 1, "Item {} underestimated", i);
    }
}

#[test]
fn test_zipfian_distribution() {
    let mut cms = CountMinSketch::new(0.01, 0.01).unwrap();

    // Simulate Zipfian distribution (realistic workload)
    // Item 0: 100 counts, Item 1: 50 counts, Item 2: 33 counts, etc.
    for i in 0..10 {
        let count = 100 / (i + 1);
        for _ in 0..count {
            cms.update(&i);
        }
    }

    // Verify high-frequency items
    assert!(cms.estimate(&0) >= 100);
    assert!(cms.estimate(&1) >= 50);
    assert!(cms.estimate(&2) >= 33);
}

// ============================================================================
// PHASE 4: Merge Tests
// ============================================================================

#[test]
fn test_merge_basic() {
    let mut cms1 = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut cms2 = CountMinSketch::new(0.01, 0.01).unwrap();

    // Add different items to each sketch
    for _ in 0..50 {
        cms1.update(&"item1");
    }
    for _ in 0..30 {
        cms2.update(&"item2");
    }

    // Merge cms2 into cms1
    let result = cms1.merge(&cms2);
    assert!(result.is_ok(), "Merge should succeed");

    // After merge, cms1 should have both items
    assert!(
        cms1.estimate(&"item1") >= 50,
        "item1 count should be preserved"
    );
    assert!(cms1.estimate(&"item2") >= 30, "item2 count should be added");
}

#[test]
fn test_merge_additive() {
    let mut cms1 = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut cms2 = CountMinSketch::new(0.01, 0.01).unwrap();

    // Add same item to both sketches
    for _ in 0..100 {
        cms1.update(&"shared");
    }
    for _ in 0..50 {
        cms2.update(&"shared");
    }

    cms1.merge(&cms2).unwrap();

    // Estimate should be >= 150 (additive)
    let estimate = cms1.estimate(&"shared");
    assert!(
        estimate >= 150,
        "Merged estimate {} should be >= 150 (additive)",
        estimate
    );
}

#[test]
fn test_merge_incompatible_epsilon() {
    let mut cms1 = CountMinSketch::new(0.01, 0.01).unwrap();
    let cms2 = CountMinSketch::new(0.001, 0.01).unwrap(); // Different epsilon

    let result = cms1.merge(&cms2);
    assert!(result.is_err(), "Should reject incompatible epsilon");
    match result {
        Err(SketchError::IncompatibleSketches { .. }) => {}
        _ => panic!("Expected IncompatibleSketches error"),
    }
}

#[test]
fn test_merge_incompatible_delta() {
    let mut cms1 = CountMinSketch::new(0.01, 0.01).unwrap();
    let cms2 = CountMinSketch::new(0.01, 0.001).unwrap(); // Different delta

    let result = cms1.merge(&cms2);
    assert!(result.is_err(), "Should reject incompatible delta");
    match result {
        Err(SketchError::IncompatibleSketches { .. }) => {}
        _ => panic!("Expected IncompatibleSketches error"),
    }
}

#[test]
fn test_merge_empty_sketches() {
    let mut cms1 = CountMinSketch::new(0.01, 0.01).unwrap();
    let cms2 = CountMinSketch::new(0.01, 0.01).unwrap();

    // Both empty
    cms1.merge(&cms2).unwrap();

    // Should still be empty
    assert_eq!(cms1.estimate(&"anything"), 0);
}

#[test]
fn test_merge_associative() {
    let mut cms1 = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut cms2 = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut cms3 = CountMinSketch::new(0.01, 0.01).unwrap();

    cms1.update(&"item");
    cms2.update(&"item");
    cms3.update(&"item");

    // Test (cms1 + cms2) + cms3
    let mut merged_a = cms1.clone();
    merged_a.merge(&cms2).unwrap();
    merged_a.merge(&cms3).unwrap();

    // Test cms1 + (cms2 + cms3)
    let mut cms2_copy = cms2.clone();
    cms2_copy.merge(&cms3).unwrap();
    let mut merged_b = cms1.clone();
    merged_b.merge(&cms2_copy).unwrap();

    // Both should give same estimate
    assert_eq!(
        merged_a.estimate(&"item"),
        merged_b.estimate(&"item"),
        "Merge should be associative"
    );
}

// ============================================================================
// PHASE 5: Property Tests
// ============================================================================

proptest! {
    #[test]
    fn prop_never_underestimates(
        items in prop::collection::vec(0u64..1000, 1..100),
        epsilon in 0.001f64..0.1,
        delta in 0.001f64..0.1
    ) {
        let mut cms = CountMinSketch::new(epsilon, delta).unwrap();
        let mut counts: HashMap<u64, u64> = HashMap::new();

        // Add items and track actual counts
        for item in items.iter() {
            cms.update(item);
            *counts.entry(*item).or_insert(0) += 1;
        }

        // Verify no underestimates
        for (item, actual_count) in counts.iter() {
            let estimate = cms.estimate(item);
            prop_assert!(
                estimate >= *actual_count,
                "Item {} underestimated: actual={}, estimate={}",
                item, actual_count, estimate
            );
        }
    }

    #[test]
    fn prop_error_bounded(
        heavy_count in 100u64..1000,
        noise_count in 100u64..1000,
    ) {
        let epsilon = 0.01;
        let delta = 0.01;
        let mut cms = CountMinSketch::new(epsilon, delta).unwrap();

        // Add heavy hitter
        for _ in 0..heavy_count {
            cms.update(&0u64);
        }

        // Add noise
        for i in 1..=noise_count {
            cms.update(&i);
        }

        let estimate = cms.estimate(&0u64);
        let total = heavy_count + noise_count;

        // Never underestimates
        prop_assert!(estimate >= heavy_count);

        // Error bounded by epsilon * N (with high probability)
        let max_error = (epsilon * total as f64).ceil() as u64;
        let error = estimate - heavy_count;

        // Note: This can fail with probability delta (0.01)
        // For testing, we use a slightly relaxed bound
        prop_assert!(
            error <= max_error * 2,
            "Error {} exceeds 2x bound {} (flaky test protection)",
            error, max_error
        );
    }

    #[test]
    fn prop_merge_increases_estimates(
        count1 in 1u64..100,
        count2 in 1u64..100,
    ) {
        let mut cms1 = CountMinSketch::new(0.01, 0.01).unwrap();
        let mut cms2 = CountMinSketch::new(0.01, 0.01).unwrap();

        // Add to both sketches
        for _ in 0..count1 {
            cms1.update(&"item");
        }
        for _ in 0..count2 {
            cms2.update(&"item");
        }

        let est1_before = cms1.estimate(&"item");

        cms1.merge(&cms2).unwrap();

        let est1_after = cms1.estimate(&"item");

        // Estimate should increase (or stay same if cms2 was empty)
        prop_assert!(est1_after >= est1_before);
        prop_assert!(est1_after >= count1 + count2);
    }

    #[test]
    fn prop_different_items_independent(
        items in prop::collection::hash_set(0u64..10000, 2..20),
    ) {
        let mut cms = CountMinSketch::new(0.001, 0.001).unwrap();
        let items_vec: Vec<u64> = items.into_iter().collect();

        // Add each item once
        for item in items_vec.iter() {
            cms.update(item);
        }

        // Each item should have estimate >= 1
        for item in items_vec.iter() {
            let estimate = cms.estimate(item);
            prop_assert!(estimate >= 1, "Item {} has estimate {}", item, estimate);
        }
    }
}

// ============================================================================
// PHASE 6: Edge Cases and Stress Tests
// ============================================================================

#[test]
fn test_very_tight_epsilon() {
    // Very small epsilon means very wide table
    let cms = CountMinSketch::new(0.0001, 0.01);
    assert!(cms.is_ok(), "Should handle very small epsilon");
}

#[test]
fn test_very_tight_delta() {
    // Very small delta means very deep table
    let cms = CountMinSketch::new(0.01, 0.0001);
    assert!(cms.is_ok(), "Should handle very small delta");
}

#[test]
fn test_large_counts() {
    let mut cms = CountMinSketch::new(0.01, 0.01).unwrap();

    // Add item many times
    for _ in 0..1_000_000 {
        cms.update(&"heavy");
    }

    let estimate = cms.estimate(&"heavy");
    assert!(estimate >= 1_000_000, "Should handle large counts");
}

#[test]
fn test_different_types() {
    let mut cms = CountMinSketch::new(0.01, 0.01).unwrap();

    // Count-Min Sketch should work with any hashable type
    cms.update(&42u64);
    cms.update(&"string");
    cms.update(&vec![1, 2, 3]);

    assert!(cms.estimate(&42u64) >= 1);
    assert!(cms.estimate(&"string") >= 1);
    assert!(cms.estimate(&vec![1, 2, 3]) >= 1);
}

#[test]
fn test_collision_resistance() {
    let mut cms = CountMinSketch::new(0.001, 0.001).unwrap(); // Tight bounds

    // Add many unique items
    for i in 0..10000 {
        cms.update(&i);
    }

    // Most items should have low estimates (close to 1)
    let mut high_error_count = 0;
    for i in 0..1000 {
        let estimate = cms.estimate(&i);
        if estimate > 10 {
            // More than 10x overestimate
            high_error_count += 1;
        }
    }

    // With good hash functions, very few items should have high error
    assert!(
        high_error_count < 50,
        "Too many high-error estimates: {}",
        high_error_count
    );
}
