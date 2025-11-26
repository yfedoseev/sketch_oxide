"""
Comprehensive tests for SALSA (Self-Adjusting Counter Sizing Algorithm) Python bindings.
Tests creation, updates, frequency estimation with confidence, merging, and adaptation.
"""

import pytest

from sketch_oxide import SALSA


class TestSALSACreation:
    """Test SALSA instantiation and parameter validation."""

    def test_basic_creation(self):
        """Test basic SALSA creation with valid parameters."""
        salsa = SALSA(epsilon=0.01, delta=0.01)
        assert salsa.epsilon() == 0.01
        assert salsa.delta() == 0.01
        assert salsa.total_updates() == 0
        assert salsa.max_observed() == 0
        assert salsa.adaptation_level() == 0

    def test_different_epsilon_delta(self):
        """Test creation with different epsilon and delta values."""
        salsa_high = SALSA(epsilon=0.001, delta=0.001)
        assert salsa_high.epsilon() == 0.001
        assert salsa_high.delta() == 0.001

        salsa_low = SALSA(epsilon=0.1, delta=0.1)
        assert salsa_low.epsilon() == 0.1
        assert salsa_low.delta() == 0.1

    def test_invalid_epsilon_zero(self):
        """Test that epsilon=0 raises error."""
        with pytest.raises(ValueError):
            SALSA(epsilon=0.0, delta=0.01)

    def test_invalid_epsilon_one(self):
        """Test that epsilon>=1 raises error."""
        with pytest.raises(ValueError):
            SALSA(epsilon=1.0, delta=0.01)

    def test_invalid_delta_zero(self):
        """Test that delta=0 raises error."""
        with pytest.raises(ValueError):
            SALSA(epsilon=0.01, delta=0.0)

    def test_invalid_delta_one(self):
        """Test that delta>=1 raises error."""
        with pytest.raises(ValueError):
            SALSA(epsilon=0.01, delta=1.0)


class TestSALSAUpdate:
    """Test updating sketches with items and frequencies."""

    def test_single_update(self):
        """Test single update."""
        salsa = SALSA(0.01, 0.01)
        salsa.update(b"item", 1)
        assert salsa.total_updates() == 1
        assert salsa.max_observed() == 1

    def test_multiple_updates_same_item(self):
        """Test multiple updates to same item."""
        salsa = SALSA(0.01, 0.01)
        salsa.update(b"apple", 30)
        salsa.update(b"apple", 20)
        salsa.update(b"apple", 10)

        assert salsa.total_updates() == 60
        assert salsa.max_observed() == 30

    def test_multiple_different_items(self):
        """Test updates to different items."""
        salsa = SALSA(0.01, 0.01)
        salsa.update(b"apple", 100)
        salsa.update(b"banana", 50)
        salsa.update(b"cherry", 30)

        assert salsa.total_updates() == 180
        assert salsa.max_observed() == 100

    def test_large_frequencies(self):
        """Test with large frequency values."""
        salsa = SALSA(0.01, 0.01)
        large = 1_000_000
        salsa.update(b"item", large)
        assert salsa.total_updates() == large
        assert salsa.max_observed() == large


class TestSALSAEstimate:
    """Test frequency estimation with confidence metric."""

    def test_single_update_estimation(self):
        """Test estimation of a single update."""
        salsa = SALSA(0.01, 0.01)
        salsa.update(b"item", 1)
        estimate, confidence = salsa.estimate(b"item")
        assert estimate >= 1
        assert confidence >= 0

    def test_estimate_returns_tuple(self):
        """Test that estimate returns (estimate, confidence) tuple."""
        salsa = SALSA(0.01, 0.01)
        salsa.update(b"item", 50)
        result = salsa.estimate(b"item")
        assert isinstance(result, tuple)
        assert len(result) == 2
        estimate, confidence = result
        assert estimate >= 50
        assert isinstance(confidence, int)

    def test_confidence_increases_with_updates(self):
        """Test that confidence increases with more updates."""
        salsa1 = SALSA(0.01, 0.01)
        salsa1.update(b"item", 1)
        _, conf1 = salsa1.estimate(b"item")

        salsa2 = SALSA(0.01, 0.01)
        for _ in range(100):
            salsa2.update(b"item", 1)
        _, conf2 = salsa2.estimate(b"item")

        assert conf2 >= conf1

    def test_nonexistent_item_estimation(self):
        """Test estimating nonexistent item."""
        salsa = SALSA(0.01, 0.01)
        salsa.update(b"item1", 100)
        estimate, confidence = salsa.estimate(b"nonexistent")
        assert estimate == 0

    def test_never_underestimate(self):
        """Test that SALSA never underestimates frequencies."""
        salsa = SALSA(0.01, 0.01)
        frequencies = [100, 50, 25, 10, 1]

        for freq in frequencies:
            salsa.update(f"item{freq}".encode(), freq)

        for freq in frequencies:
            estimate, _ = salsa.estimate(f"item{freq}".encode())
            assert estimate >= freq, f"Underestimated {freq}: got {estimate}"


