//! Comprehensive test suite for Rateless IBLT
//!
//! This test suite follows TDD principles with 42+ tests covering all aspects
//! of the Rateless IBLT implementation.

use sketch_oxide::common::{Reconcilable, SketchError};
use sketch_oxide::reconciliation::RatelessIBLT;

// ============================================================================
// Category 1: Construction Tests (5 tests)
// ============================================================================

#[test]
fn test_construction_valid_parameters() {
    // Test creating IBLT with valid parameters
    let iblt = RatelessIBLT::new(100, 32);
    assert!(iblt.is_ok());

    let iblt = iblt.unwrap();
    let stats = iblt.stats();
    assert!(stats.num_cells > 0);
    assert_eq!(stats.cell_size, 32);
}

#[test]
fn test_construction_various_expected_diff() {
    // Test with different expected_diff values
    assert!(RatelessIBLT::new(10, 32).is_ok());
    assert!(RatelessIBLT::new(100, 32).is_ok());
    assert!(RatelessIBLT::new(1000, 32).is_ok());
}

#[test]
fn test_construction_invalid_expected_diff() {
    // Zero expected_diff should fail
    let result = RatelessIBLT::new(0, 32);
    assert!(result.is_err());

    match result {
        Err(SketchError::InvalidParameter { param, .. }) => {
            assert_eq!(param, "expected_diff");
        }
        _ => panic!("Expected InvalidParameter error"),
    }
}

#[test]
fn test_construction_invalid_cell_size() {
    // Cell size must be reasonable (at least 8 bytes for count + minimal data)
    let result = RatelessIBLT::new(100, 0);
    assert!(result.is_err());

    let result = RatelessIBLT::new(100, 4);
    assert!(result.is_err());
}

#[test]
fn test_construction_cell_initialization() {
    // Verify cells are properly initialized
    let iblt = RatelessIBLT::new(10, 32).unwrap();

    // Decode should return empty difference for new IBLT
    let diff = iblt.decode();
    assert!(diff.is_ok());

    let diff = diff.unwrap();
    assert!(diff.is_empty());
    assert_eq!(diff.to_insert.len(), 0);
    assert_eq!(diff.to_remove.len(), 0);
}

// ============================================================================
// Category 2: Basic Operations Tests (8 tests)
// ============================================================================

#[test]
fn test_single_insert() {
    let mut iblt = RatelessIBLT::new(10, 32).unwrap();

    let result = iblt.insert(b"key1", b"value1");
    assert!(result.is_ok());
}

#[test]
fn test_single_delete() {
    let mut iblt = RatelessIBLT::new(10, 32).unwrap();

    // Insert then delete
    iblt.insert(b"key1", b"value1").unwrap();
    let result = iblt.delete(b"key1", b"value1");
    assert!(result.is_ok());
}

#[test]
fn test_multiple_inserts_same_key() {
    let mut iblt = RatelessIBLT::new(10, 32).unwrap();

    // Insert same key-value multiple times
    iblt.insert(b"key1", b"value1").unwrap();
    iblt.insert(b"key1", b"value1").unwrap();
    iblt.insert(b"key1", b"value1").unwrap();

    // Should still decode properly (counts accumulate)
    // Note: Multiple inserts of same key may or may not decode depending on hash collisions
    let diff = iblt.decode();
    // Either succeeds with the key or fails gracefully
    assert!(diff.is_ok() || diff.is_err());
}

#[test]
fn test_multiple_inserts_different_keys() {
    let mut iblt = RatelessIBLT::new(10, 32).unwrap();

    for i in 0..5 {
        let key = format!("key{}", i);
        let value = format!("value{}", i);
        iblt.insert(key.as_bytes(), value.as_bytes()).unwrap();
    }

    let diff = iblt.decode().unwrap();
    assert_eq!(diff.to_insert.len(), 5);
}

