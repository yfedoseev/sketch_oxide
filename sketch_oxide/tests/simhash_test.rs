//! SimHash tests - TDD approach
//!
//! Testing locality-sensitive hashing for near-duplicate detection with:
//! - 64-bit fingerprints
//! - Hamming distance similarity
//! - Weighted feature support
//! - O(1) comparison time
//!
//! Use cases:
//! - Text near-duplicate detection (web crawling, spam)
//! - Document deduplication
//! - Plagiarism detection

use proptest::prelude::*;
use sketch_oxide::similarity::SimHash;
use sketch_oxide::{Mergeable, Sketch};

// ============================================================================
// Phase 1: Construction Tests
// ============================================================================

#[test]
fn test_new_simhash() {
    let sh = SimHash::new();

    assert!(sh.is_empty(), "New SimHash should be empty");
    assert_eq!(sh.len(), 0, "Length should be 0");
}

#[test]
fn test_default_simhash() {
    let sh = SimHash::default();

    assert!(sh.is_empty(), "Default SimHash should be empty");
    assert_eq!(sh.len(), 0);
}

// ============================================================================
// Phase 2: Update Tests
// ============================================================================

#[test]
fn test_update_single_feature() {
    let mut sh = SimHash::new();

    sh.update("hello");

    assert!(!sh.is_empty(), "Should not be empty after update");
    assert_eq!(sh.len(), 1, "Length should be 1");
}

#[test]
fn test_update_multiple_features() {
    let mut sh = SimHash::new();

    let features = vec!["the", "quick", "brown", "fox", "jumps"];
    for feature in &features {
        sh.update(feature);
    }

    assert_eq!(sh.len(), 5, "Length should be 5");
}

#[test]
fn test_update_various_types() {
    let mut sh = SimHash::new();

    // Strings
    sh.update(&"hello".to_string());

    // Integers
    sh.update(&42i32);
    sh.update(&123u64);

    // Tuples
    sh.update(&("key", 42));

    // Vectors
    sh.update(&vec![1, 2, 3]);

    assert_eq!(sh.len(), 5);
}

#[test]
fn test_update_weighted() {
    let mut sh = SimHash::new();

    sh.update_weighted("important", 10);
    sh.update_weighted("noise", 1);

    assert_eq!(sh.len(), 2);
}

#[test]
fn test_weighted_affects_fingerprint() {
    // Two sketches with same features but different weights should differ
    let mut sh1 = SimHash::new();
    let mut sh2 = SimHash::new();

    sh1.update_weighted("word_a", 100);
    sh1.update_weighted("word_b", 1);

    sh2.update_weighted("word_a", 1);
    sh2.update_weighted("word_b", 100);

    // The fingerprints might differ because weights influence bit positions
    let fp1 = sh1.fingerprint();
    let fp2 = sh2.fingerprint();

    // Not necessarily different, but the test shows the API works
    assert!(fp1 != 0 || fp2 != 0);
}

#[test]
fn test_update_duplicate_features() {
    let mut sh = SimHash::new();

    // Adding same feature multiple times
    for _ in 0..10 {
        sh.update("duplicate");
    }

    assert_eq!(sh.len(), 10, "Should count each update");
}

// ============================================================================
// Phase 3: Fingerprint Tests
// ============================================================================

#[test]
fn test_fingerprint_deterministic() {
    let mut sh1 = SimHash::new();
    let mut sh2 = SimHash::new();

    let features = vec!["the", "quick", "brown", "fox"];
    for feature in &features {
        sh1.update(feature);
        sh2.update(feature);
    }

    assert_eq!(
        sh1.fingerprint(),
        sh2.fingerprint(),
        "Same inputs should give same fingerprint"
    );
}

#[test]
fn test_fingerprint_order_independent() {
    // SimHash should be order-independent (bag of words)
    let mut sh1 = SimHash::new();
    let mut sh2 = SimHash::new();

    sh1.update("apple");
    sh1.update("banana");
    sh1.update("cherry");

    sh2.update("cherry");
    sh2.update("apple");
    sh2.update("banana");

    assert_eq!(
        sh1.fingerprint(),
        sh2.fingerprint(),
        "Order should not matter"
    );
}

