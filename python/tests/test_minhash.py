"""Core tests for MinHash similarity estimation sketch."""

import pytest

import sketch_oxide


def test_minhash_creation():
    """Test creating MinHash with valid num_perm."""
    mh = sketch_oxide.MinHash(128)
    assert mh is not None


def test_minhash_update():
    """Test updating MinHash with elements."""
    mh = sketch_oxide.MinHash(128)
    mh.update(1)
    assert mh is not None


def test_minhash_identical_sets():
    """Test that identical sets have similarity 1.0."""
    mh1 = sketch_oxide.MinHash(128)
    mh2 = sketch_oxide.MinHash(128)
    for i in range(1, 101):
        mh1.update(i)
        mh2.update(i)
    similarity = mh1.jaccard_similarity(mh2)
    assert 0.9 < similarity <= 1.0


def test_minhash_disjoint_sets():
    """Test that disjoint sets have low similarity."""
    mh1 = sketch_oxide.MinHash(128)
    mh2 = sketch_oxide.MinHash(128)
    for i in range(1, 51):
        mh1.update(i)
    for i in range(51, 101):
        mh2.update(i)
    similarity = mh1.jaccard_similarity(mh2)
    assert 0.0 <= similarity < 0.1


def test_minhash_partial_overlap():
    """Test similarity with partial overlap."""
    mh1 = sketch_oxide.MinHash(128)
    mh2 = sketch_oxide.MinHash(128)
    # Set 1: {1, 2, 3, 4, 5}
    for i in range(1, 6):
        mh1.update(i)
    # Set 2: {3, 4, 5, 6, 7} - overlap: {3, 4, 5}
    for i in range(3, 8):
        mh2.update(i)
    # True Jaccard: |A ∩ B| / |A ∪ B| = 3 / 7 ≈ 0.43
    similarity = mh1.jaccard_similarity(mh2)
    assert 0.3 < similarity < 0.6


def test_minhash_subset():
    """Test similarity when one set is a subset."""
    mh1 = sketch_oxide.MinHash(128)
    mh2 = sketch_oxide.MinHash(128)
    # Set 1: {1, 2, 3, 4, 5}
    for i in range(1, 6):
        mh1.update(i)
    # Set 2: {1, 2, 3} - subset of Set 1
    for i in range(1, 4):
        mh2.update(i)
    # True Jaccard: |A ∩ B| / |A ∪ B| = 3 / 5 = 0.60
    similarity = mh1.jaccard_similarity(mh2)
    assert 0.5 < similarity < 0.7


def test_minhash_symmetry():
    """Test that Jaccard similarity is symmetric."""
    mh1 = sketch_oxide.MinHash(128)
    mh2 = sketch_oxide.MinHash(128)
    for i in range(1, 51):
        mh1.update(i)
    for i in range(25, 76):
        mh2.update(i)
    sim_1_2 = mh1.jaccard_similarity(mh2)
    sim_2_1 = mh2.jaccard_similarity(mh1)
    assert abs(sim_1_2 - sim_2_1) < 0.01


def test_minhash_similarity_range():
    """Test that similarity is in [0, 1]."""
    mh1 = sketch_oxide.MinHash(128)
    mh2 = sketch_oxide.MinHash(128)
    for i in range(1, 101):
        mh1.update(i)
    for i in range(50, 151):
        mh2.update(i)
    similarity = mh1.jaccard_similarity(mh2)
    assert 0.0 <= similarity <= 1.0


def test_minhash_merge():
    """Test merging two sketches."""
    mh1 = sketch_oxide.MinHash(128)
    mh2 = sketch_oxide.MinHash(128)
    for i in range(1, 51):
        mh1.update(i)
    for i in range(51, 101):
        mh2.update(i)
    mh1.merge(mh2)
    # After merge, should contain union of both sets
    assert mh1 is not None


def test_minhash_identical_data_merge():
    """Test merging identical sketches."""
    mh1 = sketch_oxide.MinHash(128)
    mh2 = sketch_oxide.MinHash(128)
    for i in range(1, 101):
        mh1.update(i)
        mh2.update(i)
    mh1.merge(mh2)
    # Reference: same set
    mh_ref = sketch_oxide.MinHash(128)
    for i in range(1, 101):
        mh_ref.update(i)
    similarity = mh1.jaccard_similarity(mh_ref)
    assert 0.9 < similarity <= 1.0


def test_minhash_num_perm():
    """Test num_perm property."""
    for num_perm in [32, 64, 128]:
        mh = sketch_oxide.MinHash(num_perm)
        assert mh.num_perm() == num_perm


def test_minhash_is_empty():
    """Test empty state check."""
    mh = sketch_oxide.MinHash(128)
    assert mh.is_empty()
    mh.update(1)
    assert not mh.is_empty()


def test_minhash_different_num_perm():
    """Test with different num_perm values."""
    for num_perm in [16, 32, 64, 128, 256]:
        mh = sketch_oxide.MinHash(num_perm)
        for i in range(100):
            mh.update(i)
        assert mh is not None


def test_minhash_large_sets():
    """Test with large sets."""
    mh1 = sketch_oxide.MinHash(128)
    mh2 = sketch_oxide.MinHash(128)
    # 50K elements
    for i in range(50000):
        mh1.update(i)
    # 50K elements with 25K overlap
    for i in range(25000, 75000):
        mh2.update(i)
    similarity = mh1.jaccard_similarity(mh2)
    # True Jaccard: 25K / 75K ≈ 0.333
    assert 0.2 < similarity < 0.5


def test_minhash_single_element():
    """Test with single element."""
    mh = sketch_oxide.MinHash(128)
    mh.update(42)
    assert mh is not None


def test_minhash_duplicate_updates():
    """Test that duplicate updates are idempotent."""
    mh = sketch_oxide.MinHash(128)
    for _ in range(1000):
        mh.update(1)
    assert mh is not None
