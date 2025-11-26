//! Comprehensive tests for UltraLogLog cardinality estimation
//!
//! Following TDD methodology: These tests are written BEFORE implementation.
//! They define the expected behavior and API of UltraLogLog.

use proptest::prelude::*;
use sketch_oxide::cardinality::UltraLogLog;
use sketch_oxide::{Mergeable, Sketch, SketchError};

// ============================================================================
// PHASE 1: Basic Functionality Tests
// ============================================================================

#[test]
fn test_new_ultraloglog() {
    // Test creation with valid precision values
    for precision in 4..=18 {
        let result = UltraLogLog::new(precision);
        assert!(
            result.is_ok(),
            "Should create UltraLogLog with precision {}",
            precision
        );
    }
}

#[test]
fn test_invalid_precision_too_low() {
    // Precision below 4 should fail
    let result = UltraLogLog::new(3);
    assert!(result.is_err(), "Should reject precision < 4");
    match result {
        Err(SketchError::InvalidParameter { .. }) => {}
        _ => panic!("Expected InvalidParameter error"),
    }
}

#[test]
fn test_invalid_precision_too_high() {
    // Precision above 18 should fail
    let result = UltraLogLog::new(19);
    assert!(result.is_err(), "Should reject precision > 18");
    match result {
        Err(SketchError::InvalidParameter { .. }) => {}
        _ => panic!("Expected InvalidParameter error"),
    }
}

#[test]
fn test_update_single_item() {
    let mut ull = UltraLogLog::new(12).unwrap();
    ull.update(&42u64);

    let estimate = ull.estimate();
    // Estimate should be close to 1 (within reasonable error bounds)
    assert!(
        (0.5..=2.0).contains(&estimate),
        "Single item estimate should be close to 1, got {}",
        estimate
    );
}

#[test]
fn test_update_multiple_items() {
    let mut ull = UltraLogLog::new(12).unwrap();

    // Add 1000 distinct items
    for i in 0..1000 {
        ull.update(&i);
    }

    let estimate = ull.estimate();
    // With precision 12, error should be < 5% for 1000 items
    let expected = 1000.0;
    let error = (estimate - expected).abs() / expected;

    assert!(
        error < 0.05,
        "Estimate {} should be within 5% of 1000, error: {:.2}%",
        estimate,
        error * 100.0
    );
}

#[test]
fn test_empty_sketch() {
    let ull = UltraLogLog::new(12).unwrap();

    assert!(ull.is_empty(), "New sketch should be empty");

    let estimate = ull.estimate();
    assert!(
        estimate < 1.0,
        "Empty sketch should estimate close to 0, got {}",
        estimate
    );
}

#[test]
fn test_not_empty_after_update() {
    let mut ull = UltraLogLog::new(12).unwrap();
    ull.update(&42u64);

    assert!(!ull.is_empty(), "Sketch should not be empty after update");
}

// ============================================================================
// PHASE 2: Cardinality Accuracy Tests
// ============================================================================

#[test]
fn test_small_cardinality() {
    let mut ull = UltraLogLog::new(12).unwrap();

    // Add 100 distinct items
    for i in 0..100 {
        ull.update(&i);
    }

    let estimate = ull.estimate();
    let expected = 100.0;
    let error = (estimate - expected).abs() / expected;

    assert!(
        error < 0.05,
        "Small cardinality: estimate {} should be within 5% of 100, error: {:.2}%",
        estimate,
        error * 100.0
    );
}

#[test]
fn test_medium_cardinality() {
    let mut ull = UltraLogLog::new(12).unwrap();

    // Add 10,000 distinct items
    for i in 0..10_000 {
        ull.update(&i);
    }

    let estimate = ull.estimate();
    let expected = 10_000.0;
    let error = (estimate - expected).abs() / expected;

    assert!(
        error < 0.04,
        "Medium cardinality: estimate {} should be within 4% of 10,000, error: {:.2}%",
        estimate,
        error * 100.0
    );
}

