"""
Comprehensive tests for RemovableUniversalSketch Python bindings.
Tests creation, insertions, deletions, frequency moments (L2 norm), merging, and turnstile streams.
"""

import pytest

from sketch_oxide import RemovableUniversalSketch


class TestRemovableSketchCreation:
    """Test RemovableUniversalSketch instantiation and parameter validation."""

    def test_basic_creation(self):
        """Test basic sketch creation with valid parameters."""
        rus = RemovableUniversalSketch(epsilon=0.01, delta=0.01)
        assert rus.epsilon() == 0.01
        assert rus.delta() == 0.01
        assert rus.width() > 0
        assert rus.depth() > 0

    def test_different_parameters(self):
        """Test creation with different epsilon and delta."""
        rus_high = RemovableUniversalSketch(epsilon=0.001, delta=0.001)
        assert rus_high.epsilon() == 0.001
        assert rus_high.delta() == 0.001

        rus_low = RemovableUniversalSketch(epsilon=0.1, delta=0.1)
        assert rus_low.epsilon() == 0.1
        assert rus_low.delta() == 0.1

    def test_invalid_epsilon(self):
        """Test that invalid epsilon raises ValueError."""
        with pytest.raises(ValueError):
            RemovableUniversalSketch(epsilon=0.0, delta=0.01)

        with pytest.raises(ValueError):
            RemovableUniversalSketch(epsilon=1.0, delta=0.01)

    def test_invalid_delta(self):
        """Test that invalid delta raises ValueError."""
        with pytest.raises(ValueError):
            RemovableUniversalSketch(epsilon=0.01, delta=0.0)

        with pytest.raises(ValueError):
            RemovableUniversalSketch(epsilon=0.01, delta=1.0)


class TestRemovableSketchInsert:
    """Test inserting items into the sketch."""

    def test_single_insert(self):
        """Test single item insertion."""
        rus = RemovableUniversalSketch(0.01, 0.01)
        rus.update(b"item", 1)
        assert rus.estimate(b"item") >= 1

    def test_multiple_inserts_same_item(self):
        """Test multiple inserts of the same item."""
        rus = RemovableUniversalSketch(0.01, 0.01)
        rus.update(b"page", 100)
        frequency = rus.estimate(b"page")
        assert frequency >= 100

    def test_multiple_items(self):
        """Test inserting multiple different items."""
        rus = RemovableUniversalSketch(0.01, 0.01)
        rus.update(b"item1", 100)
        rus.update(b"item2", 50)
        rus.update(b"item3", 25)

        assert rus.estimate(b"item1") >= 100
        assert rus.estimate(b"item2") >= 50
        assert rus.estimate(b"item3") >= 25

    def test_large_frequencies(self):
        """Test with large frequency values."""
        rus = RemovableUniversalSketch(0.01, 0.01)
        large_freq = 1_000_000
        rus.update(b"heavy", large_freq)
        assert rus.estimate(b"heavy") >= large_freq


class TestRemovableSketchDelete:
    """Test deleting items (negative updates)."""

    def test_simple_deletion(self):
        """Test simple deletion of items."""
        rus = RemovableUniversalSketch(0.01, 0.01)
        rus.update(b"item", 100)
        rus.update(b"item", -30)  # Delete 30

        frequency = rus.estimate(b"item")
        assert frequency >= 70

    def test_partial_deletion(self):
        """Test partial deletion reducing count."""
        rus = RemovableUniversalSketch(0.01, 0.01)
        rus.update(b"item", 100)
        assert rus.estimate(b"item") >= 100

        rus.update(b"item", -50)
        frequency_after = rus.estimate(b"item")
        assert frequency_after >= 50

    def test_complete_deletion(self):
        """Test deleting entire count."""
        rus = RemovableUniversalSketch(0.01, 0.01)
        rus.update(b"item", 100)
        rus.update(b"item", -100)

        frequency = rus.estimate(b"item")
        # Should be close to 0, but might have some error
        assert frequency >= 0

    def test_overdelete(self):
        """Test deleting more than was added."""
        rus = RemovableUniversalSketch(0.01, 0.01)
        rus.update(b"item", 50)
        rus.update(b"item", -100)  # Delete more than exists

        frequency = rus.estimate(b"item")
        # Can be negative or 0 depending on estimation
        assert isinstance(frequency, int)

    def test_alternating_updates_deletions(self):
        """Test alternating insertions and deletions."""
        rus = RemovableUniversalSketch(0.01, 0.01)

        rus.update(b"item", 100)
        assert rus.estimate(b"item") >= 100

        rus.update(b"item", -30)
        freq_1 = rus.estimate(b"item")
        assert freq_1 >= 70

        rus.update(b"item", 50)
        freq_2 = rus.estimate(b"item")
        assert freq_2 >= 120

        rus.update(b"item", -40)
        freq_3 = rus.estimate(b"item")
        assert freq_3 >= 80


