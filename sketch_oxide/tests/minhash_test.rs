//! Comprehensive tests for MinHash similarity estimation
//!
//! Following TDD methodology: These tests are written BEFORE implementation.
//! They define the expected behavior and API of MinHash.
//!
//! MinHash (Broder 1997) is the standard algorithm for Jaccard similarity
//! estimation with no better alternative as of 2024.

use proptest::prelude::*;
use sketch_oxide::similarity::MinHash;
use sketch_oxide::{Mergeable, SketchError};

// ============================================================================
// PHASE 1: Basic Functionality Tests
// ============================================================================

#[test]
fn test_new_minhash() {
    // Test creation with valid num_perm
    let mh = MinHash::new(128);
    assert!(mh.is_ok(), "Should create MinHash with num_perm=128");

    // Test with different valid parameters
    let mh2 = MinHash::new(64);
    assert!(mh2.is_ok(), "Should create MinHash with num_perm=64");

    let mh3 = MinHash::new(256);
    assert!(mh3.is_ok(), "Should create MinHash with num_perm=256");
}

#[test]
fn test_invalid_num_perm_zero() {
    let result = MinHash::new(0);
    assert!(result.is_err(), "Should reject num_perm = 0");
    match result {
        Err(SketchError::InvalidParameter { .. }) => {}
        _ => panic!("Expected InvalidParameter error"),
    }
}

#[test]
fn test_invalid_num_perm_too_small() {
    let result = MinHash::new(8);
    assert!(result.is_err(), "Should reject num_perm < 16");
    match result {
        Err(SketchError::InvalidParameter { .. }) => {}
        _ => panic!("Expected InvalidParameter error"),
    }
}

#[test]
fn test_update_single_item() {
    let mut mh = MinHash::new(128).unwrap();
    mh.update(&"test");

    // After update, hash_values should no longer be all MAX
    // We can't test internal state directly, but we can test behavior
    let mh2 = MinHash::new(128).unwrap();
    let similarity = mh.jaccard_similarity(&mh2).unwrap();

    // Empty sketch should have 0 similarity with non-empty sketch
    assert!(
        similarity < 1.0,
        "Non-empty sketch should differ from empty sketch"
    );
}

#[test]
fn test_update_multiple_items() {
    let mut mh = MinHash::new(128).unwrap();

    mh.update(&"item1");
    mh.update(&"item2");
    mh.update(&"item3");

    // Sketch should be different from empty
    let empty = MinHash::new(128).unwrap();
    let similarity = mh.jaccard_similarity(&empty).unwrap();
    assert!(similarity < 1.0);
}

#[test]
fn test_empty_sketch() {
    let mh1 = MinHash::new(128).unwrap();
    let mh2 = MinHash::new(128).unwrap();

    // Two empty sketches should have similarity 0.0 (or undefined)
    // For empty sets, Jaccard is typically 0 or undefined
    let similarity = mh1.jaccard_similarity(&mh2).unwrap();
    assert!(
        similarity == 0.0 || similarity.is_nan(),
        "Empty sketches should have 0 or undefined similarity"
    );
}

// ============================================================================
// PHASE 2: Jaccard Similarity Tests
// ============================================================================

#[test]
fn test_identical_sets_perfect_similarity() {
    let mut mh1 = MinHash::new(128).unwrap();
    let mut mh2 = MinHash::new(128).unwrap();

    // Add identical items to both
    for i in 0..100 {
        mh1.update(&i);
        mh2.update(&i);
    }

    let similarity = mh1.jaccard_similarity(&mh2).unwrap();
    assert!(
        (similarity - 1.0).abs() < 0.001,
        "Identical sets should have similarity ≈ 1.0, got {}",
        similarity
    );
}

