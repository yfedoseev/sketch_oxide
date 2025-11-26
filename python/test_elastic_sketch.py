"""
Comprehensive tests for ElasticSketch Python bindings.
Tests creation, updates, frequency estimation, heavy hitter detection, merging, and serialization.
"""

import pytest

from sketch_oxide import ElasticSketch


class TestElasticSketchCreation:
    """Test ElasticSketch instantiation and parameter validation."""

    def test_basic_creation(self):
        """Test basic sketch creation with valid parameters."""
        sketch = ElasticSketch(bucket_count=512, depth=3)
        assert sketch.bucket_count() == 512
        assert sketch.depth() == 3
        assert sketch.elastic_ratio() == 0.2  # Default
        assert sketch.is_empty()
        assert sketch.total_count() == 0

    def test_creation_with_custom_ratio(self):
        """Test sketch creation with custom elastic ratio."""
        sketch = ElasticSketch.with_elastic_ratio(bucket_count=256, depth=2, elastic_ratio=0.5)
        assert sketch.elastic_ratio() == 0.5
        assert sketch.bucket_count() == 256
        assert sketch.depth() == 2

    def test_power_of_two_rounding(self):
        """Test that bucket count is rounded to next power of 2."""
        sketch = ElasticSketch(bucket_count=500, depth=3)
        assert sketch.bucket_count() == 512  # Next power of 2

        sketch2 = ElasticSketch(bucket_count=1024, depth=3)
        assert sketch2.bucket_count() == 1024  # Already power of 2

        sketch3 = ElasticSketch(bucket_count=1000, depth=3)
        assert sketch3.bucket_count() == 1024  # Next power of 2

    def test_invalid_parameters(self):
        """Test that invalid parameters raise ValueError."""
        with pytest.raises(ValueError):
            ElasticSketch(bucket_count=0, depth=3)

        with pytest.raises(ValueError):
            ElasticSketch(bucket_count=512, depth=0)

        with pytest.raises(ValueError):
            ElasticSketch(bucket_count=512, depth=9)  # Depth > 8

        with pytest.raises(ValueError):
            ElasticSketch.with_elastic_ratio(512, 3, -0.1)  # Negative ratio

        with pytest.raises(ValueError):
            ElasticSketch.with_elastic_ratio(512, 3, 1.5)  # Ratio > 1.0


class TestElasticSketchUpdate:
    """Test updating sketches with items and frequencies."""

    def test_single_update(self):
        """Test updating sketch with a single item."""
        sketch = ElasticSketch(512, 3)
        sketch.update(b"flow1", 1)
        assert not sketch.is_empty()
        assert sketch.total_count() == 1

    def test_multiple_updates_same_item(self):
        """Test multiple updates to the same item."""
        sketch = ElasticSketch(512, 3)
        sketch.update(b"item", 1)
        sketch.update(b"item", 2)
        sketch.update(b"item", 3)
        assert sketch.total_count() == 6

    def test_multiple_items(self):
        """Test updating multiple different items."""
        sketch = ElasticSketch(512, 3)
        sketch.update(b"item1", 10)
        sketch.update(b"item2", 20)
        sketch.update(b"item3", 15)
        assert sketch.total_count() == 45

    def test_large_frequencies(self):
        """Test updating with large frequency values."""
        sketch = ElasticSketch(512, 3)
        large_count = 1_000_000
        sketch.update(b"large_item", large_count)
        assert sketch.total_count() == large_count


class TestElasticSketchEstimate:
    """Test frequency estimation accuracy."""

    def test_exact_frequency(self):
        """Test that exact frequencies are preserved."""
        sketch = ElasticSketch(512, 3)
        sketch.update(b"item", 5)
        assert sketch.estimate(b"item") == 5

    def test_different_frequencies(self):
        """Test estimation with different frequency values."""
        sketch = ElasticSketch(512, 3)
        sketch.update(b"heavy", 100)
        sketch.update(b"medium", 50)
        sketch.update(b"light", 10)

        assert sketch.estimate(b"heavy") == 100
        assert sketch.estimate(b"medium") == 50
        assert sketch.estimate(b"light") == 10

    def test_nonexistent_item(self):
        """Test that nonexistent items return 0."""
        sketch = ElasticSketch(512, 3)
        sketch.update(b"item1", 5)
        assert sketch.estimate(b"nonexistent") == 0

    def test_stability_of_estimates(self):
        """Test that estimates are stable across multiple calls."""
        sketch = ElasticSketch(512, 3)
        sketch.update(b"stable", 10)

        est1 = sketch.estimate(b"stable")
        est2 = sketch.estimate(b"stable")
        est3 = sketch.estimate(b"stable")

        assert est1 == est2 == est3 == 10


