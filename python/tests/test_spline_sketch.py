"""Comprehensive tests for SplineSketch Python bindings."""

import pytest

from sketch_oxide import SplineSketch


class TestSplineSketchBasic:
    """Basic functionality tests for SplineSketch."""

    def test_create_default(self):
        """Test creating a SplineSketch with default parameters."""
        sketch = SplineSketch()
        assert sketch.is_empty()
        assert sketch.sample_count() == 0
        assert sketch.total_weight() == 0.0
        assert sketch.max_samples() == 200

    def test_create_custom_max_samples(self):
        """Test creating a SplineSketch with custom max_samples."""
        sketch = SplineSketch(max_samples=500)
        assert sketch.max_samples() == 500
        assert sketch.is_empty()

    def test_update_single_value(self):
        """Test updating with a single unweighted value."""
        sketch = SplineSketch(200)
        sketch.update(100, 1.0)
        assert not sketch.is_empty()
        assert sketch.sample_count() == 1
        assert sketch.total_weight() == 1.0
        assert sketch.min() == 100
        assert sketch.max() == 100

    def test_update_weighted_value(self):
        """Test updating with weighted values."""
        sketch = SplineSketch(200)
        sketch.update(100, 2.0)
        assert sketch.sample_count() == 1
        assert sketch.total_weight() == 2.0

    def test_update_multiple_values(self):
        """Test adding multiple values."""
        sketch = SplineSketch(200)
        for i in range(10):
            sketch.update(i * 10, 1.0)
        assert sketch.sample_count() == 10
        assert sketch.total_weight() == 10.0
        assert sketch.min() == 0
        assert sketch.max() == 90

    def test_min_max_tracking(self):
        """Test that min and max are tracked correctly."""
        sketch = SplineSketch(200)
        sketch.update(50, 1.0)
        assert sketch.min() == 50
        assert sketch.max() == 50

        sketch.update(10, 1.0)
        assert sketch.min() == 10
        assert sketch.max() == 50

        sketch.update(100, 1.0)
        assert sketch.min() == 10
        assert sketch.max() == 100


class TestSplineSketchQuantiles:
    """Quantile query tests for SplineSketch."""

    def test_query_empty_raises_error(self):
        """Test that querying an empty sketch raises an error."""
        sketch = SplineSketch(200)
        with pytest.raises(RuntimeError):
            sketch.query(0.5)

    def test_query_single_value(self):
        """Test querying when only one value exists."""
        sketch = SplineSketch(200)
        sketch.update(42, 1.0)
        assert sketch.query(0.0) == 42
        assert sketch.query(0.5) == 42
        assert sketch.query(1.0) == 42

    def test_query_boundary_values(self):
        """Test querying at boundaries (0.0 and 1.0)."""
        sketch = SplineSketch(200)
        for i in range(1, 101):
            sketch.update(i, 1.0)

        assert sketch.query(0.0) == 1
        assert sketch.query(1.0) == 100

    def test_query_median(self):
        """Test that median query returns reasonable value."""
        sketch = SplineSketch(200)
        for i in range(1, 1001):
            sketch.update(i, 1.0)

        median = sketch.query(0.5)
        assert 400 < median < 600  # Should be near 500

    def test_query_percentiles(self):
        """Test querying various percentiles."""
        sketch = SplineSketch(300)
        for i in range(0, 1001):
            sketch.update(i, 1.0)

        q25 = sketch.query(0.25)
        q50 = sketch.query(0.50)
        q75 = sketch.query(0.75)
        q95 = sketch.query(0.95)

        # Should be roughly 250, 500, 750, 950
        assert 0 < q25 < 500
        assert q25 < q50 < q75 < q95
        assert 900 < q95 < 1000

    def test_query_monotonicity(self):
        """Test that quantiles are monotonically increasing."""
        sketch = SplineSketch(200)
        for i in range(0, 1001):
            sketch.update(i, 1.0)

        quantiles = [0.1, 0.25, 0.5, 0.75, 0.9]
        values = [sketch.query(q) for q in quantiles]

        for i in range(len(values) - 1):
            assert values[i] <= values[i + 1], f"Non-monotonic: {values}"

    def test_query_extreme_quantiles(self):
        """Test querying extreme quantiles."""
        sketch = SplineSketch(200)
        for i in range(1000):
            sketch.update(i, 1.0)

        # Values outside [0, 1] should be clamped
        q_low = sketch.query(-0.5)
        q_high = sketch.query(1.5)

        assert q_low == sketch.query(0.0)
        assert q_high == sketch.query(1.0)