#[test]
fn test_large_cardinality() {
    let mut ull = UltraLogLog::new(12).unwrap();

    // Add 100,000 distinct items
    for i in 0..100_000 {
        ull.update(&i);
    }

    let estimate = ull.estimate();
    let expected = 100_000.0;
    let error = (estimate - expected).abs() / expected;

    assert!(
        error < 0.02,
        "Large cardinality: estimate {} should be within 2% of 100,000, error: {:.2}%",
        estimate,
        error * 100.0
    );
}

#[test]
fn test_duplicate_items() {
    let mut ull = UltraLogLog::new(12).unwrap();

    // Add 100 unique items, but repeat each 10 times
    for i in 0..100 {
        for _ in 0..10 {
            ull.update(&i);
        }
    }

    let estimate = ull.estimate();
    let expected = 100.0;
    let error = (estimate - expected).abs() / expected;

    assert!(
        error < 0.10,
        "Duplicates test: estimate {} should be within 10% of 100, error: {:.2}%",
        estimate,
        error * 100.0
    );
}

#[test]
fn test_precision_affects_accuracy() {
    // Higher precision should give better accuracy
    let cardinality = 10_000;

    let mut ull_low = UltraLogLog::new(8).unwrap(); // Lower precision
    let mut ull_high = UltraLogLog::new(14).unwrap(); // Higher precision

    for i in 0..cardinality {
        ull_low.update(&i);
        ull_high.update(&i);
    }

    let estimate_low = ull_low.estimate();
    let estimate_high = ull_high.estimate();

    let error_low = (estimate_low - cardinality as f64).abs() / cardinality as f64;
    let error_high = (estimate_high - cardinality as f64).abs() / cardinality as f64;

    // High precision should generally have lower or similar error
    // (We allow some variance due to randomness)
    assert!(
        error_high <= error_low * 1.5,
        "Higher precision (p=14, error={:.2}%) should be better or comparable to lower precision (p=8, error={:.2}%)",
        error_high * 100.0,
        error_low * 100.0
    );
}

// ============================================================================
// PHASE 3: Merge Tests
// ============================================================================

#[test]
fn test_merge_empty_sketches() {
    let mut ull1 = UltraLogLog::new(12).unwrap();
    let ull2 = UltraLogLog::new(12).unwrap();

    ull1.merge(&ull2).unwrap();

    assert!(
        ull1.is_empty(),
        "Merging two empty sketches should result in empty sketch"
    );
    assert!(
        ull1.estimate() < 1.0,
        "Merged empty sketch should estimate close to 0"
    );
}

#[test]
fn test_merge_disjoint_sets() {
    let mut ull1 = UltraLogLog::new(12).unwrap();
    let mut ull2 = UltraLogLog::new(12).unwrap();

    // ull1: items 0..1000
    for i in 0..1000 {
        ull1.update(&i);
    }

    // ull2: items 1000..2000 (disjoint)
    for i in 1000..2000 {
        ull2.update(&i);
    }

    ull1.merge(&ull2).unwrap();

    let estimate = ull1.estimate();
    let expected = 2000.0;
    let error = (estimate - expected).abs() / expected;

    assert!(
        error < 0.05,
        "Merge disjoint: estimate {} should be within 5% of 2000, error: {:.2}%",
        estimate,
        error * 100.0
    );
}

#[test]
fn test_merge_overlapping_sets() {
    let mut ull1 = UltraLogLog::new(12).unwrap();
    let mut ull2 = UltraLogLog::new(12).unwrap();

    // ull1: items 0..1000
    for i in 0..1000 {
        ull1.update(&i);
    }

    // ull2: items 500..1500 (overlapping with ull1)
    for i in 500..1500 {
        ull2.update(&i);
    }

    ull1.merge(&ull2).unwrap();

    let estimate = ull1.estimate();
    let expected = 1500.0; // Union is 0..1500
    let error = (estimate - expected).abs() / expected;

    assert!(
        error < 0.05,
        "Merge overlapping: estimate {} should be within 5% of 1500, error: {:.2}%",
        estimate,
        error * 100.0
    );
}

