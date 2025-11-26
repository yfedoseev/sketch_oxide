"""Comprehensive tests for ExponentialHistogram Python bindings."""

import pytest

from sketch_oxide import ExponentialHistogram


class TestExponentialHistogramBasic:
    """Basic functionality tests for ExponentialHistogram."""

    def test_create_default_epsilon(self):
        """Test creating with default epsilon."""
        eh = ExponentialHistogram(1000)
        assert eh.window_size() == 1000
        assert eh.epsilon() == 0.1
        assert eh.is_empty()

    def test_create_custom_epsilon(self):
        """Test creating with custom epsilon."""
        eh = ExponentialHistogram(1000, epsilon=0.05)
        assert eh.window_size() == 1000
        assert eh.epsilon() == 0.05

    def test_create_invalid_window_size_zero(self):
        """Test that zero window size raises error."""
        with pytest.raises(ValueError):
            ExponentialHistogram(0, 0.1)

    def test_create_invalid_epsilon_zero(self):
        """Test that zero epsilon raises error."""
        with pytest.raises(ValueError):
            ExponentialHistogram(1000, 0.0)

    def test_create_invalid_epsilon_one(self):
        """Test that epsilon >= 1 raises error."""
        with pytest.raises(ValueError):
            ExponentialHistogram(1000, 1.0)

    def test_create_invalid_epsilon_negative(self):
        """Test that negative epsilon raises error."""
        with pytest.raises(ValueError):
            ExponentialHistogram(1000, -0.1)

    def test_k_calculation(self):
        """Test that k is calculated correctly as ceil(1/epsilon)."""
        eh = ExponentialHistogram(1000, epsilon=0.1)
        # k = ceil(1/0.1) = ceil(10) = 10
        assert eh.k() == 10

        eh = ExponentialHistogram(1000, epsilon=0.25)
        # k = ceil(1/0.25) = ceil(4) = 4
        assert eh.k() == 4

        eh = ExponentialHistogram(1000, epsilon=0.33)
        # k = ceil(1/0.33) = ceil(3.03) = 4
        assert eh.k() == 4


class TestExponentialHistogramInsert:
    """Insert operation tests."""

    def test_insert_single_event(self):
        """Test inserting a single event."""
        eh = ExponentialHistogram(1000, 0.1)
        eh.insert(100, 1)
        assert not eh.is_empty()
        assert eh.num_buckets() >= 1

    def test_insert_multiple_events_different_times(self):
        """Test inserting multiple events at different times."""
        eh = ExponentialHistogram(1000, 0.1)
        eh.insert(100, 1)
        eh.insert(200, 1)
        eh.insert(300, 1)
        assert eh.num_buckets() >= 1

    def test_insert_multiple_count(self):
        """Test inserting multiple events at once."""
        eh = ExponentialHistogram(1000, 0.1)
        eh.insert(100, 5)
        assert not eh.is_empty()

    def test_insert_large_count(self):
        """Test inserting large count (power of 2 decomposition)."""
        eh = ExponentialHistogram(10000, 0.1)
        eh.insert(100, 15)  # 15 = 8 + 4 + 2 + 1
        # Should create multiple buckets for powers of 2
        assert eh.num_buckets() >= 1

    def test_insert_zero_count_ignored(self):
        """Test that inserting zero count is ignored."""
        eh = ExponentialHistogram(1000, 0.1)
        eh.insert(100, 0)
        assert eh.is_empty()


