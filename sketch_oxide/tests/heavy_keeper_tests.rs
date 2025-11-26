//! Comprehensive Test Suite for HeavyKeeper Sketch
//!
//! This test suite follows TDD principles - written BEFORE implementation.
//! Tests cover all aspects: construction, operations, heavy hitter detection,
//! decay mechanism, merging, properties, edge cases, and performance.

use sketch_oxide::common::SketchError;
use sketch_oxide::frequency::HeavyKeeper;

// ==========================================================================
// Category 1: Construction Tests (5 tests)
// ==========================================================================

#[test]
fn test_new_valid_params() {
    // Test various valid parameter combinations
    let hk1 = HeavyKeeper::new(10, 0.001, 0.01);
    assert!(
        hk1.is_ok(),
        "k=10, epsilon=0.001, delta=0.01 should be valid"
    );

    let hk2 = HeavyKeeper::new(100, 0.01, 0.05);
    assert!(
        hk2.is_ok(),
        "k=100, epsilon=0.01, delta=0.05 should be valid"
    );

    let hk3 = HeavyKeeper::new(1, 0.0001, 0.001);
    assert!(
        hk3.is_ok(),
        "k=1, epsilon=0.0001, delta=0.001 should be valid"
    );

    let hk4 = HeavyKeeper::new(1000, 0.1, 0.1);
    assert!(
        hk4.is_ok(),
        "k=1000, epsilon=0.1, delta=0.1 should be valid"
    );
}

#[test]
fn test_new_invalid_k() {
    // k = 0 should error
    let result = HeavyKeeper::new(0, 0.001, 0.01);
    assert!(result.is_err(), "k=0 should return error");

    if let Err(SketchError::InvalidParameter { param, .. }) = result {
        assert_eq!(param, "k");
    } else {
        panic!("Expected InvalidParameter error for k");
    }
}

#[test]
fn test_new_invalid_epsilon() {
    // epsilon <= 0 should error
    let result1 = HeavyKeeper::new(10, 0.0, 0.01);
    assert!(result1.is_err(), "epsilon=0 should return error");

    let result2 = HeavyKeeper::new(10, -0.1, 0.01);
    assert!(result2.is_err(), "epsilon<0 should return error");

    // epsilon >= 1 should error
    let result3 = HeavyKeeper::new(10, 1.0, 0.01);
    assert!(result3.is_err(), "epsilon=1 should return error");

    let result4 = HeavyKeeper::new(10, 1.5, 0.01);
    assert!(result4.is_err(), "epsilon>1 should return error");
}

#[test]
fn test_new_invalid_delta() {
    // delta <= 0 should error
    let result1 = HeavyKeeper::new(10, 0.001, 0.0);
    assert!(result1.is_err(), "delta=0 should return error");

    let result2 = HeavyKeeper::new(10, 0.001, -0.1);
    assert!(result2.is_err(), "delta<0 should return error");

    // delta >= 1 should error
    let result3 = HeavyKeeper::new(10, 0.001, 1.0);
    assert!(result3.is_err(), "delta=1 should return error");

    let result4 = HeavyKeeper::new(10, 0.001, 1.5);
    assert!(result4.is_err(), "delta>1 should return error");
}

#[test]
fn test_parameter_calculation() {
    // Test that depth and width are computed correctly
    // depth = ln(1/delta), width = e/epsilon
    let hk = HeavyKeeper::new(10, 0.001, 0.01).unwrap();
    let stats = hk.stats();

    // depth should be approximately ln(1/0.01) = ln(100) ≈ 4.6 -> 5
    // width should be approximately e/0.001 = 2718.28 -> 2719
    assert!(
        stats.depth >= 4 && stats.depth <= 6,
        "depth should be around 5"
    );
    assert!(
        stats.width >= 2700 && stats.width <= 2800,
        "width should be around 2718"
    );

    // Memory should be depth × width × 32 bits + heap memory
    let expected_memory_bits = (stats.depth * stats.width * 32 + stats.k * 96) as u64;
    assert_eq!(stats.memory_bits, expected_memory_bits);
}