class TestElasticSketchHeavyHitters:
    """Test heavy hitter detection."""

    def test_basic_heavy_hitters(self):
        """Test basic heavy hitter detection."""
        sketch = ElasticSketch(512, 3)
        sketch.update(b"heavy1", 100)
        sketch.update(b"heavy2", 50)
        sketch.update(b"light", 5)

        hitters = sketch.heavy_hitters(threshold=40)
        assert len(hitters) >= 2  # At least heavy1 and heavy2
        # Results are sorted by frequency descending
        assert hitters[0][1] >= hitters[1][1]

    def test_no_heavy_hitters(self):
        """Test when threshold is higher than all items."""
        sketch = ElasticSketch(512, 3)
        sketch.update(b"item1", 10)
        sketch.update(b"item2", 5)

        hitters = sketch.heavy_hitters(threshold=100)
        assert len(hitters) == 0

    def test_all_items_are_heavy(self):
        """Test when all items exceed threshold."""
        sketch = ElasticSketch(512, 3)
        for i in range(10):
            sketch.update(f"item{i}".encode(), 100)

        hitters = sketch.heavy_hitters(threshold=50)
        assert len(hitters) >= 5

    def test_heavy_hitters_sorted(self):
        """Test that heavy hitters are sorted by frequency."""
        sketch = ElasticSketch(1024, 4)
        sketch.update(b"first", 100)
        sketch.update(b"second", 50)
        sketch.update(b"third", 75)

        hitters = sketch.heavy_hitters(threshold=40)
        # Should be sorted in descending order
        for i in range(len(hitters) - 1):
            assert hitters[i][1] >= hitters[i + 1][1]


class TestElasticSketchMerge:
    """Test merging multiple sketches."""

    def test_merge_compatible_sketches(self):
        """Test merging two compatible sketches."""
        sketch1 = ElasticSketch(512, 3)
        sketch2 = ElasticSketch(512, 3)

        sketch1.update(b"item", 5)
        sketch2.update(b"item", 3)

        sketch1.merge(sketch2)
        assert sketch1.estimate(b"item") == 8

    def test_merge_multiple_items(self):
        """Test merging sketches with different items."""
        sketch1 = ElasticSketch(512, 3)
        sketch2 = ElasticSketch(512, 3)

        sketch1.update(b"item1", 10)
        sketch2.update(b"item2", 20)

        sketch1.merge(sketch2)
        assert sketch1.estimate(b"item1") == 10
        assert sketch1.estimate(b"item2") == 20

    def test_merge_incompatible_bucket_count(self):
        """Test that merging sketches with different bucket counts fails."""
        sketch1 = ElasticSketch(512, 3)
        sketch2 = ElasticSketch(256, 3)

        with pytest.raises(ValueError):
            sketch1.merge(sketch2)

    def test_merge_incompatible_depth(self):
        """Test that merging sketches with different depths fails."""
        sketch1 = ElasticSketch(512, 3)
        sketch2 = ElasticSketch(512, 4)

        with pytest.raises(ValueError):
            sketch1.merge(sketch2)

    def test_merge_incompatible_elastic_ratio(self):
        """Test that merging sketches with different ratios fails."""
        sketch1 = ElasticSketch(512, 3)
        sketch2 = ElasticSketch.with_elastic_ratio(512, 3, 0.5)

        with pytest.raises(ValueError):
            sketch1.merge(sketch2)


class TestElasticSketchReset:
    """Test reset functionality."""

    def test_reset_clears_sketch(self):
        """Test that reset clears all data."""
        sketch = ElasticSketch(512, 3)
        sketch.update(b"item1", 5)
        sketch.update(b"item2", 3)

        assert not sketch.is_empty()
        assert sketch.total_count() == 8

        sketch.reset()

        assert sketch.is_empty()
        assert sketch.total_count() == 0
        assert sketch.estimate(b"item1") == 0

    def test_reset_preserves_parameters(self):
        """Test that reset preserves sketch parameters."""
        sketch = ElasticSketch(512, 3)
        sketch.update(b"item", 5)
        sketch.reset()

        assert sketch.bucket_count() == 512
        assert sketch.depth() == 3
        assert sketch.elastic_ratio() == 0.2


