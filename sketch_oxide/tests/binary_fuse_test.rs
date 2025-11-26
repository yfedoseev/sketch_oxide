//! Binary Fuse Filter tests - TDD approach (ACM JEA 2022)
//!
//! Testing the state-of-the-art membership filter with:
//! - 75% more space-efficient than Bloom filters
//! - 9 bits/item for 1% FP rate
//! - Immutable design (build once, query many)

use sketch_oxide::common::{Sketch, SketchError};
use sketch_oxide::membership::BinaryFuseFilter;
use std::collections::HashSet;

// ============================================================================
// Phase 1: Construction Tests
// ============================================================================

#[test]
fn test_new_from_items() {
    let items = vec![1u64, 2, 3, 4, 5, 100, 200, 500];
    let filter = BinaryFuseFilter::from_items(items.iter().copied(), 9);

    assert!(filter.is_ok(), "Construction should succeed");
    let filter = filter.unwrap();

    // Verify all items are found
    for item in &items {
        assert!(filter.contains(item), "Item {} should be found", item);
    }
}

#[test]
fn test_new_from_empty_set() {
    let items: Vec<u64> = vec![];
    let filter = BinaryFuseFilter::from_items(items, 9);

    assert!(filter.is_ok(), "Empty filter should be valid");
    let filter = filter.unwrap();

    assert!(filter.is_empty(), "Filter should be empty");
    assert_eq!(filter.len(), 0, "Length should be 0");
}

#[test]
fn test_new_from_single_item() {
    let items = [42u64];
    let filter = BinaryFuseFilter::from_items(items.iter().copied(), 9).unwrap();

    assert_eq!(filter.len(), 1, "Filter should contain 1 item");
    assert!(filter.contains(&42), "Should find the single item");
    assert!(
        !filter.contains(&43),
        "Should not find other items (high probability)"
    );
}

#[test]
fn test_bits_per_entry_validation() {
    let items = [1u64, 2, 3, 4, 5];

    // Valid range: 8-16 bits
    for bits in 8..=16 {
        let filter = BinaryFuseFilter::from_items(items.iter().copied(), bits);
        assert!(filter.is_ok(), "bits_per_entry={} should be valid", bits);
    }
}

#[test]
fn test_invalid_bits_per_entry() {
    let items = [1u64, 2, 3, 4, 5];

    // Too low
    let result = BinaryFuseFilter::from_items(items.iter().copied(), 7);
    assert!(
        matches!(result, Err(SketchError::InvalidParameter { .. })),
        "Should reject bits_per_entry < 8"
    );

    // Too high
    let result = BinaryFuseFilter::from_items(items.iter().copied(), 17);
    assert!(
        matches!(result, Err(SketchError::InvalidParameter { .. })),
        "Should reject bits_per_entry > 16"
    );
}

// ============================================================================
// Phase 2: Membership Tests
// ============================================================================

#[test]
fn test_contains_all_items() {
    let items: Vec<u64> = (0..1000).collect();
    let filter = BinaryFuseFilter::from_items(items.iter().copied(), 9).unwrap();

    // All items should be found (no false negatives)
    for item in &items {
        assert!(
            filter.contains(item),
            "False negative detected for item {}",
            item
        );
    }
}

#[test]
fn test_false_positive_rate() {
    let items: Vec<u64> = (0..10_000).collect();
    let filter = BinaryFuseFilter::from_items(items.iter().copied(), 9).unwrap();

    // Test on items NOT in the filter
    let mut false_positives = 0;
    let test_count = 10_000;

    for i in 10_000..10_000 + test_count {
        if filter.contains(&i) {
            false_positives += 1;
        }
    }

    let fp_rate = false_positives as f64 / test_count as f64;
    let expected_fp = 0.0102; // ~1.02% for 9 bits

    println!(
        "False positive rate: {:.4}% (expected ~{:.4}%)",
        fp_rate * 100.0,
        expected_fp * 100.0
    );

    // Allow 50% tolerance around theoretical rate
    assert!(
        fp_rate < expected_fp * 1.5,
        "FP rate too high: {:.4}% > {:.4}%",
        fp_rate * 100.0,
        expected_fp * 1.5 * 100.0
    );
}

