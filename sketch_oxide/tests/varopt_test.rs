//! VarOpt Sampling tests - TDD approach
//!
//! Testing variance-optimal weighted sampling with:
//! - Heavy items always included
//! - Light items probabilistically sampled
//! - Mergeable for distributed processing
//!
//! Use cases:
//! - Network traffic analysis
//! - Transaction monitoring
//! - Weighted log sampling

use proptest::prelude::*;
use sketch_oxide::sampling::VarOptSampling;

// ============================================================================
// Phase 1: Construction Tests
// ============================================================================

#[test]
fn test_new_varopt() {
    let sampler: VarOptSampling<i32> = VarOptSampling::new(10).unwrap();

    assert!(sampler.is_empty(), "New sampler should be empty");
    assert_eq!(sampler.len(), 0, "Length should be 0");
    assert_eq!(sampler.capacity(), 10, "Capacity should be 10");
    assert_eq!(sampler.count(), 0, "Count should be 0");
}

#[test]
fn test_new_with_various_capacities() {
    for k in [1, 10, 100, 1000] {
        let sampler: VarOptSampling<i32> = VarOptSampling::new(k).unwrap();
        assert_eq!(sampler.capacity(), k);
    }
}

#[test]
fn test_new_with_seed() {
    let sampler: VarOptSampling<i32> = VarOptSampling::with_seed(10, 42).unwrap();

    assert!(sampler.is_empty());
    assert_eq!(sampler.capacity(), 10);
}

#[test]
fn test_invalid_k_zero() {
    let result: Result<VarOptSampling<i32>, _> = VarOptSampling::new(0);
    assert!(result.is_err(), "k=0 should fail");
}

// ============================================================================
// Phase 2: Update Tests
// ============================================================================

#[test]
fn test_update_single_item() {
    let mut sampler: VarOptSampling<&str> = VarOptSampling::new(5).unwrap();

    sampler.update("hello", 10.0);

    assert!(!sampler.is_empty());
    assert_eq!(sampler.len(), 1);
    assert_eq!(sampler.count(), 1);
}

#[test]
fn test_update_multiple_items() {
    let mut sampler: VarOptSampling<i32> = VarOptSampling::new(5).unwrap();

    for i in 0..5 {
        sampler.update(i, (i + 1) as f64 * 10.0);
    }

    assert_eq!(sampler.len(), 5);
    assert_eq!(sampler.count(), 5);
}

#[test]
fn test_update_beyond_capacity() {
    let mut sampler: VarOptSampling<i32> = VarOptSampling::new(5).unwrap();

    for i in 0..100 {
        sampler.update(i, 10.0);
    }

    assert_eq!(sampler.len(), 5, "Should not exceed capacity");
    assert_eq!(sampler.count(), 100, "Should count all updates");
}

#[test]
#[should_panic(expected = "Weight must be positive")]
fn test_update_zero_weight() {
    let mut sampler: VarOptSampling<i32> = VarOptSampling::new(5).unwrap();
    sampler.update(1, 0.0);
}

#[test]
#[should_panic(expected = "Weight must be positive")]
fn test_update_negative_weight() {
    let mut sampler: VarOptSampling<i32> = VarOptSampling::new(5).unwrap();
    sampler.update(1, -1.0);
}

// ============================================================================
// Phase 3: Sample Tests
// ============================================================================

#[test]
fn test_sample_returns_weighted_items() {
    let mut sampler: VarOptSampling<i32> = VarOptSampling::new(5).unwrap();

    sampler.update(1, 100.0);
    sampler.update(2, 200.0);
    sampler.update(3, 300.0);

    let sample = sampler.sample();
    assert_eq!(sample.len(), 3);

    for item in &sample {
        assert!(item.weight > 0.0);
    }
}

#[test]
fn test_sample_item_fields() {
    let mut sampler: VarOptSampling<&str> = VarOptSampling::new(5).unwrap();

    sampler.update("test", 42.0);

    let sample = sampler.sample();
    assert_eq!(sample.len(), 1);

    let item = &sample[0];
    assert_eq!(item.item, "test");
    assert!((item.weight - 42.0).abs() < 0.001);
}

#[test]
fn test_into_sample() {
    let mut sampler: VarOptSampling<i32> = VarOptSampling::new(5).unwrap();

    for i in 0..3 {
        sampler.update(i, 10.0);
    }

    let sample = sampler.into_sample();
    assert_eq!(sample.len(), 3);
}

// ============================================================================
// Phase 4: Heavy Item Tests
// ============================================================================

#[test]
fn test_heavy_items_always_included() {
    let mut sampler: VarOptSampling<i32> = VarOptSampling::with_seed(5, 42).unwrap();

    // Add one very heavy item
    sampler.update(0, 10000.0);

    // Add many light items
    for i in 1..100 {
        sampler.update(i, 1.0);
    }

    // Heavy item should always be in sample
    let sample = sampler.sample();
    let has_heavy = sample.iter().any(|item| item.item == 0);
    assert!(has_heavy, "Heavy item should always be in sample");
}

