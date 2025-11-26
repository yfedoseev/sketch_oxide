"""
Benchmarks for new Tier 1 sketches

Run with: pytest benchmarks/bench_new_sketches.py --benchmark-only
"""

from sketch_oxide import (
    Grafite,
    HeavyKeeper,
    MementoFilter,
    RatelessIBLT,
    SlidingHyperLogLog,
)

# ============================================================================
# HeavyKeeper Benchmarks
# ============================================================================


def test_heavy_keeper_update_single(benchmark):
    """Benchmark single item updates"""
    hk = HeavyKeeper(k=100, epsilon=0.001, delta=0.01)
    item = "test_item"

    def run():
        for _ in range(1000):
            hk.update(item)

    benchmark(run)


def test_heavy_keeper_update_diverse(benchmark):
    """Benchmark updates with diverse items"""
    hk = HeavyKeeper(k=100, epsilon=0.001, delta=0.01)

    def run():
        for i in range(1000):
            hk.update(f"item_{i % 100}")

    benchmark(run)


def test_heavy_keeper_estimate(benchmark):
    """Benchmark frequency estimation"""
    hk = HeavyKeeper(k=100, epsilon=0.001, delta=0.01)
    for i in range(1000):
        hk.update(f"item_{i % 100}")

    def run():
        for i in range(100):
            hk.estimate(f"item_{i}")

    benchmark(run)


def test_heavy_keeper_top_k(benchmark):
    """Benchmark top-k retrieval"""
    hk = HeavyKeeper(k=100, epsilon=0.001, delta=0.01)
    for i in range(10000):
        hk.update(f"item_{i % 100}")

    benchmark(hk.top_k)


def test_heavy_keeper_decay(benchmark):
    """Benchmark exponential decay"""
    hk = HeavyKeeper(k=100, epsilon=0.001, delta=0.01)
    for i in range(10000):
        hk.update(f"item_{i % 100}")

    benchmark(hk.decay)


def test_heavy_keeper_merge(benchmark):
    """Benchmark merging two sketches"""
    hk1 = HeavyKeeper(k=100, epsilon=0.001, delta=0.01)
    hk2 = HeavyKeeper(k=100, epsilon=0.001, delta=0.01)

    for i in range(5000):
        hk1.update(f"item_{i % 100}")
        hk2.update(f"item_{(i + 50) % 100}")

    def run():
        hk1_copy = HeavyKeeper(k=100, epsilon=0.001, delta=0.01)
        for i in range(5000):
            hk1_copy.update(f"item_{i % 100}")
        hk1_copy.merge(hk2)

    benchmark(run)


def test_heavy_keeper_batch_update(benchmark):
    """Benchmark batch updates"""
    hk = HeavyKeeper(k=100, epsilon=0.001, delta=0.01)
    items = [f"item_{i % 100}" for i in range(1000)]

    def run():
        hk.update_batch(items)

    benchmark(run)


# ============================================================================
# RatelessIBLT Benchmarks
# ============================================================================


def test_rateless_iblt_insert(benchmark):
    """Benchmark insertions"""
    iblt = RatelessIBLT(expected_diff=1000, cell_size=32)

    def run():
        for i in range(100):
            iblt.insert(f"key_{i}".encode(), f"value_{i}".encode())

    benchmark(run)


def test_rateless_iblt_delete(benchmark):
    """Benchmark deletions"""
    iblt = RatelessIBLT(expected_diff=1000, cell_size=32)
    for i in range(100):
        iblt.insert(f"key_{i}".encode(), f"value_{i}".encode())

    def run():
        for i in range(100):
            iblt.delete(f"key_{i}".encode(), f"value_{i}".encode())

    benchmark(run)


def test_rateless_iblt_subtract(benchmark):
    """Benchmark subtraction"""
    iblt1 = RatelessIBLT(expected_diff=100, cell_size=32)
    iblt2 = RatelessIBLT(expected_diff=100, cell_size=32)

    for i in range(50):
        iblt1.insert(f"key_{i}".encode(), f"value_{i}".encode())
    for i in range(25, 75):
        iblt2.insert(f"key_{i}".encode(), f"value_{i}".encode())

    def run():
        iblt1_copy = RatelessIBLT(expected_diff=100, cell_size=32)
        for i in range(50):
            iblt1_copy.insert(f"key_{i}".encode(), f"value_{i}".encode())
        iblt1_copy.subtract(iblt2)

    benchmark(run)


def test_rateless_iblt_decode(benchmark):
    """Benchmark decoding"""
    iblt = RatelessIBLT(expected_diff=20, cell_size=32)
    for i in range(10):
        iblt.insert(f"key_{i}".encode(), f"value_{i}".encode())

    benchmark(iblt.decode)


# ============================================================================
# Grafite Benchmarks
# ============================================================================


def test_grafite_build_small(benchmark):
    """Benchmark building with small key set"""
    keys = list(range(100))

    def run():
        Grafite(keys, bits_per_key=6)

    benchmark(run)


def test_grafite_build_large(benchmark):
    """Benchmark building with large key set"""
    keys = list(range(10000))

    def run():
        Grafite(keys, bits_per_key=6)

    benchmark(run)


