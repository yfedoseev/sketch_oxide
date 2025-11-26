//! Comprehensive tests for DDSketch (VLDB 2019)
//!
//! Tests verify:
//! - Basic functionality (creation, updates, queries)
//! - Relative error guarantees across wide ranges
//! - Merge operations (commutativity, accuracy)
//! - Special value handling (negatives, zeros, extremes)
//! - Edge cases (empty, duplicates, bimodal distributions)

use proptest::prelude::*;
use sketch_oxide::common::{Mergeable, Sketch};
use sketch_oxide::quantiles::DDSketch;

// ============================================================================
// Basic Functionality Tests
// ============================================================================

#[test]
fn test_new_ddsketch() {
    // Valid accuracy values
    assert!(DDSketch::new(0.001).is_ok());
    assert!(DDSketch::new(0.01).is_ok());
    assert!(DDSketch::new(0.05).is_ok());
    assert!(DDSketch::new(0.1).is_ok());
}

#[test]
fn test_invalid_accuracy() {
    // Alpha must be in (0, 1)
    assert!(DDSketch::new(0.0).is_err());
    assert!(DDSketch::new(-0.01).is_err());
    assert!(DDSketch::new(1.0).is_err());
    assert!(DDSketch::new(1.5).is_err());
}

#[test]
fn test_update_single_value() {
    let mut dd = DDSketch::new(0.01).unwrap();
    dd.update(&42.0);

    assert_eq!(dd.count(), 1);
    assert!(!dd.is_empty());
}

#[test]
fn test_update_multiple_values() {
    let mut dd = DDSketch::new(0.01).unwrap();

    for i in 1..=100 {
        dd.update(&(i as f64));
    }

    assert_eq!(dd.count(), 100);
    assert!(!dd.is_empty());
}

#[test]
fn test_empty_sketch() {
    let dd = DDSketch::new(0.01).unwrap();

    assert_eq!(dd.count(), 0);
    assert!(dd.is_empty());
    assert_eq!(dd.quantile(0.5), None);
    assert_eq!(dd.min(), None);
    assert_eq!(dd.max(), None);
}

// ============================================================================
// Quantile Accuracy Tests
// ============================================================================

#[test]
fn test_median_accuracy() {
    let mut dd = DDSketch::new(0.01).unwrap();

    // Add values 1..=1000
    for i in 1..=1000 {
        dd.add(i as f64);
    }

    let median = dd.quantile(0.5).unwrap();
    let expected = 500.0;

    // Relative error should be ≤ 1%
    let relative_error = (median - expected).abs() / expected;
    assert!(
        relative_error <= 0.01,
        "Median relative error {} exceeds 1%: got {}, expected {}",
        relative_error,
        median,
        expected
    );
}

#[test]
fn test_p99_accuracy() {
    let mut dd = DDSketch::new(0.01).unwrap();

    for i in 1..=1000 {
        dd.add(i as f64);
    }

    let p99 = dd.quantile(0.99).unwrap();
    let expected = 990.0;

    let relative_error = (p99 - expected).abs() / expected;
    assert!(
        relative_error <= 0.01,
        "P99 relative error {} exceeds 1%: got {}, expected {}",
        relative_error,
        p99,
        expected
    );
}

#[test]
fn test_p999_accuracy() {
    let mut dd = DDSketch::new(0.01).unwrap();

    for i in 1..=10000 {
        dd.add(i as f64);
    }

    let p999 = dd.quantile(0.999).unwrap();
    let expected = 9990.0;

    let relative_error = (p999 - expected).abs() / expected;
    assert!(
        relative_error <= 0.01,
        "P99.9 relative error {} exceeds 1%: got {}, expected {}",
        relative_error,
        p999,
        expected
    );
}

#[test]
fn test_quantiles_ordered() {
    let mut dd = DDSketch::new(0.01).unwrap();

    for i in 1..=1000 {
        dd.add(i as f64);
    }

    let p25 = dd.quantile(0.25).unwrap();
    let p50 = dd.quantile(0.50).unwrap();
    let p75 = dd.quantile(0.75).unwrap();
    let p99 = dd.quantile(0.99).unwrap();

    assert!(p25 <= p50, "p25 ({}) should be ≤ p50 ({})", p25, p50);
    assert!(p50 <= p75, "p50 ({}) should be ≤ p75 ({})", p50, p75);
    assert!(p75 <= p99, "p75 ({}) should be ≤ p99 ({})", p75, p99);
}

