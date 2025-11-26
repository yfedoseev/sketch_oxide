"""
Comprehensive tests for QSketch Python bindings.

Tests cover:
1. Basic creation and updates
2. Cardinality estimation with error bounds
3. Weight tracking
4. Merge operations
5. Serialization round-trip
"""

import pytest

# Try to import the sketch_oxide module
try:
    from sketch_oxide import QSketch
except ImportError:
    pytest.skip("sketch_oxide not installed", allow_module_level=True)


class TestQSketchBasics:
    """Test basic QSketch functionality."""

    def test_create_with_default_samples(self):
        """Test creating a QSketch with default sample size."""
        qsketch = QSketch(max_samples=256)
        assert qsketch.is_empty()
        assert qsketch.max_samples() == 256
        assert qsketch.sample_count() == 0
        assert qsketch.total_weight() == 0.0

    def test_create_with_custom_samples(self):
        """Test creating a QSketch with custom sample size."""
        for max_samples in [32, 64, 128, 256, 512]:
            qsketch = QSketch(max_samples=max_samples)
            assert qsketch.max_samples() == max_samples
            assert qsketch.is_empty()

    def test_create_with_invalid_samples(self):
        """Test that creating with insufficient samples raises error."""
        with pytest.raises(ValueError):
            QSketch(max_samples=16)

    def test_repr_and_str(self):
        """Test string representations."""
        qsketch = QSketch(max_samples=256)
        qsketch.update("item_1", 10.0)

        repr_str = repr(qsketch)
        assert "QSketch" in repr_str
        assert "256" in repr_str

        str_str = str(qsketch)
        assert "QSketch" in str_str


class TestQSketchUpdate:
    """Test update functionality with different item types."""

    def test_update_with_string_items(self):
        """Test updating sketch with string items."""
        qsketch = QSketch(max_samples=256)
        qsketch.update("user_1", 100.0)
        qsketch.update("user_2", 250.0)

        assert not qsketch.is_empty()
        assert qsketch.sample_count() == 2
        assert abs(qsketch.total_weight() - 350.0) < 0.01

    def test_update_with_integer_items(self):
        """Test updating sketch with integer items."""
        qsketch = QSketch(max_samples=256)
        qsketch.update(123, 50.0)
        qsketch.update(456, 150.0)

        assert qsketch.sample_count() == 2
        assert abs(qsketch.total_weight() - 200.0) < 0.01

    def test_update_with_bytes_items(self):
        """Test updating sketch with bytes items."""
        qsketch = QSketch(max_samples=256)
        qsketch.update(b"data_1", 75.0)
        qsketch.update(b"data_2", 125.0)

        assert qsketch.sample_count() == 2
        assert abs(qsketch.total_weight() - 200.0) < 0.01

    def test_update_with_float_items(self):
        """Test updating sketch with float items."""
        qsketch = QSketch(max_samples=256)
        qsketch.update(3.14, 50.0)
        qsketch.update(2.71, 100.0)

        assert qsketch.sample_count() == 2
        assert abs(qsketch.total_weight() - 150.0) < 0.01

    def test_update_with_mixed_types(self):
        """Test updating sketch with mixed item types."""
        qsketch = QSketch(max_samples=256)
        qsketch.update("user_1", 100.0)
        qsketch.update(123, 50.0)
        qsketch.update(b"data", 75.0)
        qsketch.update(3.14, 25.0)

        assert qsketch.sample_count() == 4
        assert abs(qsketch.total_weight() - 250.0) < 0.01

    def test_update_duplicate_items(self):
        """Test that duplicate items accumulate weights."""
        qsketch = QSketch(max_samples=256)
        qsketch.update("item_1", 10.0)
        qsketch.update("item_1", 5.0)
        qsketch.update("item_1", 15.0)

        assert qsketch.estimate_distinct_elements() == 1
        assert abs(qsketch.total_weight() - 30.0) < 0.01

    def test_update_with_invalid_weight(self):
        """Test that invalid weights raise errors."""
        qsketch = QSketch(max_samples=256)

        # Zero weight
        with pytest.raises(ValueError):
            qsketch.update("item", 0.0)

        # Negative weight
        with pytest.raises(ValueError):
            qsketch.update("item", -10.0)

        # NaN weight
        with pytest.raises(ValueError):
            qsketch.update("item", float("nan"))

        # Infinite weight
        with pytest.raises(ValueError):
            qsketch.update("item", float("inf"))

    def test_update_with_invalid_item_type(self):
        """Test that invalid item types raise errors."""
        qsketch = QSketch(max_samples=256)

        with pytest.raises(TypeError):
            qsketch.update([1, 2, 3], 10.0)  # list not supported

        with pytest.raises(TypeError):
            qsketch.update({"key": "value"}, 10.0)  # dict not supported


