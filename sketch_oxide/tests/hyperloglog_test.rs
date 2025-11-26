//! Comprehensive tests for HyperLogLog cardinality estimation
//!
//! Tests cover:
//! - Construction and validation
//! - Basic operations (update, estimate)
//! - Accuracy and error bounds
//! - Merge operations
//! - Serialization/deserialization
//! - Edge cases
//! - Redis compatibility

use sketch_oxide::cardinality::HyperLogLog;
use sketch_oxide::{Mergeable, Sketch};

mod construction {
    use super::*;

    #[test]
    fn test_new_valid_precision() {
        for p in 4..=18 {
            let hll = HyperLogLog::new(p);
            assert!(hll.is_ok(), "Precision {} should be valid", p);
            let hll = hll.unwrap();
            assert_eq!(hll.precision(), p);
            assert_eq!(hll.num_registers(), 1 << p);
        }
    }

    #[test]
    fn test_new_invalid_precision_low() {
        for p in 0..4 {
            let hll = HyperLogLog::new(p);
            assert!(hll.is_err(), "Precision {} should be invalid", p);
        }
    }

    #[test]
    fn test_new_invalid_precision_high() {
        for p in 19..=25 {
            let hll = HyperLogLog::new(p);
            assert!(hll.is_err(), "Precision {} should be invalid", p);
        }
    }

    #[test]
    fn test_new_is_empty() {
        let hll = HyperLogLog::new(12).unwrap();
        assert!(hll.is_empty());
    }

    #[test]
    fn test_register_count() {
        let hll = HyperLogLog::new(12).unwrap();
        assert_eq!(hll.num_registers(), 4096);

        let hll = HyperLogLog::new(14).unwrap();
        assert_eq!(hll.num_registers(), 16384);
    }
}

mod basic_operations {
    use super::*;

    #[test]
    fn test_update_single() {
        let mut hll = HyperLogLog::new(12).unwrap();
        hll.update(&"hello");
        assert!(!hll.is_empty());
    }

    #[test]
    fn test_update_multiple_types() {
        let mut hll = HyperLogLog::new(12).unwrap();
        hll.update(&"string");
        hll.update(&42i32);
        hll.update(&314i64); // f64 doesn't implement Hash
        hll.update(&vec![1, 2, 3]);
        assert!(!hll.is_empty());
    }

    #[test]
    fn test_update_hash() {
        let mut hll = HyperLogLog::new(12).unwrap();
        hll.update_hash(0x123456789ABCDEF0);
        assert!(!hll.is_empty());
    }

    #[test]
    fn test_estimate_empty() {
        let hll = HyperLogLog::new(12).unwrap();
        let estimate = hll.estimate();
        assert!(estimate < 1.0, "Empty sketch should estimate ~0");
    }

    #[test]
    fn test_estimate_single() {
        let mut hll = HyperLogLog::new(12).unwrap();
        hll.update(&1);
        let estimate = hll.estimate();
        assert!(
            (0.5..=2.0).contains(&estimate),
            "Single item estimate {} should be ~1",
            estimate
        );
    }
}

mod accuracy {
    use super::*;

    #[test]
    fn test_accuracy_100() {
        let mut hll = HyperLogLog::new(12).unwrap();
        for i in 0..100 {
            hll.update(&i);
        }
        let estimate = hll.estimate();
        let error = (estimate - 100.0).abs() / 100.0;
        assert!(error < 0.15, "Error {} too high for n=100", error);
    }

    #[test]
    fn test_accuracy_1000() {
        let mut hll = HyperLogLog::new(12).unwrap();
        for i in 0..1000 {
            hll.update(&i);
        }
        let estimate = hll.estimate();
        let error = (estimate - 1000.0).abs() / 1000.0;
        assert!(error < 0.10, "Error {} too high for n=1000", error);
    }

    #[test]
    fn test_accuracy_10000() {
        let mut hll = HyperLogLog::new(12).unwrap();
        for i in 0..10_000 {
            hll.update(&i);
        }
        let estimate = hll.estimate();
        let error = (estimate - 10_000.0).abs() / 10_000.0;
        assert!(error < 0.05, "Error {} too high for n=10000", error);
    }