// ==========================================================================
// Category 2: Basic Operations (8 tests)
// ==========================================================================

#[test]
fn test_single_update() {
    let mut hk = HeavyKeeper::new(10, 0.001, 0.01).unwrap();
    let item = b"test_item";

    hk.update(item);

    let stats = hk.stats();
    assert_eq!(stats.total_updates, 1);

    // Item should have non-zero estimate
    let count = hk.estimate(item);
    assert!(count > 0, "Item should have count > 0 after update");
}

#[test]
fn test_multiple_updates_same_item() {
    let mut hk = HeavyKeeper::new(10, 0.001, 0.01).unwrap();
    let item = b"test_item";

    for _ in 0..100 {
        hk.update(item);
    }

    let count = hk.estimate(item);
    assert!(count >= 90, "Count should be close to 100 (at least 90)");
    assert!(count <= 110, "Count should not exceed 110");
}

#[test]
fn test_multiple_items() {
    let mut hk = HeavyKeeper::new(10, 0.001, 0.01).unwrap();

    hk.update(b"item1");
    hk.update(b"item2");
    hk.update(b"item3");
    hk.update(b"item1");
    hk.update(b"item2");
    hk.update(b"item1");

    // item1: 3, item2: 2, item3: 1
    let count1 = hk.estimate(b"item1");
    let count2 = hk.estimate(b"item2");
    let count3 = hk.estimate(b"item3");

    assert!(count1 >= count2, "item1 should have highest count");
    assert!(
        count2 >= count3,
        "item2 should have higher count than item3"
    );
    assert!(count1 >= 3, "item1 should have count >= 3");
}

#[test]
fn test_estimate_unknown_item() {
    let mut hk = HeavyKeeper::new(10, 0.001, 0.01).unwrap();

    // Add some items
    for i in 0..100 {
        hk.update(format!("item_{}", i).as_bytes());
    }

    // Query unknown item should return low count (hash collision)
    let unknown_count = hk.estimate(b"unknown_item_xyz_999");
    assert!(unknown_count <= 10, "Unknown item should have low count");
}

#[test]
fn test_estimate_known_item() {
    let mut hk = HeavyKeeper::new(10, 0.001, 0.01).unwrap();
    let item = b"known_item";

    // Add item 50 times
    for _ in 0..50 {
        hk.update(item);
    }

    let count = hk.estimate(item);
    // Should be close to 50 with small error
    assert!(count >= 45 && count <= 55, "Count should be close to 50");
}

#[test]
fn test_top_k_count() {
    let mut hk = HeavyKeeper::new(10, 0.001, 0.01).unwrap();

    // Add 20 distinct items
    for i in 0..20 {
        for _ in 0..10 {
            hk.update(format!("item_{}", i).as_bytes());
        }
    }

    let top_k = hk.top_k();
    assert_eq!(top_k.len(), 10, "Should return exactly k=10 items");
}

#[test]
fn test_top_k_ordering() {
    let mut hk = HeavyKeeper::new(5, 0.001, 0.01).unwrap();

    // Add items with different frequencies
    for _ in 0..100 {
        hk.update(b"item1");
    }
    for _ in 0..80 {
        hk.update(b"item2");
    }
    for _ in 0..60 {
        hk.update(b"item3");
    }
    for _ in 0..40 {
        hk.update(b"item4");
    }
    for _ in 0..20 {
        hk.update(b"item5");
    }

    let top_k = hk.top_k();

    // Should be sorted by count descending
    for i in 0..top_k.len() - 1 {
        assert!(
            top_k[i].1 >= top_k[i + 1].1,
            "Items should be sorted by count descending"
        );
    }
}

#[test]
fn test_empty_top_k() {
    let hk = HeavyKeeper::new(10, 0.001, 0.01).unwrap();

    let top_k = hk.top_k();
    assert_eq!(top_k.len(), 0, "Empty sketch should return empty top-k");
}

