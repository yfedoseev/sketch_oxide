//! Bloom Filter tests - TDD approach
//!
//! Testing the classic probabilistic membership filter with:
//! - Dynamic insertions (unlike Binary Fuse)
//! - Configurable false positive rate
//! - Zero false negatives guaranteed
//!
//! Use cases:
//! - When items arrive incrementally (streaming)
//! - When set size is unknown upfront
//! - When simplicity is preferred over space efficiency

use proptest::prelude::*;
use sketch_oxide::membership::BloomFilter;

// ============================================================================
// Phase 1: Construction Tests
// ============================================================================

#[test]
fn test_new_bloom_filter() {
    let filter = BloomFilter::new(1000, 0.01);

    assert!(filter.is_empty(), "New filter should be empty");
    assert_eq!(filter.len(), 0, "Length should be 0");
}

#[test]
fn test_new_with_different_fpr() {
    // Test various false positive rates
    let fprs = [0.001, 0.01, 0.05, 0.1, 0.5];

    for fpr in fprs {
        let filter = BloomFilter::new(1000, fpr);
        assert!(filter.is_empty(), "Filter with FPR {} should be empty", fpr);
    }
}

#[test]
fn test_with_params() {
    let filter = BloomFilter::with_params(100, 1000, 7);

    assert!(filter.is_empty(), "New filter should be empty");
}

#[test]
#[should_panic(expected = "Expected number of elements must be > 0")]
fn test_invalid_n_zero() {
    BloomFilter::new(0, 0.01);
}

#[test]
#[should_panic(expected = "False positive rate must be in (0, 1)")]
fn test_invalid_fpr_zero() {
    BloomFilter::new(100, 0.0);
}

#[test]
#[should_panic(expected = "False positive rate must be in (0, 1)")]
fn test_invalid_fpr_one() {
    BloomFilter::new(100, 1.0);
}

#[test]
#[should_panic(expected = "False positive rate must be in (0, 1)")]
fn test_invalid_fpr_negative() {
    BloomFilter::new(100, -0.1);
}

// ============================================================================
// Phase 2: Insertion Tests
// ============================================================================

#[test]
fn test_insert_single_item() {
    let mut filter = BloomFilter::new(100, 0.01);

    filter.insert(b"test_key");

    assert!(
        !filter.is_empty(),
        "Filter should not be empty after insert"
    );
    assert!(filter.contains(b"test_key"), "Should find inserted key");
}

#[test]
fn test_insert_multiple_items() {
    let mut filter = BloomFilter::new(100, 0.01);

    let keys = vec!["key1", "key2", "key3", "apple", "banana"];
    for key in &keys {
        filter.insert(key.as_bytes());
    }

    // All keys should be found (no false negatives)
    for key in &keys {
        assert!(filter.contains(key.as_bytes()), "Should find key: {}", key);
    }
}

#[test]
fn test_insert_numeric_types() {
    let mut filter = BloomFilter::new(100, 0.01);

    // Insert various numeric types as bytes
    filter.insert(&42u64.to_le_bytes());
    filter.insert(&123i32.to_le_bytes());
    filter.insert(&std::f64::consts::PI.to_le_bytes());

    assert!(filter.contains(&42u64.to_le_bytes()));
    assert!(filter.contains(&123i32.to_le_bytes()));
    assert!(filter.contains(&std::f64::consts::PI.to_le_bytes()));
}

#[test]
fn test_insert_duplicate_items() {
    let mut filter = BloomFilter::new(100, 0.01);

    // Insert same item multiple times
    for _ in 0..10 {
        filter.insert(b"duplicate");
    }

    assert!(filter.contains(b"duplicate"), "Should find duplicated key");
}

// ============================================================================
// Phase 3: Membership Tests (No False Negatives)
// ============================================================================

#[test]
fn test_no_false_negatives() {
    let mut filter = BloomFilter::new(10000, 0.01);

    // Insert 1000 items
    let items: Vec<String> = (0..1000).map(|i| format!("item_{}", i)).collect();
    for item in &items {
        filter.insert(item.as_bytes());
    }

    // All items MUST be found (zero false negatives guaranteed)
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
    let n = 10000;
    let target_fpr = 0.01;
    let mut filter = BloomFilter::new(n, target_fpr);

    // Insert n items
    for i in 0..n {
        filter.insert(&i.to_le_bytes());
    }

    // Test items NOT in the set
    let test_count = 100000;
    let mut false_positives = 0;

    for i in n..(n + test_count) {
        if filter.contains(&i.to_le_bytes()) {
            false_positives += 1;
        }
    }

    let actual_fpr = false_positives as f64 / test_count as f64;

    // Allow 3x tolerance (FPR varies)
    assert!(
        actual_fpr < target_fpr * 3.0,
        "FPR {} exceeds 3x target {}",
        actual_fpr,
        target_fpr
    );
}

#[test]
fn test_empty_filter_contains_nothing() {
    let filter = BloomFilter::new(100, 0.01);

    // Empty filter should not contain anything
    assert!(!filter.contains(b"anything"));
    assert!(!filter.contains(b""));
    assert!(!filter.contains(&[0u8; 100]));
}

// ============================================================================
// Phase 4: Merge Tests
// ============================================================================