    #[test]
    fn test_accuracy_100000() {
        let mut hll = HyperLogLog::new(12).unwrap();
        for i in 0..100_000 {
            hll.update(&i);
        }
        let estimate = hll.estimate();
        let error = (estimate - 100_000.0).abs() / 100_000.0;
        assert!(error < 0.05, "Error {} too high for n=100000", error);
    }

    #[test]
    fn test_accuracy_higher_precision() {
        let mut hll = HyperLogLog::new(14).unwrap();
        for i in 0..10_000 {
            hll.update(&i);
        }
        let estimate = hll.estimate();
        let error = (estimate - 10_000.0).abs() / 10_000.0;
        // p=14 should be more accurate than p=12
        assert!(error < 0.03, "Error {} too high for p=14", error);
    }

    #[test]
    fn test_standard_error() {
        let hll = HyperLogLog::new(12).unwrap();
        let se = hll.standard_error();
        // 1.04 / sqrt(4096) ≈ 0.01625
        assert!(
            (se - 0.01625).abs() < 0.001,
            "Standard error {} unexpected",
            se
        );
    }

    #[test]
    fn test_standard_error_precision_14() {
        let hll = HyperLogLog::new(14).unwrap();
        let se = hll.standard_error();
        // 1.04 / sqrt(16384) ≈ 0.00813
        assert!(
            (se - 0.00813).abs() < 0.001,
            "Standard error {} unexpected",
            se
        );
    }
}

mod duplicates {
    use super::*;

    #[test]
    fn test_duplicate_items_same_estimate() {
        let mut hll = HyperLogLog::new(12).unwrap();
        for _ in 0..1000 {
            hll.update(&"same_item");
        }
        let estimate = hll.estimate();
        assert!(
            estimate < 2.0,
            "1000 duplicate items should estimate ~1, got {}",
            estimate
        );
    }

    #[test]
    fn test_mixed_duplicates() {
        let mut hll = HyperLogLog::new(12).unwrap();
        // Add 100 unique items, each 10 times
        for i in 0..100 {
            for _ in 0..10 {
                hll.update(&i);
            }
        }
        let estimate = hll.estimate();
        let error = (estimate - 100.0).abs() / 100.0;
        assert!(
            error < 0.15,
            "100 unique items (repeated 10x) should estimate ~100, got {}",
            estimate
        );
    }
}

mod merge {
    use super::*;

    #[test]
    fn test_merge_basic() {
        let mut hll1 = HyperLogLog::new(12).unwrap();
        let mut hll2 = HyperLogLog::new(12).unwrap();

        for i in 0..500 {
            hll1.update(&i);
        }
        for i in 500..1000 {
            hll2.update(&i);
        }

        hll1.merge(&hll2).unwrap();
        let estimate = hll1.estimate();
        let error = (estimate - 1000.0).abs() / 1000.0;
        assert!(
            error < 0.10,
            "Merged estimate {} too far from 1000",
            estimate
        );
    }

    #[test]
    fn test_merge_overlapping() {
        let mut hll1 = HyperLogLog::new(12).unwrap();
        let mut hll2 = HyperLogLog::new(12).unwrap();

        for i in 0..1000 {
            hll1.update(&i);
        }
        for i in 500..1500 {
            hll2.update(&i);
        }

        hll1.merge(&hll2).unwrap();
        let estimate = hll1.estimate();
        let error = (estimate - 1500.0).abs() / 1500.0;
        assert!(
            error < 0.10,
            "Merged overlapping estimate {} too far from 1500",
            estimate
        );
    }

    #[test]
    fn test_merge_empty() {
        let mut hll1 = HyperLogLog::new(12).unwrap();
        let hll2 = HyperLogLog::new(12).unwrap();

        for i in 0..1000 {
            hll1.update(&i);
        }

        let estimate_before = hll1.estimate();
        hll1.merge(&hll2).unwrap();
        let estimate_after = hll1.estimate();

        assert!(
            (estimate_before - estimate_after).abs() < 1.0,
            "Merging empty should not change estimate"
        );
    }

    #[test]
    fn test_merge_into_empty() {
        let mut hll1 = HyperLogLog::new(12).unwrap();
        let mut hll2 = HyperLogLog::new(12).unwrap();

        for i in 0..1000 {
            hll2.update(&i);
        }

        hll1.merge(&hll2).unwrap();
        let estimate = hll1.estimate();
        let error = (estimate - 1000.0).abs() / 1000.0;
        assert!(
            error < 0.10,
            "Merged into empty estimate {} unexpected",
            estimate
        );
    }

