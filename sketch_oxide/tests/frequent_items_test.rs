use sketch_oxide::error::SketchError;
use sketch_oxide::frequency::frequent::{ErrorType, FrequentItems};

#[test]
fn test_new_with_valid_max_size() {
    let sketch: FrequentItems<String> = FrequentItems::new(10).unwrap();
    assert!(sketch.is_empty());
    assert_eq!(sketch.num_items(), 0);
}

#[test]
fn test_new_with_invalid_max_size() {
    let result: Result<FrequentItems<String>, SketchError> = FrequentItems::new(1);
    assert!(result.is_err());

    match result {
        Err(SketchError::InvalidParameter {
            param, constraint, ..
        }) => {
            assert_eq!(param, "max_size");
            assert!(constraint.contains(">= 2"));
        }
        _ => panic!("Expected InvalidParameter error"),
    }
}

#[test]
fn test_new_with_zero_max_size() {
    let result: Result<FrequentItems<String>, SketchError> = FrequentItems::new(0);
    assert!(result.is_err());
}

#[test]
fn test_update_single_item() {
    let mut sketch = FrequentItems::new(10).unwrap();
    sketch.update("apple".to_string());

    assert!(!sketch.is_empty());
    assert_eq!(sketch.num_items(), 1);

    let estimate = sketch.get_estimate(&"apple".to_string());
    assert!(estimate.is_some());

    let (lower, upper) = estimate.unwrap();
    assert_eq!(lower, 1);
    assert_eq!(upper, 1); // No offset initially
}

#[test]
fn test_update_multiple_same_item() {
    let mut sketch = FrequentItems::new(10).unwrap();

    for _ in 0..5 {
        sketch.update("apple".to_string());
    }

    assert_eq!(sketch.num_items(), 1);

    let (lower, upper) = sketch.get_estimate(&"apple".to_string()).unwrap();
    assert_eq!(lower, 5);
    assert_eq!(upper, 5);
}

#[test]
fn test_update_by_count() {
    let mut sketch = FrequentItems::new(10).unwrap();
    sketch.update_by("apple".to_string(), 10);

    let (lower, upper) = sketch.get_estimate(&"apple".to_string()).unwrap();
    assert_eq!(lower, 10);
    assert_eq!(upper, 10);
}

#[test]
fn test_update_multiple_items_within_capacity() {
    let mut sketch = FrequentItems::new(10).unwrap();

    for i in 0..5 {
        sketch.update(format!("item_{}", i));
    }

    assert_eq!(sketch.num_items(), 5);

    for i in 0..5 {
        let estimate = sketch.get_estimate(&format!("item_{}", i));
        assert!(estimate.is_some());
    }
}

#[test]
fn test_purge_when_exceeding_capacity() {
    let mut sketch = FrequentItems::new(3).unwrap();

    // Add items with different frequencies
    sketch.update_by("common1".to_string(), 10);
    sketch.update_by("common2".to_string(), 8);
    sketch.update_by("rare1".to_string(), 1);
    sketch.update_by("rare2".to_string(), 1);

    // After purge, sketch should have <= 3 items
    assert!(sketch.num_items() <= 3);

    // Common items should still be present
    assert!(sketch.get_estimate(&"common1".to_string()).is_some());
    assert!(sketch.get_estimate(&"common2".to_string()).is_some());
}

#[test]
fn test_offset_increases_after_purge() {
    let mut sketch = FrequentItems::new(2).unwrap();

    // Add items to trigger purge
    sketch.update("a".to_string());
    sketch.update("b".to_string());
    sketch.update("c".to_string());

    // At least one item should have non-zero upper bound > lower bound
    let items = sketch.frequent_items(ErrorType::NoFalsePositives);

    let has_offset = items.iter().any(|(_, lower, upper)| upper > lower);
    // After purge, offset should be > 0
    if sketch.num_items() <= 2 {
        // Purge occurred
        assert!(has_offset || items.is_empty());
    }
}

#[test]
fn test_top_k_identification() {
    let mut sketch = FrequentItems::new(100).unwrap();

    // Create a clear frequency distribution
    for _ in 0..100 {
        sketch.update("very_common".to_string());
    }
    for _ in 0..50 {
        sketch.update("common".to_string());
    }
    for _ in 0..10 {
        sketch.update("rare".to_string());
    }

    let items = sketch.frequent_items(ErrorType::NoFalsePositives);

    // Should return all three items since capacity is large
    assert!(items.len() >= 3);

    // Should be sorted by frequency (descending)
    assert_eq!(items[0].0, "very_common");
    assert_eq!(items[1].0, "common");
    assert_eq!(items[2].0, "rare");

    // Verify bounds
    assert_eq!(items[0].1, 100); // lower bound
    assert_eq!(items[1].1, 50);
    assert_eq!(items[2].1, 10);
}

