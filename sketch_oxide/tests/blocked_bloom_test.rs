//! Blocked Bloom Filter tests - TDD approach
//!
//! Testing the cache-efficient Bloom filter variant with:
//! - 1 cache miss per query (vs 7+ for standard Bloom)
//! - Same space efficiency as standard Bloom (~10 bits/key @ 1% FPR)
//! - Cache-line-aligned memory (64 bytes = 512 bits per block)
//!
//! Use cases:
//! - High-throughput query workloads
//! - Memory-bound systems where cache efficiency matters
//! - When query latency consistency is important

use proptest::prelude::*;
use sketch_oxide::membership::BlockedBloomFilter;

// ============================================================================
// Phase 1: Construction Tests
// ============================================================================

#[test]
fn test_new_blocked_bloom_filter() {
    let filter = BlockedBloomFilter::new(1000, 0.01);

    assert!(filter.is_empty(), "New filter should be empty");
    assert_eq!(filter.len(), 0, "Length should be 0");
}

#[test]
fn test_new_with_different_fpr() {
    let fprs = [0.001, 0.01, 0.05, 0.1, 0.5];

    for fpr in fprs {
        let filter = BlockedBloomFilter::new(1000, fpr);
        assert!(filter.is_empty(), "Filter with FPR {} should be empty", fpr);
    }
}

#[test]
fn test_with_params() {
    let filter = BlockedBloomFilter::with_params(100, 10, 7);

    assert!(filter.is_empty(), "New filter should be empty");
}

#[test]
#[should_panic(expected = "Expected number of elements must be > 0")]
fn test_invalid_n_zero() {
    BlockedBloomFilter::new(0, 0.01);
}

#[test]
#[should_panic(expected = "False positive rate must be in (0, 1)")]
fn test_invalid_fpr_zero() {
    BlockedBloomFilter::new(100, 0.0);
}

#[test]
#[should_panic(expected = "False positive rate must be in (0, 1)")]
fn test_invalid_fpr_one() {
    BlockedBloomFilter::new(100, 1.0);
}

// ============================================================================
// Phase 2: Insertion Tests
// ============================================================================

#[test]
fn test_insert_single_item() {
    let mut filter = BlockedBloomFilter::new(100, 0.01);

    filter.insert(b"test_key");

    assert!(
        !filter.is_empty(),
        "Filter should not be empty after insert"
    );
    assert!(filter.contains(b"test_key"), "Should find inserted key");
}

#[test]
fn test_insert_multiple_items() {
    let mut filter = BlockedBloomFilter::new(100, 0.01);

    let keys = vec!["key1", "key2", "key3", "apple", "banana"];
    for key in &keys {
        filter.insert(key.as_bytes());
    }

    for key in &keys {
        assert!(filter.contains(key.as_bytes()), "Should find key: {}", key);
    }
}

#[test]
fn test_insert_numeric_types() {
    let mut filter = BlockedBloomFilter::new(100, 0.01);

    filter.insert(&42u64.to_le_bytes());
    filter.insert(&123i32.to_le_bytes());

    assert!(filter.contains(&42u64.to_le_bytes()));
    assert!(filter.contains(&123i32.to_le_bytes()));
}

#[test]
fn test_insert_duplicate_items() {
    let mut filter = BlockedBloomFilter::new(100, 0.01);

    for _ in 0..10 {
        filter.insert(b"duplicate");
    }

    assert!(filter.contains(b"duplicate"));
}

// ============================================================================
// Phase 3: Membership Tests (No False Negatives)
// ============================================================================

#[test]
fn test_no_false_negatives() {
    let mut filter = BlockedBloomFilter::new(10000, 0.01);

    let items: Vec<String> = (0..1000).map(|i| format!("item_{}", i)).collect();
    for item in &items {
        filter.insert(item.as_bytes());
    }

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
    let n = 10000;
    let target_fpr = 0.01;
    let mut filter = BlockedBloomFilter::new(n, target_fpr);

    for i in 0..n {
        filter.insert(&i.to_le_bytes());
    }

    let test_count = 100000;
    let mut false_positives = 0;

    for i in n..(n + test_count) {
        if filter.contains(&i.to_le_bytes()) {
            false_positives += 1;
        }
    }

    let actual_fpr = false_positives as f64 / test_count as f64;

    // Blocked Bloom may have slightly higher FPR due to block constraints
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
    let filter = BlockedBloomFilter::new(100, 0.01);

    assert!(!filter.contains(b"anything"));
    assert!(!filter.contains(b""));
}

// ============================================================================
// Phase 4: Merge Tests
// ============================================================================

#[test]
fn test_merge_filters() {
    let mut filter1 = BlockedBloomFilter::new(100, 0.01);
    let mut filter2 = BlockedBloomFilter::new(100, 0.01);

    filter1.insert(b"key1");
    filter1.insert(b"key2");

    filter2.insert(b"key3");
    filter2.insert(b"key4");

    filter1.merge(&filter2);

    assert!(filter1.contains(b"key1"));
    assert!(filter1.contains(b"key2"));
    assert!(filter1.contains(b"key3"));
    assert!(filter1.contains(b"key4"));
}

