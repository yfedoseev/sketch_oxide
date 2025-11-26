//! TDD Tests for Phase 1: New Trait Hierarchy
//!
//! This test suite is written BEFORE implementation to define the contract
//! and expected behavior of the new traits.

use sketch_oxide::common::{Result, SketchError};
use std::hash::Hash;

// Import types that will be created
use sketch_oxide::common::{RangeFilter, Reconcilable, SetDifference, WindowedSketch};

/// Test that RangeFilter trait can be implemented
#[test]
fn test_range_filter_trait_exists() {
    // This test verifies the RangeFilter trait compiles and can be used
    // We'll create a minimal mock implementation to test trait bounds

    struct MockRangeFilter {
        min: u64,
        max: u64,
    }

    impl RangeFilter for MockRangeFilter {
        fn may_contain_range(&self, low: u64, high: u64) -> bool {
            // Simple overlap check
            !(high < self.min || low > self.max)
        }
    }

    let filter = MockRangeFilter { min: 10, max: 100 };

    // Test range queries
    assert!(filter.may_contain_range(50, 60)); // Fully contained
    assert!(filter.may_contain_range(5, 15)); // Partial overlap
    assert!(!filter.may_contain_range(200, 300)); // No overlap
}

/// Test that RangeFilter works with trait objects
#[test]
fn test_range_filter_trait_object() {
    struct SimpleRangeFilter {
        range: (u64, u64),
    }

    impl RangeFilter for SimpleRangeFilter {
        fn may_contain_range(&self, low: u64, high: u64) -> bool {
            !(high < self.range.0 || low > self.range.1)
        }
    }

    let filter = SimpleRangeFilter { range: (0, 100) };
    let trait_obj: &dyn RangeFilter = &filter;

    assert!(trait_obj.may_contain_range(10, 20));
    assert!(!trait_obj.may_contain_range(200, 300));
}

/// Test that Reconcilable trait can be implemented
#[test]
fn test_reconcilable_trait_exists() {
    #[derive(Clone, PartialEq, Debug)]
    struct MockReconcilable {
        items: Vec<Vec<u8>>,
    }

    impl Reconcilable for MockReconcilable {
        fn subtract(&mut self, other: &Self) -> Result<()> {
            // Remove items present in other
            self.items.retain(|item| !other.items.contains(item));
            Ok(())
        }

        fn decode(&self) -> Result<SetDifference> {
            Ok(SetDifference {
                to_insert: self.items.clone(),
                to_remove: Vec::new(),
            })
        }
    }

    let mut set1 = MockReconcilable {
        items: vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()],
    };
    let set2 = MockReconcilable {
        items: vec![b"b".to_vec()],
    };

    set1.subtract(&set2).unwrap();
    assert_eq!(set1.items.len(), 2);
    assert!(set1.items.contains(&b"a".to_vec()));
    assert!(set1.items.contains(&b"c".to_vec()));
}

/// Test SetDifference type
#[test]
fn test_set_difference_type() {
    let diff = SetDifference {
        to_insert: vec![b"new1".to_vec(), b"new2".to_vec()],
        to_remove: vec![b"old1".to_vec()],
    };

    assert_eq!(diff.to_insert.len(), 2);
    assert_eq!(diff.to_remove.len(), 1);
    assert_eq!(diff.to_insert[0], b"new1");
    assert_eq!(diff.to_remove[0], b"old1");
}

/// Test that Reconcilable can be used with Result types
#[test]
fn test_reconcilable_error_handling() {
    #[derive(Clone)]
    struct FailingReconcilable;

    impl Reconcilable for FailingReconcilable {
        fn subtract(&mut self, _other: &Self) -> Result<()> {
            Err(SketchError::InvalidParameter {
                param: "test".to_string(),
                value: "test".to_string(),
                constraint: "test constraint".to_string(),
            })
        }

        fn decode(&self) -> Result<SetDifference> {
            Err(SketchError::DeserializationError(
                "Cannot decode empty reconcilable".to_string(),
            ))
        }
    }

    let mut failing = FailingReconcilable;
    let other = FailingReconcilable;

    assert!(failing.subtract(&other).is_err());
    assert!(failing.decode().is_err());
}

/// Test that WindowedSketch trait can be implemented
#[test]
fn test_windowed_sketch_trait_exists() {
    #[derive(Hash, Clone)]
    struct Item {
        value: u64,
    }

    struct MockWindowedSketch {
        items: Vec<(u64, u64)>, // (item_hash, timestamp)
    }

    impl WindowedSketch for MockWindowedSketch {
        type Item = Item;

        fn update_with_timestamp(&mut self, item: Self::Item, timestamp: u64) {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::Hasher;

            let mut hasher = DefaultHasher::new();
            item.hash(&mut hasher);
            let hash = hasher.finish();

            self.items.push((hash, timestamp));
        }

        fn estimate_window(&self, current_time: u64, window_seconds: u64) -> f64 {
            let cutoff = current_time.saturating_sub(window_seconds);
            self.items.iter().filter(|(_, ts)| *ts >= cutoff).count() as f64
        }
    }

    let mut sketch = MockWindowedSketch { items: Vec::new() };

    sketch.update_with_timestamp(Item { value: 1 }, 100);
    sketch.update_with_timestamp(Item { value: 2 }, 200);
    sketch.update_with_timestamp(Item { value: 3 }, 300);

    // Count items in last 150 seconds from time 250
    // Cutoff = 250 - 150 = 100, so items at 100, 200, 300 are all included
    assert_eq!(sketch.estimate_window(250, 150), 3.0);

    // Count items in last 500 seconds from time 500
    assert_eq!(sketch.estimate_window(500, 500), 3.0);
}