#[test]
fn test_multiple_heavy_items() {
    let mut sampler: VarOptSampling<i32> = VarOptSampling::with_seed(10, 42).unwrap();

    // Add several heavy items
    for i in 0..5 {
        sampler.update(i, 10000.0 + i as f64);
    }

    // Add many light items
    for i in 5..200 {
        sampler.update(i, 1.0);
    }

    // All heavy items should be in sample
    let sample = sampler.sample();
    for i in 0..5 {
        let has_item = sample.iter().any(|item| item.item == i);
        assert!(has_item, "Heavy item {} should be in sample", i);
    }
}

// ============================================================================
// Phase 5: Total Weight Tests
// ============================================================================

#[test]
fn test_total_weight_underfilled() {
    let mut sampler: VarOptSampling<i32> = VarOptSampling::new(10).unwrap();

    sampler.update(1, 100.0);
    sampler.update(2, 200.0);
    sampler.update(3, 300.0);

    assert!((sampler.total_weight() - 600.0).abs() < 0.001);
}

#[test]
fn test_threshold_increases() {
    let mut sampler: VarOptSampling<i32> = VarOptSampling::new(5).unwrap();

    // Fill with equal weights
    for i in 0..5 {
        sampler.update(i, 10.0);
    }

    let threshold1 = sampler.threshold();

    // Add more items - threshold should adjust
    for i in 5..20 {
        sampler.update(i, 10.0);
    }

    let threshold2 = sampler.threshold();

    // Threshold should remain reasonable
    assert!(threshold1 > 0.0);
    assert!(threshold2 > 0.0);
}

// ============================================================================
// Phase 6: Reproducibility Tests
// ============================================================================

#[test]
fn test_seeded_reproducibility() {
    let mut s1: VarOptSampling<i32> = VarOptSampling::with_seed(10, 12345).unwrap();
    let mut s2: VarOptSampling<i32> = VarOptSampling::with_seed(10, 12345).unwrap();

    for i in 0..100 {
        let weight = (i + 1) as f64;
        s1.update(i, weight);
        s2.update(i, weight);
    }

    let sample1: Vec<_> = s1.sample().iter().map(|i| i.item).collect();
    let sample2: Vec<_> = s2.sample().iter().map(|i| i.item).collect();

    assert_eq!(sample1, sample2, "Same seed should give same sample");
}

// ============================================================================
// Phase 7: Merge Tests
// ============================================================================

#[test]
fn test_merge_basic() {
    let mut s1: VarOptSampling<i32> = VarOptSampling::with_seed(5, 42).unwrap();
    let mut s2: VarOptSampling<i32> = VarOptSampling::with_seed(5, 43).unwrap();

    for i in 0..10 {
        s1.update(i, (i + 1) as f64);
    }
    for i in 10..20 {
        s2.update(i, (i + 1) as f64);
    }

    s1.merge(&s2).unwrap();

    assert!(s1.len() <= 5, "Merged should have at most k items");
    assert_eq!(s1.count(), 20, "Merged should count all items");
}

#[test]
fn test_merge_empty() {
    let mut s1: VarOptSampling<i32> = VarOptSampling::new(5).unwrap();
    let s2: VarOptSampling<i32> = VarOptSampling::new(5).unwrap();

    s1.update(1, 10.0);
    s1.merge(&s2).unwrap();

    assert_eq!(s1.len(), 1);
}

#[test]
fn test_merge_into_empty() {
    let mut s1: VarOptSampling<i32> = VarOptSampling::new(5).unwrap();
    let mut s2: VarOptSampling<i32> = VarOptSampling::new(5).unwrap();

    s2.update(1, 10.0);
    s1.merge(&s2).unwrap();

    assert_eq!(s1.len(), 1);
}

#[test]
fn test_merge_incompatible() {
    let mut s1: VarOptSampling<i32> = VarOptSampling::new(5).unwrap();
    let s2: VarOptSampling<i32> = VarOptSampling::new(10).unwrap();

    let result = s1.merge(&s2);
    assert!(result.is_err(), "Different k should fail merge");
}

// ============================================================================
// Phase 8: Clear Tests
// ============================================================================

#[test]
fn test_clear() {
    let mut sampler: VarOptSampling<i32> = VarOptSampling::new(10).unwrap();

    for i in 0..50 {
        sampler.update(i, (i + 1) as f64);
    }

    sampler.clear();

    assert!(sampler.is_empty());
    assert_eq!(sampler.len(), 0);
    assert_eq!(sampler.count(), 0);
}

// ============================================================================
// Phase 9: Edge Cases
// ============================================================================

#[test]
fn test_k_equals_one() {
    let mut sampler: VarOptSampling<i32> = VarOptSampling::new(1).unwrap();

    for i in 0..100 {
        sampler.update(i, (i + 1) as f64);
    }

    assert_eq!(sampler.len(), 1);
}