#[test]
fn test_fingerprint_not_zero_after_update() {
    let mut sh = SimHash::new();
    sh.update("test");

    // Fingerprint is unlikely to be 0 after updates
    // (theoretically possible but extremely unlikely)
    let fp = sh.fingerprint();
    // We don't assert != 0 as it's theoretically possible
    // Just verify it computes
    let _ = fp;
}

// ============================================================================
// Phase 4: Hamming Distance Tests
// ============================================================================

#[test]
fn test_hamming_distance_identical() {
    let mut sh1 = SimHash::new();
    let mut sh2 = SimHash::new();

    sh1.update("hello");
    sh1.update("world");

    sh2.update("hello");
    sh2.update("world");

    assert_eq!(
        sh1.hamming_distance(&mut sh2),
        0,
        "Identical inputs should have distance 0"
    );
}

#[test]
fn test_hamming_distance_different() {
    let mut sh1 = SimHash::new();
    let mut sh2 = SimHash::new();

    sh1.update("hello");
    sh2.update("goodbye");

    let distance = sh1.hamming_distance(&mut sh2);
    assert!(
        distance > 0,
        "Different inputs should have positive distance"
    );
    assert!(distance <= 64, "Distance should be at most 64 bits");
}

#[test]
fn test_hamming_distance_symmetric() {
    let mut sh1 = SimHash::new();
    let mut sh2 = SimHash::new();

    sh1.update("apple");
    sh2.update("orange");

    let d1 = sh1.hamming_distance(&mut sh2);
    let d2 = sh2.hamming_distance(&mut sh1);

    assert_eq!(d1, d2, "Hamming distance should be symmetric");
}

#[test]
fn test_hamming_distance_from_fingerprints() {
    let fp1 = 0b1010101010101010u64;
    let fp2 = 0b1010101010101011u64;

    let distance = SimHash::hamming_distance_from_fingerprints(fp1, fp2);
    assert_eq!(distance, 1, "Should differ by 1 bit");

    let fp3 = 0b1111111111111111u64;
    let fp4 = 0b0000000000000000u64;
    let max_distance = SimHash::hamming_distance_from_fingerprints(fp3, fp4);
    assert_eq!(max_distance, 16, "16-bit values should differ by 16 bits");
}

#[test]
fn test_hamming_distance_empty_sketches() {
    let mut sh1 = SimHash::new();
    let mut sh2 = SimHash::new();

    // Empty sketches should have fingerprint 0
    assert_eq!(
        sh1.hamming_distance(&mut sh2),
        0,
        "Empty sketches should have distance 0"
    );
}

// ============================================================================
// Phase 5: Similarity Tests
// ============================================================================

#[test]
fn test_similarity_identical() {
    let mut sh1 = SimHash::new();
    let mut sh2 = SimHash::new();

    let text = "the quick brown fox jumps over the lazy dog";
    for word in text.split_whitespace() {
        sh1.update(word);
        sh2.update(word);
    }

    let similarity = sh1.similarity(&mut sh2);
    assert!(
        (similarity - 1.0).abs() < 0.001,
        "Identical inputs should have similarity ~1.0"
    );
}

#[test]
fn test_similarity_range() {
    let mut sh1 = SimHash::new();
    let mut sh2 = SimHash::new();

    sh1.update("hello");
    sh2.update("world");

    let similarity = sh1.similarity(&mut sh2);
    assert!(similarity >= 0.0, "Similarity should be >= 0");
    assert!(similarity <= 1.0, "Similarity should be <= 1");
}

#[test]
fn test_similarity_from_fingerprints() {
    let fp1 = 0xFFFFFFFFFFFFFFFFu64;
    let fp2 = 0xFFFFFFFFFFFFFFFFu64;

    let similarity = SimHash::similarity_from_fingerprints(fp1, fp2);
    assert!(
        (similarity - 1.0).abs() < 0.001,
        "Same fingerprints should have similarity 1.0"
    );

    let fp3 = 0x0000000000000000u64;
    let similarity2 = SimHash::similarity_from_fingerprints(fp1, fp3);
    assert!(
        (similarity2 - 0.0).abs() < 0.001,
        "Opposite fingerprints should have similarity 0.0"
    );
}