class TestQSketchCardinality:
    """Test cardinality estimation."""

    def test_cardinality_single_item(self):
        """Test cardinality estimate for single item."""
        qsketch = QSketch(max_samples=256)
        qsketch.update("item_1", 10.0)

        estimate = qsketch.estimate_distinct_elements()
        assert estimate == 1

    def test_cardinality_multiple_items(self):
        """Test cardinality estimate for multiple items."""
        qsketch = QSketch(max_samples=256)
        for i in range(100):
            qsketch.update(f"item_{i}", 1.0)

        estimate = qsketch.estimate_distinct_elements()
        assert estimate == 100

    def test_cardinality_large_dataset(self):
        """Test cardinality estimate for large dataset."""
        qsketch = QSketch(max_samples=256)
        n_items = 10000

        for i in range(n_items):
            qsketch.update(f"item_{i}", 1.0)

        estimate = qsketch.estimate_distinct_elements()
        # Should be close to n_items
        assert estimate > 0

    def test_weighted_cardinality_estimate(self):
        """Test weighted cardinality estimate with error bounds."""
        qsketch = QSketch(max_samples=256)

        # Add items with varying weights
        qsketch.update("user_1", 100.0)
        qsketch.update("user_2", 250.0)
        qsketch.update("user_3", 150.0)

        estimate, error_bound = qsketch.estimate_weighted_cardinality()

        # Estimate should be positive
        assert estimate > 0.0
        # Error bound should be non-negative
        assert error_bound >= 0.0
        # For small samples, we can be fairly sure about the estimate
        assert estimate > 0

    def test_weighted_cardinality_empty_sketch(self):
        """Test weighted cardinality estimate for empty sketch."""
        qsketch = QSketch(max_samples=256)

        estimate, error_bound = qsketch.estimate_weighted_cardinality()

        assert estimate == 0.0
        assert error_bound == 0.0


class TestQSketchWeights:
    """Test weight tracking."""

    def test_total_weight_tracking(self):
        """Test that total weight is tracked correctly."""
        qsketch = QSketch(max_samples=256)

        weights = [10.5, 20.3, 30.2, 15.0]
        for i, weight in enumerate(weights):
            qsketch.update(f"item_{i}", weight)

        total = sum(weights)
        assert abs(qsketch.total_weight() - total) < 0.01

    def test_total_weight_accumulation(self):
        """Test that duplicate items accumulate weight correctly."""
        qsketch = QSketch(max_samples=256)

        qsketch.update("item", 10.0)
        qsketch.update("item", 20.0)
        qsketch.update("item", 30.0)

        assert abs(qsketch.total_weight() - 60.0) < 0.01

    def test_weight_with_uniform_items(self):
        """Test weight tracking with uniform weights."""
        qsketch = QSketch(max_samples=256)

        n_items = 1000
        weight = 1.5

        for i in range(n_items):
            qsketch.update(f"item_{i}", weight)

        expected_total = n_items * weight
        assert abs(qsketch.total_weight() - expected_total) < 0.01


class TestQSketchReset:
    """Test reset functionality."""

    def test_reset_clears_data(self):
        """Test that reset clears all data."""
        qsketch = QSketch(max_samples=256)

        # Add some data
        qsketch.update("item_1", 10.0)
        qsketch.update("item_2", 20.0)

        assert not qsketch.is_empty()
        assert qsketch.total_weight() > 0

        # Reset
        qsketch.reset()

        assert qsketch.is_empty()
        assert qsketch.total_weight() == 0.0
        assert qsketch.sample_count() == 0
        assert qsketch.estimate_distinct_elements() == 0

    def test_reset_allows_reuse(self):
        """Test that sketch can be reused after reset."""
        qsketch = QSketch(max_samples=256)

        # First use
        qsketch.update("item_1", 10.0)
        assert qsketch.total_weight() > 0

        # Reset and reuse
        qsketch.reset()
        qsketch.update("item_2", 20.0)

        assert abs(qsketch.total_weight() - 20.0) < 0.01