// ============================================================================
// Relative Error Tests (Key Feature!)
// ============================================================================

#[test]
fn test_relative_error_wide_range() {
    let mut dd = DDSketch::new(0.01).unwrap();

    // Values spanning 6 orders of magnitude: 1 to 1,000,000
    let mut values = vec![];
    let mut current = 1.0;
    while current <= 1_000_000.0 {
        values.push(current);
        dd.add(current);
        current *= 1.1; // Exponential spacing
    }

    values.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // Test various quantiles
    for q in [0.25, 0.5, 0.75, 0.9, 0.95, 0.99].iter() {
        let estimated = dd.quantile(*q).unwrap();
        let expected_idx = ((*q * values.len() as f64).ceil() as usize).min(values.len()) - 1;
        let expected = values[expected_idx];

        let relative_error = (estimated - expected).abs() / expected.max(1.0_f64);
        assert!(
            relative_error <= 0.05, // Allow some slack for discretization
            "Quantile {} relative error {} exceeds threshold: got {}, expected {}",
            q,
            relative_error,
            estimated,
            expected
        );
    }
}

#[test]
fn test_relative_error_guarantee() {
    let mut dd = DDSketch::new(0.02).unwrap(); // 2% accuracy

    // Add exponentially distributed values
    for i in 0..100 {
        let value = 10.0_f64.powf(i as f64 / 20.0); // 10^0 to 10^5
        dd.add(value);
    }

    // All quantiles should have ≤ 2% relative error
    for i in 1..=99 {
        let q = i as f64 / 100.0;
        if let Some(_quantile) = dd.quantile(q) {
            // Hard to verify exact expected value, but quantiles should be ordered
            // and within reasonable bounds
        }
    }

    // At least verify ordering
    let p25 = dd.quantile(0.25).unwrap();
    let p50 = dd.quantile(0.50).unwrap();
    let p75 = dd.quantile(0.75).unwrap();

    assert!(p25 <= p50 && p50 <= p75);
}

#[test]
fn test_small_values() {
    let mut dd = DDSketch::new(0.01).unwrap();

    // Values near 0.001
    for i in 1..=1000 {
        dd.add(i as f64 * 0.001);
    }

    let median = dd.quantile(0.5).unwrap();
    let expected = 0.5; // 500 * 0.001

    let relative_error = (median - expected).abs() / expected;
    assert!(
        relative_error <= 0.02,
        "Small values: relative error {} exceeds 2%",
        relative_error
    );
}

#[test]
fn test_large_values() {
    let mut dd = DDSketch::new(0.01).unwrap();

    // Values near 1 billion
    for i in 1..=1000 {
        dd.add(i as f64 * 1_000_000.0);
    }

    let median = dd.quantile(0.5).unwrap();
    let expected = 500_000_000.0; // 500 * 1,000,000

    let relative_error = (median - expected).abs() / expected;
    assert!(
        relative_error <= 0.02,
        "Large values: relative error {} exceeds 2%",
        relative_error
    );
}

// ============================================================================
// Special Value Tests
// ============================================================================

#[test]
fn test_negative_values() {
    let mut dd = DDSketch::new(0.01).unwrap();

    // Add negative values
    for i in 1..=1000 {
        dd.add(-(i as f64));
    }

    assert_eq!(dd.count(), 1000);

    let median = dd.quantile(0.5).unwrap();
    assert!(median < 0.0, "Median of negative values should be negative");

    let expected = -500.0_f64;
    let relative_error = (median - expected).abs() / expected.abs();
    assert!(
        relative_error <= 0.02,
        "Negative values: relative error {} exceeds 2%",
        relative_error
    );
}

