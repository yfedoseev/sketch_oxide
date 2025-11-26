import pytest

from sketch_oxide import (
    Grafite,
    HeavyKeeper,
    MementoFilter,
    RatelessIBLT,
    SlidingHyperLogLog,
)


class TestHeavyKeeper:
    """Test suite for HeavyKeeper top-k frequency estimation"""

    def test_construction(self):
        """Test basic construction"""
        hk = HeavyKeeper(k=10, epsilon=0.001, delta=0.01)
        assert hk is not None
        assert hk.is_empty()

    def test_invalid_parameters(self):
        """Test parameter validation"""
        with pytest.raises(ValueError):
            HeavyKeeper(k=0, epsilon=0.001, delta=0.01)  # k must be > 0
        with pytest.raises(ValueError):
            HeavyKeeper(k=10, epsilon=0.0, delta=0.01)  # epsilon must be in (0, 1)
        with pytest.raises(ValueError):
            HeavyKeeper(k=10, epsilon=0.001, delta=0.0)  # delta must be in (0, 1)

    def test_update_and_estimate_str(self):
        """Test updating with strings"""
        hk = HeavyKeeper(k=10, epsilon=0.001, delta=0.01)
        for i in range(100):
            hk.update(f"item_{i % 10}")

        count = hk.estimate("item_5")
        assert count > 0
        assert count >= 8  # Should have at least 8 out of 10 occurrences

    def test_update_and_estimate_int(self):
        """Test updating with integers"""
        hk = HeavyKeeper(k=10, epsilon=0.001, delta=0.01)
        for i in range(100):
            hk.update(i % 10)

        count = hk.estimate(5)
        assert count > 0
        assert count >= 8

    def test_update_and_estimate_bytes(self):
        """Test updating with bytes"""
        hk = HeavyKeeper(k=10, epsilon=0.001, delta=0.01)
        for i in range(100):
            hk.update(f"item_{i % 10}".encode())

        count = hk.estimate(b"item_5")
        assert count > 0

    def test_top_k(self):
        """Test retrieving top-k items"""
        hk = HeavyKeeper(k=5, epsilon=0.001, delta=0.01)
        for i in range(100):
            hk.update(f"item_{i % 10}")

        top_k = hk.top_k()
        assert len(top_k) <= 5
        assert all(isinstance(item, tuple) for item in top_k)
        assert all(len(item) == 2 for item in top_k)

        # Check sorted by count descending
        if len(top_k) > 1:
            counts = [count for _, count in top_k]
            assert counts == sorted(counts, reverse=True)

    def test_decay(self):
        """Test exponential decay"""
        hk = HeavyKeeper(k=10, epsilon=0.001, delta=0.01)
        hk.update("item")
        before = hk.estimate("item")
        assert before > 0

        hk.decay()
        after = hk.estimate("item")
        assert after < before or after == 0

    def test_merge(self):
        """Test merging two HeavyKeeper sketches"""
        hk1 = HeavyKeeper(k=10, epsilon=0.001, delta=0.01)
        hk2 = HeavyKeeper(k=10, epsilon=0.001, delta=0.01)

        for _ in range(50):
            hk1.update("item")
        for _ in range(30):
            hk2.update("item")

        hk1.merge(hk2)
        count = hk1.estimate("item")
        assert count >= 70  # Should be roughly 80

    def test_incompatible_merge(self):
        """Test merging incompatible sketches fails"""
        hk1 = HeavyKeeper(k=10, epsilon=0.001, delta=0.01)
        hk2 = HeavyKeeper(k=5, epsilon=0.001, delta=0.01)  # Different k

        with pytest.raises(ValueError):
            hk1.merge(hk2)

    def test_update_batch(self):
        """Test batch updates"""
        hk = HeavyKeeper(k=10, epsilon=0.001, delta=0.01)
        items = ["apple", "banana", "apple", "cherry", "apple"]
        hk.update_batch(items)

        assert hk.estimate("apple") >= 2
        assert hk.estimate("banana") >= 1

    def test_stats(self):
        """Test statistics retrieval"""
        hk = HeavyKeeper(k=100, epsilon=0.001, delta=0.01)
        stats = hk.stats()

        assert "total_updates" in stats
        assert "k" in stats
        assert "memory_bits" in stats
        assert "depth" in stats
        assert "width" in stats
        assert stats["k"] == 100

    def test_repr(self):
        """Test string representation"""
        hk = HeavyKeeper(k=100, epsilon=0.001, delta=0.01)
        repr_str = repr(hk)
        assert "HeavyKeeper" in repr_str
        assert "100" in repr_str