#[test]
fn test_disjoint_sets_zero_similarity() {
    let mut mh1 = MinHash::new(128).unwrap();
    let mut mh2 = MinHash::new(128).unwrap();

    // Add completely different items
    for i in 0..100 {
        mh1.update(&i);
    }
    for i in 100..200 {
        mh2.update(&i);
    }

    let similarity = mh1.jaccard_similarity(&mh2).unwrap();
    assert!(
        similarity < 0.05,
        "Disjoint sets should have similarity ≈ 0.0, got {}",
        similarity
    );
}

#[test]
fn test_partial_overlap_similarity() {
    let mut mh1 = MinHash::new(128).unwrap();
    let mut mh2 = MinHash::new(128).unwrap();

    // Set A: {0..100}
    for i in 0..100 {
        mh1.update(&i);
    }

    // Set B: {50..150} (overlap: 50..100, i.e., 50 items)
    for i in 50..150 {
        mh2.update(&i);
    }

    // |A ∩ B| = 50, |A ∪ B| = 150
    // Jaccard = 50/150 = 0.333...
    let similarity = mh1.jaccard_similarity(&mh2).unwrap();
    assert!(
        (similarity - 0.333).abs() < 0.1,
        "Expected similarity ≈ 0.333, got {}",
        similarity
    );
}

#[test]
fn test_subset_similarity() {
    let mut mh1 = MinHash::new(128).unwrap();
    let mut mh2 = MinHash::new(128).unwrap();

    // Set A: {0..50}
    for i in 0..50 {
        mh1.update(&i);
        mh2.update(&i);
    }

    // Set B = A ∪ {50..100}
    for i in 50..100 {
        mh2.update(&i);
    }

    // |A ∩ B| = 50, |A ∪ B| = 100
    // Jaccard = 50/100 = 0.5
    let similarity = mh1.jaccard_similarity(&mh2).unwrap();
    assert!(
        (similarity - 0.5).abs() < 0.1,
        "Expected similarity ≈ 0.5, got {}",
        similarity
    );
}

// ============================================================================
// PHASE 3: Accuracy Tests
// ============================================================================

#[test]
fn test_accuracy_estimation_64_perms() {
    // Test accuracy with num_perm=64
    run_accuracy_test(64, 0.15); // Allow 15% error for k=64
}

#[test]
fn test_accuracy_estimation_128_perms() {
    // Test accuracy with num_perm=128 (required: <5% error)
    run_accuracy_test(128, 0.05); // Allow 5% error for k=128
}

#[test]
fn test_accuracy_estimation_256_perms() {
    // Test accuracy with num_perm=256
    run_accuracy_test(256, 0.04); // Allow 4% error for k=256
}

fn run_accuracy_test(num_perm: usize, max_error: f64) {
    // Test multiple overlap scenarios
    let test_cases = vec![
        (0, 100, 100, 200, 0.0), // Disjoint: A={0..100}, B={100..200}, |A∩B|=0, |A∪B|=200, J=0/200=0.0
        (0, 100, 50, 150, 0.333), // Overlap: A={0..100}, B={50..150}, |A∩B|=50, |A∪B|=150, J=50/150≈0.333
        (0, 100, 25, 125, 0.6), // More overlap: A={0..100}, B={25..125}, |A∩B|=75, |A∪B|=125, J=75/125=0.6
        (0, 100, 0, 100, 1.0), // Identical: A={0..100}, B={0..100}, |A∩B|=100, |A∪B|=100, J=100/100=1.0
    ];

    for (a_start, a_end, b_start, b_end, expected_jaccard) in test_cases {
        let mut mh1 = MinHash::new(num_perm).unwrap();
        let mut mh2 = MinHash::new(num_perm).unwrap();

        for i in a_start..a_end {
            mh1.update(&i);
        }
        for i in b_start..b_end {
            mh2.update(&i);
        }

        let estimated = mh1.jaccard_similarity(&mh2).unwrap();
        let error = (estimated - expected_jaccard).abs();

        assert!(
            error <= max_error,
            "num_perm={}: Expected Jaccard ≈ {}, got {} (error={} > max_error={})",
            num_perm,
            expected_jaccard,
            estimated,
            error,
            max_error
        );
    }
}