    #[test]
    fn test_merge_precision_mismatch() {
        let mut hll1 = HyperLogLog::new(10).unwrap();
        let hll2 = HyperLogLog::new(12).unwrap();

        let result = hll1.merge(&hll2);
        assert!(result.is_err(), "Different precisions should fail to merge");
    }

    #[test]
    fn test_merge_multiple() {
        let mut hlls: Vec<HyperLogLog> = (0..10).map(|_| HyperLogLog::new(12).unwrap()).collect();

        // Each sketch gets 100 items
        for (idx, hll) in hlls.iter_mut().enumerate() {
            for i in 0..100 {
                hll.update(&(idx * 100 + i));
            }
        }

        // Merge all into the first
        let (first, rest) = hlls.split_at_mut(1);
        for hll in rest.iter() {
            first[0].merge(hll).unwrap();
        }

        let estimate = first[0].estimate();
        let error = (estimate - 1000.0).abs() / 1000.0;
        assert!(
            error < 0.10,
            "Merged 10 sketches estimate {} too far from 1000",
            estimate
        );
    }
}

mod serialization {
    use super::*;

    #[test]
    fn test_to_bytes_from_bytes() {
        let mut hll = HyperLogLog::new(12).unwrap();
        for i in 0..1000 {
            hll.update(&i);
        }

        let bytes = hll.to_bytes();
        let restored = HyperLogLog::from_bytes(&bytes).unwrap();

        assert_eq!(hll.precision(), restored.precision());
        assert_eq!(hll.registers(), restored.registers());
    }

    #[test]
    fn test_serialize_deserialize_empty() {
        let hll = HyperLogLog::new(12).unwrap();
        let bytes = hll.serialize();
        let restored = HyperLogLog::deserialize(&bytes).unwrap();

        assert!(restored.is_empty());
        assert_eq!(hll.precision(), restored.precision());
    }

    #[test]
    fn test_bytes_length() {
        let hll = HyperLogLog::new(12).unwrap();
        let bytes = hll.to_bytes();
        // 1 byte precision + 4096 registers
        assert_eq!(bytes.len(), 1 + 4096);
    }