def test_grafite_point_query(benchmark):
    """Benchmark point queries"""
    keys = list(range(0, 10000, 10))
    filter = Grafite(keys, bits_per_key=6)

    def run():
        for i in range(1000):
            filter.may_contain(i * 10)

    benchmark(run)


def test_grafite_range_query(benchmark):
    """Benchmark range queries"""
    keys = list(range(0, 10000, 10))
    filter = Grafite(keys, bits_per_key=6)

    def run():
        for i in range(100):
            filter.may_contain_range(i * 100, i * 100 + 50)

    benchmark(run)


# ============================================================================
# MementoFilter Benchmarks
# ============================================================================


def test_memento_insert(benchmark):
    """Benchmark insertions"""
    filter = MementoFilter(expected_elements=10000, fpr=0.01)

    def run():
        for i in range(1000):
            filter.insert(i, f"value_{i}".encode())

    benchmark(run)


def test_memento_range_query(benchmark):
    """Benchmark range queries"""
    filter = MementoFilter(expected_elements=10000, fpr=0.01)
    for i in range(1000):
        filter.insert(i, f"value_{i}".encode())

    def run():
        for i in range(100):
            filter.may_contain_range(i * 10, i * 10 + 50)

    benchmark(run)


def test_memento_sequential_insert_query(benchmark):
    """Benchmark mixed insert and query operations"""
    filter = MementoFilter(expected_elements=10000, fpr=0.01)

    def run():
        for i in range(500):
            filter.insert(i, f"value_{i}".encode())
            if i % 10 == 0:
                filter.may_contain_range(0, i)

    benchmark(run)


# ============================================================================
# SlidingHyperLogLog Benchmarks
# ============================================================================


def test_sliding_hll_update(benchmark):
    """Benchmark updates with timestamps"""
    hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
    timestamp = 1000

    def run():
        for i in range(1000):
            hll.update(f"item_{i}", timestamp=timestamp + i)

    benchmark(run)


def test_sliding_hll_update_integers(benchmark):
    """Benchmark updates with integer items"""
    hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
    timestamp = 1000

    def run():
        for i in range(1000):
            hll.update(i, timestamp=timestamp + i)

    benchmark(run)


def test_sliding_hll_estimate_window(benchmark):
    """Benchmark window estimation"""
    hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
    for i in range(10000):
        hll.update(i, timestamp=1000 + i)

    def run():
        for i in range(100):
            hll.estimate_window(current_time=5000 + i, window_seconds=600)

    benchmark(run)


def test_sliding_hll_estimate_total(benchmark):
    """Benchmark total estimation"""
    hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
    for i in range(10000):
        hll.update(i, timestamp=1000)

    benchmark(hll.estimate_total)


def test_sliding_hll_decay(benchmark):
    """Benchmark decay operation"""
    hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
    for i in range(10000):
        hll.update(i, timestamp=1000 + (i % 1000))

    def run():
        hll.decay(current_time=5000, window_seconds=600)

    benchmark(run)


def test_sliding_hll_merge(benchmark):
    """Benchmark merging two sketches"""
    hll1 = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
    hll2 = SlidingHyperLogLog(precision=12, max_window_seconds=3600)

    for i in range(5000):
        hll1.update(i, timestamp=1000)
        hll2.update(i + 2500, timestamp=1000)

    def run():
        hll1_copy = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
        for i in range(5000):
            hll1_copy.update(i, timestamp=1000)
        hll1_copy.merge(hll2)

    benchmark(run)


def test_sliding_hll_serialize(benchmark):
    """Benchmark serialization"""
    hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
    for i in range(10000):
        hll.update(i, timestamp=1000)

    benchmark(hll.serialize)


def test_sliding_hll_deserialize(benchmark):
    """Benchmark deserialization"""
    hll = SlidingHyperLogLog(precision=12, max_window_seconds=3600)
    for i in range(10000):
        hll.update(i, timestamp=1000)
    data = hll.serialize()

    def run():
        SlidingHyperLogLog.deserialize(data)

    benchmark(run)


# ============================================================================
# Comparative Benchmarks
# ============================================================================


def test_compare_heavy_keeper_vs_batch(benchmark):
    """Compare individual vs batch updates for HeavyKeeper"""
    items = [f"item_{i % 100}" for i in range(1000)]

    def individual():
        hk = HeavyKeeper(k=100, epsilon=0.001, delta=0.01)
        for item in items:
            hk.update(item)

    def batch():
        hk = HeavyKeeper(k=100, epsilon=0.001, delta=0.01)
        hk.update_batch(items)

    # Benchmark batch (should be faster)
    benchmark(batch)


def test_compare_grafite_point_vs_range(benchmark):
    """Compare point vs range queries for Grafite"""
    keys = list(range(0, 10000, 10))
    filter = Grafite(keys, bits_per_key=6)

    def point_queries():
        for i in range(1000):
            filter.may_contain(i * 10)

    def range_queries():
        for i in range(1000):
            filter.may_contain_range(i * 10, i * 10 + 1)

    # Benchmark point queries
    benchmark(point_queries)