class TestElasticSketchSerialization:
    """Test serialization and deserialization."""

    def test_serialize_deserialize(self):
        """Test that serialization and deserialization preserve state."""
        sketch = ElasticSketch(512, 3)
        sketch.update(b"item1", 5)
        sketch.update(b"item2", 3)

        data = sketch.serialize()
        assert isinstance(data, bytes)

        restored = ElasticSketch.deserialize(data)
        assert restored.bucket_count() == 512
        assert restored.depth() == 3
        assert restored.estimate(b"item1") == 5
        assert restored.estimate(b"item2") == 3

    def test_deserialize_invalid_data(self):
        """Test that deserializing invalid data raises error."""
        with pytest.raises(ValueError):
            ElasticSketch.deserialize(b"invalid")

    def test_serialize_empty_sketch(self):
        """Test serializing an empty sketch."""
        sketch = ElasticSketch(512, 3)
        data = sketch.serialize()
        restored = ElasticSketch.deserialize(data)
        assert restored.is_empty()

    def test_roundtrip_with_multiple_updates(self):
        """Test complete roundtrip with complex data."""
        sketch = ElasticSketch(1024, 4)
        for i in range(100):
            sketch.update(f"flow{i}".encode(), i + 1)

        data = sketch.serialize()
        restored = ElasticSketch.deserialize(data)

        for i in range(100):
            assert restored.estimate(f"flow{i}".encode()) == i + 1


class TestElasticSketchMemory:
    """Test memory usage tracking."""

    def test_memory_usage_calculation(self):
        """Test that memory usage is calculated correctly."""
        sketch = ElasticSketch(512, 3)
        memory = sketch.memory_usage()
        assert memory > 0
        # Memory should be bucket_count * depth * bucket_size
        # Each bucket is approximately 25 bytes
        expected_min = 512 * 3 * 20  # Conservative estimate
        assert memory >= expected_min


class TestElasticSketchRepr:
    """Test string representations."""

    def test_repr(self):
        """Test __repr__ output."""
        sketch = ElasticSketch(512, 3)
        repr_str = repr(sketch)
        assert "ElasticSketch" in repr_str
        assert "512" in repr_str
        assert "3" in repr_str

    def test_str(self):
        """Test __str__ output."""
        sketch = ElasticSketch(512, 3)
        str_str = str(sketch)
        assert "ElasticSketch" in str_str


class TestElasticSketchIntegration:
    """Integration tests for complete workflows."""

    def test_network_traffic_simulation(self):
        """Simulate network traffic monitoring."""
        sketch = ElasticSketch(bucket_count=1024, depth=4)

        # Simulate traffic from different flows
        flows = {
            b"tcp:192.168.1.1:80": 1000,
            b"tcp:192.168.1.2:443": 500,
            b"tcp:10.0.0.1:22": 100,
            b"udp:8.8.8.8:53": 5000,
            b"icmp:192.168.2.1": 50,
        }

        for flow, packets in flows.items():
            sketch.update(flow, packets)

        # Check estimations
        for flow, packets in flows.items():
            assert sketch.estimate(flow) == packets

        # Find heavy hitters
        hitters = sketch.heavy_hitters(threshold=300)
        assert len(hitters) >= 3

    def test_zipfian_distribution(self):
        """Test with Zipfian frequency distribution."""
        sketch = ElasticSketch(1024, 4)

        # Add items with Zipfian distribution
        for i in range(1, 101):
            freq = 1000 // i  # 1/k distribution
            sketch.update(f"item{i}".encode(), freq)

        # Verify top items
        top_freq = sketch.estimate(b"item1")
        assert top_freq == 1000

        # Get heavy hitters
        hitters = sketch.heavy_hitters(threshold=50)
        assert len(hitters) > 10

    def test_merge_distributed_collection(self):
        """Test merging sketches from distributed collection points."""
        # Simulate 3 collection points
        sketches = [ElasticSketch(512, 3) for _ in range(3)]

        # Each point collects different portions of traffic
        items = [(b"flow1", 30), (b"flow2", 20), (b"flow3", 10)]

        # Distribute data across sketches
        for i, (item, freq) in enumerate(items):
            sketches[0].update(item, freq)
            sketches[1].update(item, freq // 2)
            sketches[2].update(item, freq // 3)

        # Merge all into one
        sketches[0].merge(sketches[1])
        sketches[0].merge(sketches[2])

        # Verify totals
        assert sketches[0].estimate(b"flow1") >= 30
        assert sketches[0].estimate(b"flow2") >= 20


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