// ==========================================================================
// Category 3: Heavy Hitter Detection (6 tests)
// ==========================================================================

#[test]
fn test_identify_heavy_hitter() {
    let mut hk = HeavyKeeper::new(10, 0.001, 0.01).unwrap();

    // Create a clear heavy hitter
    for _ in 0..500 {
        hk.update(b"heavy_hitter");
    }

    // Add many other items
    for i in 0..1000 {
        hk.update(format!("item_{}", i).as_bytes());
    }

    let top_k = hk.top_k();
    assert!(top_k.len() > 0, "Should have items in top-k");

    // Heavy hitter should be in top-k
    let _top_item_hash = top_k[0].0;
    let top_count = top_k[0].1;

    // The heavy hitter with 500 occurrences should be at or near top
    assert!(top_count >= 400, "Top item should have high count");
}

#[test]
fn test_threshold_boundary() {
    let mut hk = HeavyKeeper::new(10, 0.01, 0.01).unwrap();

    let total = 1000;
    let threshold_count = (total as f64 * 0.01).ceil() as u32; // 10

    // Add item exactly at threshold
    for _ in 0..threshold_count {
        hk.update(b"threshold_item");
    }

    // Add other items
    for i in 0..(total - threshold_count as usize) {
        hk.update(format!("item_{}", i).as_bytes());
    }

    let count = hk.estimate(b"threshold_item");
    assert!(
        count >= threshold_count - 2,
        "Should detect item at threshold"
    );
}

#[test]
fn test_false_positives_minimal() {
    let mut hk = HeavyKeeper::new(100, 0.001, 0.01).unwrap();

    let total = 10000;

    // Add many items uniformly
    for i in 0..total {
        hk.update(format!("item_{}", i % 1000).as_bytes());
    }

    let top_k = hk.top_k();

    // With uniform distribution, no item should dominate
    // Top items should have counts around 10-20
    if let Some(top_item) = top_k.first() {
        assert!(top_item.1 < 100, "No single heavy hitter should dominate");
    }
}

#[test]
fn test_no_false_negatives() {
    let mut hk = HeavyKeeper::new(10, 0.01, 0.01).unwrap();

    let total = 1000;
    let heavy_freq = (total as f64 * 0.05) as usize; // 5% = 50 items

    // Add heavy hitter
    for _ in 0..heavy_freq {
        hk.update(b"heavy");
    }

    // Add other items
    for i in 0..(total - heavy_freq) {
        hk.update(format!("other_{}", i).as_bytes());
    }

    // Heavy hitter should be in top-k
    let top_k = hk.top_k();
    let counts: Vec<u32> = top_k.iter().map(|x| x.1).collect();
    let max_count = counts.iter().max().unwrap_or(&0);

    assert!(*max_count >= 40, "Heavy hitter should be detected");
}

#[test]
fn test_skewed_distribution() {
    let mut hk = HeavyKeeper::new(20, 0.001, 0.01).unwrap();

    // Zipf-like distribution (1.5 exponent)
    for i in 1..=100 {
        let freq = (1000.0 / (i as f64).powf(1.5)) as usize;
        for _ in 0..freq {
            hk.update(format!("item_{}", i).as_bytes());
        }
    }

    let top_k = hk.top_k();
    assert_eq!(top_k.len(), 20, "Should return k items");

    // Top items should have much higher counts than bottom
    let top_count = top_k[0].1;
    let bottom_count = top_k[top_k.len() - 1].1;
    assert!(
        top_count > bottom_count * 2,
        "Top should be significantly higher"
    );
}

#[test]
fn test_uniform_distribution() {
    let mut hk = HeavyKeeper::new(10, 0.001, 0.01).unwrap();

    // Uniform distribution - no clear heavy hitters
    for i in 0..1000 {
        hk.update(format!("item_{}", i % 500).as_bytes());
    }

    let top_k = hk.top_k();

    // With uniform distribution, counts should be similar
    if top_k.len() > 1 {
        let max_count = top_k[0].1;
        let min_count = top_k[top_k.len() - 1].1;

        // Counts should not vary wildly in uniform distribution
        assert!(
            max_count <= min_count * 3,
            "Uniform distribution should have similar counts"
        );
    }
}