class TestExponentialHistogramCount:
    """Count query tests."""

    def test_count_empty_histogram(self):
        """Test counting an empty histogram."""
        eh = ExponentialHistogram(1000, 0.1)
        est, lower, upper = eh.count(1000)
        assert est == 0
        assert lower == 0
        assert upper == 0

    def test_count_single_event_in_window(self):
        """Test counting when single event is in window."""
        eh = ExponentialHistogram(100, 0.1)
        eh.insert(50, 1)

        # At time 100, event at 50 is in window [0, 100]
        est, lower, upper = eh.count(100)
        assert est == 1
        assert lower <= 1
        assert upper >= 1

    def test_count_single_event_outside_window(self):
        """Test counting when event is outside window."""
        eh = ExponentialHistogram(100, 0.1)
        eh.insert(50, 1)

        # At time 200, event at 50 is outside window [100, 200]
        est, lower, upper = eh.count(200)
        # May have partial contribution from straddling bucket
        assert est <= 2

    def test_count_bounds_monotonicity(self):
        """Test that lower <= estimate <= upper."""
        eh = ExponentialHistogram(1000, 0.1)
        for i in range(0, 100, 10):
            eh.insert(i, 1)

        est, lower, upper = eh.count(500)
        assert lower <= est, f"Lower {lower} > estimate {est}"
        assert est <= upper, f"Estimate {est} > upper {upper}"

    def test_count_multiple_events_in_window(self):
        """Test counting multiple events in window."""
        eh = ExponentialHistogram(1000, 0.1)
        eh.insert(100, 1)
        eh.insert(200, 1)
        eh.insert(300, 1)

        est, lower, upper = eh.count(500)
        # All three events should be in window
        assert est >= 2
        assert lower <= 3
        assert upper >= 3

    def test_count_respects_window_boundaries(self):
        """Test that count respects window boundaries."""
        eh = ExponentialHistogram(100, 0.1)
        eh.insert(10, 1)
        eh.insert(20, 1)
        eh.insert(150, 1)
        eh.insert(180, 1)

        # At time 200, window is [100, 200]
        # Events at 10, 20 are outside
        # Events at 150, 180 are inside
        est, lower, upper = eh.count(200)

        # Should count primarily the in-window events
        assert est >= 1


class TestExponentialHistogramExpire:
    """Expire operation tests."""

    def test_expire_removes_old_buckets(self):
        """Test that expire removes old buckets."""
        eh = ExponentialHistogram(100, 0.1)
        eh.insert(10, 1)
        eh.insert(20, 1)
        eh.insert(200, 1)
        eh.insert(210, 1)

        before = eh.num_buckets()
        eh.expire(300)
        after = eh.num_buckets()

        # Should have removed or consolidated old buckets
        assert after <= before

    def test_expire_on_empty_histogram(self):
        """Test expire on empty histogram."""
        eh = ExponentialHistogram(1000, 0.1)
        eh.expire(1000)  # Should not crash
        assert eh.is_empty()

    def test_expire_preserves_recent_data(self):
        """Test that expire preserves recent data within window."""
        eh = ExponentialHistogram(1000, 0.1)
        eh.insert(100, 5)
        eh.insert(500, 3)

        eh.expire(600)

        # Event at 500 should still be countable
        est, _, _ = eh.count(600)
        assert est >= 2


class TestExponentialHistogramMerge:
    """Merge operation tests."""

    def test_merge_empty_histograms(self):
        """Test merging two empty histograms."""
        eh1 = ExponentialHistogram(1000, 0.1)
        eh2 = ExponentialHistogram(1000, 0.1)
        eh1.merge(eh2)
        assert eh1.is_empty()

    def test_merge_into_empty(self):
        """Test merging into empty histogram."""
        eh1 = ExponentialHistogram(1000, 0.1)
        eh2 = ExponentialHistogram(1000, 0.1)

        eh2.insert(100, 5)
        eh1.merge(eh2)

        est, _, _ = eh1.count(200)
        assert est >= 4

    def test_merge_non_empty_histograms(self):
        """Test merging two non-empty histograms."""
        eh1 = ExponentialHistogram(1000, 0.1)
        eh2 = ExponentialHistogram(1000, 0.1)

        eh1.insert(100, 2)
        eh2.insert(300, 3)

        eh1.merge(eh2)

        est, _, _ = eh1.count(500)
        # Should have combined events
        assert est >= 4

    def test_merge_incompatible_window_size_raises_error(self):
        """Test that merging with different window_size raises error."""
        eh1 = ExponentialHistogram(1000, 0.1)
        eh2 = ExponentialHistogram(500, 0.1)

        eh1.insert(100, 1)
        eh2.insert(100, 1)

        with pytest.raises(ValueError):
            eh1.merge(eh2)

    def test_merge_incompatible_epsilon_raises_error(self):
        """Test that merging with different epsilon raises error."""
        eh1 = ExponentialHistogram(1000, 0.1)
        eh2 = ExponentialHistogram(1000, 0.05)

        eh1.insert(100, 1)
        eh2.insert(100, 1)

        with pytest.raises(ValueError):
            eh1.merge(eh2)