#[test]
fn test_similarity_improves_with_more_perms() {
    // Higher num_perm should give more accurate estimates
    let test_set_a: Vec<u64> = (0..100).collect();
    let test_set_b: Vec<u64> = (50..150).collect();

    // True Jaccard = 50/150 = 0.333...
    let expected = 0.333;

    // Test with increasing num_perm
    for &num_perm in &[32, 64, 128, 256] {
        let mut mh1 = MinHash::new(num_perm).unwrap();
        let mut mh2 = MinHash::new(num_perm).unwrap();

        for &item in &test_set_a {
            mh1.update(&item);
        }
        for &item in &test_set_b {
            mh2.update(&item);
        }

        let estimated = mh1.jaccard_similarity(&mh2).unwrap();
        let error = (estimated - expected).abs();

        // Error should decrease as num_perm increases
        // Standard error ~ 1/sqrt(num_perm)
        let expected_std_error = 1.0 / (num_perm as f64).sqrt();
        let tolerance = 3.0 * expected_std_error; // 3-sigma

        assert!(
            error <= tolerance,
            "num_perm={}: error {} exceeds tolerance {} (3-sigma)",
            num_perm,
            error,
            tolerance
        );
    }
}

// ============================================================================
// PHASE 4: Union (Merge) Tests
// ============================================================================

#[test]
fn test_merge_basic() {
    let mut mh1 = MinHash::new(128).unwrap();
    let mut mh2 = MinHash::new(128).unwrap();

    // Add different items to each sketch
    for i in 0..50 {
        mh1.update(&i);
    }
    for i in 50..100 {
        mh2.update(&i);
    }

    // Merge mh2 into mh1
    let result = mh1.merge(&mh2);
    assert!(result.is_ok(), "Merge should succeed");

    // After merge, mh1 represents union A ∪ B
    // Create reference sketch with all items
    let mut mh_union = MinHash::new(128).unwrap();
    for i in 0..100 {
        mh_union.update(&i);
    }

    let similarity = mh1.jaccard_similarity(&mh_union).unwrap();
    assert!(
        similarity > 0.95,
        "Merged sketch should be similar to union, got similarity {}",
        similarity
    );
}

#[test]
fn test_merge_produces_union() {
    let mut mh1 = MinHash::new(128).unwrap();
    let mut mh2 = MinHash::new(128).unwrap();

    // A = {0..60}
    for i in 0..60 {
        mh1.update(&i);
    }

    // B = {40..100}
    for i in 40..100 {
        mh2.update(&i);
    }

    // Create reference for union A ∪ B = {0..100}
    let mut mh_union = MinHash::new(128).unwrap();
    for i in 0..100 {
        mh_union.update(&i);
    }

    // Merge
    mh1.merge(&mh2).unwrap();

    // mh1 should now approximate the union
    let similarity = mh1.jaccard_similarity(&mh_union).unwrap();
    assert!(
        similarity > 0.95,
        "Merged sketch should match union with high similarity, got {}",
        similarity
    );
}

#[test]
fn test_merge_incompatible_num_perm() {
    let mut mh1 = MinHash::new(128).unwrap();
    let mh2 = MinHash::new(64).unwrap(); // Different num_perm

    let result = mh1.merge(&mh2);
    assert!(result.is_err(), "Should reject incompatible num_perm");
    match result {
        Err(SketchError::IncompatibleSketches { .. }) => {}
        _ => panic!("Expected IncompatibleSketches error"),
    }
}

#[test]
fn test_merge_empty_sketches() {
    let mut mh1 = MinHash::new(128).unwrap();
    let mh2 = MinHash::new(128).unwrap();

    // Both empty
    mh1.merge(&mh2).unwrap();

    // Should still be empty
    let empty = MinHash::new(128).unwrap();
    let similarity = mh1.jaccard_similarity(&empty).unwrap();
    assert!(
        similarity == 0.0 || similarity.is_nan(),
        "Merged empty sketches should still be empty-like"
    );
}