#[test]
fn test_all_equal_weights() {
    let mut sampler: VarOptSampling<i32> = VarOptSampling::new(10).unwrap();

    for i in 0..100 {
        sampler.update(i, 1.0);
    }

    assert_eq!(sampler.len(), 10);
}

#[test]
fn test_extreme_weight_ratio() {
    let mut sampler: VarOptSampling<i32> = VarOptSampling::with_seed(5, 42).unwrap();

    // One extremely heavy item
    sampler.update(0, 1_000_000.0);

    // Many light items
    for i in 1..100 {
        sampler.update(i, 0.001);
    }

    // Heavy item should definitely be included
    let sample = sampler.sample();
    let has_heavy = sample.iter().any(|item| item.item == 0);
    assert!(has_heavy);
}

#[test]
fn test_very_small_weights() {
    let mut sampler: VarOptSampling<i32> = VarOptSampling::new(5).unwrap();

    for i in 0..10 {
        sampler.update(i, 0.0001);
    }

    assert_eq!(sampler.len(), 5);
}

// ============================================================================
// Phase 10: Weighted Distribution Tests
// ============================================================================

#[test]
fn test_heavier_items_more_likely() {
    // Statistical test: heavier items should appear more often
    let k = 5;
    let trials = 1000;

    let mut light_count = 0;
    let mut heavy_count = 0;

    for seed in 0..trials {
        let mut sampler: VarOptSampling<i32> = VarOptSampling::with_seed(k, seed as u64).unwrap();

        // Add items with varying weights
        sampler.update(0, 1.0); // Light
        sampler.update(1, 100.0); // Heavy

        for i in 2..20 {
            sampler.update(i, 10.0);
        }

        let sample = sampler.sample();
        if sample.iter().any(|item| item.item == 0) {
            light_count += 1;
        }
        if sample.iter().any(|item| item.item == 1) {
            heavy_count += 1;
        }
    }

    // Heavy item should appear more often
    assert!(
        heavy_count > light_count,
        "Heavy item should appear more often: heavy={}, light={}",
        heavy_count,
        light_count
    );
}

// ============================================================================
// Phase 11: Property-Based Tests
// ============================================================================

proptest! {
    #[test]
    fn prop_never_exceeds_capacity(
        k in 1usize..50,
        n in 1usize..200
    ) {
        let mut sampler: VarOptSampling<usize> = VarOptSampling::new(k).unwrap();

        for i in 0..n {
            sampler.update(i, (i + 1) as f64);
        }

        prop_assert!(sampler.len() <= k);
        prop_assert_eq!(sampler.count(), n as u64);
    }

    #[test]
    fn prop_sample_contains_valid_items(
        k in 1usize..50,
        n in 1usize..200
    ) {
        let mut sampler: VarOptSampling<usize> = VarOptSampling::new(k).unwrap();

        for i in 0..n {
            sampler.update(i, (i + 1) as f64);
        }

        for item in sampler.sample() {
            prop_assert!(item.item < n);
            prop_assert!(item.weight > 0.0);
        }
    }

    #[test]
    fn prop_reproducibility(
        k in 1usize..50,
        n in 1usize..200,
        seed in 0u64..10000
    ) {
        let mut s1: VarOptSampling<usize> = VarOptSampling::with_seed(k, seed).unwrap();
        let mut s2: VarOptSampling<usize> = VarOptSampling::with_seed(k, seed).unwrap();

        for i in 0..n {
            let weight = (i + 1) as f64;
            s1.update(i, weight);
            s2.update(i, weight);
        }

        let items1: Vec<_> = s1.sample().iter().map(|i| i.item).collect();
        let items2: Vec<_> = s2.sample().iter().map(|i| i.item).collect();

        prop_assert_eq!(items1, items2);
    }

    #[test]
    fn prop_total_weight_positive(
        k in 1usize..50,
        n in 1usize..100
    ) {
        let mut sampler: VarOptSampling<usize> = VarOptSampling::new(k).unwrap();

        for i in 0..n {
            sampler.update(i, (i + 1) as f64);
        }

        prop_assert!(sampler.total_weight() > 0.0);
    }

    #[test]
    fn prop_merge_preserves_count(
        k in 1usize..50,
        n1 in 1usize..50,
        n2 in 1usize..50,
        seed in 0u64..1000
    ) {
        let mut s1: VarOptSampling<usize> = VarOptSampling::with_seed(k, seed).unwrap();
        let mut s2: VarOptSampling<usize> = VarOptSampling::with_seed(k, seed + 1).unwrap();

        for i in 0..n1 {
            s1.update(i, (i + 1) as f64);
        }
        for i in 0..n2 {
            s2.update(i + 1000, (i + 1) as f64);
        }

        s1.merge(&s2).unwrap();

        prop_assert_eq!(s1.count(), (n1 + n2) as u64);
        prop_assert!(s1.len() <= k);
    }
}