class TestSplineSketchMerging:
    """Merge operation tests for SplineSketch."""

    def test_merge_empty_sketches(self):
        """Test merging two empty sketches."""
        sketch1 = SplineSketch(200)
        sketch2 = SplineSketch(200)
        sketch1.merge(sketch2)
        assert sketch1.is_empty()

    def test_merge_into_empty(self):
        """Test merging a non-empty sketch into an empty one."""
        sketch1 = SplineSketch(200)
        sketch2 = SplineSketch(200)

        for i in range(100):
            sketch2.update(i, 1.0)

        sketch1.merge(sketch2)
        assert sketch1.sample_count() > 0
        assert sketch1.min() == 0
        assert sketch1.max() == 99

    def test_merge_non_empty_sketches(self):
        """Test merging two non-empty sketches."""
        sketch1 = SplineSketch(200)
        sketch2 = SplineSketch(200)

        for i in range(0, 500):
            sketch1.update(i, 1.0)
        for i in range(500, 1000):
            sketch2.update(i, 1.0)

        sketch1.merge(sketch2)
        assert sketch1.min() == 0
        assert sketch1.max() == 999

    def test_merge_incompatible_sketches_raises_error(self):
        """Test that merging sketches with different max_samples raises error."""
        sketch1 = SplineSketch(200)
        sketch2 = SplineSketch(500)

        sketch1.update(10, 1.0)
        sketch2.update(20, 1.0)

        with pytest.raises(ValueError):
            sketch1.merge(sketch2)

    def test_merge_preserves_quantiles(self):
        """Test that merging preserves quantile accuracy."""
        sketch1 = SplineSketch(300)
        sketch2 = SplineSketch(300)

        # Create two sketches with different ranges
        for i in range(0, 500):
            sketch1.update(i, 1.0)
        for i in range(500, 1000):
            sketch2.update(i, 1.0)

        q50_before = sketch1.query(0.5)
        sketch1.merge(sketch2)
        q50_after = sketch1.query(0.5)

        # After merge with second half of data, median should shift up
        assert q50_after > q50_before


class TestSplineSketchReset:
    """Reset functionality tests."""

    def test_reset_clears_data(self):
        """Test that reset clears all data."""
        sketch = SplineSketch(200)
        for i in range(100):
            sketch.update(i, 1.0)

        assert not sketch.is_empty()
        sketch.reset()
        assert sketch.is_empty()

    def test_reset_clears_stats(self):
        """Test that reset clears statistics."""
        sketch = SplineSketch(200)
        sketch.update(100, 3.0)

        sketch.reset()
        assert sketch.sample_count() == 0
        assert sketch.total_weight() == 0.0
        assert sketch.min() is None
        assert sketch.max() is None

    def test_reset_allows_reuse(self):
        """Test that sketch can be reused after reset."""
        sketch = SplineSketch(200)
        sketch.update(100, 1.0)
        sketch.reset()

        # Should be able to add new data
        sketch.update(200, 1.0)
        assert sketch.sample_count() == 1
        assert sketch.min() == 200
        assert sketch.max() == 200


class TestSplineSketchSerialization:
    """Serialization/deserialization tests."""

    def test_serialize_empty_sketch(self):
        """Test serializing an empty sketch."""
        sketch = SplineSketch(200)
        data = sketch.serialize()
        assert isinstance(data, bytes)
        assert len(data) > 0

    def test_deserialize_empty_sketch(self):
        """Test deserializing an empty sketch."""
        sketch = SplineSketch(200)
        data = sketch.serialize()

        restored = SplineSketch.deserialize(data)
        assert restored.is_empty()
        assert restored.max_samples() == 200

    def test_serialize_deserialize_with_data(self):
        """Test round-trip serialization with data."""
        sketch = SplineSketch(200)
        for i in range(100):
            sketch.update(i, 1.0)

        data = sketch.serialize()
        restored = SplineSketch.deserialize(data)

        assert restored.sample_count() == sketch.sample_count()
        assert restored.total_weight() == sketch.total_weight()
        assert restored.min() == sketch.min()
        assert restored.max() == sketch.max()

    def test_deserialize_preserves_quantiles(self):
        """Test that deserialized sketch preserves quantile queries."""
        sketch = SplineSketch(200)
        for i in range(0, 1001):
            sketch.update(i, 1.0)

        original_q50 = sketch.query(0.5)
        original_q95 = sketch.query(0.95)

        data = sketch.serialize()
        restored = SplineSketch.deserialize(data)

        restored_q50 = restored.query(0.5)
        restored_q95 = restored.query(0.95)

        # Should be very close
        assert abs(original_q50 - restored_q50) < 10
        assert abs(original_q95 - restored_q95) < 10

    def test_deserialize_invalid_data_raises_error(self):
        """Test that deserializing invalid data raises error."""
        with pytest.raises(ValueError):
            SplineSketch.deserialize(b"invalid data")