class TestRemovableSketchTurnstile:
    """Test turnstile stream operations (insertions and deletions)."""

    def test_turnstile_stream_basic(self):
        """Test basic turnstile stream."""
        rus = RemovableUniversalSketch(0.01, 0.01)

        rus.update(b"item", 100)
        assert rus.estimate(b"item") >= 100

        rus.update(b"item", -50)
        freq_after_delete = rus.estimate(b"item")
        assert freq_after_delete >= 50

        rus.update(b"item", 100)
        freq_after_reinsert = rus.estimate(b"item")
        assert freq_after_reinsert >= 150

    def test_turnstile_multiple_items(self):
        """Test turnstile with multiple items."""
        rus = RemovableUniversalSketch(0.01, 0.01)

        # Add items
        rus.update(b"page1", 100)
        rus.update(b"page2", 50)
        rus.update(b"page3", 25)

        # Delete some traffic (e.g., spam removal)
        rus.update(b"page1", -20)
        rus.update(b"page2", -10)

        # Check states
        assert rus.estimate(b"page1") >= 80
        assert rus.estimate(b"page2") >= 40
        assert rus.estimate(b"page3") >= 25

    def test_zero_update_no_change(self):
        """Test that zero-delta updates don't change state."""
        rus = RemovableUniversalSketch(0.01, 0.01)
        rus.update(b"item", 100)
        freq_before = rus.estimate(b"item")

        rus.update(b"item", 0)  # No-op
        freq_after = rus.estimate(b"item")

        assert freq_before == freq_after


class TestRemovableSketchL2Norm:
    """Test L2 norm (frequency moment) computation."""

    def test_l2_norm_single_item(self):
        """Test L2 norm with single item."""
        rus = RemovableUniversalSketch(0.01, 0.01)
        rus.update(b"item", 100)

        l2 = rus.l2_norm()
        assert l2 > 0.0
        assert l2 >= 100.0  # L2 norm >= max frequency

    def test_l2_norm_multiple_items(self):
        """Test L2 norm with multiple items."""
        rus = RemovableUniversalSketch(0.01, 0.01)
        rus.update(b"item1", 100)
        rus.update(b"item2", 50)
        rus.update(b"item3", 25)

        l2 = rus.l2_norm()
        assert l2 > 0.0
        assert l2 >= 100.0  # At least the max frequency

    def test_l2_norm_increases_with_updates(self):
        """Test that L2 norm increases with more updates."""
        rus1 = RemovableUniversalSketch(0.01, 0.01)
        rus1.update(b"item", 100)
        l2_1 = rus1.l2_norm()

        rus2 = RemovableUniversalSketch(0.01, 0.01)
        rus2.update(b"item1", 100)
        rus2.update(b"item2", 100)
        l2_2 = rus2.l2_norm()

        # More items should increase L2 norm
        assert l2_2 >= l2_1

    def test_l2_norm_decreases_with_deletions(self):
        """Test that L2 norm decreases with deletions."""
        rus = RemovableUniversalSketch(0.01, 0.01)
        rus.update(b"item1", 100)
        rus.update(b"item2", 50)
        l2_before = rus.l2_norm()

        rus.update(b"item1", -50)
        rus.update(b"item2", -25)
        l2_after = rus.l2_norm()

        # L2 norm should be lower after deletions
        assert l2_after <= l2_before

    def test_l2_norm_empty_sketch(self):
        """Test L2 norm of empty sketch."""
        rus = RemovableUniversalSketch(0.01, 0.01)
        l2 = rus.l2_norm()
        assert l2 == 0.0

    def test_l2_norm_zipfian_distribution(self):
        """Test L2 norm with Zipfian distribution."""
        rus = RemovableUniversalSketch(0.01, 0.01)

        # Add items with Zipfian distribution
        for i in range(1, 21):
            freq = 1000 // i
            rus.update(f"item{i}".encode(), freq)

        l2 = rus.l2_norm()
        assert l2 > 0.0
        assert l2 >= 1000.0  # At least the max frequency