#[test]
fn test_merge_commutative() {
    let mut ull1a = UltraLogLog::new(12).unwrap();
    let mut ull1b = UltraLogLog::new(12).unwrap();
    let mut ull2a = UltraLogLog::new(12).unwrap();
    let mut ull2b = UltraLogLog::new(12).unwrap();

    // Fill both pairs identically
    for i in 0..1000 {
        ull1a.update(&i);
        ull1b.update(&i);
    }

    for i in 500..1500 {
        ull2a.update(&i);
        ull2b.update(&i);
    }

    // Merge in different orders
    ull1a.merge(&ull2a).unwrap(); // A.merge(B)
    ull2b.merge(&ull1b).unwrap(); // B.merge(A)

    let estimate_ab = ull1a.estimate();
    let estimate_ba = ull2b.estimate();

    // Estimates should be very close (allowing for floating point variance)
    let diff = (estimate_ab - estimate_ba).abs();
    assert!(
        diff < 1.0,
        "Merge should be commutative: A.merge(B)={} vs B.merge(A)={}",
        estimate_ab,
        estimate_ba
    );
}

#[test]
fn test_merge_incompatible_precision() {
    let mut ull1 = UltraLogLog::new(12).unwrap();
    let ull2 = UltraLogLog::new(14).unwrap();

    let result = ull1.merge(&ull2);

    assert!(
        result.is_err(),
        "Should reject merge of sketches with different precision"
    );
    match result {
        Err(SketchError::IncompatibleSketches { .. }) => {}
        _ => panic!("Expected IncompatibleSketches error"),
    }
}

// ============================================================================
// PHASE 4: Trait Implementation Tests
// ============================================================================

#[test]
fn test_sketch_trait() {
    let mut ull: Box<dyn Sketch<Item = u64>> = Box::new(UltraLogLog::new(12).unwrap());

    // Test trait methods
    ull.update(&42);
    assert!(!ull.is_empty());

    let estimate = ull.estimate();
    assert!(estimate > 0.0);
}

#[test]
fn test_mergeable_trait() {
    let mut ull1 = UltraLogLog::new(12).unwrap();
    let ull2 = UltraLogLog::new(12).unwrap();

    // Should compile as Mergeable trait
    let result = ull1.merge(&ull2);
    assert!(result.is_ok());
}

#[test]
fn test_serialization_roundtrip() {
    let mut ull1 = UltraLogLog::new(12).unwrap();

    // Add some items
    for i in 0..1000 {
        ull1.update(&i);
    }

    let estimate1 = ull1.estimate();

    // Serialize
    let bytes = ull1.serialize();
    assert!(!bytes.is_empty(), "Serialized bytes should not be empty");

    // Deserialize
    let ull2 = UltraLogLog::deserialize(&bytes).unwrap();
    let estimate2 = ull2.estimate();

    // Estimates should be identical after roundtrip
    assert_eq!(
        estimate1, estimate2,
        "Estimate should be preserved after serialization roundtrip"
    );

    assert_eq!(
        ull1.is_empty(),
        ull2.is_empty(),
        "Empty state should be preserved"
    );
}

#[test]
fn test_deserialization_invalid_data() {
    let invalid_bytes = vec![0xFF; 10]; // Random invalid data

    let result = UltraLogLog::deserialize(&invalid_bytes);
    assert!(result.is_err(), "Should reject invalid serialized data");

    match result {
        Err(SketchError::DeserializationError(_)) => {}
        _ => panic!("Expected DeserializationError"),
    }
}

#[test]
fn test_serialization_empty_sketch() {
    let ull1 = UltraLogLog::new(12).unwrap();

    let bytes = ull1.serialize();
    let ull2 = UltraLogLog::deserialize(&bytes).unwrap();

    assert!(ull2.is_empty(), "Empty state should survive roundtrip");
    assert!(ull2.estimate() < 1.0);
}

// ============================================================================
// PHASE 5: Additional Edge Cases
// ============================================================================

#[test]
fn test_different_data_types() {
    let mut ull = UltraLogLog::new(12).unwrap();

    // Test with different types
    ull.update(&42u64);
    ull.update(&99u64);
    ull.update(&1000u64);

    let estimate = ull.estimate();
    assert!(
        (2.0..=5.0).contains(&estimate),
        "Should handle different values"
    );
}