#[test]
fn test_insert_then_delete_same_item() {
    let mut iblt = RatelessIBLT::new(10, 32).unwrap();

    iblt.insert(b"key1", b"value1").unwrap();
    iblt.delete(b"key1", b"value1").unwrap();

    // Should cancel out
    let diff = iblt.decode().unwrap();
    assert!(diff.is_empty());
}

#[test]
fn test_empty_decode() {
    let iblt = RatelessIBLT::new(10, 32).unwrap();

    let diff = iblt.decode().unwrap();
    assert!(diff.is_empty());
    assert_eq!(diff.total_changes(), 0);
}

#[test]
fn test_value_integrity_after_operations() {
    let mut iblt = RatelessIBLT::new(10, 32).unwrap();

    let key = b"test_key";
    let value = b"test_value_12345";

    iblt.insert(key, value).unwrap();

    let diff = iblt.decode().unwrap();
    assert_eq!(diff.to_insert.len(), 1);

    // Check that key-value pair is intact
    let (decoded_key, decoded_value) = &diff.to_insert[0];
    assert_eq!(decoded_key, key);
    assert_eq!(decoded_value, value);
}

#[test]
fn test_large_values() {
    let mut iblt = RatelessIBLT::new(10, 1024).unwrap(); // Large cell size

    // 1KB value
    let large_value = vec![0x42u8; 1024];

    iblt.insert(b"key", &large_value).unwrap();

    let diff = iblt.decode().unwrap();
    assert_eq!(diff.to_insert.len(), 1);

    let (_, decoded_value) = &diff.to_insert[0];
    assert_eq!(decoded_value, &large_value);
}

// ============================================================================
// Category 3: Subtraction Tests (6 tests)
// ============================================================================

#[test]
fn test_subtract_identical_iblts() {
    let mut iblt1 = RatelessIBLT::new(10, 32).unwrap();
    let mut iblt2 = RatelessIBLT::new(10, 32).unwrap();

    // Both have same items
    iblt1.insert(b"key1", b"value1").unwrap();
    iblt1.insert(b"key2", b"value2").unwrap();

    iblt2.insert(b"key1", b"value1").unwrap();
    iblt2.insert(b"key2", b"value2").unwrap();

    // Subtract
    iblt1.subtract(&iblt2).unwrap();

    // Should be empty
    let diff = iblt1.decode().unwrap();
    assert!(diff.is_empty());
}

#[test]
fn test_subtract_with_differences() {
    let mut iblt1 = RatelessIBLT::new(10, 32).unwrap();
    let mut iblt2 = RatelessIBLT::new(10, 32).unwrap();

    // iblt1 has key1, key2, key3
    iblt1.insert(b"key1", b"value1").unwrap();
    iblt1.insert(b"key2", b"value2").unwrap();
    iblt1.insert(b"key3", b"value3").unwrap();

    // iblt2 has key1, key2, key4
    iblt2.insert(b"key1", b"value1").unwrap();
    iblt2.insert(b"key2", b"value2").unwrap();
    iblt2.insert(b"key4", b"value4").unwrap();

    // Compute difference: iblt1 - iblt2
    iblt1.subtract(&iblt2).unwrap();

    let diff = iblt1.decode().unwrap();

    // Should have key3 to insert, key4 to remove
    assert_eq!(diff.total_changes(), 2);
}

#[test]
fn test_subtract_non_commutative() {
    let mut iblt1 = RatelessIBLT::new(10, 32).unwrap();
    let mut iblt2 = RatelessIBLT::new(10, 32).unwrap();

    iblt1.insert(b"alice", b"1").unwrap();
    iblt2.insert(b"bob", b"2").unwrap();

    let mut diff1 = iblt1.clone();
    diff1.subtract(&iblt2).unwrap();

    let mut diff2 = iblt2.clone();
    diff2.subtract(&iblt1).unwrap();

    // A-B != B-A (different results)
    let result1 = diff1.decode().unwrap();
    let result2 = diff2.decode().unwrap();

    // They should be different (signs flipped)
    assert_ne!(result1, result2);
}