class TestRatelessIBLT:
    """Test suite for Rateless IBLT set reconciliation"""

    def test_construction(self):
        """Test basic construction"""
        iblt = RatelessIBLT(expected_diff=100, cell_size=32)
        assert iblt is not None

    def test_invalid_parameters(self):
        """Test parameter validation"""
        with pytest.raises(ValueError):
            RatelessIBLT(expected_diff=0, cell_size=32)
        with pytest.raises(ValueError):
            RatelessIBLT(expected_diff=100, cell_size=4)  # Too small

    def test_basic_insert(self):
        """Test basic insertion"""
        iblt = RatelessIBLT(expected_diff=10, cell_size=32)
        iblt.insert(b"key", b"value")
        # No error means success

    def test_basic_decode(self):
        """Test decoding with single item"""
        iblt = RatelessIBLT(expected_diff=10, cell_size=32)
        iblt.insert(b"key", b"value")

        result = iblt.decode()
        assert "to_insert" in result
        assert "to_remove" in result
        assert len(result["to_insert"]) == 1
        assert len(result["to_remove"]) == 0

    def test_symmetric_difference(self):
        """Test computing symmetric difference"""
        alice = RatelessIBLT(expected_diff=10, cell_size=32)
        bob = RatelessIBLT(expected_diff=10, cell_size=32)

        # Shared items
        alice.insert(b"shared1", b"value1")
        bob.insert(b"shared1", b"value1")
        alice.insert(b"shared2", b"value2")
        bob.insert(b"shared2", b"value2")

        # Alice-only item
        alice.insert(b"alice_only", b"alice_value")

        # Bob-only item
        bob.insert(b"bob_only", b"bob_value")

        # Compute difference
        alice.subtract(bob)
        result = alice.decode()

        assert len(result["to_insert"]) == 1  # alice_only
        assert len(result["to_remove"]) == 1  # bob_only

    def test_delete(self):
        """Test deletion"""
        iblt = RatelessIBLT(expected_diff=10, cell_size=32)
        iblt.insert(b"key", b"value")
        iblt.delete(b"key", b"value")

        result = iblt.decode()
        # After insert then delete, should decode to empty
        assert len(result["to_insert"]) == 0
        assert len(result["to_remove"]) == 0

    def test_incompatible_subtract(self):
        """Test subtracting incompatible IBLTs fails"""
        iblt1 = RatelessIBLT(expected_diff=10, cell_size=32)
        iblt2 = RatelessIBLT(expected_diff=20, cell_size=32)

        with pytest.raises(ValueError):
            iblt1.subtract(iblt2)

    def test_stats(self):
        """Test statistics retrieval"""
        iblt = RatelessIBLT(expected_diff=100, cell_size=32)
        stats = iblt.stats()

        assert "num_cells" in stats
        assert "cell_size" in stats
        assert stats["cell_size"] == 32

    def test_repr(self):
        """Test string representation"""
        iblt = RatelessIBLT(expected_diff=100, cell_size=32)
        repr_str = repr(iblt)
        assert "RatelessIBLT" in repr_str