class TestRemovableSketchMerge:
    """Test merging RemovableUniversalSketches."""

    def test_merge_simple(self):
        """Test simple merge of two sketches."""
        rus1 = RemovableUniversalSketch(0.01, 0.01)
        rus2 = RemovableUniversalSketch(0.01, 0.01)

        rus1.update(b"item", 100)
        rus2.update(b"item", 50)

        rus1.merge(rus2)
        assert rus1.estimate(b"item") >= 150

    def test_merge_different_items(self):
        """Test merging sketches with different items."""
        rus1 = RemovableUniversalSketch(0.01, 0.01)
        rus2 = RemovableUniversalSketch(0.01, 0.01)

        rus1.update(b"item1", 100)
        rus2.update(b"item2", 50)

        rus1.merge(rus2)

        assert rus1.estimate(b"item1") >= 100
        assert rus1.estimate(b"item2") >= 50

    def test_merge_with_deletions(self):
        """Test merging sketches that contain deletions."""
        rus1 = RemovableUniversalSketch(0.01, 0.01)
        rus2 = RemovableUniversalSketch(0.01, 0.01)

        rus1.update(b"item", 100)
        rus1.update(b"item", -20)  # Net: 80

        rus2.update(b"item", 50)
        rus2.update(b"item", -10)  # Net: 40

        rus1.merge(rus2)
        freq = rus1.estimate(b"item")
        assert freq >= 120  # 80 + 40

    def test_merge_incompatible_epsilon(self):
        """Test merging sketches with different epsilon."""
        rus1 = RemovableUniversalSketch(0.01, 0.01)
        rus2 = RemovableUniversalSketch(0.001, 0.01)

        with pytest.raises(ValueError):
            rus1.merge(rus2)

    def test_merge_incompatible_delta(self):
        """Test merging sketches with different delta."""
        rus1 = RemovableUniversalSketch(0.01, 0.01)
        rus2 = RemovableUniversalSketch(0.01, 0.001)

        with pytest.raises(ValueError):
            rus1.merge(rus2)

    def test_merge_l2_norm_additivity(self):
        """Test that L2 norm behaves correctly after merge."""
        rus1 = RemovableUniversalSketch(0.01, 0.01)
        rus2 = RemovableUniversalSketch(0.01, 0.01)

        rus1.update(b"item1", 100)
        rus2.update(b"item2", 100)

        l2_before = rus1.l2_norm()

        rus1.merge(rus2)
        l2_after = rus1.l2_norm()

        # L2 norm should increase after merge
        assert l2_after >= l2_before


class TestRemovableSketchParameters:
    """Test access to sketch parameters."""

    def test_width_and_depth(self):
        """Test getting width and depth."""
        rus = RemovableUniversalSketch(0.01, 0.01)
        width = rus.width()
        depth = rus.depth()

        assert width > 0
        assert depth > 0

    def test_parameter_consistency(self):
        """Test that parameters are consistent."""
        rus = RemovableUniversalSketch(0.01, 0.01)
        eps1 = rus.epsilon()
        eps2 = rus.epsilon()
        assert eps1 == eps2

        del1 = rus.delta()
        del2 = rus.delta()
        assert del1 == del2