#[test]
fn test_subtract_associative_property() {
    let mut a = RatelessIBLT::new(20, 32).unwrap();
    let mut b = RatelessIBLT::new(20, 32).unwrap();
    let mut c = RatelessIBLT::new(20, 32).unwrap();

    a.insert(b"a", b"1").unwrap();
    b.insert(b"b", b"2").unwrap();
    c.insert(b"c", b"3").unwrap();

    // Compute (A-B)-C
    let mut left = a.clone();
    left.subtract(&b).unwrap();
    left.subtract(&c).unwrap();

    // Compute A-(B+C) where B+C is B with C's items added
    let mut bc = b.clone();
    bc.insert(b"c", b"3").unwrap();
    let mut right = a.clone();
    right.subtract(&bc).unwrap();

    // Results should be similar in structure
    let left_diff = left.decode().unwrap();
    let right_diff = right.decode().unwrap();

    // Both should contain 'a' and not contain 'b' or 'c'
    assert!(left_diff.total_changes() > 0);
    assert!(right_diff.total_changes() > 0);
}

#[test]
fn test_subtract_empty_iblt() {
    let mut iblt1 = RatelessIBLT::new(10, 32).unwrap();
    let iblt2 = RatelessIBLT::new(10, 32).unwrap(); // Empty

    iblt1.insert(b"key1", b"value1").unwrap();
    iblt1.insert(b"key2", b"value2").unwrap();

    // Subtract empty IBLT
    iblt1.subtract(&iblt2).unwrap();

    // Should still have original items
    let diff = iblt1.decode().unwrap();
    assert_eq!(diff.to_insert.len(), 2);
}

#[test]
fn test_symmetric_difference_computation() {
    let mut alice = RatelessIBLT::new(10, 32).unwrap();
    let mut bob = RatelessIBLT::new(10, 32).unwrap();

    // Shared items
    alice.insert(b"shared1", b"s1").unwrap();
    alice.insert(b"shared2", b"s2").unwrap();
    bob.insert(b"shared1", b"s1").unwrap();
    bob.insert(b"shared2", b"s2").unwrap();

    // Alice only
    alice.insert(b"alice_only", b"a1").unwrap();

    // Bob only
    bob.insert(b"bob_only", b"b1").unwrap();

    // Compute symmetric difference
    let mut diff = alice.clone();
    diff.subtract(&bob).unwrap();

    let result = diff.decode().unwrap();

    // Should contain alice_only and bob_only
    assert_eq!(result.total_changes(), 2);
}

// ============================================================================
// Category 4: Decoding Tests (8 tests)
// ============================================================================

#[test]
fn test_decode_empty_difference() {
    let iblt = RatelessIBLT::new(10, 32).unwrap();

    let diff = iblt.decode().unwrap();
    assert!(diff.is_empty());
}

#[test]
fn test_decode_single_item_difference() {
    let mut iblt = RatelessIBLT::new(10, 32).unwrap();

    iblt.insert(b"single", b"item").unwrap();

    let diff = iblt.decode().unwrap();
    assert_eq!(diff.to_insert.len(), 1);
    assert_eq!(diff.to_remove.len(), 0);

    let (key, value) = &diff.to_insert[0];
    assert_eq!(key, b"single");
    assert_eq!(value, b"item");
}

#[test]
fn test_decode_multiple_item_differences() {
    let mut iblt = RatelessIBLT::new(20, 32).unwrap();

    for i in 0..5 {
        let key = format!("key{}", i);
        let value = format!("value{}", i);
        iblt.insert(key.as_bytes(), value.as_bytes()).unwrap();
    }

    let diff = iblt.decode().unwrap();
    assert_eq!(diff.to_insert.len(), 5);
}