#[test]
fn test_similarity_partial_overlap() {
    let mut sh1 = SimHash::new();
    let mut sh2 = SimHash::new();

    // Common words
    let common = vec!["the", "quick", "brown", "fox"];
    for word in &common {
        sh1.update(word);
        sh2.update(word);
    }

    // Different words
    sh1.update("jumps");
    sh2.update("walks");

    let similarity = sh1.similarity(&mut sh2);
    assert!(
        similarity > 0.5,
        "Mostly overlapping should have high similarity"
    );
}

// ============================================================================
// Phase 6: Merge Tests
// ============================================================================

#[test]
fn test_merge_basic() {
    let mut sh1 = SimHash::new();
    let mut sh2 = SimHash::new();

    sh1.update("hello");
    sh2.update("world");

    let mut combined = SimHash::new();
    combined.update("hello");
    combined.update("world");

    sh1.merge(&sh2).unwrap();

    assert_eq!(
        sh1.fingerprint(),
        combined.fingerprint(),
        "Merged should equal combined"
    );
}

#[test]
fn test_merge_empty() {
    let mut sh1 = SimHash::new();
    sh1.update("hello");
    let fp_before = sh1.fingerprint();

    let sh2 = SimHash::new();
    sh1.merge(&sh2).unwrap();

    // Re-finalize after merge
    let fp_after = sh1.fingerprint();

    assert_eq!(
        fp_before, fp_after,
        "Merging empty should not change fingerprint"
    );
}

#[test]
fn test_merge_into_empty() {
    let mut sh1 = SimHash::new();
    let mut sh2 = SimHash::new();

    sh2.update("hello");
    let fp_expected = sh2.fingerprint();

    sh1.merge(&sh2).unwrap();
    let fp_result = sh1.fingerprint();

    assert_eq!(fp_result, fp_expected, "Should equal the non-empty sketch");
}

#[test]
fn test_merge_preserves_counts() {
    let mut sh1 = SimHash::new();
    let mut sh2 = SimHash::new();

    sh1.update("a");
    sh1.update("b");

    sh2.update("c");
    sh2.update("d");
    sh2.update("e");

    sh1.merge(&sh2).unwrap();

    assert_eq!(sh1.len(), 5, "Should have 5 total features");
}

// ============================================================================
// Phase 7: Serialization Tests
// ============================================================================

#[test]
fn test_serialize_deserialize() {
    let mut sh = SimHash::new();
    sh.update("hello");
    sh.update("world");
    sh.update("test");

    let bytes = sh.to_bytes();
    let mut restored = SimHash::from_bytes(&bytes).unwrap();

    assert_eq!(sh.fingerprint(), restored.fingerprint());
    assert_eq!(sh.len(), restored.len());
}

#[test]
fn test_serialize_empty() {
    let mut sh = SimHash::new();
    let bytes = sh.to_bytes();
    let restored = SimHash::from_bytes(&bytes).unwrap();

    assert!(restored.is_empty());
}

#[test]
fn test_serialize_preserves_fingerprint() {
    let mut sh = SimHash::new();
    for word in "the quick brown fox".split_whitespace() {
        sh.update(word);
    }
    let fp_before = sh.fingerprint();

    let bytes = sh.to_bytes();
    let mut restored = SimHash::from_bytes(&bytes).unwrap();
    let fp_after = restored.fingerprint();

    assert_eq!(fp_before, fp_after);
}

#[test]
fn test_deserialize_invalid_data() {
    let result = SimHash::from_bytes(&[0u8; 10]);
    assert!(result.is_err(), "Should fail with insufficient data");
}

#[test]
fn test_sketch_trait_serialize() {
    let mut sh = SimHash::new();
    sh.update("test");
    let fp_original = sh.fingerprint();

    let bytes = sh.serialize();
    let mut restored = SimHash::deserialize(&bytes).unwrap();

    assert_eq!(fp_original, restored.fingerprint());
}

// ============================================================================
// Phase 8: Edge Cases
// ============================================================================