#[test]
fn test_zero_values() {
    let mut ull = UltraLogLog::new(12).unwrap();

    // Add zero multiple times
    for _ in 0..100 {
        ull.update(&0u64);
    }

    let estimate = ull.estimate();
    // Should recognize it's the same value
    assert!(
        estimate < 2.0,
        "Multiple zeros should be counted as one unique item"
    );
}

#[test]
fn test_max_values() {
    let mut ull = UltraLogLog::new(12).unwrap();

    // Test with u64::MAX
    ull.update(&u64::MAX);
    ull.update(&(u64::MAX - 1));
    ull.update(&(u64::MAX - 2));

    let estimate = ull.estimate();
    assert!(
        (2.0..=5.0).contains(&estimate),
        "Should handle maximum values"
    );
}

#[test]
fn test_sequential_vs_random_order() {
    // Adding items in different orders should give similar estimates
    let mut ull_seq = UltraLogLog::new(12).unwrap();
    let mut ull_random = UltraLogLog::new(12).unwrap();

    let items: Vec<u64> = (0..1000).collect();

    // Sequential
    for &item in &items {
        ull_seq.update(&item);
    }

    // Reversed order (simulating different order)
    for &item in items.iter().rev() {
        ull_random.update(&item);
    }

    let estimate_seq = ull_seq.estimate();
    let estimate_random = ull_random.estimate();

    // Should be identical regardless of insertion order
    assert_eq!(
        estimate_seq, estimate_random,
        "Estimate should not depend on insertion order"
    );
}

#[test]
fn test_multiple_precision_values() {
    // Test accuracy at different precision levels
    let cardinalities = vec![100, 1000, 10_000];
    let precisions = vec![8, 10, 12, 14, 16];

    for &cardinality in &cardinalities {
        for &precision in &precisions {
            let mut ull = UltraLogLog::new(precision).unwrap();

            for i in 0..cardinality {
                ull.update(&i);
            }

            let estimate = ull.estimate();
            let error = (estimate - cardinality as f64).abs() / cardinality as f64;

            // Error should generally decrease with precision
            // But we allow reasonable variance
            assert!(
                error < 0.15,
                "At precision {}, cardinality {}: error {:.2}% should be reasonable",
                precision,
                cardinality,
                error * 100.0
            );
        }
    }
}

// ============================================================================
// PHASE 4: Property-Based Tests
// ============================================================================