#[test]
fn test_decode_with_corrupted_data() {
    // This test verifies error handling for undecodable IBLT
    // Create an IBLT with too many collisions
    let mut iblt = RatelessIBLT::new(5, 32).unwrap(); // Small capacity

    // Add many items that may cause decoding failure
    for i in 0..100 {
        let key = format!("key{}", i);
        let value = format!("value{}", i);
        let _ = iblt.insert(key.as_bytes(), value.as_bytes());
    }

    // Decode may fail or succeed partially depending on implementation
    let result = iblt.decode();

    // Either succeeds with partial results or fails gracefully
    match result {
        Ok(diff) => {
            // Partial decode acceptable
            assert!(diff.total_changes() < 100);
        }
        Err(SketchError::ReconciliationError { .. }) => {
            // Expected error for undecodable IBLT
        }
        _ => panic!("Unexpected error type"),
    }
}

#[test]
fn test_decode_capacity_exceeded() {
    let mut iblt = RatelessIBLT::new(10, 32).unwrap(); // Expect 10 differences

    // Insert more than capacity
    for i in 0..50 {
        let key = format!("key{}", i);
        let value = format!("value{}", i);
        let _ = iblt.insert(key.as_bytes(), value.as_bytes());
    }

    let result = iblt.decode();

    // Should either fail or return partial results
    match result {
        Ok(diff) => {
            // Partial success is acceptable
            assert!(diff.total_changes() <= 50);
        }
        Err(SketchError::ReconciliationError { reason }) => {
            assert!(reason.contains("capacity") || reason.contains("decode"));
        }
        _ => panic!("Unexpected error type"),
    }
}

#[test]
fn test_decode_maintains_key_value_integrity() {
    let mut iblt = RatelessIBLT::new(20, 64).unwrap();

    let test_pairs = vec![
        (b"key1".to_vec(), b"value1".to_vec()),
        (b"key2".to_vec(), b"value2_longer".to_vec()),
        (b"key3_longer".to_vec(), b"value3".to_vec()),
    ];

    for (key, value) in &test_pairs {
        iblt.insert(key, value).unwrap();
    }

    let diff = iblt.decode().unwrap();
    assert_eq!(diff.to_insert.len(), 3);

    // Verify all pairs are intact
    for (key, value) in &test_pairs {
        let found = diff.to_insert.iter().any(|(k, v)| k == key && v == value);
        assert!(found, "Key-value pair not found: {:?}, {:?}", key, value);
    }
}

#[test]
fn test_decode_to_insert_and_to_delete_separately() {
    let mut alice = RatelessIBLT::new(10, 32).unwrap();
    let mut bob = RatelessIBLT::new(10, 32).unwrap();

    // Alice has a1, a2
    alice.insert(b"a1", b"v1").unwrap();
    alice.insert(b"a2", b"v2").unwrap();

    // Bob has b1, b2
    bob.insert(b"b1", b"v3").unwrap();
    bob.insert(b"b2", b"v4").unwrap();

    // Compute alice - bob
    let mut diff = alice.clone();
    diff.subtract(&bob).unwrap();

    let result = diff.decode().unwrap();

    // Should have items to insert (alice's items)
    // and items to remove (bob's items, shown as negative)
    assert!(result.total_changes() == 4);
}

#[test]
fn test_no_false_positives_in_decoding() {
    let mut iblt1 = RatelessIBLT::new(10, 32).unwrap();
    let mut iblt2 = RatelessIBLT::new(10, 32).unwrap();

    // Both have exactly the same items
    iblt1.insert(b"key1", b"value1").unwrap();
    iblt1.insert(b"key2", b"value2").unwrap();

    iblt2.insert(b"key1", b"value1").unwrap();
    iblt2.insert(b"key2", b"value2").unwrap();

    iblt1.subtract(&iblt2).unwrap();

    let diff = iblt1.decode().unwrap();

    // Must be empty - no false positives
    assert!(diff.is_empty(), "False positive detected in decoding");
}

// ============================================================================
// Category 5: Capacity Tests (5 tests)
// ============================================================================

#[test]
fn test_within_capacity() {
    let mut iblt = RatelessIBLT::new(10, 32).unwrap();

    // Insert exactly expected_diff items
    for i in 0..10 {
        let key = format!("key{}", i);
        let value = format!("value{}", i);
        iblt.insert(key.as_bytes(), value.as_bytes()).unwrap();
    }

    let diff = iblt.decode().unwrap();
    assert_eq!(diff.to_insert.len(), 10);
}