class TestSALSAAdaptation:
    """Test SALSA's adaptive counter mechanism."""

    def test_adaptation_level_tracks(self):
        """Test that adaptation level can be tracked."""
        salsa = SALSA(0.01, 0.01)
        initial_level = salsa.adaptation_level()
        assert initial_level >= 0

        # Add some large frequencies
        for _ in range(100):
            salsa.update(b"heavy", 100_000)

        final_level = salsa.adaptation_level()
        assert final_level >= initial_level

    def test_adaptation_with_skewed_distribution(self):
        """Test adaptation under skewed/heavy-tailed distribution."""
        salsa = SALSA(0.01, 0.01)

        # Add items with Zipfian distribution
        for i in range(1, 101):
            freq = 10000 // i
            salsa.update(f"item{i}".encode(), freq)

        assert salsa.total_updates() > 0
        # Sketch should adapt to handle the heavy hitters


class TestSALSAMerge:
    """Test merging SALSA sketches."""

    def test_merge_compatible_sketches(self):
        """Test merging sketches with same parameters."""
        salsa1 = SALSA(0.01, 0.01)
        salsa2 = SALSA(0.01, 0.01)

        salsa1.update(b"item", 100)
        salsa2.update(b"item", 50)

        salsa1.merge(salsa2)

        estimate, _ = salsa1.estimate(b"item")
        assert estimate >= 150

    def test_merge_different_items(self):
        """Test merging sketches with different items."""
        salsa1 = SALSA(0.01, 0.01)
        salsa2 = SALSA(0.01, 0.01)

        salsa1.update(b"apple", 100)
        salsa2.update(b"banana", 50)

        salsa1.merge(salsa2)

        apple_est, _ = salsa1.estimate(b"apple")
        banana_est, _ = salsa1.estimate(b"banana")

        assert apple_est >= 100
        assert banana_est >= 50

    def test_merge_incompatible_epsilon(self):
        """Test that merging sketches with different epsilon fails."""
        salsa1 = SALSA(0.01, 0.01)
        salsa2 = SALSA(0.001, 0.01)

        salsa1.update(b"item", 100)

        with pytest.raises(ValueError):
            salsa1.merge(salsa2)

    def test_merge_incompatible_delta(self):
        """Test that merging sketches with different delta fails."""
        salsa1 = SALSA(0.01, 0.01)
        salsa2 = SALSA(0.01, 0.001)

        salsa1.update(b"item", 100)

        with pytest.raises(ValueError):
            salsa1.merge(salsa2)

    def test_merge_empty_sketches(self):
        """Test merging empty sketches."""
        salsa1 = SALSA(0.01, 0.01)
        salsa2 = SALSA(0.01, 0.01)

        salsa1.merge(salsa2)

        estimate, _ = salsa1.estimate(b"any_item")
        assert estimate == 0

    def test_merge_multiple_sketches(self):
        """Test merging multiple sketches in sequence."""
        sketches = [SALSA(0.01, 0.01) for _ in range(3)]

        sketches[0].update(b"a", 10)
        sketches[1].update(b"a", 20)
        sketches[2].update(b"a", 30)

        sketches[0].merge(sketches[1])
        sketches[0].merge(sketches[2])

        estimate, _ = sketches[0].estimate(b"a")
        assert estimate >= 60