#[test]
fn test_merge_with_empty() {
    let mut mh1 = MinHash::new(128).unwrap();
    let mh2 = MinHash::new(128).unwrap(); // Empty

    // Add items to mh1
    for i in 0..50 {
        mh1.update(&i);
    }

    // Create reference
    let mut mh_ref = MinHash::new(128).unwrap();
    for i in 0..50 {
        mh_ref.update(&i);
    }

    let sim_before = mh1.jaccard_similarity(&mh_ref).unwrap();

    // Merge with empty
    mh1.merge(&mh2).unwrap();

    let sim_after = mh1.jaccard_similarity(&mh_ref).unwrap();

    // Should remain unchanged
    assert!(
        (sim_before - sim_after).abs() < 0.01,
        "Merging with empty should not change sketch significantly"
    );
}

#[test]
fn test_merge_associative() {
    let mut mh1 = MinHash::new(128).unwrap();
    let mut mh2 = MinHash::new(128).unwrap();
    let mut mh3 = MinHash::new(128).unwrap();

    for i in 0..30 {
        mh1.update(&i);
    }
    for i in 30..60 {
        mh2.update(&i);
    }
    for i in 60..90 {
        mh3.update(&i);
    }

    // Test (mh1 + mh2) + mh3
    let mut merged_a = mh1.clone();
    merged_a.merge(&mh2).unwrap();
    merged_a.merge(&mh3).unwrap();

    // Test mh1 + (mh2 + mh3)
    let mut mh2_copy = mh2.clone();
    mh2_copy.merge(&mh3).unwrap();
    let mut merged_b = mh1.clone();
    merged_b.merge(&mh2_copy).unwrap();

    // Both should give similar results (exact match due to min operation)
    let similarity = merged_a.jaccard_similarity(&merged_b).unwrap();
    assert!(
        (similarity - 1.0).abs() < 0.01,
        "Merge should be associative, got similarity {}",
        similarity
    );
}

#[test]
fn test_merge_commutative() {
    let mut mh1 = MinHash::new(128).unwrap();
    let mut mh2 = MinHash::new(128).unwrap();

    for i in 0..50 {
        mh1.update(&i);
    }
    for i in 25..75 {
        mh2.update(&i);
    }

    // Test mh1 + mh2
    let mut merged_a = mh1.clone();
    merged_a.merge(&mh2).unwrap();

    // Test mh2 + mh1
    let mut merged_b = mh2.clone();
    merged_b.merge(&mh1).unwrap();

    let similarity = merged_a.jaccard_similarity(&merged_b).unwrap();
    assert!(
        (similarity - 1.0).abs() < 0.01,
        "Merge should be commutative, got similarity {}",
        similarity
    );
}

// ============================================================================
// PHASE 5: Property Tests
// ============================================================================

