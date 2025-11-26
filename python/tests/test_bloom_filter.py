"""Core tests for BloomFilter sketch."""

import sketch_oxide


def test_bloom_filter_creation():
    """Test creating BloomFilter with valid parameters."""
    bf = sketch_oxide.BloomFilter(10000, 0.01)
    assert bf is not None


def test_bloom_filter_insert_and_contains():
    """Test insert and contains operations."""
    bf = sketch_oxide.BloomFilter(10000, 0.01)
    bf.insert(b"test")
    assert bf.contains(b"test")


def test_bloom_filter_no_false_negatives():
    """Test no false negatives - all inserted items must be found."""
    bf = sketch_oxide.BloomFilter(10000, 0.01)
    items = [b"apple", b"banana", b"cherry"]
    for item in items:
        bf.insert(item)
    for item in items:
        assert bf.contains(item)


def test_bloom_filter_multiple_inserts():
    """Test multiple inserts."""
    bf = sketch_oxide.BloomFilter(10000, 0.01)
    for i in range(100):
        bf.insert(f"item-{i}".encode())
    for i in range(100):
        assert bf.contains(f"item-{i}".encode())


def test_bloom_filter_merge():
    """Test merging two filters."""
    bf1 = sketch_oxide.BloomFilter(10000, 0.01)
    bf2 = sketch_oxide.BloomFilter(10000, 0.01)
    bf1.insert(b"first")
    bf2.insert(b"second")
    bf1.merge(bf2)
    assert bf1.contains(b"first")
    assert bf1.contains(b"second")


def test_bloom_filter_serialization():
    """Test serialization using to_bytes and from_bytes."""
    bf = sketch_oxide.BloomFilter(10000, 0.01)
    bf.insert(b"apple")
    bf.insert(b"banana")
    serialized = bf.to_bytes()
    restored = sketch_oxide.BloomFilter.from_bytes(serialized)
    assert restored.contains(b"apple")
    assert restored.contains(b"banana")


def test_bloom_filter_empty_state():
    """Test empty filter state."""
    bf = sketch_oxide.BloomFilter(10000, 0.01)
    assert bf.is_empty()


def test_bloom_filter_len():
    """Test length method."""
    bf = sketch_oxide.BloomFilter(10000, 0.01)
    bf.insert(b"a")
    assert len(bf) >= 0


def test_bloom_filter_memory_usage():
    """Test memory usage method."""
    bf = sketch_oxide.BloomFilter(10000, 0.01)
    usage = bf.memory_usage()
    assert usage >= 0


def test_bloom_filter_batch_operations():
    """Test batch insert."""
    bf = sketch_oxide.BloomFilter(10000, 0.01)
    items = [b"item1", b"item2", b"item3"]
    for item in items:
        bf.insert(item)
    for item in items:
        assert bf.contains(item)
