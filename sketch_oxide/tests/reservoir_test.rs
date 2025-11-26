//! Reservoir Sampling tests - TDD approach
//!
//! Testing uniform random sampling from streams with:
//! - Algorithm R (Vitter 1985)
//! - O(1) update time
//! - Uniform inclusion probability k/n
//!
//! Use cases:
//! - Log sampling
//! - A/B testing
//! - Database query sampling

use proptest::prelude::*;
use sketch_oxide::sampling::ReservoirSampling;

// ============================================================================
// Phase 1: Construction Tests
// ============================================================================

#[test]
fn test_new_reservoir() {
    let reservoir: ReservoirSampling<i32> = ReservoirSampling::new(10).unwrap();

    assert!(reservoir.is_empty(), "New reservoir should be empty");
    assert_eq!(reservoir.len(), 0, "Length should be 0");
    assert_eq!(reservoir.capacity(), 10, "Capacity should be 10");
    assert_eq!(reservoir.count(), 0, "Count should be 0");
}

#[test]
fn test_new_with_various_capacities() {
    for k in [1, 10, 100, 1000] {
        let reservoir: ReservoirSampling<i32> = ReservoirSampling::new(k).unwrap();
        assert_eq!(reservoir.capacity(), k);
    }
}

#[test]
fn test_new_with_seed() {
    let reservoir: ReservoirSampling<i32> = ReservoirSampling::with_seed(10, 42).unwrap();

    assert!(reservoir.is_empty());
    assert_eq!(reservoir.capacity(), 10);
}

#[test]
fn test_invalid_k_zero() {
    let result: Result<ReservoirSampling<i32>, _> = ReservoirSampling::new(0);
    assert!(result.is_err(), "k=0 should fail");
}

// ============================================================================
// Phase 2: Update Tests
// ============================================================================

#[test]
fn test_update_single_item() {
    let mut reservoir: ReservoirSampling<&str> = ReservoirSampling::new(5).unwrap();

    reservoir.update("hello");

    assert!(!reservoir.is_empty());
    assert_eq!(reservoir.len(), 1);
    assert_eq!(reservoir.count(), 1);
}

#[test]
fn test_update_fills_reservoir() {
    let mut reservoir: ReservoirSampling<i32> = ReservoirSampling::new(5).unwrap();

    for i in 0..5 {
        reservoir.update(i);
    }

    assert_eq!(reservoir.len(), 5);
    assert_eq!(reservoir.count(), 5);
}

#[test]
fn test_update_beyond_capacity() {
    let mut reservoir: ReservoirSampling<i32> = ReservoirSampling::new(5).unwrap();

    for i in 0..100 {
        reservoir.update(i);
    }

    assert_eq!(reservoir.len(), 5, "Should not exceed capacity");
    assert_eq!(reservoir.count(), 100, "Should count all updates");
}

#[test]
fn test_update_various_types() {
    // Strings
    let mut r1: ReservoirSampling<String> = ReservoirSampling::new(5).unwrap();
    r1.update("hello".to_string());
    assert_eq!(r1.len(), 1);

    // Integers
    let mut r2: ReservoirSampling<i64> = ReservoirSampling::new(5).unwrap();
    r2.update(42);
    assert_eq!(r2.len(), 1);

    // Tuples
    let mut r3: ReservoirSampling<(i32, String)> = ReservoirSampling::new(5).unwrap();
    r3.update((1, "test".to_string()));
    assert_eq!(r3.len(), 1);
}

// ============================================================================
// Phase 3: Sample Tests
// ============================================================================

#[test]
fn test_sample_returns_items() {
    let mut reservoir: ReservoirSampling<i32> = ReservoirSampling::new(5).unwrap();

    for i in 0..10 {
        reservoir.update(i);
    }

    let sample = reservoir.sample();
    assert_eq!(sample.len(), 5);
}

#[test]
fn test_sample_contains_actual_items() {
    let mut reservoir: ReservoirSampling<i32> = ReservoirSampling::with_seed(3, 42).unwrap();

    for i in 0..100 {
        reservoir.update(i);
    }

    let sample = reservoir.sample();
    for item in sample {
        assert!(
            *item >= 0 && *item < 100,
            "Sample should contain valid items"
        );
    }
}

#[test]
fn test_into_sample() {
    let mut reservoir: ReservoirSampling<i32> = ReservoirSampling::new(5).unwrap();

    for i in 0..10 {
        reservoir.update(i);
    }

    let sample = reservoir.into_sample();
    assert_eq!(sample.len(), 5);
}