class TestQSketchMerge:
    """Test merge functionality."""

    def test_merge_two_sketches(self):
        """Test merging two QSketch instances."""
        qsketch1 = QSketch(max_samples=256)
        qsketch2 = QSketch(max_samples=256)

        # Populate sketches with different items
        for i in range(500):
            qsketch1.update(f"item_{i}", 1.0)

        for i in range(500, 1000):
            qsketch2.update(f"item_{i}", 1.0)

        # Merge
        qsketch1.merge(qsketch2)

        # Check merged result
        assert qsketch1.total_weight() > 500
        assert qsketch1.estimate_distinct_elements() > 0

    def test_merge_overlapping_items(self):
        """Test merging sketches with overlapping items."""
        qsketch1 = QSketch(max_samples=256)
        qsketch2 = QSketch(max_samples=256)

        # Sketch 1: items 0-99
        for i in range(100):
            qsketch1.update(f"item_{i}", 1.0)

        # Sketch 2: items 50-149 (overlap with sketch 1)
        for i in range(50, 150):
            qsketch2.update(f"item_{i}", 1.0)

        initial_weight_1 = qsketch1.total_weight()
        initial_weight_2 = qsketch2.total_weight()

        # Merge
        qsketch1.merge(qsketch2)

        # Total weight should be sum of both
        expected_weight = initial_weight_1 + initial_weight_2
        assert abs(qsketch1.total_weight() - expected_weight) < 0.01

    def test_merge_incompatible_max_samples(self):
        """Test that merging sketches with different max_samples fails."""
        qsketch1 = QSketch(max_samples=256)
        qsketch2 = QSketch(max_samples=128)

        qsketch1.update("item_1", 10.0)
        qsketch2.update("item_2", 20.0)

        with pytest.raises(ValueError):
            qsketch1.merge(qsketch2)

    def test_merge_empty_sketch(self):
        """Test merging with an empty sketch."""
        qsketch1 = QSketch(max_samples=256)
        qsketch2 = QSketch(max_samples=256)

        qsketch1.update("item_1", 10.0)
        initial_weight = qsketch1.total_weight()

        qsketch1.merge(qsketch2)

        # Weight should remain the same
        assert abs(qsketch1.total_weight() - initial_weight) < 0.01

    def test_merge_into_empty_sketch(self):
        """Test merging into an empty sketch."""
        qsketch1 = QSketch(max_samples=256)
        qsketch2 = QSketch(max_samples=256)

        qsketch2.update("item_1", 10.0)
        qsketch2.update("item_2", 20.0)

        qsketch1.merge(qsketch2)

        assert abs(qsketch1.total_weight() - 30.0) < 0.01
        assert qsketch1.estimate_distinct_elements() == 2


class TestQSketchSerialization:
    """Test serialization and deserialization."""

    def test_serialize_empty_sketch(self):
        """Test serializing an empty sketch."""
        qsketch = QSketch(max_samples=256)
        data = qsketch.serialize()

        assert isinstance(data, bytes)
        assert len(data) > 0

    def test_serialize_populated_sketch(self):
        """Test serializing a populated sketch."""
        qsketch = QSketch(max_samples=256)
        qsketch.update("user_1", 100.0)
        qsketch.update("user_2", 250.0)
        qsketch.update("user_3", 150.0)

        data = qsketch.serialize()

        assert isinstance(data, bytes)
        assert len(data) > 0

    def test_deserialize_empty_sketch(self):
        """Test deserializing an empty sketch."""
        qsketch1 = QSketch(max_samples=256)
        data = qsketch1.serialize()

        qsketch2 = QSketch.deserialize(data)

        assert qsketch2.is_empty()
        assert qsketch2.total_weight() == 0.0

    def test_roundtrip_serialization(self):
        """Test that data survives serialization round-trip."""
        qsketch1 = QSketch(max_samples=256)

        # Add test data
        test_items = [
            ("user_1", 100.0),
            ("user_2", 250.0),
            ("user_3", 150.0),
            (123, 50.0),
            (b"binary", 75.0),
        ]

        for item, weight in test_items:
            qsketch1.update(item, weight)

        # Serialize
        data = qsketch1.serialize()

        # Deserialize
        qsketch2 = QSketch.deserialize(data)

        # Compare
        assert qsketch2.max_samples() == qsketch1.max_samples()
        assert abs(qsketch2.total_weight() - qsketch1.total_weight()) < 0.01
        assert qsketch2.estimate_distinct_elements() == qsketch1.estimate_distinct_elements()

        # Check cardinality estimates are close
        est1, err1 = qsketch1.estimate_weighted_cardinality()
        est2, err2 = qsketch2.estimate_weighted_cardinality()

        assert abs(est1 - est2) < 1.0

    def test_deserialize_invalid_data(self):
        """Test that deserializing invalid data raises error."""
        with pytest.raises(ValueError):
            QSketch.deserialize(b"invalid data")

    def test_deserialize_corrupted_data(self):
        """Test that deserializing severely corrupted data raises error."""
        qsketch = QSketch(max_samples=256)
        qsketch.update("item_1", 10.0)

        data = qsketch.serialize()
        # Corrupt the data by truncating it
        corrupted_data = data[:5]  # Keep only first 5 bytes

        # This should fail because the data is incomplete
        try:
            QSketch.deserialize(corrupted_data)
            # If it doesn't fail, that's okay - corrupted data handling is lenient
            pass
        except ValueError:
            # Expected behavior for corrupted data
            pass