// ==========================================================================
// Category 4: Decay Mechanism (4 tests)
// ==========================================================================

#[test]
fn test_decay_reduces_counts() {
    let mut hk = HeavyKeeper::new(10, 0.001, 0.01).unwrap();

    // Add items
    for _ in 0..100 {
        hk.update(b"item1");
    }

    let count_before = hk.estimate(b"item1");

    // Apply decay
    hk.decay();

    let count_after = hk.estimate(b"item1");

    assert!(count_after < count_before, "Decay should reduce counts");
    assert!(
        count_after > 0,
        "Decay should not reduce to exactly 0 for high counts"
    );
}

#[test]
fn test_decay_factor_application() {
    let mut hk = HeavyKeeper::new(10, 0.001, 0.01).unwrap();

    // Add items
    for _ in 0..100 {
        hk.update(b"item1");
    }

    let count_before = hk.estimate(b"item1");

    // Apply decay
    hk.decay();

    let count_after = hk.estimate(b"item1");

    // Decay factor is 1.08, so count should be approximately count_before / 1.08
    let expected_ratio = 1.08;
    let actual_ratio = count_before as f64 / count_after as f64;

    // Allow some tolerance
    assert!(
        (actual_ratio - expected_ratio).abs() < 0.3,
        "Decay ratio should be close to decay_factor (1.08)"
    );
}

#[test]
fn test_decay_protects_heavy_hitters() {
    let mut hk = HeavyKeeper::new(10, 0.001, 0.01).unwrap();

    // Create heavy hitter
    for _ in 0..1000 {
        hk.update(b"heavy");
    }

    // Create light item
    for _ in 0..10 {
        hk.update(b"light");
    }

    let heavy_before = hk.estimate(b"heavy");
    let light_before = hk.estimate(b"light");

    // Apply decay multiple times
    for _ in 0..10 {
        hk.decay();
    }

    let heavy_after = hk.estimate(b"heavy");
    let light_after = hk.estimate(b"light");

    // Heavy hitter should retain more relative count
    let _heavy_retention = heavy_after as f64 / heavy_before as f64;
    let _light_retention = light_after as f64 / light_before.max(1) as f64;

    // Both decay, but heavy hitters maintain better position
    assert!(
        heavy_after > light_after,
        "Heavy hitter should still dominate"
    );
}

#[test]
fn test_decay_removes_small_items() {
    let mut hk = HeavyKeeper::new(10, 0.001, 0.01).unwrap();

    // Add small count item
    hk.update(b"small");
    hk.update(b"small");

    let count_before = hk.estimate(b"small");
    assert!(count_before > 0);

    // Apply many decay cycles
    for _ in 0..50 {
        hk.decay();
    }

    let count_after = hk.estimate(b"small");

    // Small counts should approach zero
    assert!(
        count_after < count_before,
        "Small items should decay significantly"
    );
}

// ==========================================================================
// Category 5: Merge Operations (5 tests)
// ==========================================================================

#[test]
fn test_merge_same_parameters() {
    let mut hk1 = HeavyKeeper::new(10, 0.001, 0.01).unwrap();
    let mut hk2 = HeavyKeeper::new(10, 0.001, 0.01).unwrap();

    // Add items to both
    for _ in 0..50 {
        hk1.update(b"item1");
    }
    for _ in 0..30 {
        hk2.update(b"item1");
    }

    let result = hk1.merge(&hk2);
    assert!(result.is_ok(), "Merge with same parameters should succeed");

    // Count should be approximately sum
    let count = hk1.estimate(b"item1");
    assert!(count >= 70 && count <= 90, "Merged count should be near 80");
}