#[test]
fn test_merge_empty_filter() {
    let mut filter1 = BlockedBloomFilter::new(100, 0.01);
    let filter2 = BlockedBloomFilter::new(100, 0.01);

    filter1.insert(b"key1");
    filter1.merge(&filter2);

    assert!(filter1.contains(b"key1"));
}

// ============================================================================
// Phase 5: Serialization Tests
// ============================================================================

#[test]
fn test_serialize_deserialize() {
    let mut filter = BlockedBloomFilter::new(100, 0.01);
    filter.insert(b"test_key");
    filter.insert(b"another_key");

    let bytes = filter.to_bytes();
    let restored = BlockedBloomFilter::from_bytes(&bytes).unwrap();

    assert!(restored.contains(b"test_key"));
    assert!(restored.contains(b"another_key"));
    assert!(!restored.contains(b"missing_key"));
}

#[test]
fn test_serialize_empty_filter() {
    let filter = BlockedBloomFilter::new(100, 0.01);

    let bytes = filter.to_bytes();
    let restored = BlockedBloomFilter::from_bytes(&bytes).unwrap();

    assert!(restored.is_empty());
}

// ============================================================================
// Phase 6: Cache Efficiency Tests
// ============================================================================

#[test]
fn test_block_alignment() {
    // Each block is 512 bits (64 bytes = cache line)
    let filter = BlockedBloomFilter::new(1000, 0.01);

    // Verify filter was created (internal structure is cache-aligned)
    assert!(filter.is_empty());
}

#[test]
fn test_consistent_block_access() {
    let mut filter = BlockedBloomFilter::new(1000, 0.01);

    // Insert items that should hash to different blocks
    for i in 0u64..100 {
        filter.insert(&i.to_le_bytes());
    }

    // All items should be found, each requiring only 1 cache line access
    for i in 0u64..100 {
        assert!(filter.contains(&i.to_le_bytes()));
    }
}

// ============================================================================
// Phase 7: Edge Cases
// ============================================================================

#[test]
fn test_empty_key() {
    let mut filter = BlockedBloomFilter::new(100, 0.01);

    filter.insert(b"");
    assert!(filter.contains(b""));
}

#[test]
fn test_very_long_key() {
    let mut filter = BlockedBloomFilter::new(100, 0.01);

    let long_key = vec![b'x'; 10000];
    filter.insert(&long_key);

    assert!(filter.contains(&long_key));
}

#[test]
fn test_binary_data() {
    let mut filter = BlockedBloomFilter::new(100, 0.01);

    let binary_key = vec![0u8, 1, 2, 255, 0, 128];
    filter.insert(&binary_key);

    assert!(filter.contains(&binary_key));
}

#[test]
fn test_small_filter() {
    let mut filter = BlockedBloomFilter::new(1, 0.5);

    filter.insert(b"key");
    assert!(filter.contains(b"key"));
}

// ============================================================================
// Phase 8: Property-Based Tests
// ============================================================================

proptest! {
    #[test]
    fn prop_no_false_negatives(keys in prop::collection::vec(prop::collection::vec(any::<u8>(), 1..100), 1..100)) {
        let mut filter = BlockedBloomFilter::new(keys.len().max(1), 0.01);

        for key in &keys {
            filter.insert(key);
        }

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
        let mut filter1 = BlockedBloomFilter::new(size, 0.01);
        let mut filter2 = BlockedBloomFilter::new(size, 0.01);

        for key in &keys1 {
            filter1.insert(key);
        }
        for key in &keys2 {
            filter2.insert(key);
        }

        filter1.merge(&filter2);

        for key in &keys1 {
            prop_assert!(filter1.contains(key));
        }
        for key in &keys2 {
            prop_assert!(filter1.contains(key));
        }
    }

    #[test]
    fn prop_serialization_roundtrip(keys in prop::collection::vec(prop::collection::vec(any::<u8>(), 1..50), 1..50)) {
        let mut filter = BlockedBloomFilter::new(keys.len().max(1), 0.01);

        for key in &keys {
            filter.insert(key);
        }

        let bytes = filter.to_bytes();
        let restored = BlockedBloomFilter::from_bytes(&bytes).unwrap();

        for key in &keys {
            prop_assert!(restored.contains(key));
        }
    }
}

// ============================================================================
// Phase 9: Comparison Tests (Blocked vs Standard Bloom)
// ============================================================================

#[test]
fn test_blocked_bloom_advantage_cache_locality() {
    // Blocked Bloom: All k hash lookups within 1 cache line (512 bits)
    // Standard Bloom: k hash lookups can be anywhere in the bit array
    //
    // For high-throughput queries, this difference is significant
    // (measured in benchmarks, not functional tests)

    let mut filter = BlockedBloomFilter::new(10000, 0.01);

    // Insert many items
    for i in 0u64..1000 {
        filter.insert(&i.to_le_bytes());
    }

    // All lookups should complete (cache efficiency tested in benchmarks)
    for i in 0u64..1000 {
        assert!(filter.contains(&i.to_le_bytes()));
    }
}