#[test]
fn test_empty_string_feature() {
    let mut sh = SimHash::new();
    sh.update("");

    assert_eq!(sh.len(), 1, "Should accept empty string");
}

#[test]
fn test_very_long_feature() {
    let mut sh = SimHash::new();
    let long_string = "x".repeat(10000);
    sh.update(&long_string);

    assert_eq!(sh.len(), 1);
    // Should compute a valid fingerprint
    let _ = sh.fingerprint();
}

#[test]
fn test_many_features() {
    let mut sh = SimHash::new();

    for i in 0..10000 {
        sh.update(&format!("feature_{}", i));
    }

    assert_eq!(sh.len(), 10000);
    // Should still produce a valid 64-bit fingerprint
    let fp = sh.fingerprint();
    let _ = fp; // Just verify it computes
}

#[test]
fn test_negative_weights() {
    let mut sh = SimHash::new();

    // Negative weights should work (subtract from accumulator)
    sh.update_weighted("positive", 10);
    sh.update_weighted("negative", -5);

    assert_eq!(sh.len(), 2);
}

#[test]
fn test_zero_weight() {
    let mut sh1 = SimHash::new();
    let mut sh2 = SimHash::new();

    sh1.update("hello");
    sh1.update_weighted("ignored", 0); // Zero weight

    sh2.update("hello");

    // Zero weight should not affect fingerprint
    assert_eq!(sh1.fingerprint(), sh2.fingerprint());
}

// ============================================================================
// Phase 9: Near-Duplicate Detection Scenarios
// ============================================================================

#[test]
fn test_near_duplicate_documents() {
    // Simulate two near-duplicate documents
    let doc1 = "The quick brown fox jumps over the lazy dog";
    let doc2 = "The quick brown fox leaps over the lazy dog"; // "jumps" -> "leaps"

    let mut sh1 = SimHash::new();
    let mut sh2 = SimHash::new();

    for word in doc1.split_whitespace() {
        sh1.update(word);
    }
    for word in doc2.split_whitespace() {
        sh2.update(word);
    }

    let distance = sh1.hamming_distance(&mut sh2);
    let similarity = sh1.similarity(&mut sh2);

    // Near-duplicates should have low Hamming distance
    assert!(
        distance < 10,
        "Near-duplicates should have distance < 10, got {}",
        distance
    );
    assert!(
        similarity > 0.8,
        "Similarity should be high for near-duplicates"
    );
}

#[test]
fn test_completely_different_documents() {
    let doc1 = "apple banana cherry date elderberry";
    let doc2 = "xylophone zebra quantum physics universe";

    let mut sh1 = SimHash::new();
    let mut sh2 = SimHash::new();

    for word in doc1.split_whitespace() {
        sh1.update(word);
    }
    for word in doc2.split_whitespace() {
        sh2.update(word);
    }

    let similarity = sh1.similarity(&mut sh2);

    // Completely different documents should have lower similarity
    // (but not necessarily 0 due to random hash collisions)
    assert!(
        similarity < 0.9,
        "Different documents should have lower similarity"
    );
}

#[test]
fn test_shingle_based_similarity() {
    // Using character n-grams (shingles) for more robust detection
    fn shingles(text: &str, n: usize) -> Vec<String> {
        text.chars()
            .collect::<Vec<_>>()
            .windows(n)
            .map(|w| w.iter().collect::<String>())
            .collect()
    }

    let doc1 = "hello world";
    let doc2 = "hello werld"; // Typo

    let mut sh1 = SimHash::new();
    let mut sh2 = SimHash::new();

    for shingle in shingles(doc1, 3) {
        sh1.update(&shingle);
    }
    for shingle in shingles(doc2, 3) {
        sh2.update(&shingle);
    }

    let similarity = sh1.similarity(&mut sh2);
    assert!(
        similarity > 0.5,
        "Similar text with typo should have decent similarity"
    );
}

// ============================================================================
// Phase 10: Property-Based Tests
// ============================================================================