// ============================================================================
// Phase 4: Inclusion Probability Tests
// ============================================================================

#[test]
fn test_inclusion_probability_empty() {
    let reservoir: ReservoirSampling<i32> = ReservoirSampling::new(10).unwrap();
    assert_eq!(reservoir.inclusion_probability(), 0.0);
}

#[test]
fn test_inclusion_probability_underfilled() {
    let mut reservoir: ReservoirSampling<i32> = ReservoirSampling::new(10).unwrap();

    for i in 0..5 {
        reservoir.update(i);
    }

    // All items are in sample when count < k
    assert!((reservoir.inclusion_probability() - 1.0).abs() < 0.001);
}

#[test]
fn test_inclusion_probability_overfilled() {
    let mut reservoir: ReservoirSampling<i32> = ReservoirSampling::new(10).unwrap();

    for i in 0..100 {
        reservoir.update(i);
    }

    // k/n = 10/100 = 0.1
    assert!((reservoir.inclusion_probability() - 0.1).abs() < 0.001);
}

// ============================================================================
// Phase 5: Reproducibility Tests
// ============================================================================

#[test]
fn test_seeded_reproducibility() {
    let mut r1: ReservoirSampling<i32> = ReservoirSampling::with_seed(10, 12345).unwrap();
    let mut r2: ReservoirSampling<i32> = ReservoirSampling::with_seed(10, 12345).unwrap();

    for i in 0..1000 {
        r1.update(i);
        r2.update(i);
    }

    assert_eq!(
        r1.sample(),
        r2.sample(),
        "Same seed should give same sample"
    );
}

#[test]
fn test_different_seeds_different_samples() {
    let mut r1: ReservoirSampling<i32> = ReservoirSampling::with_seed(10, 111).unwrap();
    let mut r2: ReservoirSampling<i32> = ReservoirSampling::with_seed(10, 222).unwrap();

    for i in 0..1000 {
        r1.update(i);
        r2.update(i);
    }

    // Different seeds should (almost certainly) give different samples
    // This is probabilistic but extremely likely
    assert_ne!(
        r1.sample(),
        r2.sample(),
        "Different seeds should likely give different samples"
    );
}

// ============================================================================
// Phase 6: Merge Tests
// ============================================================================

#[test]
fn test_merge_basic() {
    let mut r1: ReservoirSampling<i32> = ReservoirSampling::with_seed(5, 42).unwrap();
    let mut r2: ReservoirSampling<i32> = ReservoirSampling::with_seed(5, 43).unwrap();

    for i in 0..10 {
        r1.update(i);
    }
    for i in 10..20 {
        r2.update(i);
    }

    r1.merge(&r2).unwrap();

    assert_eq!(r1.len(), 5, "Merged should have k items");
    assert_eq!(r1.count(), 20, "Merged should count all items");
}

#[test]
fn test_merge_empty() {
    let mut r1: ReservoirSampling<i32> = ReservoirSampling::new(5).unwrap();
    let r2: ReservoirSampling<i32> = ReservoirSampling::new(5).unwrap();

    r1.update(1);
    r1.update(2);

    r1.merge(&r2).unwrap();

    assert_eq!(r1.len(), 2);
}

#[test]
fn test_merge_into_empty() {
    let mut r1: ReservoirSampling<i32> = ReservoirSampling::new(5).unwrap();
    let mut r2: ReservoirSampling<i32> = ReservoirSampling::new(5).unwrap();

    r2.update(1);
    r2.update(2);

    r1.merge(&r2).unwrap();

    assert_eq!(r1.len(), 2);
}

#[test]
fn test_merge_incompatible() {
    let mut r1: ReservoirSampling<i32> = ReservoirSampling::new(5).unwrap();
    let r2: ReservoirSampling<i32> = ReservoirSampling::new(10).unwrap();

    let result = r1.merge(&r2);
    assert!(result.is_err(), "Different k should fail merge");
}

// ============================================================================
// Phase 7: Clear Tests
// ============================================================================

#[test]
fn test_clear() {
    let mut reservoir: ReservoirSampling<i32> = ReservoirSampling::new(10).unwrap();

    for i in 0..50 {
        reservoir.update(i);
    }

    reservoir.clear();

    assert!(reservoir.is_empty());
    assert_eq!(reservoir.len(), 0);
    assert_eq!(reservoir.count(), 0);
}

// ============================================================================
// Phase 8: Edge Cases
// ============================================================================

#[test]
fn test_k_equals_one() {
    let mut reservoir: ReservoirSampling<i32> = ReservoirSampling::new(1).unwrap();

    for i in 0..100 {
        reservoir.update(i);
    }

    assert_eq!(reservoir.len(), 1);
    let sample = reservoir.sample();
    assert!(sample[0] >= 0 && sample[0] < 100);
}