/// Test WindowedSketch with trait object
#[test]
fn test_windowed_sketch_trait_object() {
    #[derive(Hash, Clone)]
    struct SimpleItem {
        id: u32,
    }

    struct SimpleWindowedSketch {
        count: u64,
    }

    impl WindowedSketch for SimpleWindowedSketch {
        type Item = SimpleItem;

        fn update_with_timestamp(&mut self, _item: Self::Item, _timestamp: u64) {
            self.count += 1;
        }

        fn estimate_window(&self, _current_time: u64, _window_seconds: u64) -> f64 {
            self.count as f64
        }
    }

    let mut sketch = SimpleWindowedSketch { count: 0 };
    sketch.update_with_timestamp(SimpleItem { id: 1 }, 100);
    sketch.update_with_timestamp(SimpleItem { id: 2 }, 200);

    assert_eq!(sketch.estimate_window(300, 100), 2.0);
}

/// Test that traits can coexist with existing Sketch trait
#[test]
fn test_trait_coexistence() {
    // Verify that new traits don't conflict with existing traits
    // This is a compilation test - if it compiles, traits coexist properly

    use sketch_oxide::Sketch;

    #[derive(Clone)]
    struct MultiTraitSketch {
        data: Vec<u8>,
    }

    // Implement existing Sketch trait
    impl Sketch for MultiTraitSketch {
        type Item = u64;

        fn update(&mut self, item: &Self::Item) {
            self.data.extend_from_slice(&item.to_le_bytes());
        }

        fn estimate(&self) -> f64 {
            self.data.len() as f64
        }

        fn is_empty(&self) -> bool {
            self.data.is_empty()
        }

        fn serialize(&self) -> Vec<u8> {
            self.data.clone()
        }

        fn deserialize(bytes: &[u8]) -> Result<Self> {
            Ok(Self {
                data: bytes.to_vec(),
            })
        }
    }

    // Also implement new RangeFilter trait
    impl RangeFilter for MultiTraitSketch {
        fn may_contain_range(&self, _low: u64, _high: u64) -> bool {
            !self.data.is_empty()
        }
    }

    let mut sketch = MultiTraitSketch { data: Vec::new() };
    sketch.update(&42);

    assert_eq!(sketch.estimate(), 8.0);
    assert!(sketch.may_contain_range(0, 100));
}

/// Test that SetDifference can be cloned and debugged
#[test]
fn test_set_difference_traits() {
    let diff1 = SetDifference {
        to_insert: vec![b"a".to_vec()],
        to_remove: vec![b"b".to_vec()],
    };

    // Test Clone
    let diff2 = diff1.clone();
    assert_eq!(diff2.to_insert, diff1.to_insert);
    assert_eq!(diff2.to_remove, diff1.to_remove);

    // Test Debug formatting
    let debug_str = format!("{:?}", diff1);
    assert!(debug_str.contains("SetDifference"));
}

/// Test edge cases for RangeFilter
#[test]
fn test_range_filter_edge_cases() {
    struct EdgeCaseFilter;

    impl RangeFilter for EdgeCaseFilter {
        fn may_contain_range(&self, low: u64, high: u64) -> bool {
            // Always return false for this test
            low > high // Invalid range
        }
    }

    let filter = EdgeCaseFilter;

    // Test with same low and high
    assert!(!filter.may_contain_range(10, 10));

    // Test with max values
    assert!(!filter.may_contain_range(u64::MAX, u64::MAX));
}

/// Test that Reconcilable works with generic types
#[test]
fn test_reconcilable_generic() {
    #[derive(Clone, PartialEq, Debug)]
    struct GenericReconcilable<T> {
        items: Vec<T>,
    }

    impl<T: Clone + PartialEq> Reconcilable for GenericReconcilable<T> {
        fn subtract(&mut self, other: &Self) -> Result<()> {
            self.items.retain(|item| !other.items.contains(item));
            Ok(())
        }

        fn decode(&self) -> Result<SetDifference> {
            // For this test, we'll return empty SetDifference
            Ok(SetDifference {
                to_insert: Vec::new(),
                to_remove: Vec::new(),
            })
        }
    }

    let mut set1 = GenericReconcilable {
        items: vec![1, 2, 3, 4],
    };
    let set2 = GenericReconcilable { items: vec![2, 4] };

    set1.subtract(&set2).unwrap();
    assert_eq!(set1.items, vec![1, 3]);
}

/// Test documentation examples compile
#[test]
fn test_trait_documentation_examples() {
    // This test verifies that the documentation examples are valid
    // and demonstrate proper usage patterns

    // Example 1: RangeFilter usage
    struct DocumentedRangeFilter {
        min: u64,
        max: u64,
    }

    impl RangeFilter for DocumentedRangeFilter {
        fn may_contain_range(&self, low: u64, high: u64) -> bool {
            !(high < self.min || low > self.max)
        }
    }

    let filter = DocumentedRangeFilter { min: 100, max: 200 };
    assert!(filter.may_contain_range(150, 175));

    // Example 2: WindowedSketch usage
    #[derive(Hash, Clone)]
    struct Event {
        id: u64,
    }

    struct EventCounter {
        events: Vec<(u64, u64)>,
    }

    impl WindowedSketch for EventCounter {
        type Item = Event;

        fn update_with_timestamp(&mut self, item: Self::Item, timestamp: u64) {
            self.events.push((item.id, timestamp));
        }

        fn estimate_window(&self, current_time: u64, window_seconds: u64) -> f64 {
            let cutoff = current_time.saturating_sub(window_seconds);
            self.events.iter().filter(|(_, ts)| *ts >= cutoff).count() as f64
        }
    }

    let mut counter = EventCounter { events: Vec::new() };
    counter.update_with_timestamp(Event { id: 1 }, 1000);
    assert_eq!(counter.estimate_window(2000, 1500), 1.0);
}