class TestGrafite:
    """Test suite for Grafite optimal range filter"""

    def test_construction(self):
        """Test basic construction"""
        keys = [10, 20, 30, 40, 50]
        filter = Grafite(keys, bits_per_key=6)
        assert filter is not None

    def test_invalid_parameters(self):
        """Test parameter validation"""
        with pytest.raises(ValueError):
            Grafite([], bits_per_key=6)  # Empty keys
        with pytest.raises(ValueError):
            Grafite([1, 2, 3], bits_per_key=1)  # bits_per_key too small
        with pytest.raises(ValueError):
            Grafite([1, 2, 3], bits_per_key=20)  # bits_per_key too large

    def test_point_query(self):
        """Test point queries"""
        keys = [100, 200, 300, 400, 500]
        filter = Grafite(keys, bits_per_key=6)

        # Keys that exist
        assert filter.may_contain(100)
        assert filter.may_contain(200)
        assert filter.may_contain(300)

    def test_range_query(self):
        """Test range queries"""
        keys = [10, 20, 30, 40, 50]
        filter = Grafite(keys, bits_per_key=6)

        # Ranges containing keys
        assert filter.may_contain_range(15, 25)  # Contains 20
        assert filter.may_contain_range(10, 50)  # Contains all
        assert filter.may_contain_range(25, 35)  # Contains 30

    def test_deduplication(self):
        """Test automatic deduplication"""
        keys = [10, 20, 20, 30, 30, 30]  # Duplicates
        filter = Grafite(keys, bits_per_key=6)

        stats = filter.stats()
        assert stats["key_count"] == 3  # Should have 3 unique keys

    def test_expected_fpr(self):
        """Test FPR calculation"""
        filter = Grafite([1, 2, 3], bits_per_key=6)

        # FPR = range_width / 2^(bits_per_key - 2)
        # For bits_per_key=6: denominator = 2^4 = 16
        fpr = filter.expected_fpr(10)
        expected = 10.0 / 16.0  # 0.625
        assert abs(fpr - expected) < 0.001

    def test_stats(self):
        """Test statistics retrieval"""
        keys = [1, 2, 3, 4, 5]
        filter = Grafite(keys, bits_per_key=6)
        stats = filter.stats()

        assert "key_count" in stats
        assert "bits_per_key" in stats
        assert "total_bits" in stats
        assert stats["key_count"] == 5
        assert stats["bits_per_key"] == 6

    def test_key_count(self):
        """Test key count accessor"""
        keys = [1, 2, 3, 4, 5]
        filter = Grafite(keys, bits_per_key=6)
        assert filter.key_count() == 5

    def test_bits_per_key(self):
        """Test bits per key accessor"""
        filter = Grafite([1, 2, 3], bits_per_key=8)
        assert filter.bits_per_key() == 8

    def test_repr(self):
        """Test string representation"""
        filter = Grafite([1, 2, 3], bits_per_key=6)
        repr_str = repr(filter)
        assert "Grafite" in repr_str


class TestMementoFilter:
    """Test suite for Memento Filter dynamic range filter"""

    def test_construction(self):
        """Test basic construction"""
        filter = MementoFilter(expected_elements=1000, fpr=0.01)
        assert filter is not None
        assert filter.is_empty()
        assert filter.len() == 0

    def test_invalid_parameters(self):
        """Test parameter validation"""
        with pytest.raises(ValueError):
            MementoFilter(expected_elements=0, fpr=0.01)
        with pytest.raises(ValueError):
            MementoFilter(expected_elements=1000, fpr=0.0)
        with pytest.raises(ValueError):
            MementoFilter(expected_elements=1000, fpr=1.0)

    def test_basic_insertion(self):
        """Test basic insertion"""
        filter = MementoFilter(expected_elements=1000, fpr=0.01)
        filter.insert(42, b"value")

        assert filter.len() == 1
        assert not filter.is_empty()

    def test_range_query(self):
        """Test range queries"""
        filter = MementoFilter(expected_elements=1000, fpr=0.01)
        filter.insert(50, b"value")

        assert filter.may_contain_range(45, 55)  # Contains 50
        assert filter.may_contain_range(50, 50)  # Point query

    def test_multiple_insertions(self):
        """Test multiple insertions"""
        filter = MementoFilter(expected_elements=1000, fpr=0.01)

        for i in range(100):
            filter.insert(i, f"value_{i}".encode())

        assert filter.len() == 100

    def test_range_bounds(self):
        """Test range bounds tracking"""
        filter = MementoFilter(expected_elements=1000, fpr=0.01)

        assert filter.range() is None  # Empty filter

        filter.insert(10, b"value1")
        assert filter.range() == (10, 10)

        filter.insert(100, b"value2")
        assert filter.range() == (10, 100)

        filter.insert(50, b"value3")
        assert filter.range() == (10, 100)  # Still (10, 100)

    def test_capacity_exceeded(self):
        """Test that exceeding capacity raises error"""
        filter = MementoFilter(expected_elements=10, fpr=0.01)

        for i in range(10):
            filter.insert(i, b"value")

        # 11th insertion should fail
        with pytest.raises(ValueError):
            filter.insert(10, b"value")

    def test_stats(self):
        """Test statistics retrieval"""
        filter = MementoFilter(expected_elements=1000, fpr=0.01)
        filter.insert(42, b"value")

        stats = filter.stats()
        assert "num_elements" in stats
        assert "capacity" in stats
        assert "fpr_target" in stats
        assert "num_expansions" in stats
        assert "load_factor" in stats

        assert stats["num_elements"] == 1
        assert stats["capacity"] == 1000
        assert abs(stats["fpr_target"] - 0.01) < 0.001

    def test_len_method(self):
        """Test __len__ method"""
        filter = MementoFilter(expected_elements=1000, fpr=0.01)
        assert len(filter) == 0

        filter.insert(1, b"value")
        assert len(filter) == 1

    def test_repr(self):
        """Test string representation"""
        filter = MementoFilter(expected_elements=1000, fpr=0.01)
        repr_str = repr(filter)
        assert "MementoFilter" in repr_str