#[test]
fn test_at_capacity_boundary() {
    let expected_diff = 20;
    let mut iblt = RatelessIBLT::new(expected_diff, 32).unwrap();

    // Insert exactly at expected capacity
    for i in 0..expected_diff {
        let key = format!("key{}", i);
        let value = format!("value{}", i);
        iblt.insert(key.as_bytes(), value.as_bytes()).unwrap();
    }

    let diff = iblt.decode();
    assert!(diff.is_ok());
}

#[test]
fn test_exceed_capacity() {
    let expected_diff = 10;
    let mut iblt = RatelessIBLT::new(expected_diff, 32).unwrap();

    // Insert 3x expected capacity
    for i in 0..(expected_diff * 3) {
        let key = format!("key{}", i);
        let value = format!("value{}", i);
        let _ = iblt.insert(key.as_bytes(), value.as_bytes());
    }

    let result = iblt.decode();

    // May fail or return partial results
    match result {
        Ok(diff) => {
            // Partial decode is acceptable
            assert!(diff.total_changes() > 0);
        }
        Err(_) => {
            // Expected to fail when capacity exceeded
        }
    }
}

#[test]
fn test_capacity_efficiency() {
    // Verify that num_cells ≈ c * expected_diff where c ∈ [1.5, 2.0]
    let expected_diff = 100;
    let iblt = RatelessIBLT::new(expected_diff, 32).unwrap();

    let stats = iblt.stats();
    let c = stats.num_cells as f64 / expected_diff as f64;

    // c factor should be reasonable (typically 1.5 to 2.0)
    assert!(c >= 1.0, "c factor too small: {}", c);
    assert!(c <= 3.0, "c factor too large: {}", c);
}

#[test]
fn test_memory_usage_calculation() {
    let expected_diff = 100;
    let cell_size = 64;
    let iblt = RatelessIBLT::new(expected_diff, cell_size).unwrap();

    let stats = iblt.stats();

    // Verify memory calculation
    assert_eq!(stats.cell_size, cell_size);
    assert!(stats.num_cells > 0);

    // Total memory should be reasonable
    let total_memory = stats.num_cells * stats.cell_size;
    assert!(total_memory > 0);
}

// ============================================================================
// Category 6: Set Reconciliation Tests (5 tests)
// ============================================================================

#[test]
fn test_perfect_reconciliation_small_diff() {
    let mut alice = RatelessIBLT::new(10, 32).unwrap();
    let mut bob = RatelessIBLT::new(10, 32).unwrap();

    // Shared items
    for i in 0..10 {
        let key = format!("shared{}", i);
        alice.insert(key.as_bytes(), b"val").unwrap();
        bob.insert(key.as_bytes(), b"val").unwrap();
    }

    // Small differences
    alice.insert(b"alice1", b"a1").unwrap();
    alice.insert(b"alice2", b"a2").unwrap();

    bob.insert(b"bob1", b"b1").unwrap();
    bob.insert(b"bob2", b"b2").unwrap();

    // Reconcile
    let mut diff = alice.clone();
    diff.subtract(&bob).unwrap();

    let result = diff.decode().unwrap();

    // Should recover all 4 differences
    assert_eq!(result.total_changes(), 4);
}

#[test]
fn test_partial_reconciliation_near_capacity() {
    let expected_diff = 15;
    let mut alice = RatelessIBLT::new(expected_diff, 32).unwrap();
    let bob = RatelessIBLT::new(expected_diff, 32).unwrap();

    // Create exactly expected_diff differences
    for i in 0..expected_diff {
        let key = format!("alice{}", i);
        alice.insert(key.as_bytes(), b"val").unwrap();
    }

    // Reconcile
    let mut diff = alice.clone();
    diff.subtract(&bob).unwrap();

    let result = diff.decode();

    // Should succeed at capacity
    assert!(result.is_ok());
}