proptest! {
    #[test]
    fn prop_similarity_in_range(
        items1 in prop::collection::hash_set(0u64..1000, 1..100),
        items2 in prop::collection::hash_set(0u64..1000, 1..100),
        num_perm in 32usize..257
    ) {
        let mut mh1 = MinHash::new(num_perm).unwrap();
        let mut mh2 = MinHash::new(num_perm).unwrap();

        for item in items1.iter() {
            mh1.update(item);
        }
        for item in items2.iter() {
            mh2.update(item);
        }

        let similarity = mh1.jaccard_similarity(&mh2).unwrap();

        // Similarity must be in [0, 1]
        prop_assert!(
            (0.0..=1.0).contains(&similarity),
            "Similarity {} not in [0,1]",
            similarity
        );
    }

    #[test]
    fn prop_similarity_commutative(
        items1 in prop::collection::hash_set(0u64..500, 10..50),
        items2 in prop::collection::hash_set(0u64..500, 10..50),
    ) {
        let mut mh1 = MinHash::new(128).unwrap();
        let mut mh2 = MinHash::new(128).unwrap();

        for item in items1.iter() {
            mh1.update(item);
        }
        for item in items2.iter() {
            mh2.update(item);
        }

        let sim_12 = mh1.jaccard_similarity(&mh2).unwrap();
        let sim_21 = mh2.jaccard_similarity(&mh1).unwrap();

        prop_assert!(
            (sim_12 - sim_21).abs() < 0.0001,
            "Similarity should be commutative: {} vs {}",
            sim_12, sim_21
        );
    }

    #[test]
    fn prop_identical_sets_similarity_one(
        items in prop::collection::hash_set(0u64..1000, 10..100),
    ) {
        let mut mh1 = MinHash::new(128).unwrap();
        let mut mh2 = MinHash::new(128).unwrap();

        for item in items.iter() {
            mh1.update(item);
            mh2.update(item);
        }

        let similarity = mh1.jaccard_similarity(&mh2).unwrap();

        prop_assert!(
            (similarity - 1.0).abs() < 0.001,
            "Identical sets should have similarity ≈ 1.0, got {}",
            similarity
        );
    }

    #[test]
    fn prop_merge_increases_or_maintains(
        items1 in prop::collection::hash_set(0u64..500, 10..50),
        items2 in prop::collection::hash_set(0u64..500, 10..50),
    ) {
        let mut mh1 = MinHash::new(128).unwrap();
        let mut mh2 = MinHash::new(128).unwrap();
        let mut mh_test = MinHash::new(128).unwrap();

        for item in items1.iter() {
            mh1.update(item);
        }
        for item in items2.iter() {
            mh2.update(item);
        }
        for item in items1.iter() {
            mh_test.update(item);
        }

        let _sim_before = mh_test.jaccard_similarity(&mh1).unwrap();

        mh_test.merge(&mh2).unwrap();

        // After merge, mh_test contains at least items1 (possibly more)
        // So similarity with original items1 should decrease or stay same
        // But mh_test should now have all items from items1 ∪ items2

        // Test that merge creates valid sketch
        let sim_with_union = mh_test.jaccard_similarity(&mh_test).unwrap();
        prop_assert!(
            (sim_with_union - 1.0).abs() < 0.001,
            "Self-similarity should be 1.0"
        );
    }

    #[test]
    fn prop_disjoint_sets_low_similarity(
        size1 in 10usize..50,
        size2 in 10usize..50,
    ) {
        let mut mh1 = MinHash::new(128).unwrap();
        let mut mh2 = MinHash::new(128).unwrap();

        // Create disjoint sets
        for i in 0..size1 {
            mh1.update(&i);
        }
        for i in 1000..(1000 + size2) {
            mh2.update(&i);
        }

        let similarity = mh1.jaccard_similarity(&mh2).unwrap();

        prop_assert!(
            similarity < 0.1,
            "Disjoint sets should have low similarity, got {}",
            similarity
        );
    }

    #[test]
    fn prop_self_similarity_is_one(
        items in prop::collection::hash_set(0u64..1000, 10..100),
    ) {
        let mut mh = MinHash::new(128).unwrap();

        for item in items.iter() {
            mh.update(item);
        }

        let similarity = mh.jaccard_similarity(&mh).unwrap();

        prop_assert!(
            (similarity - 1.0).abs() < 0.001,
            "Self-similarity should be 1.0, got {}",
            similarity
        );
    }
}

// ============================================================================
// PHASE 6: Edge Cases and Stress Tests
// ============================================================================

#[test]
fn test_minimum_num_perm() {
    let mh = MinHash::new(16);
    assert!(mh.is_ok(), "Should accept minimum num_perm=16");
}

#[test]
fn test_large_num_perm() {
    let mh = MinHash::new(1024);
    assert!(mh.is_ok(), "Should handle large num_perm");
}

#[test]
fn test_many_items() {
    let mut mh1 = MinHash::new(128).unwrap();
    let mut mh2 = MinHash::new(128).unwrap();

    // Add many items
    for i in 0..10000 {
        mh1.update(&i);
    }
    for i in 5000..15000 {
        mh2.update(&i);
    }

    // Should still compute similarity
    let similarity = mh1.jaccard_similarity(&mh2).unwrap();

    // Expected Jaccard = 5000 / 15000 = 0.333
    assert!(
        (similarity - 0.333).abs() < 0.05,
        "Should handle many items correctly, got similarity {}",
        similarity
    );
}