    #[test]
    fn test_deserialize_invalid_precision() {
        let mut bytes = vec![20u8]; // Invalid precision
        bytes.extend_from_slice(&[0u8; 1 << 20]); // Won't matter
        let result = HyperLogLog::from_bytes(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_truncated() {
        let bytes = vec![12u8]; // Only precision, no registers
        let result = HyperLogLog::from_bytes(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_empty_bytes() {
        let result = HyperLogLog::from_bytes(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_estimates_equal_after_serialization() {
        let mut hll = HyperLogLog::new(12).unwrap();
        for i in 0..5000 {
            hll.update(&i);
        }

        let estimate_before = hll.estimate();
        let bytes = hll.serialize();
        let restored = HyperLogLog::deserialize(&bytes).unwrap();
        let estimate_after = restored.estimate();

        assert!(
            (estimate_before - estimate_after).abs() < 0.001,
            "Estimates should be identical after serialization"
        );
    }
}

mod redis_compatibility {
    use super::*;

    #[test]
    fn test_to_redis_bytes_header() {
        let hll = HyperLogLog::new(14).unwrap();
        let bytes = hll.to_redis_bytes();

        // Check HYLL header
        assert_eq!(&bytes[0..4], b"HYLL");
        // Check encoding (1 = dense)
        assert_eq!(bytes[4], 1);
    }

    #[test]
    fn test_from_redis_bytes_invalid_header() {
        let bytes = b"XXXX".to_vec();
        let result = HyperLogLog::from_redis_bytes(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_redis_bytes_too_short() {
        let bytes = b"HYL".to_vec();
        let result = HyperLogLog::from_redis_bytes(&bytes);
        assert!(result.is_err());
    }
}

mod edge_cases {
    use super::*;

    #[test]
    fn test_large_cardinality() {
        let mut hll = HyperLogLog::new(14).unwrap();
        for i in 0..1_000_000 {
            hll.update(&i);
        }
        let estimate = hll.estimate();
        let error = (estimate - 1_000_000.0).abs() / 1_000_000.0;
        assert!(error < 0.02, "Error {} too high for n=1M", error);
    }

    #[test]
    fn test_string_items() {
        let mut hll = HyperLogLog::new(12).unwrap();
        for i in 0..1000 {
            hll.update(&format!("user_{}", i));
        }
        let estimate = hll.estimate();
        let error = (estimate - 1000.0).abs() / 1000.0;
        assert!(error < 0.10, "String items error {} too high", error);
    }

    #[test]
    fn test_byte_array_items() {
        let mut hll = HyperLogLog::new(12).unwrap();
        for i in 0u32..1000 {
            hll.update(&i.to_le_bytes());
        }
        let estimate = hll.estimate();
        let error = (estimate - 1000.0).abs() / 1000.0;
        assert!(error < 0.10, "Byte array items error {} too high", error);
    }

    #[test]
    fn test_minimum_precision() {
        let mut hll = HyperLogLog::new(4).unwrap();
        for i in 0..100 {
            hll.update(&i);
        }
        // With only 16 registers, accuracy is poor but should still work
        let estimate = hll.estimate();
        assert!(estimate > 0.0, "Should produce some estimate");
    }

    #[test]
    fn test_maximum_precision() {
        let mut hll = HyperLogLog::new(18).unwrap();
        for i in 0..1000 {
            hll.update(&i);
        }
        let estimate = hll.estimate();
        let error = (estimate - 1000.0).abs() / 1000.0;
        // Maximum precision should be very accurate
        assert!(error < 0.02, "Max precision error {} too high", error);
    }
}

mod sketch_trait {
    use super::*;

    #[test]
    fn test_sketch_update() {
        let mut hll = HyperLogLog::new(12).unwrap();
        Sketch::update(&mut hll, &42u64);
        assert!(!hll.is_empty());
    }

    #[test]
    fn test_sketch_estimate() {
        let mut hll = HyperLogLog::new(12).unwrap();
        for i in 0u64..1000 {
            Sketch::update(&mut hll, &i);
        }
        let estimate = Sketch::estimate(&hll);
        let error = (estimate - 1000.0).abs() / 1000.0;
        assert!(error < 0.10);
    }

    #[test]
    fn test_sketch_is_empty() {
        let hll = HyperLogLog::new(12).unwrap();
        assert!(Sketch::is_empty(&hll));
    }

    #[test]
    fn test_sketch_serialize_deserialize() {
        let mut hll = HyperLogLog::new(12).unwrap();
        for i in 0u64..100 {
            Sketch::update(&mut hll, &i);
        }

        let bytes = Sketch::serialize(&hll);
        let restored = HyperLogLog::deserialize(&bytes).unwrap();

        assert_eq!(hll.precision(), restored.precision());
    }
}

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_estimate_non_negative(n in 0usize..10000) {
            let mut hll = HyperLogLog::new(12).unwrap();
            for i in 0..n {
                hll.update(&i);
            }
            prop_assert!(hll.estimate() >= 0.0);
        }

        #[test]
        fn test_merge_commutative(
            items1 in prop::collection::vec(0u64..10000, 0..500),
            items2 in prop::collection::vec(0u64..10000, 0..500)
        ) {
            let mut hll1a = HyperLogLog::new(12).unwrap();
            let mut hll1b = HyperLogLog::new(12).unwrap();
            let mut hll2a = HyperLogLog::new(12).unwrap();
            let mut hll2b = HyperLogLog::new(12).unwrap();

            for &item in &items1 {
                hll1a.update(&item);
                hll1b.update(&item);
            }
            for &item in &items2 {
                hll2a.update(&item);
                hll2b.update(&item);
            }

            // Merge in different orders
            hll1a.merge(&hll2a).unwrap();
            hll2b.merge(&hll1b).unwrap();

            // Results should be very close (floating point)
            let diff = (hll1a.estimate() - hll2b.estimate()).abs();
            prop_assert!(diff < 1.0, "Merge should be commutative");
        }

        #[test]
        fn test_serialization_roundtrip(items in prop::collection::vec(0u64..10000, 0..1000)) {
            let mut hll = HyperLogLog::new(12).unwrap();
            for item in items {
                hll.update(&item);
            }

            let bytes = hll.to_bytes();
            let restored = HyperLogLog::from_bytes(&bytes).unwrap();

            prop_assert_eq!(hll.precision(), restored.precision());
            prop_assert_eq!(hll.registers(), restored.registers());
        }
    }
}