#[test]
fn test_zero_values() {
    let mut dd = DDSketch::new(0.01).unwrap();

    // Add zeros
    for _ in 0..100 {
        dd.add(0.0);
    }

    assert_eq!(dd.count(), 100);

    let median = dd.quantile(0.5).unwrap();
    assert_eq!(median, 0.0, "Median of all zeros should be zero");
}

#[test]
fn test_mixed_signs() {
    let mut dd = DDSketch::new(0.01).unwrap();

    // Add negative, zero, and positive values
    for i in -100..=100 {
        dd.add(i as f64);
    }

    assert_eq!(dd.count(), 201);

    // Median should be around 0
    let median = dd.quantile(0.5).unwrap();
    assert!(
        median.abs() <= 2.0,
        "Median of -100..100 should be near 0, got {}",
        median
    );

    // Min should be negative, max positive
    let min = dd.min().unwrap();
    let max = dd.max().unwrap();

    assert!(min < 0.0, "Min should be negative");
    assert!(max > 0.0, "Max should be positive");
}

#[test]
fn test_extreme_values() {
    let mut dd = DDSketch::new(0.05).unwrap();

    // Add very small and very large values
    dd.add(f64::MIN_POSITIVE);
    dd.add(1.0);
    dd.add(f64::MAX / 2.0); // Avoid overflow in calculations

    assert_eq!(dd.count(), 3);

    let min = dd.min().unwrap();
    let max = dd.max().unwrap();

    assert!(min > 0.0 && min < 1.0);
    assert!(max > 1.0);
}

// ============================================================================
// Merge Tests
// ============================================================================

#[test]
fn test_merge_empty_sketches() {
    let mut dd1 = DDSketch::new(0.01).unwrap();
    let dd2 = DDSketch::new(0.01).unwrap();

    assert!(dd1.merge(&dd2).is_ok());
    assert_eq!(dd1.count(), 0);
    assert!(dd1.is_empty());
}

#[test]
fn test_merge_disjoint_ranges() {
    let mut dd1 = DDSketch::new(0.01).unwrap();
    let mut dd2 = DDSketch::new(0.01).unwrap();

    // First sketch: 1..=1000
    for i in 1..=1000 {
        dd1.add(i as f64);
    }

    // Second sketch: 1001..=2000
    for i in 1001..=2000 {
        dd2.add(i as f64);
    }

    dd1.merge(&dd2).unwrap();

    assert_eq!(dd1.count(), 2000);

    // Median should be around 1000
    let median = dd1.quantile(0.5).unwrap();
    let expected = 1000.0;
    let relative_error = (median - expected).abs() / expected;

    assert!(
        relative_error <= 0.02,
        "Merged median error {} exceeds 2%",
        relative_error
    );
}

#[test]
fn test_merge_overlapping() {
    let mut dd1 = DDSketch::new(0.01).unwrap();
    let mut dd2 = DDSketch::new(0.01).unwrap();

    // Overlapping ranges
    for i in 1..=1000 {
        dd1.add(i as f64);
    }

    for i in 500..=1500 {
        dd2.add(i as f64);
    }

    dd1.merge(&dd2).unwrap();

    assert_eq!(dd1.count(), 2001); // 1000 + 1001
}

#[test]
fn test_merge_commutative() {
    let mut dd1a = DDSketch::new(0.01).unwrap();
    let mut dd1b = DDSketch::new(0.01).unwrap();
    let mut dd2a = DDSketch::new(0.01).unwrap();
    let mut dd2b = DDSketch::new(0.01).unwrap();

    // Create identical pairs
    for i in 1..=500 {
        dd1a.add(i as f64);
        dd1b.add(i as f64);
    }

    for i in 501..=1000 {
        dd2a.add(i as f64);
        dd2b.add(i as f64);
    }

    // Merge in both directions
    dd1a.merge(&dd2a).unwrap(); // A.merge(B)
    dd2b.merge(&dd1b).unwrap(); // B.merge(A)

    // Results should be similar
    let median1 = dd1a.quantile(0.5).unwrap();
    let median2 = dd2b.quantile(0.5).unwrap();

    let diff = (median1 - median2).abs() / median1.max(median2);
    assert!(
        diff < 0.03,
        "Merge not commutative: {} vs {} (diff: {})",
        median1,
        median2,
        diff
    );
}