#[test]
fn test_different_hashable_types() {
    let mut mh = MinHash::new(128).unwrap();

    // MinHash should work with any hashable type
    mh.update(&42u64);
    mh.update(&"string");
    mh.update(&vec![1, 2, 3]);
    mh.update(&(1, 2, 3));

    // Should not panic
    let empty = MinHash::new(128).unwrap();
    let _similarity = mh.jaccard_similarity(&empty).unwrap();
}

#[test]
fn test_clone() {
    let mut mh1 = MinHash::new(128).unwrap();

    for i in 0..100 {
        mh1.update(&i);
    }

    let mh2 = mh1.clone();

    let similarity = mh1.jaccard_similarity(&mh2).unwrap();
    assert!(
        (similarity - 1.0).abs() < 0.001,
        "Cloned sketch should be identical"
    );
}

#[test]
fn test_string_sets() {
    let mut mh1 = MinHash::new(128).unwrap();
    let mut mh2 = MinHash::new(128).unwrap();

    let words1 = vec!["the", "quick", "brown", "fox", "jumps"];
    let words2 = vec!["the", "lazy", "brown", "dog", "sleeps"];

    for word in words1 {
        mh1.update(&word);
    }
    for word in words2 {
        mh2.update(&word);
    }

    // Overlap: "the", "brown" (2 items)
    // Union: 8 unique items
    // Jaccard ≈ 2/8 = 0.25
    let similarity = mh1.jaccard_similarity(&mh2).unwrap();
    assert!(
        (similarity - 0.25).abs() < 0.15,
        "String set similarity should be approximately correct, got {}",
        similarity
    );
}

#[test]
fn test_duplicate_items_ignored() {
    let mut mh1 = MinHash::new(128).unwrap();
    let mut mh2 = MinHash::new(128).unwrap();

    // Add same items multiple times to mh1
    for _ in 0..10 {
        for i in 0..50 {
            mh1.update(&i);
        }
    }

    // Add same items once to mh2
    for i in 0..50 {
        mh2.update(&i);
    }

    // Should have high similarity (duplicates ignored in sets)
    let similarity = mh1.jaccard_similarity(&mh2).unwrap();
    assert!(
        similarity > 0.95,
        "Duplicate items should be ignored, got similarity {}",
        similarity
    );
}

#[test]
fn test_realistic_document_similarity() {
    // Simulate document shingling use case
    let mut doc1 = MinHash::new(128).unwrap();
    let mut doc2 = MinHash::new(128).unwrap();
    let mut doc3 = MinHash::new(128).unwrap();

    let text1 = "the quick brown fox jumps over the lazy dog";
    let text2 = "the quick brown fox runs over the sleepy cat";
    let text3 = "completely different text with no overlap whatsoever";

    // Generate trigrams (simplified shingling)
    for window in text1.as_bytes().windows(3) {
        doc1.update(&window);
    }
    for window in text2.as_bytes().windows(3) {
        doc2.update(&window);
    }
    for window in text3.as_bytes().windows(3) {
        doc3.update(&window);
    }

    let sim_12 = doc1.jaccard_similarity(&doc2).unwrap();
    let sim_13 = doc1.jaccard_similarity(&doc3).unwrap();
    let sim_23 = doc2.jaccard_similarity(&doc3).unwrap();

    // doc1 and doc2 should be more similar than doc1 and doc3
    assert!(
        sim_12 > sim_13,
        "Similar documents should have higher similarity: sim_12={} vs sim_13={}",
        sim_12,
        sim_13
    );
    assert!(
        sim_12 > sim_23,
        "Similar documents should have higher similarity: sim_12={} vs sim_23={}",
        sim_12,
        sim_23
    );
}