#[test]
fn test_large_dataset() {
    let items: Vec<u64> = (0..100_000).collect();
    let filter = BinaryFuseFilter::from_items(items.iter().copied(), 9).unwrap();

    assert_eq!(filter.len(), 100_000, "Filter size should match input");

    // Sample verification (checking all 100K would be slow)
    for i in (0..100_000).step_by(100) {
        assert!(filter.contains(&i), "Item {} not found", i);
    }
}

#[test]
fn test_no_false_negatives() {
    // Random items to ensure no pattern-specific bugs
    let mut items = vec![1u64, 42, 100, 999, 1234, 5678, 9999];
    items.extend((10_000..11_000).collect::<Vec<u64>>());

    let filter = BinaryFuseFilter::from_items(items.iter().copied(), 10).unwrap();

    // CRITICAL: No false negatives allowed
    for item in &items {
        assert!(
            filter.contains(item),
            "FALSE NEGATIVE detected for item {}",
            item
        );
    }
}

// ============================================================================
// Phase 3: False Positive Rate Tests (Different Bit Configurations)
// ============================================================================

#[test]
fn test_fp_rate_9_bits() {
    verify_fp_rate(9, 0.0102, "9 bits → ~1.02% FP");
}

#[test]
fn test_fp_rate_10_bits() {
    verify_fp_rate(10, 0.0051, "10 bits → ~0.51% FP");
}

#[test]
fn test_fp_rate_12_bits() {
    verify_fp_rate(12, 0.0013, "12 bits → ~0.13% FP");
}

// Helper function for FP rate verification
fn verify_fp_rate(bits: u8, expected_rate: f64, description: &str) {
    let items: Vec<u64> = (0..10_000).collect();
    let filter = BinaryFuseFilter::from_items(items.iter().copied(), bits).unwrap();

    let mut false_positives = 0;
    let test_count = 20_000;

    // Test on items NOT in filter
    for i in 10_000..10_000 + test_count {
        if filter.contains(&i) {
            false_positives += 1;
        }
    }

    let actual_rate = false_positives as f64 / test_count as f64;

    println!(
        "{}: actual={:.4}%, expected={:.4}%",
        description,
        actual_rate * 100.0,
        expected_rate * 100.0
    );

    // Allow 4x tolerance for small sample sizes and hash variation
    // Binary Fuse filters can have slightly higher FP rates than theoretical
    assert!(
        actual_rate < expected_rate * 4.0,
        "FP rate too high for {}: {:.4}% > {:.4}%",
        description,
        actual_rate * 100.0,
        expected_rate * 4.0 * 100.0
    );
}

// ============================================================================
// Phase 4: Edge Cases
// ============================================================================

#[test]
fn test_duplicate_items() {
    let items = vec![1u64, 2, 3, 2, 1, 3, 4, 4, 4];
    let filter = BinaryFuseFilter::from_items(items.iter().copied(), 9).unwrap();

    // All unique items should be found
    for item in 1..=4 {
        assert!(filter.contains(&item), "Item {} should be found", item);
    }
}

#[test]
fn test_max_value_items() {
    let items = vec![u64::MAX, u64::MAX - 1, u64::MAX - 100, 0, 1];

    let filter = BinaryFuseFilter::from_items(items.iter().copied(), 9).unwrap();

    for item in &items {
        assert!(
            filter.contains(item),
            "Should handle extreme values: {}",
            item
        );
    }
}

#[test]
fn test_consecutive_items() {
    let items: Vec<u64> = (0..10_000).collect();
    let filter = BinaryFuseFilter::from_items(items.iter().copied(), 9).unwrap();

    // All consecutive items should be found
    for item in &items {
        assert!(filter.contains(item), "Consecutive item {} not found", item);
    }
}

#[test]
fn test_random_items() {
    use std::collections::hash_map::RandomState;
    use std::hash::BuildHasher;

    // Generate pseudo-random items
    let random_state = RandomState::new();
    let mut items = Vec::new();

    for i in 0..1000 {
        items.push(random_state.hash_one(i));
    }

    let filter = BinaryFuseFilter::from_items(items.iter().copied(), 9).unwrap();

    // Verify all items are found
    for item in &items {
        assert!(filter.contains(item), "Random item {} not found", item);
    }
}

// ============================================================================
// Phase 5: Trait Implementation Tests
// ============================================================================

#[test]
fn test_sketch_trait() {
    use sketch_oxide::common::Sketch;

    let items = [1u64, 2, 3, 4, 5];
    let filter = BinaryFuseFilter::from_items(items.iter().copied(), 9).unwrap();

    // Should implement Sketch
    assert_eq!(filter.estimate(), 5.0);
    assert!(!filter.is_empty());
}