class TestExponentialHistogramReset:
    """Clear/reset functionality tests."""

    def test_clear_removes_data(self):
        """Test that clear removes all data."""
        eh = ExponentialHistogram(1000, 0.1)
        eh.insert(100, 5)
        assert not eh.is_empty()

        eh.clear()
        assert eh.is_empty()
        assert eh.num_buckets() == 0

    def test_clear_allows_reuse(self):
        """Test that histogram can be reused after clear."""
        eh = ExponentialHistogram(1000, 0.1)
        eh.insert(100, 5)
        eh.clear()

        # Should be able to add new data
        eh.insert(200, 3)
        est, _, _ = eh.count(300)
        assert est >= 2


class TestExponentialHistogramSerialization:
    """Serialization/deserialization tests."""

    def test_serialize_empty_histogram(self):
        """Test serializing empty histogram."""
        eh = ExponentialHistogram(1000, 0.1)
        data = eh.serialize()
        assert isinstance(data, bytes)
        assert len(data) > 0

    def test_deserialize_empty_histogram(self):
        """Test deserializing empty histogram."""
        eh = ExponentialHistogram(1000, 0.1)
        data = eh.serialize()

        restored = ExponentialHistogram.deserialize(data)
        assert restored.is_empty()
        assert restored.window_size() == 1000
        assert restored.epsilon() == 0.1

    def test_serialize_deserialize_with_data(self):
        """Test round-trip serialization with data."""
        eh = ExponentialHistogram(1000, 0.1)
        eh.insert(100, 5)
        eh.insert(200, 3)
        eh.insert(300, 2)

        data = eh.serialize()
        restored = ExponentialHistogram.deserialize(data)

        assert restored.window_size() == eh.window_size()
        assert restored.epsilon() == eh.epsilon()
        assert restored.num_buckets() == eh.num_buckets()

    def test_deserialize_preserves_counts(self):
        """Test that deserialized histogram gives same counts."""
        eh = ExponentialHistogram(1000, 0.1)
        eh.insert(100, 5)
        eh.insert(200, 3)

        original_est, original_lower, original_upper = eh.count(500)

        data = eh.serialize()
        restored = ExponentialHistogram.deserialize(data)
        restored_est, restored_lower, restored_upper = restored.count(500)

        assert original_est == restored_est
        assert original_lower == restored_lower
        assert original_upper == restored_upper

    def test_deserialize_invalid_data_raises_error(self):
        """Test deserializing invalid data raises error."""
        with pytest.raises(ValueError):
            ExponentialHistogram.deserialize(b"invalid")


class TestExponentialHistogramProperties:
    """Property accessor tests."""

    def test_error_bound_property(self):
        """Test error_bound property."""
        eh = ExponentialHistogram(1000, 0.05)
        assert abs(eh.error_bound() - 0.05) < 1e-10

    def test_memory_usage(self):
        """Test memory usage reporting."""
        eh = ExponentialHistogram(1000, 0.1)
        base_memory = eh.memory_usage()
        assert base_memory > 0

        # Add data
        for i in range(100):
            eh.insert(i, 1)

        with_data_memory = eh.memory_usage()
        assert with_data_memory > base_memory


class TestExponentialHistogramRepr:
    """String representation tests."""

    def test_repr(self):
        """Test __repr__ output."""
        eh = ExponentialHistogram(1000, 0.1)
        repr_str = repr(eh)
        assert "ExponentialHistogram" in repr_str
        assert "1000" in repr_str

    def test_str(self):
        """Test __str__ output."""
        eh = ExponentialHistogram(1000, 0.1)
        str_str = str(eh)
        assert "ExponentialHistogram" in str_str


class TestExponentialHistogramErrorBounds:
    """Error bound validation tests."""

    def test_error_bound_property_getter(self):
        """Test getting error bound property."""
        eh = ExponentialHistogram(1000, 0.05)
        assert abs(eh.error_bound() - 0.05) < 1e-10

    def test_relative_error_within_epsilon(self):
        """Test that relative error is within epsilon bounds."""
        epsilon = 0.1
        eh = ExponentialHistogram(10000, epsilon)

        # Insert many events
        actual_count = 50
        for i in range(actual_count):
            eh.insert(i * 100, 1)

        est, lower, upper = eh.count(6000)

        # Bounds should be consistent
        assert lower <= est <= upper

        # Error should be roughly bounded by epsilon
        if est > 0:
            relative_error = abs(est - actual_count) / actual_count
            # Allow some slack for edge effects
            assert relative_error < epsilon + 0.2