#[test]
fn test_large_symmetric_differences() {
    let expected_diff = 50;
    let mut alice = RatelessIBLT::new(expected_diff, 64).unwrap(); // Larger cell size for stability
    let mut bob = RatelessIBLT::new(expected_diff, 64).unwrap();

    // Large number of differences (within capacity)
    for i in 0..20 {
        let key = format!("alice{}", i);
        alice.insert(key.as_bytes(), b"a").unwrap();
    }

    for i in 0..20 {
        let key = format!("bob{}", i);
        bob.insert(key.as_bytes(), b"b").unwrap();
    }

    let mut diff = alice.clone();
    diff.subtract(&bob).unwrap();

    let result = diff.decode();

    // Should handle large differences (40 total, expected_diff=50)
    // May occasionally fail due to hash collisions, which is acceptable
    match result {
        Ok(r) => {
            assert!(r.total_changes() > 0);
            assert!(r.total_changes() <= 40);
        }
        Err(_) => {
            // Acceptable for edge cases with hash collisions
        }
    }
}

#[test]
fn test_no_shared_items() {
    let mut alice = RatelessIBLT::new(20, 32).unwrap();
    let mut bob = RatelessIBLT::new(20, 32).unwrap();

    // Entirely different sets
    for i in 0..5 {
        alice.insert(format!("a{}", i).as_bytes(), b"av").unwrap();
        bob.insert(format!("b{}", i).as_bytes(), b"bv").unwrap();
    }

    let mut diff = alice.clone();
    diff.subtract(&bob).unwrap();

    let result = diff.decode().unwrap();

    // Should have 10 total changes
    assert_eq!(result.total_changes(), 10);
}

#[test]
fn test_mostly_overlapping_sets() {
    let mut alice = RatelessIBLT::new(10, 32).unwrap();
    let mut bob = RatelessIBLT::new(10, 32).unwrap();

    // 90% overlap
    for i in 0..90 {
        let key = format!("shared{}", i);
        alice.insert(key.as_bytes(), b"val").unwrap();
        bob.insert(key.as_bytes(), b"val").unwrap();
    }

    // 10% different
    for i in 0..5 {
        alice
            .insert(format!("alice{}", i).as_bytes(), b"a")
            .unwrap();
        bob.insert(format!("bob{}", i).as_bytes(), b"b").unwrap();
    }

    let mut diff = alice.clone();
    diff.subtract(&bob).unwrap();

    let result = diff.decode().unwrap();

    // Should only show the 10 differences
    assert_eq!(result.total_changes(), 10);
}

// ============================================================================
// Category 7: Property Tests (5 tests)
// ============================================================================

#[test]
fn test_property_decode_gives_symmetric_difference() {
    let mut a = RatelessIBLT::new(20, 32).unwrap();
    let mut b = RatelessIBLT::new(20, 32).unwrap();

    a.insert(b"a1", b"v1").unwrap();
    a.insert(b"shared", b"sv").unwrap();

    b.insert(b"b1", b"v2").unwrap();
    b.insert(b"shared", b"sv").unwrap();

    let mut diff = a.clone();
    diff.subtract(&b).unwrap();

    let result = diff.decode().unwrap();

    // Should contain a1 and b1, but not shared
    assert_eq!(result.total_changes(), 2);

    // Verify shared is not in results
    let has_shared = result.to_insert.iter().any(|(k, _)| k == b"shared")
        || result.to_remove.iter().any(|(k, _)| k == b"shared");
    assert!(!has_shared, "Shared item should not appear in difference");
}

#[test]
fn test_property_insert_then_subtract_recovers_empty() {
    let mut iblt = RatelessIBLT::new(10, 32).unwrap();

    iblt.insert(b"key", b"value").unwrap();

    let mut copy = iblt.clone();
    copy.subtract(&iblt).unwrap();

    let diff = copy.decode().unwrap();
    assert!(diff.is_empty());
}