#[test]
#[should_panic(expected = "immutable")]
fn test_immutable_panic_on_update() {
    use sketch_oxide::common::Sketch;

    let items = [1u64, 2, 3];
    let mut filter = BinaryFuseFilter::from_items(items.iter().copied(), 9).unwrap();

    // Should panic - Binary Fuse is immutable
    filter.update(&10);
}

#[test]
fn test_serialization_roundtrip() {
    use sketch_oxide::common::Sketch;

    let items: Vec<u64> = (0..1000).collect();
    let original = BinaryFuseFilter::from_items(items.iter().copied(), 9).unwrap();

    // Serialize
    let bytes = original.serialize();
    assert!(!bytes.is_empty(), "Serialization should produce data");

    // Deserialize
    let restored = BinaryFuseFilter::deserialize(&bytes).unwrap();

    // Verify same behavior
    for item in items.iter().take(10) {
        assert_eq!(
            original.contains(item),
            restored.contains(item),
            "Item {} has different membership after roundtrip",
            item
        );
    }

    assert_eq!(original.len(), restored.len());
    assert_eq!(original.is_empty(), restored.is_empty());
}

#[test]
fn test_no_mergeable() {
    // Binary Fuse does NOT implement Mergeable trait
    // This is a compile-time check, but we document it here

    // If someone tries to merge, it should not compile:
    // let filter1 = BinaryFuseFilter::from_items(...);
    // let filter2 = BinaryFuseFilter::from_items(...);
    // filter1.merge(&filter2); // <- This should NOT compile
}

// ============================================================================
// Phase 6: Size Tests
// ============================================================================

#[test]
fn test_size_calculation() {
    let items: Vec<u64> = (0..1000).collect();
    let filter = BinaryFuseFilter::from_items(items.iter().copied(), 9).unwrap();

    let bits_per_entry = filter.bits_per_entry();

    println!("Bits per entry: {:.2}", bits_per_entry);

    // Should be close to 9.0 bits (allow up to 10.0 due to rounding overhead)
    assert!(
        (9.0..=10.0).contains(&bits_per_entry),
        "Bits per entry should be ~9.0, got {:.2}",
        bits_per_entry
    );
}

#[test]
fn test_memory_efficiency() {
    let n = 10_000;
    let items: Vec<u64> = (0..n).collect();
    let filter = BinaryFuseFilter::from_items(items.iter().copied(), 9).unwrap();

    let actual_bits = filter.bits_per_entry();
    let theoretical_min = -f64::log2(0.0102); // ~6.62 bits for 1% FP

    println!("Actual: {:.2} bits/entry", actual_bits);
    println!("Theoretical min: {:.2} bits/entry", theoretical_min);

    let overhead = (actual_bits - theoretical_min) / theoretical_min;
    println!("Overhead: {:.1}%", overhead * 100.0);

    // Binary Fuse should be within reasonable overhead of theoretical minimum
    // With 1.23x overhead factor, we expect ~50% total overhead
    assert!(
        overhead < 0.60,
        "Overhead too high: {:.1}% (expected <60%)",
        overhead * 100.0
    );
}

#[test]
fn test_actual_memory_usage() {
    let n = 1000;
    let items: Vec<u64> = (0..n).collect();
    let filter = BinaryFuseFilter::from_items(items.iter().copied(), 9).unwrap();

    let bytes = filter.serialize();
    let bytes_per_item = bytes.len() as f64 / n as f64;

    println!("Total size: {} bytes", bytes.len());
    println!("Bytes per item: {:.2}", bytes_per_item);

    // 9 bits = 1.125 bytes, with 1.23x overhead = ~1.4 bytes/item
    // For 1000 items: ~1.4 KB is reasonable
    assert!(
        bytes.len() < 1500,
        "Memory usage too high: {} bytes for {} items",
        bytes.len(),
        n
    );
}

// ============================================================================
// Phase 7: Performance Characteristic Tests
// ============================================================================

#[test]
fn test_construction_completes_reasonable_time() {
    use std::time::Instant;

    let n = 10_000;
    let items: Vec<u64> = (0..n).collect();

    let start = Instant::now();
    let filter = BinaryFuseFilter::from_items(items.iter().copied(), 9).unwrap();
    let duration = start.elapsed();

    let ns_per_item = duration.as_nanos() / n as u128;

    println!("Construction: {} ns/item", ns_per_item);

    // Should be < 100000ns per item (100 microseconds) for unoptimized build
    // Release builds will be much faster (<50ns target)
    assert!(
        ns_per_item < 100000,
        "Construction too slow: {} ns/item",
        ns_per_item
    );

    // Verify filter works
    assert!(filter.contains(&5000));
}