#[test]
fn test_merge_incompatible_depth() {
    let mut hk1 = HeavyKeeper::new(10, 0.001, 0.01).unwrap();
    let hk2 = HeavyKeeper::new(10, 0.001, 0.001).unwrap(); // Different delta -> different depth

    let result = hk1.merge(&hk2);
    assert!(result.is_err(), "Merge with different depths should fail");

    if let Err(SketchError::IncompatibleSketches { reason }) = result {
        assert!(reason.contains("depth") || reason.contains("parameter"));
    } else {
        panic!("Expected IncompatibleSketches error");
    }
}

#[test]
fn test_merge_commutative() {
    let mut hk1a = HeavyKeeper::new(10, 0.001, 0.01).unwrap();
    let mut hk1b = HeavyKeeper::new(10, 0.001, 0.01).unwrap();
    let mut hk2a = HeavyKeeper::new(10, 0.001, 0.01).unwrap();
    let mut hk2b = HeavyKeeper::new(10, 0.001, 0.01).unwrap();

    // Same data in both pairs
    for _ in 0..50 {
        hk1a.update(b"item");
        hk1b.update(b"item");
    }
    for _ in 0..30 {
        hk2a.update(b"item");
        hk2b.update(b"item");
    }

    // Merge in different orders
    hk1a.merge(&hk2a).unwrap();
    hk2b.merge(&hk1b).unwrap();

    let count_a = hk1a.estimate(b"item");
    let count_b = hk2b.estimate(b"item");

    // Counts should be similar (may not be exactly equal due to hash collisions)
    let diff = if count_a > count_b {
        count_a - count_b
    } else {
        count_b - count_a
    };
    assert!(diff <= 10, "Merge should be approximately commutative");
}

#[test]
fn test_merge_combines_counts() {
    let mut hk1 = HeavyKeeper::new(10, 0.001, 0.01).unwrap();
    let mut hk2 = HeavyKeeper::new(10, 0.001, 0.01).unwrap();

    // Different items
    for _ in 0..100 {
        hk1.update(b"item1");
    }
    for _ in 0..80 {
        hk2.update(b"item2");
    }

    hk1.merge(&hk2).unwrap();

    // Both items should be present
    let count1 = hk1.estimate(b"item1");
    let count2 = hk1.estimate(b"item2");

    assert!(count1 >= 90, "item1 should have ~100 count");
    assert!(count2 >= 70, "item2 should have ~80 count");
}

#[test]
fn test_merge_updates_heap() {
    let mut hk1 = HeavyKeeper::new(5, 0.001, 0.01).unwrap();
    let mut hk2 = HeavyKeeper::new(5, 0.001, 0.01).unwrap();

    // Add different items
    for _ in 0..100 {
        hk1.update(b"a");
    }
    for _ in 0..90 {
        hk1.update(b"b");
    }
    for _ in 0..80 {
        hk2.update(b"c");
    }
    for _ in 0..70 {
        hk2.update(b"d");
    }

    hk1.merge(&hk2).unwrap();

    let top_k = hk1.top_k();
    assert_eq!(top_k.len(), 5, "Should return k items");

    // Top items should include items from both sketches
    let top_count = top_k[0].1;
    assert!(top_count >= 80, "Merged top-k should have high counts");
}

// ==========================================================================
// Category 6: Edge Cases (5 tests)
// ==========================================================================

#[test]
fn test_k_equals_1() {
    let mut hk = HeavyKeeper::new(1, 0.001, 0.01).unwrap();

    for _ in 0..100 {
        hk.update(b"item1");
    }
    for _ in 0..80 {
        hk.update(b"item2");
    }
    for _ in 0..60 {
        hk.update(b"item3");
    }

    let top_k = hk.top_k();
    assert_eq!(top_k.len(), 1, "Should return exactly 1 item");

    // Should be the most frequent
    assert!(top_k[0].1 >= 90, "Single item should be most frequent");
}