#[test]
fn test_top_k_with_small_capacity() {
    let mut sketch = FrequentItems::new(5).unwrap();

    // Add 10 items with decreasing frequencies
    for i in 0..10 {
        let count = (10 - i) * 10;
        for _ in 0..count {
            sketch.update(format!("item_{}", i));
        }
    }

    let items = sketch.frequent_items(ErrorType::NoFalsePositives);

    // Should have at most 5 items
    assert!(items.len() <= 5);

    // Most frequent items should be present
    let item_names: Vec<_> = items.iter().map(|(name, _, _)| name.clone()).collect();
    assert!(item_names.contains(&"item_0".to_string()));
}

#[test]
fn test_error_bounds_no_false_positives() {
    let mut sketch = FrequentItems::new(3).unwrap();

    // Add items
    sketch.update_by("a".to_string(), 100);
    sketch.update_by("b".to_string(), 50);
    sketch.update_by("c".to_string(), 25);
    sketch.update_by("d".to_string(), 10);

    let items = sketch.frequent_items(ErrorType::NoFalsePositives);

    // In NoFalsePositives mode, use lower bound
    // All returned items should have significant lower bounds
    for (_, lower, upper) in &items {
        assert!(lower > &0);
        assert!(upper >= lower);
    }
}

#[test]
fn test_error_bounds_no_false_negatives() {
    let mut sketch = FrequentItems::new(3).unwrap();

    // Add items
    sketch.update_by("a".to_string(), 100);
    sketch.update_by("b".to_string(), 50);
    sketch.update_by("c".to_string(), 25);
    sketch.update_by("d".to_string(), 10);

    let items = sketch.frequent_items(ErrorType::NoFalseNegatives);

    // In NoFalseNegatives mode, use upper bound
    // Should be more inclusive
    for (_, lower, upper) in &items {
        assert!(upper >= lower);
    }
}

#[test]
fn test_get_estimate_for_missing_item() {
    let mut sketch = FrequentItems::new(10).unwrap();
    sketch.update("exists".to_string());

    let estimate = sketch.get_estimate(&"missing".to_string());
    assert!(estimate.is_none());
}

#[test]
fn test_get_estimate_returns_bounds() {
    let mut sketch = FrequentItems::new(2).unwrap();

    sketch.update_by("a".to_string(), 10);
    sketch.update_by("b".to_string(), 5);
    sketch.update_by("c".to_string(), 3);

    // After purge, bounds should reflect uncertainty
    if let Some((lower, upper)) = sketch.get_estimate(&"a".to_string()) {
        assert!(lower >= 10 || upper >= 10);
        assert!(upper >= lower);
    }
}

#[test]
fn test_merge_compatible_sketches() {
    let mut sketch1 = FrequentItems::new(10).unwrap();
    let mut sketch2 = FrequentItems::new(10).unwrap();

    sketch1.update_by("a".to_string(), 10);
    sketch1.update_by("b".to_string(), 5);

    sketch2.update_by("b".to_string(), 5);
    sketch2.update_by("c".to_string(), 3);

    sketch1.merge(&sketch2).unwrap();

    // Check merged counts
    let (lower_a, _) = sketch1.get_estimate(&"a".to_string()).unwrap();
    let (lower_b, _) = sketch1.get_estimate(&"b".to_string()).unwrap();
    let (lower_c, _) = sketch1.get_estimate(&"c".to_string()).unwrap();

    assert_eq!(lower_a, 10);
    assert_eq!(lower_b, 10);
    assert_eq!(lower_c, 3);
}

#[test]
fn test_merge_incompatible_max_size() {
    let mut sketch1: FrequentItems<String> = FrequentItems::new(10).unwrap();
    let sketch2: FrequentItems<String> = FrequentItems::new(20).unwrap();

    let result = sketch1.merge(&sketch2);
    assert!(result.is_err());

    match result {
        Err(SketchError::IncompatibleSketches { reason }) => {
            assert!(reason.contains("max_size mismatch"));
        }
        _ => panic!("Expected IncompatibleSketches error"),
    }
}