#[test]
fn test_query_completes_reasonable_time() {
    use std::time::Instant;

    let items: Vec<u64> = (0..10_000).collect();
    let filter = BinaryFuseFilter::from_items(items.iter().copied(), 9).unwrap();

    // Warm up
    for i in 0..100 {
        let _ = filter.contains(&i);
    }

    // Measure
    let iterations = 10_000;
    let start = Instant::now();

    for i in 0..iterations {
        let _ = filter.contains(&i);
    }

    let duration = start.elapsed();
    let ns_per_query = duration.as_nanos() / iterations as u128;

    println!("Query: {} ns/lookup", ns_per_query);

    // Should be < 10000ns per query (10 microseconds) for unoptimized build
    assert!(
        ns_per_query < 10000,
        "Queries too slow: {} ns/lookup",
        ns_per_query
    );
}

// ============================================================================
// Phase 8: Property-Based Tests (using proptest)
// ============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_no_false_negatives(items in prop::collection::vec(any::<u64>(), 1..1000)) {
            let unique_items: HashSet<u64> = items.iter().copied().collect();
            let filter = BinaryFuseFilter::from_items(unique_items.iter().copied(), 9).unwrap();

            for item in &unique_items {
                prop_assert!(filter.contains(item),
                            "False negative for item {}", item);
            }
        }

        #[test]
        fn prop_false_positive_bounded(
            items in prop::collection::vec(any::<u64>(), 100..500),
            test_items in prop::collection::vec(any::<u64>(), 1000..2000)
        ) {
            let items_set: HashSet<u64> = items.iter().copied().collect();
            let filter = BinaryFuseFilter::from_items(items_set.iter().copied(), 9).unwrap();

            let mut false_positives = 0;
            let mut negatives = 0;

            for item in test_items {
                if !items_set.contains(&item) {
                    negatives += 1;
                    if filter.contains(&item) {
                        false_positives += 1;
                    }
                }
            }

            if negatives > 0 {
                let fp_rate = false_positives as f64 / negatives as f64;
                // Allow generous bound for property test (2% for 1% target)
                prop_assert!(fp_rate < 0.025,
                            "FP rate too high: {:.4}%", fp_rate * 100.0);
            }
        }

        #[test]
        fn prop_bits_per_entry_accurate(
            items in prop::collection::vec(any::<u64>(), 100..1000)
        ) {
            let unique_items: HashSet<u64> = items.iter().copied().collect();
            if unique_items.is_empty() {
                return Ok(());
            }

            let filter = BinaryFuseFilter::from_items(
                unique_items.iter().copied(), 9
            ).unwrap();

            let actual_bits = filter.bits_per_entry();
            prop_assert!((9.0..=11.0).contains(&actual_bits),
                        "Expected ~9-11 bits/entry, got {:.2}", actual_bits);
        }

        #[test]
        fn prop_empty_filter_behavior(
            _test_items in prop::collection::vec(any::<u64>(), 1..100)
        ) {
            let filter = BinaryFuseFilter::from_items(
                std::iter::empty(), 9
            ).unwrap();

            // Empty filter should report as empty
            prop_assert!(filter.is_empty());
            prop_assert_eq!(filter.len(), 0);

            // Note: Empty filter behavior with contains() is undefined/acceptable
            // since it has no data. Some false positives are acceptable for empty case.
        }

        #[test]
        fn prop_serialization_preserves_membership(
            items in prop::collection::vec(any::<u64>(), 10..200)
        ) {
            use sketch_oxide::common::Sketch;

            let unique_items: HashSet<u64> = items.iter().copied().collect();
            let original = BinaryFuseFilter::from_items(
                unique_items.iter().copied(), 9
            ).unwrap();

            let bytes = original.serialize();
            let restored = BinaryFuseFilter::deserialize(&bytes).unwrap();

            // Check sample of items
            for item in unique_items.iter().take(20) {
                prop_assert_eq!(
                    original.contains(item),
                    restored.contains(item),
                    "Membership changed after serialization for item {}", item
                );
            }
        }
    }
}