class TestSplineSketchCompression:
    """Tests for compression when exceeding max_samples."""

    def test_compression_triggered_when_exceeding_max(self):
        """Test that compression is triggered when max_samples is exceeded."""
        sketch = SplineSketch(max_samples=100)

        # Add more values than max_samples
        for i in range(500):
            sketch.update(i, 1.0)

        # Should be compressed to at most max_samples + some margin
        assert sketch.sample_count() <= 150

    def test_compression_preserves_bounds(self):
        """Test that compression preserves min/max bounds."""
        sketch = SplineSketch(max_samples=50)

        for i in range(0, 1000):
            sketch.update(i, 1.0)

        assert sketch.min() == 0
        assert sketch.max() == 999

    def test_compression_preserves_quantiles(self):
        """Test that compression preserves quantile accuracy."""
        sketch = SplineSketch(max_samples=100)

        for i in range(0, 10000):
            sketch.update(i, 1.0)

        q50 = sketch.query(0.5)
        # For uniform 0-10000, q50 should be around 5000
        assert 4000 < q50 < 6000


class TestSplineSketchRepr:
    """String representation tests."""

    def test_repr(self):
        """Test __repr__ output."""
        sketch = SplineSketch(200)
        repr_str = repr(sketch)
        assert "SplineSketch" in repr_str
        assert "200" in repr_str

    def test_str(self):
        """Test __str__ output."""
        sketch = SplineSketch(200)
        str_str = str(sketch)
        assert "SplineSketch" in str_str

    def test_len(self):
        """Test __len__ returns sample count."""
        sketch = SplineSketch(200)
        sketch.update(100, 1.0)
        sketch.update(200, 1.0)
        assert len(sketch) == 2


class TestSplineSketchEdgeCases:
    """Edge case and stress tests."""

    def test_large_values(self):
        """Test with large u64 values."""
        sketch = SplineSketch(200)
        large_val = 2**63 - 1  # Max i64 as positive
        sketch.update(large_val, 1.0)
        assert sketch.max() == large_val

    def test_many_weighted_updates(self):
        """Test with many weighted updates."""
        sketch = SplineSketch(200)
        sketch.update(100, 100.0)
        sketch.update(200, 50.0)
        sketch.update(300, 30.0)

        total = 100.0 + 50.0 + 30.0
        assert abs(sketch.total_weight() - total) < 0.01

    def test_zero_weight_ignored(self):
        """Test that zero weight updates are ignored."""
        sketch = SplineSketch(200)
        sketch.update(100, 0.0)
        assert sketch.is_empty()

    def test_small_max_samples(self):
        """Test with very small max_samples (should be increased to minimum)."""
        sketch = SplineSketch(max_samples=5)
        # Should be increased to minimum (10)
        assert sketch.max_samples() >= 10

    def test_identical_values(self):
        """Test with all identical values."""
        sketch = SplineSketch(200)
        for _ in range(100):
            sketch.update(42, 1.0)

        assert sketch.query(0.0) == 42
        assert sketch.query(0.5) == 42
        assert sketch.query(1.0) == 42

    def test_bimodal_distribution(self):
        """Test with bimodal distribution."""
        sketch = SplineSketch(200)

        # First mode: 0-100
        for i in range(0, 100):
            sketch.update(i, 1.0)

        # Second mode: 900-1000
        for i in range(900, 1000):
            sketch.update(i, 1.0)

        # Median should be somewhere in between
        median = sketch.query(0.5)
        assert 0 <= median <= 1000