class TestSALSAParameters:
    """Test access to sketch parameters."""

    def test_width_and_depth(self):
        """Test getting width and depth parameters."""
        salsa = SALSA(0.01, 0.01)
        width = salsa.width()
        depth = salsa.depth()

        assert width > 0
        assert depth > 0

    def test_parameters_consistency(self):
        """Test that parameters are consistent across calls."""
        salsa = SALSA(0.01, 0.01)
        eps1 = salsa.epsilon()
        eps2 = salsa.epsilon()
        assert eps1 == eps2

        del1 = salsa.delta()
        del2 = salsa.delta()
        assert del1 == del2


class TestSALSARepr:
    """Test string representations."""

    def test_repr(self):
        """Test __repr__ output."""
        salsa = SALSA(0.01, 0.01)
        repr_str = repr(salsa)
        assert "SALSA" in repr_str
        assert "0.01" in repr_str

    def test_str(self):
        """Test __str__ output."""
        salsa = SALSA(0.01, 0.01)
        str_str = str(salsa)
        assert "SALSA" in str_str


class TestSALSAIntegration:
    """Integration tests for complete workflows."""

    def test_uniform_distribution(self):
        """Test with uniform frequency distribution."""
        salsa = SALSA(0.01, 0.01)

        # Add items with uniform frequency
        for i in range(100):
            salsa.update(f"item{i}".encode(), 10)

        # All items should have similar estimates
        for i in range(10):
            est, _ = salsa.estimate(f"item{i}".encode())
            assert est >= 10

    def test_heavy_hitter_detection(self):
        """Test SALSA for detecting heavy hitters."""
        salsa = SALSA(0.01, 0.01)

        # Add heavy and light items
        salsa.update(b"heavy1", 1000)
        salsa.update(b"heavy2", 800)
        salsa.update(b"light", 10)

        heavy1_est, _ = salsa.estimate(b"heavy1")
        heavy2_est, _ = salsa.estimate(b"heavy2")
        light_est, _ = salsa.estimate(b"light")

        # Heavy items should have much higher estimates
        assert heavy1_est >= 1000
        assert heavy2_est >= 800
        assert light_est >= 10

    def test_distributed_collection_and_merge(self):
        """Test SALSA in distributed measurement scenario."""
        # Simulate 5 measurement points
        sketches = [SALSA(0.01, 0.01) for _ in range(5)]

        # Each point measures traffic
        items = [(b"flow1", 100), (b"flow2", 50), (b"flow3", 25)]

        # Distribute measurements
        for point_id, sketch in enumerate(sketches):
            for item, base_freq in items:
                # Add some variation per point
                freq = base_freq + point_id * 10
                sketch.update(item, freq)

        # Merge all into one
        for i in range(1, len(sketches)):
            sketches[0].merge(sketches[i])

        # Verify merged results
        f1_est, _ = sketches[0].estimate(b"flow1")
        assert f1_est >= 100  # Original frequency

    def test_high_accuracy_mode(self):
        """Test SALSA with high accuracy parameters."""
        salsa = SALSA(epsilon=0.001, delta=0.001)

        test_items = [(b"a", 1000), (b"b", 500), (b"c", 250)]

        for item, freq in test_items:
            salsa.update(item, freq)

        # With high accuracy, estimates should be close
        for item, freq in test_items:
            est, conf = salsa.estimate(item)
            assert est >= freq
            assert conf >= 0  # Should have some confidence

    def test_low_memory_mode(self):
        """Test SALSA with low memory parameters."""
        salsa = SALSA(epsilon=0.1, delta=0.1)

        # Add many items
        for i in range(1000):
            salsa.update(f"item{i}".encode(), i % 100 + 1)

        # Should still give reasonable estimates
        est, conf = salsa.estimate(b"item0")
        assert est >= 1

    def test_incremental_updates(self):
        """Test incremental updating over time."""
        salsa = SALSA(0.01, 0.01)

        # Simulate streaming data arriving in batches
        batches = [
            [(b"user1", 10), (b"user2", 5)],
            [(b"user1", 20), (b"user3", 15)],
            [(b"user1", 15), (b"user2", 10)],
        ]

        for batch in batches:
            for item, freq in batch:
                salsa.update(item, freq)

        # Check final estimates
        user1_est, _ = salsa.estimate(b"user1")
        user2_est, _ = salsa.estimate(b"user2")
        user3_est, _ = salsa.estimate(b"user3")

        assert user1_est >= 45  # 10 + 20 + 15
        assert user2_est >= 15  # 5 + 10


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