#[test]
fn test_large_k() {
    let mut reservoir: ReservoirSampling<i32> = ReservoirSampling::new(10000).unwrap();

    for i in 0..5000 {
        reservoir.update(i);
    }

    // k > n: all items should be in reservoir
    assert_eq!(reservoir.len(), 5000);
}

#[test]
fn test_exact_k_items() {
    let mut reservoir: ReservoirSampling<i32> = ReservoirSampling::new(10).unwrap();

    for i in 0..10 {
        reservoir.update(i);
    }

    assert_eq!(reservoir.len(), 10);
    assert_eq!(reservoir.count(), 10);
    assert!((reservoir.inclusion_probability() - 1.0).abs() < 0.001);
}

// ============================================================================
// Phase 9: Statistical Property Tests
// ============================================================================

#[test]
fn test_uniform_distribution_approximate() {
    // Run many trials and check that each item appears roughly equally often
    let k = 10;
    let n = 100;
    let trials = 10000;

    let mut counts = vec![0u32; n];

    for seed in 0..trials {
        let mut reservoir: ReservoirSampling<usize> =
            ReservoirSampling::with_seed(k, seed as u64).unwrap();

        for i in 0..n {
            reservoir.update(i);
        }

        for &item in reservoir.sample() {
            counts[item] += 1;
        }
    }

    // Expected count per item: trials * k / n = 10000 * 10 / 100 = 1000
    let expected = (trials * k / n) as f64;

    // Check that all counts are within reasonable range (±30%)
    for (i, &count) in counts.iter().enumerate() {
        let ratio = count as f64 / expected;
        assert!(
            ratio > 0.7 && ratio < 1.3,
            "Item {} has count {} (expected ~{}), ratio {}",
            i,
            count,
            expected,
            ratio
        );
    }
}

// ============================================================================
// Phase 10: Property-Based Tests
// ============================================================================

proptest! {
    #[test]
    fn prop_never_exceeds_capacity(
        k in 1usize..100,
        n in 1usize..1000
    ) {
        let mut reservoir: ReservoirSampling<usize> = ReservoirSampling::new(k).unwrap();

        for i in 0..n {
            reservoir.update(i);
        }

        prop_assert!(reservoir.len() <= k);
        prop_assert_eq!(reservoir.count(), n as u64);
    }

    #[test]
    fn prop_sample_contains_valid_items(
        k in 1usize..50,
        n in 1usize..500
    ) {
        let mut reservoir: ReservoirSampling<usize> = ReservoirSampling::new(k).unwrap();

        for i in 0..n {
            reservoir.update(i);
        }

        for &item in reservoir.sample() {
            prop_assert!(item < n);
        }
    }

    #[test]
    fn prop_reproducibility(
        k in 1usize..50,
        n in 1usize..500,
        seed in 0u64..10000
    ) {
        let mut r1: ReservoirSampling<usize> = ReservoirSampling::with_seed(k, seed).unwrap();
        let mut r2: ReservoirSampling<usize> = ReservoirSampling::with_seed(k, seed).unwrap();

        for i in 0..n {
            r1.update(i);
            r2.update(i);
        }

        prop_assert_eq!(r1.sample(), r2.sample());
    }

    #[test]
    fn prop_inclusion_probability_correct(
        k in 1usize..100,
        n in 1usize..1000
    ) {
        let mut reservoir: ReservoirSampling<usize> = ReservoirSampling::new(k).unwrap();

        for i in 0..n {
            reservoir.update(i);
        }

        let expected_prob = k.min(n) as f64 / n as f64;
        prop_assert!((reservoir.inclusion_probability() - expected_prob).abs() < 0.001);
    }

    #[test]
    fn prop_merge_preserves_count(
        k in 1usize..50,
        n1 in 1usize..100,
        n2 in 1usize..100,
        seed in 0u64..1000
    ) {
        let mut r1: ReservoirSampling<usize> = ReservoirSampling::with_seed(k, seed).unwrap();
        let mut r2: ReservoirSampling<usize> = ReservoirSampling::with_seed(k, seed + 1).unwrap();

        for i in 0..n1 {
            r1.update(i);
        }
        for i in 0..n2 {
            r2.update(i + 1000); // Different items
        }

        r1.merge(&r2).unwrap();

        prop_assert_eq!(r1.count(), (n1 + n2) as u64);
        prop_assert!(r1.len() <= k);
    }
}