#[test]
fn test_merge_filters() {
    let mut filter1 = BloomFilter::new(100, 0.01);
    let mut filter2 = BloomFilter::new(100, 0.01);

    filter1.insert(b"key1");
    filter1.insert(b"key2");

    filter2.insert(b"key3");
    filter2.insert(b"key4");

    filter1.merge(&filter2);

    // Merged filter should contain all keys
    assert!(filter1.contains(b"key1"));
    assert!(filter1.contains(b"key2"));
    assert!(filter1.contains(b"key3"));
    assert!(filter1.contains(b"key4"));
}

#[test]
fn test_merge_empty_filter() {
    let mut filter1 = BloomFilter::new(100, 0.01);
    let filter2 = BloomFilter::new(100, 0.01);

    filter1.insert(b"key1");
    filter1.merge(&filter2);

    assert!(
        filter1.contains(b"key1"),
        "Should still contain original key"
    );
}

#[test]
fn test_merge_into_empty_filter() {
    let mut filter1 = BloomFilter::new(100, 0.01);
    let mut filter2 = BloomFilter::new(100, 0.01);

    filter2.insert(b"key1");
    filter1.merge(&filter2);

    assert!(filter1.contains(b"key1"), "Should contain merged key");
}

// ============================================================================
// Phase 5: Serialization Tests
// ============================================================================

#[test]
fn test_serialize_deserialize() {
    let mut filter = BloomFilter::new(100, 0.01);
    filter.insert(b"test_key");
    filter.insert(b"another_key");

    let bytes = filter.to_bytes();
    let restored = BloomFilter::from_bytes(&bytes).unwrap();

    assert!(restored.contains(b"test_key"));
    assert!(restored.contains(b"another_key"));
    assert!(!restored.contains(b"missing_key"));
}

#[test]
fn test_serialize_empty_filter() {
    let filter = BloomFilter::new(100, 0.01);

    let bytes = filter.to_bytes();
    let restored = BloomFilter::from_bytes(&bytes).unwrap();

    assert!(restored.is_empty());
}

// ============================================================================
// Phase 6: Edge Cases
// ============================================================================

#[test]
fn test_empty_key() {
    let mut filter = BloomFilter::new(100, 0.01);

    filter.insert(b"");
    assert!(filter.contains(b""), "Should handle empty key");
}

#[test]
fn test_very_long_key() {
    let mut filter = BloomFilter::new(100, 0.01);

    let long_key = vec![b'x'; 10000];
    filter.insert(&long_key);

    assert!(filter.contains(&long_key), "Should handle very long key");
}

#[test]
fn test_binary_data() {
    let mut filter = BloomFilter::new(100, 0.01);

    // Binary data with null bytes
    let binary_key = vec![0u8, 1, 2, 255, 0, 128];
    filter.insert(&binary_key);

    assert!(filter.contains(&binary_key), "Should handle binary data");
}

#[test]
fn test_small_filter() {
    let mut filter = BloomFilter::new(1, 0.5);

    filter.insert(b"key");
    assert!(filter.contains(b"key"));
}

// ============================================================================
// Phase 7: Property-Based Tests
// ============================================================================

proptest! {
    #[test]
    fn prop_no_false_negatives(keys in prop::collection::vec(prop::collection::vec(any::<u8>(), 1..100), 1..100)) {
        let mut filter = BloomFilter::new(keys.len().max(1), 0.01);

        for key in &keys {
            filter.insert(key);
        }

        // All inserted keys MUST be found
        for key in &keys {
            prop_assert!(filter.contains(key), "False negative for key {:?}", key);
        }
    }

    #[test]
    fn prop_merge_preserves_membership(
        keys1 in prop::collection::vec(prop::collection::vec(any::<u8>(), 1..50), 1..50),
        keys2 in prop::collection::vec(prop::collection::vec(any::<u8>(), 1..50), 1..50)
    ) {
        let size = (keys1.len() + keys2.len()).max(1);
        let mut filter1 = BloomFilter::new(size, 0.01);
        let mut filter2 = BloomFilter::new(size, 0.01);

        for key in &keys1 {
            filter1.insert(key);
        }
        for key in &keys2 {
            filter2.insert(key);
        }

        filter1.merge(&filter2);

        // All keys from both filters should be found
        for key in &keys1 {
            prop_assert!(filter1.contains(key));
        }
        for key in &keys2 {
            prop_assert!(filter1.contains(key));
        }
    }

    #[test]
    fn prop_serialization_roundtrip(keys in prop::collection::vec(prop::collection::vec(any::<u8>(), 1..50), 1..50)) {
        let mut filter = BloomFilter::new(keys.len().max(1), 0.01);

        for key in &keys {
            filter.insert(key);
        }

        let bytes = filter.to_bytes();
        let restored = BloomFilter::from_bytes(&bytes).unwrap();

        for key in &keys {
            prop_assert!(restored.contains(key), "Key lost in serialization");
        }
    }
}

// ============================================================================
// Phase 8: Comparison with Binary Fuse (When to Use Bloom)
// ============================================================================

#[test]
fn test_bloom_advantage_incremental_inserts() {
    // Bloom filter allows incremental inserts - Binary Fuse does not
    let mut filter = BloomFilter::new(1000, 0.01);

    // Simulate streaming data - insert one at a time
    for i in 0u64..100 {
        filter.insert(&i.to_le_bytes());

        // Can query immediately after each insert
        assert!(filter.contains(&i.to_le_bytes()));
    }

    // This pattern is impossible with Binary Fuse (requires all items upfront)
}
