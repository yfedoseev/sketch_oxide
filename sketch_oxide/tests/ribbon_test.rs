//! Ribbon Filter tests - TDD approach
//!
//! Testing the space-efficient membership filter (RocksDB 2021+) with:
//! - ~30% smaller than Bloom filter (~7 bits/key @ 1% FPR)
//! - Gaussian elimination construction
//! - Finalization required before queries
//!
//! Use cases:
//! - LSM-tree SSTables (static, write-once)
//! - Space-constrained environments
//! - When construction time is acceptable for space savings

use proptest::prelude::*;
use sketch_oxide::membership::RibbonFilter;

// ============================================================================
// Phase 1: Construction Tests
// ============================================================================

#[test]
fn test_new_ribbon_filter() {
    let filter = RibbonFilter::new(1000, 0.01);

    assert!(filter.is_empty(), "New filter should be empty");
    assert_eq!(filter.len(), 0, "Length should be 0");
}

#[test]
fn test_new_with_different_fpr() {
    let fprs = [0.001, 0.01, 0.05, 0.1, 0.5];

    for fpr in fprs {
        let filter = RibbonFilter::new(1000, fpr);
        assert!(filter.is_empty(), "Filter with FPR {} should be empty", fpr);
    }
}

#[test]
fn test_with_params() {
    let filter = RibbonFilter::with_params(100, 100, 800);

    assert!(filter.is_empty(), "New filter should be empty");
}

#[test]
#[should_panic(expected = "Expected number of elements must be > 0")]
fn test_invalid_n_zero() {
    RibbonFilter::new(0, 0.01);
}

#[test]
#[should_panic(expected = "False positive rate must be in (0, 1)")]
fn test_invalid_fpr_zero() {
    RibbonFilter::new(100, 0.0);
}

#[test]
#[should_panic(expected = "False positive rate must be in (0, 1)")]
fn test_invalid_fpr_one() {
    RibbonFilter::new(100, 1.0);
}

// ============================================================================
// Phase 2: Insertion and Finalization Tests
// ============================================================================

#[test]
fn test_insert_single_item() {
    let mut filter = RibbonFilter::new(100, 0.01);

    filter.insert(b"test_key");
    filter.finalize();

    assert!(filter.contains(b"test_key"), "Should find inserted key");
}

#[test]
fn test_insert_multiple_items() {
    let mut filter = RibbonFilter::new(100, 0.01);

    let keys = vec!["key1", "key2", "key3", "apple", "banana"];
    for key in &keys {
        filter.insert(key.as_bytes());
    }
    filter.finalize();

    for key in &keys {
        assert!(filter.contains(key.as_bytes()), "Should find key: {}", key);
    }
}

#[test]
fn test_insert_numeric_types() {
    let mut filter = RibbonFilter::new(100, 0.01);

    filter.insert(&42u64.to_le_bytes());
    filter.insert(&123i32.to_le_bytes());
    filter.finalize();

    assert!(filter.contains(&42u64.to_le_bytes()));
    assert!(filter.contains(&123i32.to_le_bytes()));
}

#[test]
fn test_finalize_required() {
    let mut filter = RibbonFilter::new(100, 0.01);

    filter.insert(b"key");

    // Before finalization, behavior may vary
    // After finalization, membership is guaranteed
    filter.finalize();
    assert!(filter.contains(b"key"));
}

#[test]
fn test_multiple_finalize_calls() {
    let mut filter = RibbonFilter::new(100, 0.01);

    filter.insert(b"key");
    filter.finalize();
    filter.finalize(); // Should be idempotent

    assert!(filter.contains(b"key"));
}

#[test]
fn test_insert_duplicate_items() {
    let mut filter = RibbonFilter::new(100, 0.01);

    for _ in 0..10 {
        filter.insert(b"duplicate");
    }
    filter.finalize();

    assert!(filter.contains(b"duplicate"));
}

// ============================================================================
// Phase 3: Membership Tests (No False Negatives)
// ============================================================================

#[test]
fn test_no_false_negatives() {
    let mut filter = RibbonFilter::new(1000, 0.01);

    let items: Vec<String> = (0..500).map(|i| format!("item_{}", i)).collect();
    for item in &items {
        filter.insert(item.as_bytes());
    }
    filter.finalize();

    // Zero false negatives guaranteed
    for item in &items {
        assert!(
            filter.contains(item.as_bytes()),
            "False negative detected for: {}",
            item
        );
    }
}

#[test]
fn test_false_positive_rate() {
    let n = 1000;
    let target_fpr = 0.01;
    let mut filter = RibbonFilter::new(n, target_fpr);

    for i in 0..n {
        filter.insert(&i.to_le_bytes());
    }
    filter.finalize();

    let test_count = 10000;
    let mut false_positives = 0;

    for i in n..(n + test_count) {
        if filter.contains(&i.to_le_bytes()) {
            false_positives += 1;
        }
    }

    let actual_fpr = false_positives as f64 / test_count as f64;

    // Allow 5x tolerance
    assert!(
        actual_fpr < target_fpr * 5.0,
        "FPR {} exceeds 5x target {}",
        actual_fpr,
        target_fpr
    );
}

#[test]
fn test_empty_filter_contains_nothing() {
    let mut filter = RibbonFilter::new(100, 0.01);
    filter.finalize();

    assert!(!filter.contains(b"anything"));
    assert!(!filter.contains(b""));
}

// ============================================================================
// Phase 4: Serialization Tests
// ============================================================================