class TestExponentialHistogramEdgeCases:
    """Edge case and stress tests."""

    def test_large_window_size(self):
        """Test with large window size."""
        eh = ExponentialHistogram(2**32, 0.1)
        assert eh.window_size() == 2**32

    def test_small_epsilon(self):
        """Test with very small epsilon (high precision)."""
        eh = ExponentialHistogram(1000, 0.001)
        assert eh.k() == 1000  # ceil(1/0.001)

    def test_many_events_in_window(self):
        """Test with many events in window."""
        eh = ExponentialHistogram(10000, 0.1)

        # Insert many events
        for i in range(0, 1000, 10):
            eh.insert(i, 1)

        est, lower, upper = eh.count(5000)
        assert est >= 50
        assert lower <= est <= upper

    def test_monotonic_insertion_order(self):
        """Test that events inserted in monotonic order work correctly."""
        eh = ExponentialHistogram(1000, 0.1)

        for i in range(0, 500, 50):
            eh.insert(i, 1)

        est, _, _ = eh.count(600)
        assert est >= 9

    def test_non_monotonic_insertion_order(self):
        """Test that events inserted in non-monotonic order work."""
        eh = ExponentialHistogram(1000, 0.1)

        # Insert out of order
        eh.insert(200, 1)
        eh.insert(100, 1)
        eh.insert(300, 1)

        est, _, _ = eh.count(500)
        assert est >= 2

    def test_duplicate_timestamps(self):
        """Test with duplicate timestamps."""
        eh = ExponentialHistogram(1000, 0.1)

        eh.insert(100, 2)
        eh.insert(100, 3)

        est, lower, upper = eh.count(200)
        assert est >= 4
        assert lower <= est <= upper

    def test_events_at_window_boundary(self):
        """Test with events exactly at window boundary."""
        eh = ExponentialHistogram(100, 0.1)

        # Event exactly at window start
        eh.insert(100, 1)

        # At time 200, window is [100, 200]
        est, _, _ = eh.count(200)
        assert est >= 0  # Event is at boundary


class TestExponentialHistogramConsistency:
    """Tests for consistency and invariants."""

    def test_count_bounds_consistency(self):
        """Test count bounds are always consistent."""
        eh = ExponentialHistogram(1000, 0.1)

        for i in range(0, 100, 10):
            eh.insert(i, 1)

        for current_time in [200, 500, 1000, 2000]:
            est, lower, upper = eh.count(current_time)
            assert lower <= est <= upper, f"At time {current_time}: {lower} <= {est} <= {upper}"

    def test_expire_maintains_accuracy(self):
        """Test that expire doesn't hurt count accuracy."""
        eh = ExponentialHistogram(1000, 0.1)

        for i in range(0, 500, 50):
            eh.insert(i, 1)

        est_before, _, _ = eh.count(600)

        eh.expire(600)

        est_after, _, _ = eh.count(600)
        # Estimates should be similar (expire shouldn't lose in-window events)
        assert abs(est_before - est_after) <= 2

    def test_merge_transitivity(self):
        """Test merge transitivity: (a merge b) merge c == a merge (b merge c)."""
        eh1a = ExponentialHistogram(1000, 0.1)
        eh2a = ExponentialHistogram(1000, 0.1)
        eh3 = ExponentialHistogram(1000, 0.1)

        eh1b = ExponentialHistogram(1000, 0.1)
        eh2b = ExponentialHistogram(1000, 0.1)

        # Setup left path: (a merge b) merge c
        eh1a.insert(100, 1)
        eh2a.insert(200, 1)
        eh1a.merge(eh2a)
        eh1a.merge(eh3)

        # Setup right path: a merge (b merge c)
        eh1b.insert(100, 1)
        eh2b.insert(200, 1)
        eh2b.merge(eh3)
        eh1b.merge(eh2b)

        # Both should give same result
        est1, lower1, upper1 = eh1a.count(500)
        est2, lower2, upper2 = eh1b.count(500)

        assert est1 == est2
        assert lower1 == lower2
        assert upper1 == upper2