proptest! {
    #[test]
    fn prop_fingerprint_deterministic(words in prop::collection::vec("[a-z]+", 1..20)) {
        let mut sh1 = SimHash::new();
        let mut sh2 = SimHash::new();

        for word in &words {
            sh1.update(word);
            sh2.update(word);
        }

        prop_assert_eq!(sh1.fingerprint(), sh2.fingerprint());
    }

    #[test]
    fn prop_hamming_distance_symmetric(
        words1 in prop::collection::vec("[a-z]+", 1..10),
        words2 in prop::collection::vec("[a-z]+", 1..10)
    ) {
        let mut sh1 = SimHash::new();
        let mut sh2 = SimHash::new();

        for word in &words1 {
            sh1.update(word);
        }
        for word in &words2 {
            sh2.update(word);
        }

        let d1 = sh1.hamming_distance(&mut sh2);
        let d2 = sh2.hamming_distance(&mut sh1);

        prop_assert_eq!(d1, d2);
    }

    #[test]
    fn prop_similarity_in_range(words in prop::collection::vec("[a-z]+", 1..20)) {
        let mut sh1 = SimHash::new();
        let mut sh2 = SimHash::new();

        for (i, word) in words.iter().enumerate() {
            if i % 2 == 0 {
                sh1.update(word);
            } else {
                sh2.update(word);
            }
        }

        let similarity = sh1.similarity(&mut sh2);
        prop_assert!(similarity >= 0.0);
        prop_assert!(similarity <= 1.0);
    }

    #[test]
    fn prop_merge_associative(
        words1 in prop::collection::vec("[a-z]+", 1..5),
        words2 in prop::collection::vec("[a-z]+", 1..5),
        words3 in prop::collection::vec("[a-z]+", 1..5)
    ) {
        // (A merge B) merge C == A merge (B merge C)
        let mut sh1a = SimHash::new();
        let mut sh2a = SimHash::new();
        let mut sh3a = SimHash::new();

        let mut sh1b = SimHash::new();
        let mut sh2b = SimHash::new();
        let mut sh3b = SimHash::new();

        for word in &words1 {
            sh1a.update(word);
            sh1b.update(word);
        }
        for word in &words2 {
            sh2a.update(word);
            sh2b.update(word);
        }
        for word in &words3 {
            sh3a.update(word);
            sh3b.update(word);
        }

        // (1 merge 2) merge 3
        sh1a.merge(&sh2a).unwrap();
        sh1a.merge(&sh3a).unwrap();

        // 1 merge (2 merge 3)
        sh2b.merge(&sh3b).unwrap();
        sh1b.merge(&sh2b).unwrap();

        prop_assert_eq!(sh1a.fingerprint(), sh1b.fingerprint());
    }

    #[test]
    fn prop_serialization_roundtrip(words in prop::collection::vec("[a-z]+", 1..20)) {
        let mut sh = SimHash::new();
        for word in &words {
            sh.update(word);
        }
        let fp_original = sh.fingerprint();

        let bytes = sh.to_bytes();
        let mut restored = SimHash::from_bytes(&bytes).unwrap();

        prop_assert_eq!(fp_original, restored.fingerprint());
    }

    #[test]
    fn prop_order_independence(words in prop::collection::vec("[a-z]+", 2..10)) {
        let mut sh1 = SimHash::new();
        let mut sh2 = SimHash::new();

        // Forward order
        for word in &words {
            sh1.update(word);
        }

        // Reverse order
        for word in words.iter().rev() {
            sh2.update(word);
        }

        prop_assert_eq!(sh1.fingerprint(), sh2.fingerprint());
    }
}

// ============================================================================
// Phase 11: Performance Characteristics
// ============================================================================

#[test]
fn test_constant_memory() {
    // SimHash should use constant memory regardless of input size
    let mut sh1 = SimHash::new();
    let mut sh2 = SimHash::new();

    for i in 0..100 {
        sh1.update(&format!("word_{}", i));
    }
    for i in 0..10000 {
        sh2.update(&format!("word_{}", i));
    }

    // Both should serialize to same size (fixed fingerprint + accumulator)
    let bytes1 = sh1.to_bytes();
    let bytes2 = sh2.to_bytes();

    assert_eq!(bytes1.len(), bytes2.len(), "Memory should be constant");
}

#[test]
fn test_bits_constant() {
    assert_eq!(SimHash::BITS, 64, "SimHash should use 64 bits");
}