class TestSlidingHyperLogLog:
    """Test suite for Sliding HyperLogLog time-windowed cardinality"""

    def test_construction(self):
        """Test basic construction"""
        hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
        assert hll is not None
        assert hll.is_empty()

    def test_invalid_precision(self):
        """Test precision validation"""
        with pytest.raises(ValueError):
            SlidingHyperLogLog(precision=3, max_window_seconds=3600)
        with pytest.raises(ValueError):
            SlidingHyperLogLog(precision=17, max_window_seconds=3600)

    def test_update_str(self):
        """Test updating with strings"""
        hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
        hll.update("user_123", timestamp=1000)
        assert not hll.is_empty()

    def test_update_int(self):
        """Test updating with integers"""
        hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
        hll.update(42, timestamp=1000)
        assert not hll.is_empty()

    def test_update_bytes(self):
        """Test updating with bytes"""
        hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
        hll.update(b"data", timestamp=1000)
        assert not hll.is_empty()

    def test_update_float(self):
        """Test updating with floats"""
        hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
        hll.update(3.14159, timestamp=1000)
        assert not hll.is_empty()

    def test_estimate_total(self):
        """Test total cardinality estimation"""
        hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)

        # Add 100 unique items
        for i in range(100):
            hll.update(i, timestamp=1000)

        estimate = hll.estimate_total()
        assert estimate > 50  # Should be close to 100
        assert estimate < 150  # Within reasonable error

    def test_estimate_window(self):
        """Test window-based estimation"""
        hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)

        # Add items at different times
        for i in range(50):
            hll.update(i, timestamp=1000)
        for i in range(50, 100):
            hll.update(i, timestamp=2000)

        # Window covering only second batch
        estimate = hll.estimate_window(current_time=2500, window_seconds=600)
        assert estimate > 20  # Should see some of the second batch

        # Window covering both batches
        estimate = hll.estimate_window(current_time=2500, window_seconds=2000)
        assert estimate > 50  # Should see more items

    def test_decay(self):
        """Test decay operation"""
        hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)

        hll.update("old_item", timestamp=1000)
        before = hll.estimate_total()

        # Decay with window that excludes old item
        hll.decay(current_time=5000, window_seconds=600)

        # Item should be decayed
        estimate = hll.estimate_window(current_time=5000, window_seconds=600)
        assert estimate < before or estimate == 0

    def test_merge(self):
        """Test merging two sketches"""
        hll1 = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
        hll2 = SlidingHyperLogLog(precision=12, max_window_seconds=3600)

        for i in range(100):
            hll1.update(i, timestamp=1000)
        for i in range(50, 150):
            hll2.update(i, timestamp=1000)

        hll1.merge(hll2)
        estimate = hll1.estimate_total()
        assert estimate > 100  # Should estimate ~150 unique items

    def test_incompatible_merge(self):
        """Test merging incompatible sketches fails"""
        hll1 = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
        hll2 = SlidingHyperLogLog(precision=10, max_window_seconds=3600)

        with pytest.raises(ValueError):
            hll1.merge(hll2)

    def test_precision(self):
        """Test precision accessor"""
        hll = SlidingHyperLogLog(precision=14, max_window_seconds=3600)
        assert hll.precision() == 14

    def test_num_registers(self):
        """Test num_registers accessor"""
        hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
        assert hll.num_registers() == 4096  # 2^12

    def test_standard_error(self):
        """Test standard error calculation"""
        hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
        error = hll.standard_error()
        expected = 1.04 / (4096**0.5)  # 1.04 / sqrt(2^12)
        assert abs(error - expected) < 0.001

    def test_serialization(self):
        """Test serialization and deserialization"""
        hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)

        for i in range(100):
            hll.update(i, timestamp=1000)

        # Serialize
        data = hll.serialize()
        assert isinstance(data, bytes)
        assert len(data) > 0

        # Deserialize
        restored = SlidingHyperLogLog.deserialize(data)
        assert restored.precision() == 12
        assert abs(restored.estimate_total() - hll.estimate_total()) < 1.0

    def test_stats(self):
        """Test statistics retrieval"""
        hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
        stats = hll.stats()

        assert "precision" in stats
        assert "max_window_seconds" in stats
        assert "total_updates" in stats
        assert stats["precision"] == 12
        assert stats["max_window_seconds"] == 3600

    def test_repr(self):
        """Test string representation"""
        hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
        repr_str = repr(hll)
        assert "SlidingHyperLogLog" in repr_str
        assert "12" in repr_str