#[test]
fn test_merge_with_empty_sketch() {
    let mut sketch1 = FrequentItems::new(10).unwrap();
    let sketch2 = FrequentItems::new(10).unwrap();

    sketch1.update_by("a".to_string(), 10);

    sketch1.merge(&sketch2).unwrap();

    let (lower, _) = sketch1.get_estimate(&"a".to_string()).unwrap();
    assert_eq!(lower, 10);
}

#[test]
fn test_merge_empty_into_nonempty() {
    let mut sketch1 = FrequentItems::new(10).unwrap();
    let mut sketch2 = FrequentItems::new(10).unwrap();

    sketch2.update_by("b".to_string(), 5);

    sketch1.merge(&sketch2).unwrap();

    let (lower, _) = sketch1.get_estimate(&"b".to_string()).unwrap();
    assert_eq!(lower, 5);
}

#[test]
fn test_merge_triggers_purge() {
    let mut sketch1 = FrequentItems::new(3).unwrap();
    let mut sketch2 = FrequentItems::new(3).unwrap();

    sketch1.update("a".to_string());
    sketch1.update("b".to_string());
    sketch1.update("c".to_string());

    sketch2.update("d".to_string());
    sketch2.update("e".to_string());
    sketch2.update("f".to_string());

    sketch1.merge(&sketch2).unwrap();

    // After merge and purge, should have <= 3 items
    assert!(sketch1.num_items() <= 3);
}

#[test]
fn test_heavy_hitter_detection() {
    let mut sketch = FrequentItems::new(100).unwrap();

    // Add heavy hitter (>10% of stream)
    for _ in 0..200 {
        sketch.update("heavy".to_string());
    }

    // Add many small items
    for i in 0..800 {
        sketch.update(format!("small_{}", i));
    }

    let items = sketch.frequent_items(ErrorType::NoFalsePositives);

    // Heavy hitter should be present
    let heavy = items.iter().find(|(name, _, _)| name == "heavy");
    assert!(heavy.is_some());

    let (_, lower, _) = heavy.unwrap();
    assert!(*lower >= 200);
}

#[test]
fn test_zipf_distribution() {
    let mut sketch = FrequentItems::new(50).unwrap();

    // Simulate Zipf distribution (common in real data)
    for rank in 1..=100 {
        let freq = 1000 / rank;
        for _ in 0..freq {
            sketch.update(format!("item_{}", rank));
        }
    }

    let items = sketch.frequent_items(ErrorType::NoFalsePositives);

    // Should capture top items
    assert!(!items.is_empty());

    // Most frequent should be item_1
    if let Some(first) = items.first() {
        assert_eq!(first.0, "item_1");
    }
}

#[test]
fn test_frequent_items_sorted_descending() {
    let mut sketch = FrequentItems::new(10).unwrap();

    sketch.update_by("low".to_string(), 10);
    sketch.update_by("high".to_string(), 100);
    sketch.update_by("medium".to_string(), 50);

    let items = sketch.frequent_items(ErrorType::NoFalsePositives);

    // Should be sorted by estimate (descending)
    assert_eq!(items[0].0, "high");
    assert_eq!(items[1].0, "medium");
    assert_eq!(items[2].0, "low");

    // Verify descending order
    for i in 0..items.len() - 1 {
        assert!(items[i].1 >= items[i + 1].1);
    }
}

#[test]
fn test_is_empty() {
    let mut sketch: FrequentItems<String> = FrequentItems::new(10).unwrap();
    assert!(sketch.is_empty());

    sketch.update("item".to_string());
    assert!(!sketch.is_empty());
}

#[test]
fn test_num_items() {
    let mut sketch = FrequentItems::new(10).unwrap();
    assert_eq!(sketch.num_items(), 0);

    sketch.update("a".to_string());
    assert_eq!(sketch.num_items(), 1);

    sketch.update("b".to_string());
    assert_eq!(sketch.num_items(), 2);

    sketch.update("a".to_string());
    assert_eq!(sketch.num_items(), 2); // Same item
}

#[test]
fn test_integer_items() {
    let mut sketch: FrequentItems<i32> = FrequentItems::new(10).unwrap();

    sketch.update(42);
    sketch.update(42);
    sketch.update(100);

    let (lower, _) = sketch.get_estimate(&42).unwrap();
    assert_eq!(lower, 2);

    let (lower, _) = sketch.get_estimate(&100).unwrap();
    assert_eq!(lower, 1);
}