proptest! {
    /// Property: Estimate should always be non-negative
    #[test]
    fn prop_estimate_non_negative(items in prop::collection::vec(any::<u64>(), 0..1000)) {
        let mut ull = UltraLogLog::new(12).unwrap();
        for item in items {
            ull.update(&item);
        }
        prop_assert!(ull.estimate() >= 0.0, "Estimate should always be non-negative");
    }

    /// Property: Adding items should never decrease cardinality estimate
    #[test]
    fn prop_monotonic_updates(items in prop::collection::vec(any::<u64>(), 100..200)) {
        let mut ull = UltraLogLog::new(12).unwrap();

        let mut prev_estimate = 0.0;
        for (i, item) in items.iter().enumerate() {
            ull.update(item);

            if i > 50 {
                // After some items, check monotonicity (with tolerance for variance)
                let current_estimate = ull.estimate();
                // Allow small decreases due to bias correction transitions
                prop_assert!(
                    current_estimate >= prev_estimate * 0.95,
                    "Estimate should generally not decrease: {} -> {}",
                    prev_estimate,
                    current_estimate
                );
                prev_estimate = current_estimate;
            }
        }
    }

    /// Property: Merge should increase or maintain cardinality
    #[test]
    fn prop_merge_increases_estimate(
        items1 in prop::collection::vec(any::<u64>(), 100..500),
        items2 in prop::collection::vec(any::<u64>(), 100..500)
    ) {
        let mut ull1 = UltraLogLog::new(12).unwrap();
        let mut ull2 = UltraLogLog::new(12).unwrap();

        for item in &items1 {
            ull1.update(item);
        }
        for item in &items2 {
            ull2.update(item);
        }

        let before = ull1.estimate();
        ull1.merge(&ull2).unwrap();
        let after = ull1.estimate();

        prop_assert!(
            after >= before * 0.95,
            "Merge should not significantly decrease cardinality: {} -> {}",
            before,
            after
        );
    }

    /// Property: Merge is commutative (A∪B = B∪A)
    #[test]
    fn prop_merge_commutative(
        items1 in prop::collection::vec(any::<u64>(), 50..200),
        items2 in prop::collection::vec(any::<u64>(), 50..200)
    ) {
        let mut ull1a = UltraLogLog::new(12).unwrap();
        let mut ull1b = UltraLogLog::new(12).unwrap();
        let mut ull2a = UltraLogLog::new(12).unwrap();
        let mut ull2b = UltraLogLog::new(12).unwrap();

        for item in &items1 {
            ull1a.update(item);
            ull1b.update(item);
        }
        for item in &items2 {
            ull2a.update(item);
            ull2b.update(item);
        }

        ull1a.merge(&ull2a).unwrap();
        ull2b.merge(&ull1b).unwrap();

        let estimate_ab = ull1a.estimate();
        let estimate_ba = ull2b.estimate();

        // Should be identical (allowing for floating point precision)
        prop_assert!(
            (estimate_ab - estimate_ba).abs() < 0.001,
            "Merge should be commutative: {} vs {}",
            estimate_ab,
            estimate_ba
        );
    }

    /// Property: Serialization roundtrip preserves estimate
    #[test]
    fn prop_serialization_preserves_estimate(items in prop::collection::vec(any::<u64>(), 100..1000)) {
        let mut ull1 = UltraLogLog::new(12).unwrap();

        for item in items {
            ull1.update(&item);
        }

        let estimate1 = ull1.estimate();
        let bytes = ull1.serialize();
        let ull2 = UltraLogLog::deserialize(&bytes).unwrap();
        let estimate2 = ull2.estimate();

        prop_assert_eq!(estimate1, estimate2, "Serialization should preserve estimate");
    }

    /// Property: Empty sketch always estimates close to 0
    #[test]
    fn prop_empty_sketch_near_zero(precision in 4u8..=18) {
        let ull = UltraLogLog::new(precision).unwrap();
        let estimate = ull.estimate();

        prop_assert!(ull.is_empty(), "New sketch should be empty");
        prop_assert!(
            estimate < 1.0,
            "Empty sketch should estimate close to 0, got {}",
            estimate
        );
    }

    /// Property: Adding duplicate items doesn't inflate estimate excessively
    #[test]
    fn prop_duplicates_dont_inflate(
        unique_items in prop::collection::vec(any::<u64>(), 50..150),
        repeat_count in 2usize..10
    ) {
        let mut ull = UltraLogLog::new(12).unwrap();

        // Add each unique item multiple times
        for item in &unique_items {
            for _ in 0..repeat_count {
                ull.update(item);
            }
        }

        let estimate = ull.estimate();
        let expected = unique_items.len() as f64;
        let error = (estimate - expected).abs() / expected.max(1.0);

        prop_assert!(
            error < 0.20,
            "Duplicates shouldn't inflate estimate excessively: estimated {}, expected {}, error {:.1}%",
            estimate,
            expected,
            error * 100.0
        );
    }

    /// Property: Different precision values all give reasonable estimates
    #[test]
    fn prop_precision_gives_reasonable_estimates(
        precision in 8u8..=16,
        cardinality in 100usize..2000
    ) {
        let mut ull = UltraLogLog::new(precision).unwrap();

        for i in 0..cardinality {
            ull.update(&(i as u64));
        }

        let estimate = ull.estimate();
        let expected = cardinality as f64;
        let error = (estimate - expected).abs() / expected;

        // Standard error for UltraLogLog is ~1.04/sqrt(m)
        // We allow 5x standard error for high confidence
        let m = (1 << precision) as f64;
        let std_error = 1.04 / m.sqrt();
        let tolerance = 5.0 * std_error;

        prop_assert!(
            error < tolerance.max(0.15),
            "Precision {} with cardinality {}: error {:.2}% should be within {}%",
            precision,
            cardinality,
            error * 100.0,
            tolerance * 100.0
        );
    }
}