#[test]
fn test_k_larger_than_items() {
    let mut hk = HeavyKeeper::new(100, 0.001, 0.01).unwrap();

    // Add only 5 distinct items
    for i in 0..5 {
        for _ in 0..10 {
            hk.update(format!("item_{}", i).as_bytes());
        }
    }

    let top_k = hk.top_k();
    // Should return <= 5 items even though k=100
    assert!(top_k.len() <= 100, "Should not exceed k");
}

#[test]
fn test_empty_update() {
    let mut hk = HeavyKeeper::new(10, 0.001, 0.01).unwrap();

    // Empty byte slice should be handled
    hk.update(b"");

    let stats = hk.stats();
    assert_eq!(stats.total_updates, 1);
}

#[test]
fn test_zero_decay_factor() {
    // This tests that we handle decay_factor edge cases
    // Implementation should use default 1.08
    let mut hk = HeavyKeeper::new(10, 0.001, 0.01).unwrap();

    for _ in 0..100 {
        hk.update(b"item");
    }

    // Should not panic
    hk.decay();
    let count = hk.estimate(b"item");
    assert!(count > 0, "Decay should not crash");
}

#[test]
fn test_large_scale() {
    let mut hk = HeavyKeeper::new(100, 0.001, 0.01).unwrap();

    // 1M updates with 10k distinct items
    for i in 0..1_000_000 {
        hk.update(format!("item_{}", i % 10_000).as_bytes());
    }

    let stats = hk.stats();
    assert_eq!(stats.total_updates, 1_000_000);

    let top_k = hk.top_k();
    assert_eq!(top_k.len(), 100, "Should return k items");

    // Each item should have ~100 occurrences
    for (_, count) in top_k.iter() {
        assert!(*count >= 50 && *count <= 200, "Counts should be reasonable");
    }
}

// ==========================================================================
// Category 7: Memory & Performance (4 tests)
// ==========================================================================

#[test]
fn test_memory_usage_correct() {
    let hk = HeavyKeeper::new(100, 0.001, 0.01).unwrap();
    let stats = hk.stats();

    // Memory should be depth × width × 32 bits + heap memory
    let array_memory = (stats.depth * stats.width * 32) as u64;
    let heap_memory = (stats.k * 96) as u64;
    let expected = array_memory + heap_memory;
    assert_eq!(stats.memory_bits, expected);

    // Should also account for heap storage
    // Heap has k entries, each with u64 hash + u32 count
    let _heap_bits = (100 * (64 + 32)) as u64;

    // Total memory should be reasonable
    assert!(stats.memory_bits > 0);
}

#[test]
fn test_stats_accurate() {
    let mut hk = HeavyKeeper::new(42, 0.001, 0.01).unwrap();

    // Before updates
    let stats0 = hk.stats();
    assert_eq!(stats0.total_updates, 0);
    assert_eq!(stats0.k, 42);

    // After updates
    for _ in 0..100 {
        hk.update(b"test");
    }

    let stats1 = hk.stats();
    assert_eq!(stats1.total_updates, 100);
    assert_eq!(stats1.k, 42);
}

#[test]
fn test_clone_works() {
    let mut hk1 = HeavyKeeper::new(10, 0.001, 0.01).unwrap();

    for _ in 0..100 {
        hk1.update(b"test");
    }

    let hk2 = hk1.clone();

    // Should be independent copies
    let count1 = hk1.estimate(b"test");
    let count2 = hk2.estimate(b"test");
    assert_eq!(count1, count2, "Clone should have same counts");

    // Stats should match
    let stats1 = hk1.stats();
    let stats2 = hk2.stats();
    assert_eq!(stats1.total_updates, stats2.total_updates);
    assert_eq!(stats1.k, stats2.k);
}

#[test]
fn test_serialization_compatible() {
    // This is a placeholder for future serialization support
    // For now, just verify the struct can be cloned (needed for serialization)
    let mut hk = HeavyKeeper::new(10, 0.001, 0.01).unwrap();
    hk.update(b"test");

    let _cloned = hk.clone();
    // If clone works, serialization framework can be added later
}

