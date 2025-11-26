"""Core tests for DDSketch quantile estimation sketch."""

import random

import pytest

import sketch_oxide


def test_ddsketch_creation():
    """Test creating DDSketch with valid relative accuracy."""
    ds = sketch_oxide.DDSketch(0.01)
    assert ds is not None


def test_ddsketch_update():
    """Test updating DDSketch with values."""
    ds = sketch_oxide.DDSketch(0.01)
    ds.update(100.0)
    assert ds is not None


def test_ddsketch_quantile():
    """Test quantile estimation."""
    ds = sketch_oxide.DDSketch(0.01)
    for i in range(1, 1001):
        ds.update(float(i))
    median = ds.quantile(0.5)
    assert 400 < median < 600


def test_ddsketch_monotonic_quantiles():
    """Test that quantiles are monotonically increasing."""
    ds = sketch_oxide.DDSketch(0.01)
    for i in range(1, 1001):
        ds.update(float(i))
    q25 = ds.quantile(0.25)
    q50 = ds.quantile(0.5)
    q75 = ds.quantile(0.75)
    assert q25 <= q50
    assert q50 <= q75


def test_ddsketch_merge():
    """Test merging two sketches."""
    ds1 = sketch_oxide.DDSketch(0.01)
    ds2 = sketch_oxide.DDSketch(0.01)
    for i in range(1, 501):
        ds1.update(float(i))
    for i in range(501, 1001):
        ds2.update(float(i))
    ds1.merge(ds2)
    median = ds1.quantile(0.5)
    true_median = 500.5
    relative_error = abs(median - true_median) / true_median
    assert relative_error < 0.2


def test_ddsketch_negative_values():
    """Test handling negative values."""
    ds = sketch_oxide.DDSketch(0.01)
    for i in range(-100, 101):
        ds.update(float(i))
    median = ds.quantile(0.5)
    assert abs(median - 0) < 5


def test_ddsketch_duplicate_values():
    """Test handling duplicate values."""
    ds = sketch_oxide.DDSketch(0.01)
    for _ in range(100):
        ds.update(50.0)
    median = ds.quantile(0.5)
    assert abs(median - 50.0) < 2


def test_ddsketch_single_value():
    """Test handling single value."""
    ds = sketch_oxide.DDSketch(0.01)
    ds.update(42.0)
    q50 = ds.quantile(0.5)
    assert abs(q50 - 42.0) < 2


def test_ddsketch_min_max():
    """Test min and max tracking."""
    ds = sketch_oxide.DDSketch(0.01)
    for i in range(1, 101):
        ds.update(float(i))
    assert ds.min() == 1.0
    assert ds.max() == 100.0


def test_ddsketch_count():
    """Test count tracking."""
    ds = sketch_oxide.DDSketch(0.01)
    for i in range(100):
        ds.update(float(i))
    assert ds.count() == 100


def test_ddsketch_is_empty():
    """Test empty state check."""
    ds = sketch_oxide.DDSketch(0.01)
    assert ds.is_empty()
    ds.update(1.0)
    assert not ds.is_empty()


def test_ddsketch_large_dataset():
    """Test with large dataset."""
    ds = sketch_oxide.DDSketch(0.01)
    for _ in range(100000):
        ds.update(random.random() * 1000)
    median = ds.quantile(0.5)
    p99 = ds.quantile(0.99)
    assert median > 0
    assert p99 > median


def test_ddsketch_different_accuracy():
    """Test with different relative accuracy values."""
    for alpha in [0.001, 0.01, 0.05, 0.1]:
        ds = sketch_oxide.DDSketch(alpha)
        for i in range(1, 1001):
            ds.update(float(i))
        median = ds.quantile(0.5)
        assert median > 0
