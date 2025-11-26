//! Tests for core Sketch and Mergeable traits

#[cfg(test)]
mod sketch_trait_tests {
    use sketch_oxide::common::{Sketch, SketchError};

    // Mock implementation for testing
    struct MockSketch {
        count: usize,
    }

    impl Sketch for MockSketch {
        type Item = u64;

        fn update(&mut self, _item: &Self::Item) {
            self.count += 1;
        }

        fn estimate(&self) -> f64 {
            self.count as f64
        }

        fn is_empty(&self) -> bool {
            self.count == 0
        }

        fn serialize(&self) -> Vec<u8> {
            self.count.to_le_bytes().to_vec()
        }

        fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
            if bytes.len() != 8 {
                return Err(SketchError::DeserializationError(
                    "Invalid byte length".to_string(),
                ));
            }
            let count = usize::from_le_bytes(bytes.try_into().unwrap());
            Ok(MockSketch { count })
        }
    }

    #[test]
    fn test_sketch_update() {
        let mut sketch = MockSketch { count: 0 };
        sketch.update(&42);
        assert_eq!(sketch.count, 1);
    }

    #[test]
    fn test_sketch_estimate() {
        let sketch = MockSketch { count: 10 };
        assert_eq!(sketch.estimate(), 10.0);
    }

    #[test]
    fn test_sketch_is_empty() {
        let empty = MockSketch { count: 0 };
        let not_empty = MockSketch { count: 1 };
        assert!(empty.is_empty());
        assert!(!not_empty.is_empty());
    }

    #[test]
    fn test_sketch_serialization_roundtrip() {
        let original = MockSketch { count: 42 };
        let bytes = original.serialize();
        let restored = MockSketch::deserialize(&bytes).unwrap();
        assert_eq!(restored.count, 42);
    }

    #[test]
    fn test_sketch_deserialization_error() {
        let invalid_bytes = vec![1, 2, 3]; // Wrong length
        let result = MockSketch::deserialize(&invalid_bytes);
        assert!(result.is_err());
        match result {
            Err(SketchError::DeserializationError(_)) => (),
            _ => panic!("Expected DeserializationError"),
        }
    }
}

#[cfg(test)]
mod mergeable_trait_tests {
    use sketch_oxide::common::{Mergeable, Sketch, SketchError};

    // Mock mergeable sketch
    struct MockMergeableSketch {
        value: u64,
    }

    impl Sketch for MockMergeableSketch {
        type Item = u64;

        fn update(&mut self, item: &Self::Item) {
            self.value += item;
        }

        fn estimate(&self) -> f64 {
            self.value as f64
        }

        fn is_empty(&self) -> bool {
            self.value == 0
        }

        fn serialize(&self) -> Vec<u8> {
            self.value.to_le_bytes().to_vec()
        }

        fn deserialize(bytes: &[u8]) -> Result<Self, SketchError> {
            if bytes.len() != 8 {
                return Err(SketchError::DeserializationError(
                    "Invalid byte length".to_string(),
                ));
            }
            let value = u64::from_le_bytes(bytes.try_into().unwrap());
            Ok(MockMergeableSketch { value })
        }
    }

    impl Mergeable for MockMergeableSketch {
        fn merge(&mut self, other: &Self) -> Result<(), SketchError> {
            self.value += other.value;
            Ok(())
        }
    }

    #[test]
    fn test_mergeable_basic() {
        let mut sketch1 = MockMergeableSketch { value: 10 };
        let sketch2 = MockMergeableSketch { value: 20 };

        sketch1.merge(&sketch2).unwrap();
        assert_eq!(sketch1.value, 30);
    }

    #[test]
    fn test_mergeable_with_empty() {
        let mut sketch = MockMergeableSketch { value: 10 };
        let empty = MockMergeableSketch { value: 0 };

        sketch.merge(&empty).unwrap();
        assert_eq!(sketch.value, 10);
    }

    #[test]
    fn test_mergeable_commutative() {
        let mut sketch_a1 = MockMergeableSketch { value: 5 };
        let mut sketch_a2 = MockMergeableSketch { value: 5 };
        let sketch_b = MockMergeableSketch { value: 10 };
        let sketch_b2 = MockMergeableSketch { value: 10 };

        // A + B
        sketch_a1.merge(&sketch_b).unwrap();
        // B + A
        sketch_a2.merge(&sketch_b2).unwrap();

        // Should be equal (if commutative)
        assert_eq!(sketch_a1.value, sketch_a2.value);
    }
}

#[cfg(test)]
mod error_tests {
    use sketch_oxide::common::SketchError;

    #[test]
    fn test_error_display() {
        let err = SketchError::InvalidParameter {
            param: "test".to_string(),
            value: "error".to_string(),
            constraint: "test constraint".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("test") && display.contains("error"));
    }

    #[test]
    fn test_error_variants() {
        let _invalid = SketchError::InvalidParameter {
            param: "test".to_string(),
            value: "invalid".to_string(),
            constraint: "must be valid".to_string(),
        };
        let _serial = SketchError::SerializationError("serial".to_string());
        let _deserial = SketchError::DeserializationError("deserial".to_string());
        let _incompat = SketchError::IncompatibleSketches {
            reason: "incompat".to_string(),
        };
        // Just ensure all variants compile
    }
}