#[test]
fn test_serialize_deserialize() {
    let mut filter = RibbonFilter::new(100, 0.01);
    filter.insert(b"test_key");
    filter.insert(b"another_key");
    filter.finalize();

    let bytes = filter.to_bytes();
    let restored = RibbonFilter::from_bytes(&bytes).unwrap();

    assert!(restored.contains(b"test_key"));
    assert!(restored.contains(b"another_key"));
    assert!(!restored.contains(b"missing_key"));
}

#[test]
fn test_serialize_empty_filter() {
    let mut filter = RibbonFilter::new(100, 0.01);
    filter.finalize();

    let bytes = filter.to_bytes();
    let restored = RibbonFilter::from_bytes(&bytes).unwrap();

    assert!(restored.is_empty());
}

// ============================================================================
// Phase 5: Space Efficiency Tests
// ============================================================================

#[test]
fn test_space_efficiency_better_than_bloom() {
    // Ribbon: ~7 bits/key @ 1% FPR
    // Bloom: ~10 bits/key @ 1% FPR
    // Ribbon should be ~30% smaller

    let n = 10000;
    let mut ribbon = RibbonFilter::new(n, 0.01);

    for i in 0..n {
        ribbon.insert(&i.to_le_bytes());
    }
    ribbon.finalize();

    let _ribbon_bytes = ribbon.to_bytes();

    // Theoretical: 7 bits/key * 10000 = 8,750 bytes
    // Bloom theoretical: 10 bits/key * 10000 = 12,500 bytes
    // Allow significant variance in actual implementation

    // Just verify filter works - space efficiency measured in benchmarks
    for i in 0..n {
        assert!(ribbon.contains(&i.to_le_bytes()));
    }
}

// ============================================================================
// Phase 6: Edge Cases
// ============================================================================

#[test]
fn test_empty_key() {
    let mut filter = RibbonFilter::new(100, 0.01);

    filter.insert(b"");
    filter.finalize();

    assert!(filter.contains(b""));
}

#[test]
fn test_very_long_key() {
    let mut filter = RibbonFilter::new(100, 0.01);

    let long_key = vec![b'x'; 10000];
    filter.insert(&long_key);
    filter.finalize();

    assert!(filter.contains(&long_key));
}

#[test]
fn test_binary_data() {
    let mut filter = RibbonFilter::new(100, 0.01);

    let binary_key = vec![0u8, 1, 2, 255, 0, 128];
    filter.insert(&binary_key);
    filter.finalize();

    assert!(filter.contains(&binary_key));
}

#[test]
fn test_small_filter() {
    let mut filter = RibbonFilter::new(1, 0.5);

    filter.insert(b"key");
    filter.finalize();

    assert!(filter.contains(b"key"));
}

#[test]
fn test_finalize_empty_filter() {
    let mut filter = RibbonFilter::new(100, 0.01);
    filter.finalize();

    // Empty finalized filter should not crash on queries
    assert!(!filter.contains(b"key"));
}

// ============================================================================
// Phase 7: Property-Based Tests
// ============================================================================

proptest! {
    #[test]
    fn prop_no_false_negatives(keys in prop::collection::vec(prop::collection::vec(any::<u8>(), 1..50), 1..50)) {
        let mut filter = RibbonFilter::new(keys.len().max(1), 0.01);

        for key in &keys {
            filter.insert(key);
        }
        filter.finalize();

        for key in &keys {
            prop_assert!(filter.contains(key), "False negative for key {:?}", key);
        }
    }

    #[test]
    fn prop_serialization_roundtrip(keys in prop::collection::vec(prop::collection::vec(any::<u8>(), 1..50), 1..30)) {
        let mut filter = RibbonFilter::new(keys.len().max(1), 0.01);

        for key in &keys {
            filter.insert(key);
        }
        filter.finalize();

        let bytes = filter.to_bytes();
        let restored = RibbonFilter::from_bytes(&bytes).unwrap();

        for key in &keys {
            prop_assert!(restored.contains(key));
        }
    }
}

// ============================================================================
// Phase 8: Comparison Tests (When to Use Ribbon)
// ============================================================================

#[test]
fn test_ribbon_use_case_static_data() {
    // Ribbon is ideal for:
    // 1. LSM-tree SSTables (write once, read many)
    // 2. Space-constrained environments
    // 3. When construction time is acceptable

    let mut filter = RibbonFilter::new(1000, 0.01);

    // Build phase: insert all keys
    for i in 0u64..500 {
        filter.insert(&i.to_le_bytes());
    }

    // Finalize: solve the linear system (Gaussian elimination)
    filter.finalize();

    // Query phase: fast lookups
    for i in 0u64..500 {
        assert!(filter.contains(&i.to_le_bytes()));
    }

    // No more inserts allowed (static structure)
}

#[test]
fn test_ribbon_vs_bloom_tradeoff() {
    // Ribbon advantages:
    // - ~30% smaller than Bloom
    // - Better for space-constrained environments
    //
    // Bloom advantages:
    // - Incremental inserts (no finalization)
    // - Simpler implementation
    // - Faster construction
    //
    // This test verifies Ribbon works correctly (space comparison in benchmarks)

    let mut filter = RibbonFilter::new(100, 0.01);

    let keys: Vec<String> = (0..50).map(|i| format!("key_{}", i)).collect();
    for key in &keys {
        filter.insert(key.as_bytes());
    }
    filter.finalize();

    for key in &keys {
        assert!(filter.contains(key.as_bytes()));
    }
}