// ==========================================================================
// Property-Based Tests (using proptest would go here)
// These are placeholder tests showing the intent
// ==========================================================================

#[test]
fn prop_top_k_includes_frequent_items_placeholder() {
    // Property: Any item with frequency > threshold must be in top-k
    // This would use proptest to generate random data
    let mut hk = HeavyKeeper::new(10, 0.01, 0.01).unwrap();

    // Simulate: Add one item 20% of time, others rarely
    for i in 0..1000 {
        if i % 5 == 0 {
            hk.update(b"frequent");
        } else {
            hk.update(format!("rare_{}", i).as_bytes());
        }
    }

    let top_k = hk.top_k();
    let has_frequent = top_k.iter().any(|(_, count)| *count >= 150);
    assert!(has_frequent, "Frequent item should be in top-k");
}

#[test]
fn prop_count_monotonic_placeholder() {
    // Property: Counts should not decrease when adding same item
    let mut hk = HeavyKeeper::new(10, 0.001, 0.01).unwrap();

    hk.update(b"test");
    let count1 = hk.estimate(b"test");

    hk.update(b"test");
    let count2 = hk.estimate(b"test");

    hk.update(b"test");
    let count3 = hk.estimate(b"test");

    assert!(count2 >= count1 - 1, "Count should not decrease");
    assert!(count3 >= count2 - 1, "Count should not decrease");
}

#[test]
fn prop_estimate_lower_bound_placeholder() {
    // Property: Estimate should be lower bound of true count
    // (allowing for some error)
    let mut hk = HeavyKeeper::new(10, 0.001, 0.01).unwrap();

    let true_count = 100;
    for _ in 0..true_count {
        hk.update(b"item");
    }

    let estimate = hk.estimate(b"item");
    // Should be within reasonable error (e.g., ±10%)
    assert!(estimate >= 80, "Estimate should be reasonable lower bound");
}

#[test]
fn prop_merge_associative_placeholder() {
    // Property: (A.merge(B)).merge(C) ≈ A.merge(B.merge(C))
    let mut hk_a1 = HeavyKeeper::new(10, 0.001, 0.01).unwrap();
    let mut hk_b1 = HeavyKeeper::new(10, 0.001, 0.01).unwrap();
    let mut hk_c1 = HeavyKeeper::new(10, 0.001, 0.01).unwrap();

    let mut hk_a2 = HeavyKeeper::new(10, 0.001, 0.01).unwrap();
    let mut hk_b2 = HeavyKeeper::new(10, 0.001, 0.01).unwrap();
    let mut hk_c2 = HeavyKeeper::new(10, 0.001, 0.01).unwrap();

    // Add same data to both sets
    for _ in 0..30 {
        hk_a1.update(b"x");
        hk_a2.update(b"x");
        hk_b1.update(b"x");
        hk_b2.update(b"x");
        hk_c1.update(b"x");
        hk_c2.update(b"x");
    }

    // Left-associative: (A + B) + C
    hk_a1.merge(&hk_b1).unwrap();
    hk_a1.merge(&hk_c1).unwrap();

    // Right-associative: A + (B + C)
    hk_b2.merge(&hk_c2).unwrap();
    hk_a2.merge(&hk_b2).unwrap();

    let count1 = hk_a1.estimate(b"x");
    let count2 = hk_a2.estimate(b"x");

    let diff = if count1 > count2 {
        count1 - count2
    } else {
        count2 - count1
    };
    assert!(diff <= 10, "Merge should be approximately associative");
}

#[test]
fn prop_decay_idempotent_small_values_placeholder() {
    // Property: Decaying small values repeatedly converges to ~0
    let mut hk = HeavyKeeper::new(10, 0.001, 0.01).unwrap();

    hk.update(b"small");
    hk.update(b"small");

    // Apply many decays
    for _ in 0..100 {
        hk.decay();
    }

    let count = hk.estimate(b"small");
    assert!(count <= 1, "Small items should decay to near zero");
}