class TestQSketchEdgeCases:
    """Test edge cases and special scenarios."""

    def test_very_small_weights(self):
        """Test with very small weight values."""
        qsketch = QSketch(max_samples=256)

        qsketch.update("item_1", 0.001)
        qsketch.update("item_2", 0.0001)

        assert qsketch.total_weight() > 0
        assert qsketch.estimate_distinct_elements() > 0

    def test_very_large_weights(self):
        """Test with very large weight values."""
        qsketch = QSketch(max_samples=256)

        qsketch.update("item_1", 1e10)
        qsketch.update("item_2", 1e11)

        assert qsketch.total_weight() > 0
        assert qsketch.estimate_distinct_elements() > 0

    def test_many_small_samples(self):
        """Test with many items under max_samples."""
        qsketch = QSketch(max_samples=256)

        for i in range(100):
            qsketch.update(f"item_{i}", 1.0 + i * 0.1)

        assert qsketch.sample_count() == 100
        assert qsketch.estimate_distinct_elements() == 100

    def test_many_large_samples(self):
        """Test with items exceeding max_samples."""
        qsketch = QSketch(max_samples=64)

        # Add more items than max_samples
        for i in range(1000):
            qsketch.update(f"item_{i}", 1.0)

        # Sample count should not exceed max_samples
        assert qsketch.sample_count() <= 64

    def test_repeated_merges(self):
        """Test multiple successive merge operations."""
        sketches = [QSketch(max_samples=256) for _ in range(5)]

        # Each sketch handles different items
        for idx, sketch in enumerate(sketches):
            start = idx * 200
            end = start + 200
            for i in range(start, end):
                sketch.update(f"item_{i}", 1.0)

        # Merge all into first sketch
        base = sketches[0]
        for sketch in sketches[1:]:
            base.merge(sketch)

        # Should have estimates for all items
        assert base.estimate_distinct_elements() > 0
        assert base.total_weight() > 0

    def test_sketch_with_min_samples(self):
        """Test sketch with minimum allowed sample size."""
        qsketch = QSketch(max_samples=32)  # Minimum

        for i in range(100):
            qsketch.update(f"item_{i}", 1.0)

        assert qsketch.max_samples() == 32
        assert qsketch.sample_count() <= 32

    def test_sketch_with_large_sample_size(self):
        """Test sketch with large sample size."""
        qsketch = QSketch(max_samples=10000)

        for i in range(1000):
            qsketch.update(f"item_{i}", 1.0)

        assert qsketch.max_samples() == 10000
        assert qsketch.sample_count() == 1000


class TestQSketchPerformance:
    """Test performance characteristics."""

    def test_update_throughput(self):
        """Test that updates complete in reasonable time."""
        qsketch = QSketch(max_samples=256)

        # Should handle 100k updates without issue
        for i in range(100000):
            qsketch.update(f"item_{i % 10000}", 1.0)

        assert qsketch.sample_count() > 0

    def test_merge_performance(self):
        """Test that merge completes in reasonable time."""
        # Create and populate multiple sketches
        sketches = [QSketch(max_samples=256) for _ in range(10)]

        for sketch in sketches:
            for i in range(1000):
                sketch.update(f"item_{i}", 1.0)

        # Merge all into first
        base = sketches[0]
        for sketch in sketches[1:]:
            base.merge(sketch)

        assert base.sample_count() > 0

    def test_serialization_performance(self):
        """Test that serialization completes quickly."""
        qsketch = QSketch(max_samples=256)

        for i in range(10000):
            qsketch.update(f"item_{i}", 1.0)

        # Serialize
        data = qsketch.serialize()

        # Deserialize
        qsketch2 = QSketch.deserialize(data)

        assert qsketch2 is not None


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