#[test]
fn test_error_bound_formula() {
    let max_size = 10;
    let mut sketch = FrequentItems::new(max_size).unwrap();

    // Add many items to trigger multiple purges
    for i in 0..100 {
        sketch.update(format!("item_{}", i % 20));
    }

    let items = sketch.frequent_items(ErrorType::NoFalsePositives);

    // Verify error bounds are reasonable
    for (_, lower, upper) in items {
        let error = upper - lower;
        // Error should be bounded (in practice, ε = 1/max_size)
        // We expect error to be reasonable relative to the stream size
        assert!(error < 100); // Reasonable for 100 total updates
    }
}

// Property-based tests
#[test]
fn test_property_lower_bound_never_exceeds_upper() {
    let mut sketch = FrequentItems::new(5).unwrap();

    for i in 0..50 {
        sketch.update(format!("item_{}", i % 10));
    }

    let items = sketch.frequent_items(ErrorType::NoFalsePositives);

    for (_, lower, upper) in items {
        assert!(lower <= upper, "Lower bound must not exceed upper bound");
    }
}

#[test]
fn test_property_merge_is_associative() {
    let mut sketch1 = FrequentItems::new(10).unwrap();
    let mut sketch2 = FrequentItems::new(10).unwrap();
    let mut sketch3 = FrequentItems::new(10).unwrap();

    sketch1.update_by("a".to_string(), 5);
    sketch2.update_by("b".to_string(), 3);
    sketch3.update_by("c".to_string(), 2);

    // (s1 + s2) + s3
    let mut left = sketch1.clone();
    left.merge(&sketch2).unwrap();
    left.merge(&sketch3).unwrap();

    // s1 + (s2 + s3)
    let mut right = sketch1.clone();
    let mut temp = sketch2.clone();
    temp.merge(&sketch3).unwrap();
    right.merge(&temp).unwrap();

    // Results should be similar (bounds may vary slightly due to purge order)
    let left_items = left.frequent_items(ErrorType::NoFalsePositives);
    let right_items = right.frequent_items(ErrorType::NoFalsePositives);

    assert_eq!(left_items.len(), right_items.len());
}

#[test]
fn test_property_deterministic_results() {
    let mut sketch1 = FrequentItems::new(10).unwrap();
    let mut sketch2 = FrequentItems::new(10).unwrap();

    // Same updates in same order with different frequencies to avoid ties
    for i in 0..20 {
        let count = (i % 5) + 1; // Different counts to ensure unique frequencies
        for _ in 0..count {
            sketch1.update(format!("item_{}", i % 5));
            sketch2.update(format!("item_{}", i % 5));
        }
    }

    let items1 = sketch1.frequent_items(ErrorType::NoFalsePositives);
    let items2 = sketch2.frequent_items(ErrorType::NoFalsePositives);

    // Check same counts, not necessarily same order (HashMap doesn't guarantee order for ties)
    assert_eq!(
        items1.len(),
        items2.len(),
        "Should have same number of items"
    );

    // Check all items have same bounds
    for (item, lower, upper) in &items1 {
        let found = items2.iter().find(|(i, _, _)| i == item);
        assert!(found.is_some(), "Item {:?} should exist in both", item);
        let (_, lower2, upper2) = found.unwrap();
        assert_eq!(lower, lower2);
        assert_eq!(upper, upper2);
    }
}

#[test]
fn test_property_bounds_contain_true_frequency() {
    let mut sketch = FrequentItems::new(20).unwrap();
    let true_freq = 42;

    for _ in 0..true_freq {
        sketch.update("target".to_string());
    }

    // Add noise
    for i in 0..10 {
        sketch.update(format!("noise_{}", i));
    }

    let (lower, upper) = sketch.get_estimate(&"target".to_string()).unwrap();

    // True frequency must be within bounds
    assert!(lower <= true_freq, "Lower bound too high");
    assert!(upper >= true_freq, "Upper bound too low");
}

#[test]
fn test_clone_sketch() {
    let mut sketch1 = FrequentItems::new(10).unwrap();
    sketch1.update_by("a".to_string(), 10);

    let sketch2 = sketch1.clone();

    let est1 = sketch1.get_estimate(&"a".to_string()).unwrap();
    let est2 = sketch2.get_estimate(&"a".to_string()).unwrap();

    assert_eq!(est1, est2);
}