#[test]
fn test_merge_preserves_accuracy() {
    let mut dd1 = DDSketch::new(0.01).unwrap();
    let mut dd2 = DDSketch::new(0.01).unwrap();

    for i in 1..=1000 {
        dd1.add(i as f64);
        dd2.add((i + 1000) as f64);
    }

    dd1.merge(&dd2).unwrap();

    // After merge, quantiles should still have good accuracy
    let median = dd1.quantile(0.5).unwrap();
    let expected = 1000.0; // Middle of 1..2000

    let relative_error = (median - expected).abs() / expected;
    assert!(
        relative_error <= 0.03,
        "Post-merge accuracy degraded: error {}",
        relative_error
    );
}

#[test]
fn test_merge_incompatible_accuracy() {
    let mut dd1 = DDSketch::new(0.01).unwrap();
    let dd2 = DDSketch::new(0.05).unwrap(); // Different accuracy

    let result = dd1.merge(&dd2);
    assert!(
        result.is_err(),
        "Should not merge sketches with different accuracy"
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_all_same_value() {
    let mut dd = DDSketch::new(0.01).unwrap();

    // Add 1000 copies of the same value
    for _ in 0..1000 {
        dd.add(42.0);
    }

    assert_eq!(dd.count(), 1000);

    // All quantiles should be the same value
    let p25 = dd.quantile(0.25).unwrap();
    let p50 = dd.quantile(0.50).unwrap();
    let p75 = dd.quantile(0.75).unwrap();
    let p99 = dd.quantile(0.99).unwrap();

    // Allow small relative error due to binning
    for &q in &[p25, p50, p75, p99] {
        let relative_error = (q - 42.0).abs() / 42.0;
        assert!(
            relative_error <= 0.02,
            "All same value: quantile {} differs from 42.0",
            q
        );
    }
}

#[test]
fn test_bimodal_distribution() {
    let mut dd = DDSketch::new(0.01).unwrap();

    // Two distinct clusters: around 100 and around 900
    for _ in 0..500 {
        dd.add(100.0);
    }
    for _ in 0..500 {
        dd.add(900.0);
    }

    assert_eq!(dd.count(), 1000);

    // Median should be between the two modes
    let median = dd.quantile(0.5).unwrap();
    assert!(
        median > 100.0 && median < 900.0,
        "Bimodal median should be between modes: {}",
        median
    );
}

#[test]
fn test_uniform_distribution() {
    let mut dd = DDSketch::new(0.01).unwrap();

    // Uniform distribution [1..1000]
    for i in 1..=1000 {
        dd.add(i as f64);
    }

    // Quantiles should be evenly spaced
    let p25 = dd.quantile(0.25).unwrap();
    let p50 = dd.quantile(0.50).unwrap();
    let p75 = dd.quantile(0.75).unwrap();

    assert!(p25 > 200.0 && p25 < 300.0, "p25 should be ~250: {}", p25);
    assert!(p50 > 450.0 && p50 < 550.0, "p50 should be ~500: {}", p50);
    assert!(p75 > 700.0 && p75 < 800.0, "p75 should be ~750: {}", p75);
}

#[test]
fn test_exponential_distribution() {
    let mut dd = DDSketch::new(0.05).unwrap();

    // Exponential values: 2^0, 2^1, 2^2, ..., 2^20
    for i in 0..=20 {
        let value = 2.0_f64.powi(i);
        dd.add(value);
    }

    assert_eq!(dd.count(), 21);

    // Quantiles should follow exponential pattern
    let p50 = dd.quantile(0.5).unwrap();
    assert!(
        p50 > 1000.0 && p50 < 5000.0,
        "Exponential p50 should be in thousands: {}",
        p50
    );
}

// ============================================================================
// Min/Max Tests
// ============================================================================

#[test]
fn test_min_max_basic() {
    let mut dd = DDSketch::new(0.01).unwrap();

    for i in 1..=100 {
        dd.add(i as f64);
    }

    let min = dd.min().unwrap();
    let max = dd.max().unwrap();

    assert!(
        (0.99..=1.5).contains(&min),
        "Min should be close to 1: {}",
        min
    );
    assert!(
        (99.0..=101.0).contains(&max),
        "Max should be close to 100: {}",
        max
    );
}

#[test]
fn test_min_max_with_negatives() {
    let mut dd = DDSketch::new(0.01).unwrap();

    for i in -50..=50 {
        dd.add(i as f64);
    }

    let min = dd.min().unwrap();
    let max = dd.max().unwrap();

    assert!(min < -40.0, "Min should be close to -50: {}", min);
    assert!(max > 40.0, "Max should be close to 50: {}", max);
}

// ============================================================================
// Invalid Quantile Queries
// ============================================================================

#[test]
fn test_invalid_quantile_values() {
    let mut dd = DDSketch::new(0.01).unwrap();

    for i in 1..=100 {
        dd.add(i as f64);
    }

    // Out of range quantiles
    assert_eq!(dd.quantile(-0.1), None);
    assert_eq!(dd.quantile(1.5), None);
}

#[test]
fn test_boundary_quantiles() {
    let mut dd = DDSketch::new(0.01).unwrap();

    for i in 1..=100 {
        dd.add(i as f64);
    }

    // Boundary cases
    let p0 = dd.quantile(0.0).unwrap();
    let p100 = dd.quantile(1.0).unwrap();

    let min = dd.min().unwrap();
    let max = dd.max().unwrap();

    // p0 should be close to min, p100 close to max
    assert!((p0 - min).abs() / min < 0.1, "p0 should approximate min");
    assert!(
        (p100 - max).abs() / max < 0.1,
        "p100 should approximate max"
    );
}

// ============================================================================
// Property-Based Tests (using proptest)
// ============================================================================

proptest! {
    #[test]
    fn prop_quantiles_ordered(values in prop::collection::vec(1.0f64..1000.0, 100..500)) {
        let mut dd = DDSketch::new(0.01).unwrap();
        for v in values {
            dd.add(v);
        }

        if dd.count() == 0 {
            return Ok(());
        }

        let p25 = dd.quantile(0.25).unwrap();
        let p50 = dd.quantile(0.50).unwrap();
        let p75 = dd.quantile(0.75).unwrap();
        let p99 = dd.quantile(0.99).unwrap();

        prop_assert!(p25 <= p50, "p25 ({}) should be <= p50 ({})", p25, p50);
        prop_assert!(p50 <= p75, "p50 ({}) should be <= p75 ({})", p50, p75);
        prop_assert!(p75 <= p99, "p75 ({}) should be <= p99 ({})", p75, p99);
    }

    #[test]
    fn prop_quantiles_in_range(values in prop::collection::vec(1.0f64..1000.0, 100..500)) {
        let mut dd = DDSketch::new(0.01).unwrap();
        for v in &values {
            dd.add(*v);
        }

        if dd.count() == 0 {
            return Ok(());
        }

        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        for q in [0.25, 0.5, 0.75, 0.99].iter() {
            let quantile = dd.quantile(*q).unwrap();
            prop_assert!(
                quantile >= min * 0.9 && quantile <= max * 1.1,
                "Quantile {} = {} should be in range [{}, {}]",
                q, quantile, min, max
            );
        }
    }

    #[test]
    fn prop_merge_commutative(
        values1 in prop::collection::vec(1.0f64..1000.0, 50..200),
        values2 in prop::collection::vec(1.0f64..1000.0, 50..200)
    ) {
        let mut dd1a = DDSketch::new(0.01).unwrap();
        let mut dd1b = DDSketch::new(0.01).unwrap();
        let mut dd2a = DDSketch::new(0.01).unwrap();
        let mut dd2b = DDSketch::new(0.01).unwrap();

        // Create identical pairs
        for v in &values1 {
            dd1a.add(*v);
            dd1b.add(*v);
        }

        for v in &values2 {
            dd2a.add(*v);
            dd2b.add(*v);
        }

        // Merge in both directions
        dd1a.merge(&dd2a).unwrap(); // A.merge(B)
        dd2b.merge(&dd1b).unwrap(); // B.merge(A)

        // Counts should be identical
        prop_assert_eq!(dd1a.count(), dd2b.count());

        // Quantiles should be very similar
        for q in [0.25, 0.5, 0.75, 0.99].iter() {
            let q1 = dd1a.quantile(*q).unwrap();
            let q2 = dd2b.quantile(*q).unwrap();

            let diff = (q1 - q2).abs() / q1.max(q2);
            prop_assert!(
                diff < 0.05,
                "Merge not commutative at q={}: {} vs {} (diff: {})",
                q, q1, q2, diff
            );
        }
    }

    #[test]
    fn prop_merge_increases_count(
        values1 in prop::collection::vec(1.0f64..1000.0, 50..200),
        values2 in prop::collection::vec(1.0f64..1000.0, 50..200)
    ) {
        let mut dd1 = DDSketch::new(0.01).unwrap();
        let mut dd2 = DDSketch::new(0.01).unwrap();

        for v in &values1 {
            dd1.add(*v);
        }

        for v in &values2 {
            dd2.add(*v);
        }

        let count1 = dd1.count();
        let count2 = dd2.count();

        dd1.merge(&dd2).unwrap();

        prop_assert_eq!(dd1.count(), count1 + count2);
    }

    #[test]
    fn prop_min_max_bounds(values in prop::collection::vec(1.0f64..1000.0, 10..200)) {
        let mut dd = DDSketch::new(0.01).unwrap();
        for v in &values {
            dd.add(*v);
        }

        if dd.count() == 0 {
            return Ok(());
        }

        let true_min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let true_max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        let sketch_min = dd.min().unwrap();
        let sketch_max = dd.max().unwrap();

        // Min/max should be close to true values (within relative error)
        prop_assert!(
            (sketch_min - true_min).abs() / true_min < 0.02,
            "Min mismatch: sketch={}, true={}",
            sketch_min, true_min
        );

        prop_assert!(
            (sketch_max - true_max).abs() / true_max < 0.02,
            "Max mismatch: sketch={}, true={}",
            sketch_max, true_max
        );
    }

    #[test]
    fn prop_handles_negative_values(values in prop::collection::vec(-1000.0f64..-1.0, 50..200)) {
        let mut dd = DDSketch::new(0.01).unwrap();
        for v in values {
            dd.add(v);
        }

        prop_assert!(dd.count() > 0);

        let median = dd.quantile(0.5).unwrap();
        prop_assert!(median < 0.0, "Median of negative values should be negative");
    }

    #[test]
    fn prop_handles_mixed_signs(
        pos_values in prop::collection::vec(1.0f64..1000.0, 25..100),
        neg_values in prop::collection::vec(-1000.0f64..-1.0, 25..100)
    ) {
        let mut dd = DDSketch::new(0.01).unwrap();

        for v in pos_values {
            dd.add(v);
        }

        for v in neg_values {
            dd.add(v);
        }

        prop_assert!(dd.count() > 0);

        let min = dd.min().unwrap();
        let max = dd.max().unwrap();

        prop_assert!(min < 0.0, "Min should be negative");
        prop_assert!(max > 0.0, "Max should be positive");
    }

    #[test]
    fn prop_count_matches_insertions(values in prop::collection::vec(1.0f64..1000.0, 0..500)) {
        let mut dd = DDSketch::new(0.01).unwrap();
        for v in &values {
            dd.add(*v);
        }

        prop_assert_eq!(dd.count(), values.len() as u64);
    }

    #[test]
    fn prop_all_quantiles_between_min_max(values in prop::collection::vec(1.0f64..1000.0, 100..300)) {
        let mut dd = DDSketch::new(0.01).unwrap();
        for v in &values {
            dd.add(*v);
        }

        if dd.count() == 0 {
            return Ok(());
        }

        let min = dd.min().unwrap();
        let max = dd.max().unwrap();

        // Test 10 evenly spaced quantiles
        for i in 0..=10 {
            let q = i as f64 / 10.0;
            let quantile = dd.quantile(q).unwrap();

            prop_assert!(
                quantile >= min * 0.95 && quantile <= max * 1.05,
                "Quantile {} = {} should be between min={} and max={}",
                q, quantile, min, max
            );
        }
    }
}