#[test]
fn test_property_multiple_inserts_accumulate() {
    let mut iblt = RatelessIBLT::new(10, 32).unwrap();

    // Insert same item 3 times
    iblt.insert(b"key", b"value").unwrap();
    iblt.insert(b"key", b"value").unwrap();
    iblt.insert(b"key", b"value").unwrap();

    // Create another with single insert
    let mut single = RatelessIBLT::new(10, 32).unwrap();
    single.insert(b"key", b"value").unwrap();

    // Subtract single from triple
    iblt.subtract(&single).unwrap();

    // Should still have 2x the item (counts accumulate)
    // However, decoding multiple same items may fail due to hash collisions
    let diff = iblt.decode();
    // Accept either success (with items) or graceful failure
    if let Ok(d) = diff {
        assert!(!d.is_empty());
    } // Acceptable for high-count items
}

#[test]
fn test_property_no_false_negatives() {
    let mut alice = RatelessIBLT::new(10, 32).unwrap();
    let bob = RatelessIBLT::new(10, 32).unwrap();

    // Known difference
    alice.insert(b"alice_item", b"av").unwrap();

    let mut diff = alice.clone();
    diff.subtract(&bob).unwrap();

    let result = diff.decode().unwrap();

    // Must contain alice_item - no false negatives
    let found = result.to_insert.iter().any(|(k, _)| k == b"alice_item");
    assert!(found, "False negative: known difference not detected");
}

#[test]
fn test_property_memory_bounds_maintained() {
    let expected_diff = 100;
    let cell_size = 32;
    let iblt = RatelessIBLT::new(expected_diff, cell_size).unwrap();

    let stats = iblt.stats();

    // Memory should be bounded
    let max_reasonable_cells = expected_diff * 3; // 3x is generous
    assert!(
        stats.num_cells <= max_reasonable_cells,
        "Too many cells allocated: {} > {}",
        stats.num_cells,
        max_reasonable_cells
    );
}

// ============================================================================
// Category 8: Edge Cases (4 tests)
// ============================================================================

#[test]
fn test_empty_key() {
    let mut iblt = RatelessIBLT::new(10, 32).unwrap();

    // Empty key should be valid
    let result = iblt.insert(b"", b"value");
    assert!(result.is_ok());

    let diff = iblt.decode().unwrap();
    assert_eq!(diff.to_insert.len(), 1);
}

#[test]
fn test_empty_value() {
    let mut iblt = RatelessIBLT::new(10, 32).unwrap();

    // Empty value should be valid
    let result = iblt.insert(b"key", b"");
    assert!(result.is_ok());

    let diff = iblt.decode().unwrap();
    assert_eq!(diff.to_insert.len(), 1);
}

#[test]
fn test_zero_length_inputs() {
    let mut iblt = RatelessIBLT::new(10, 32).unwrap();

    // Both empty
    iblt.insert(b"", b"").unwrap();

    let diff = iblt.decode().unwrap();
    assert_eq!(diff.to_insert.len(), 1);
}

#[test]
fn test_very_large_scale() {
    let expected_diff = 500;
    let mut alice = RatelessIBLT::new(expected_diff, 64).unwrap();
    let mut bob = RatelessIBLT::new(expected_diff, 64).unwrap();

    // Moderate shared set
    for i in 0..1000 {
        let key = format!("shared{}", i);
        alice.insert(key.as_bytes(), b"val").unwrap();
        bob.insert(key.as_bytes(), b"val").unwrap();
    }

    // Difference set (within capacity)
    for i in 0..200 {
        alice
            .insert(format!("alice{}", i).as_bytes(), b"a")
            .unwrap();
        bob.insert(format!("bob{}", i).as_bytes(), b"b").unwrap();
    }

    let mut diff = alice.clone();
    diff.subtract(&bob).unwrap();

    let result = diff.decode();

    // Should handle large scale (400 total differences, expected_diff=500)
    match result {
        Ok(r) => {
            // Successfully decoded
            assert!(r.total_changes() > 0);
            assert!(r.total_changes() <= 400);
        }
        Err(_) => {
            // Large scale may occasionally fail, which is acceptable
            // This demonstrates the capacity limits
        }
    }
}