class TestRemovableSketchRepr:
    """Test string representations."""

    def test_repr(self):
        """Test __repr__ output."""
        rus = RemovableUniversalSketch(0.01, 0.01)
        repr_str = repr(rus)
        assert "RemovableUniversalSketch" in repr_str
        assert "0.01" in repr_str

    def test_str(self):
        """Test __str__ output."""
        rus = RemovableUniversalSketch(0.01, 0.01)
        str_str = str(rus)
        assert "RemovableUniversalSketch" in str_str


class TestRemovableSketchIntegration:
    """Integration tests for complete workflows."""

    def test_cache_traffic_tracking(self):
        """Simulate cache hit/miss tracking as turnstile stream."""
        rus = RemovableUniversalSketch(0.01, 0.01)

        # Track cache accesses (hits increase, evictions decrease)
        rus.update(b"page_a", 1000)  # 1000 hits
        rus.update(b"page_b", 500)
        rus.update(b"page_c", 200)

        # Some pages evicted/invalidated
        rus.update(b"page_a", -100)  # Removed 100 entries
        rus.update(b"page_c", -50)

        # Check remaining
        assert rus.estimate(b"page_a") >= 900
        assert rus.estimate(b"page_b") >= 500
        assert rus.estimate(b"page_c") >= 150

    def test_network_flow_analysis(self):
        """Analyze network flows with packet arrivals and retransmissions."""
        rus = RemovableUniversalSketch(0.01, 0.01)

        # Original packets
        rus.update(b"flow1:src:192.168.1.1:dst:10.0.0.1", 1000)
        rus.update(b"flow2:src:192.168.1.2:dst:10.0.0.2", 500)

        # Retransmissions (count as additions)
        rus.update(b"flow1:src:192.168.1.1:dst:10.0.0.1", 100)

        # Lost packets (count as deletions)
        rus.update(b"flow2:src:192.168.1.2:dst:10.0.0.2", -50)

        # Check estimates
        flow1_est = rus.estimate(b"flow1:src:192.168.1.1:dst:10.0.0.1")
        flow2_est = rus.estimate(b"flow2:src:192.168.1.2:dst:10.0.0.2")

        assert flow1_est >= 1100
        assert flow2_est >= 450

    def test_distributed_cache_merge(self):
        """Merge cache statistics from multiple replicas."""
        caches = [RemovableUniversalSketch(0.01, 0.01) for _ in range(3)]

        # Each cache tracks pages independently
        pages = [b"home", b"product", b"checkout"]
        base_counts = [1000, 500, 100]

        for cache_id, cache in enumerate(caches):
            for page, base_count in zip(pages, base_counts):
                # Each cache has slightly different traffic
                count = base_count + (cache_id * 100)
                cache.update(page, count)

        # Merge all caches
        for i in range(1, len(caches)):
            caches[0].merge(caches[i])

        # Verify merged totals
        home_est = caches[0].estimate(b"home")
        assert home_est >= 1000  # At least base

    def test_turnstile_with_l2_tracking(self):
        """Track L2 norm through turnstile operations."""
        rus = RemovableUniversalSketch(0.01, 0.01)

        # Initial state
        rus.update(b"a", 100)
        rus.update(b"b", 50)
        l2_initial = rus.l2_norm()
        assert l2_initial > 0

        # Add more
        rus.update(b"c", 75)
        l2_after_add = rus.l2_norm()
        assert l2_after_add >= l2_initial

        # Delete some
        rus.update(b"a", -30)
        rus.update(b"b", -20)
        l2_after_delete = rus.l2_norm()
        # L2 should be lower after deletions
        assert l2_after_delete <= l2_after_add

    def test_high_accuracy_measurement(self):
        """Test with high accuracy parameters."""
        rus = RemovableUniversalSketch(epsilon=0.001, delta=0.001)

        measurements = [
            (b"metric1", 10000),
            (b"metric2", 5000),
            (b"metric3", 2500),
        ]

        for metric, count in measurements:
            rus.update(metric, count)

        # Verify estimates are accurate
        for metric, count in measurements:
            est = rus.estimate(metric)
            assert est >= count

        # Check L2 norm
        l2 = rus.l2_norm()
        assert l2 >= 10000


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
