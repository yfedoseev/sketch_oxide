"""
Performance benchmarks for Tier 2 sketches in sketch_oxide.

Benchmarks cover:
- Vacuum Filter: Insert, query, delete throughput
- GRF: Build time, range query performance
- NitroSketch: Update throughput with sampling, sync performance
- UnivMon: Update throughput, multi-metric query performance
- LearnedBloomFilter: Build time, query throughput

Uses pytest-benchmark for accurate measurements with warmup and statistics.

Run with:
    pytest python/benchmarks/bench_tier2_sketches.py -v
    pytest python/benchmarks/bench_tier2_sketches.py -v --benchmark-only
"""

from typing import Any

from sketch_oxide import (
    GRF,
    LearnedBloomFilter,
    NitroSketch,
    UnivMon,
    VacuumFilter,
)

# ============================================================================
# VACUUM FILTER BENCHMARKS
# ============================================================================


class TestVacuumFilterBenchmarks:
    """Benchmark VacuumFilter operations."""

    def test_vacuum_insert_throughput(self, benchmark: Any) -> None:
        """Benchmark VacuumFilter insert throughput."""
        vf = VacuumFilter(capacity=10000, fpr=0.01)
        keys = [f"key_{i}".encode() for i in range(1000)]

        def insert_batch() -> None:
            for key in keys:
                vf.insert(key)

        benchmark(insert_batch)

    def test_vacuum_query_throughput(self, benchmark: Any) -> None:
        """Benchmark VacuumFilter query throughput."""
        vf = VacuumFilter(capacity=10000, fpr=0.01)
        # Pre-populate
        for i in range(1000):
            vf.insert(f"key_{i}".encode())

        keys = [f"key_{i}".encode() for i in range(1000)]

        def query_batch() -> None:
            for key in keys:
                vf.contains(key)

        benchmark(query_batch)

    def test_vacuum_delete_throughput(self, benchmark: Any) -> None:
        """Benchmark VacuumFilter delete throughput."""
        vf = VacuumFilter(capacity=10000, fpr=0.01)
        # Pre-populate
        for i in range(1000):
            vf.insert(f"key_{i}".encode())

        keys = [f"key_{i}".encode() for i in range(1000)]

        def delete_batch() -> None:
            for key in keys:
                vf.delete(key)

        benchmark(delete_batch)

    def test_vacuum_mixed_operations(self, benchmark: Any) -> None:
        """Benchmark VacuumFilter mixed operations."""
        vf = VacuumFilter(capacity=10000, fpr=0.01)

        def mixed_ops() -> None:
            for i in range(100):
                vf.insert(f"key_{i}".encode())
            for i in range(50, 150):
                vf.contains(f"key_{i}".encode())
            for i in range(25, 75):
                vf.delete(f"key_{i}".encode())

        benchmark(mixed_ops)


# ============================================================================
# GRF BENCHMARKS
# ============================================================================


class TestGRFBenchmarks:
    """Benchmark GRF operations."""

    def test_grf_build_sorted(self, benchmark: Any) -> None:
        """Benchmark GRF build with sorted keys."""
        keys = [i.to_bytes(8, "big") for i in range(10000)]

        def build() -> None:
            GRF.build(keys, bits_per_key=12)

        benchmark(build)

    def test_grf_point_query(self, benchmark: Any) -> None:
        """Benchmark GRF point queries."""
        keys = [i.to_bytes(8, "big") for i in range(10000)]
        grf = GRF.build(keys, bits_per_key=12)
        query_keys = [i.to_bytes(8, "big") for i in range(5000)]

        def point_query() -> None:
            for key in query_keys:
                grf.may_contain(key)

        benchmark(point_query)

    def test_grf_range_query(self, benchmark: Any) -> None:
        """Benchmark GRF range queries."""
        keys = [i.to_bytes(8, "big") for i in range(10000)]
        grf = GRF.build(keys, bits_per_key=12)

        def range_query() -> None:
            for i in range(0, 10000, 100):
                low = i.to_bytes(8, "big")
                high = (i + 100).to_bytes(8, "big")
                grf.may_contain_range(low, high)

        benchmark(range_query)

    def test_grf_expected_fpr(self, benchmark: Any) -> None:
        """Benchmark GRF FPR estimation."""
        keys = [i.to_bytes(8, "big") for i in range(10000)]
        grf = GRF.build(keys, bits_per_key=12)

        def fpr_calc() -> None:
            for width in [10, 50, 100, 500]:
                grf.expected_fpr(width)

        benchmark(fpr_calc)


# ============================================================================
# NITROSKETCH BENCHMARKS
# ============================================================================


class TestNitroSketchBenchmarks:
    """Benchmark NitroSketch operations."""

    def test_nitrosketch_update_low_sample(self, benchmark: Any) -> None:
        """Benchmark NitroSketch update with low sampling rate."""
        nitro = NitroSketch(epsilon=0.01, delta=0.01, sample_rate=0.01)

        def update_low() -> None:
            for i in range(1000):
                nitro.update(f"flow_{i}".encode())

        benchmark(update_low)

    def test_nitrosketch_update_medium_sample(self, benchmark: Any) -> None:
        """Benchmark NitroSketch update with medium sampling rate."""
        nitro = NitroSketch(epsilon=0.01, delta=0.01, sample_rate=0.1)

        def update_medium() -> None:
            for i in range(1000):
                nitro.update(f"flow_{i}".encode())

        benchmark(update_medium)

    def test_nitrosketch_update_high_sample(self, benchmark: Any) -> None:
        """Benchmark NitroSketch update with high sampling rate."""
        nitro = NitroSketch(epsilon=0.01, delta=0.01, sample_rate=1.0)

        def update_high() -> None:
            for i in range(1000):
                nitro.update(f"flow_{i}".encode())

        benchmark(update_high)

    def test_nitrosketch_sync(self, benchmark: Any) -> None:
        """Benchmark NitroSketch synchronization."""
        nitro = NitroSketch(epsilon=0.01, delta=0.01, sample_rate=0.1)
        # Pre-populate
        for i in range(10000):
            nitro.update(f"flow_{i}".encode())

        def sync() -> None:
            nitro.sync(1.0)

        benchmark(sync)

    def test_nitrosketch_query_throughput(self, benchmark: Any) -> None:
        """Benchmark NitroSketch query throughput."""
        nitro = NitroSketch(epsilon=0.01, delta=0.01, sample_rate=0.1)
        # Pre-populate
        for i in range(10000):
            nitro.update(f"flow_{i}".encode())
        nitro.sync(1.0)

        keys = [f"flow_{i}".encode() for i in range(1000)]

        def query_batch() -> None:
            for key in keys:
                nitro.query(key)

        benchmark(query_batch)


# ============================================================================
# UNIVMON BENCHMARKS
# ============================================================================


class TestUnivMonBenchmarks:
    """Benchmark UnivMon operations."""

    def test_univmon_update_throughput(self, benchmark: Any) -> None:
        """Benchmark UnivMon update throughput."""
        um = UnivMon(max_stream_size=100000, epsilon=0.01, delta=0.01)
        keys = [f"192.168.{i // 256}.{i % 256}".encode() for i in range(10000)]

        def update_batch() -> None:
            for i, key in enumerate(keys):
                um.update(key, 1500.0 if i % 2 == 0 else 800.0)

        benchmark(update_batch)

    def test_univmon_estimate_l1(self, benchmark: Any) -> None:
        """Benchmark UnivMon L1 estimation."""
        um = UnivMon(max_stream_size=100000, epsilon=0.01, delta=0.01)
        # Pre-populate
        for i in range(1000):
            um.update(f"key_{i}".encode(), float(i))

        def estimate() -> float:
            return um.estimate_l1()

        benchmark(estimate)

    def test_univmon_estimate_l2(self, benchmark: Any) -> None:
        """Benchmark UnivMon L2 estimation."""
        um = UnivMon(max_stream_size=100000, epsilon=0.01, delta=0.01)
        # Pre-populate
        for i in range(1000):
            um.update(f"key_{i}".encode(), float(i))

        def estimate() -> float:
            return um.estimate_l2()

        benchmark(estimate)

    def test_univmon_estimate_entropy(self, benchmark: Any) -> None:
        """Benchmark UnivMon entropy estimation."""
        um = UnivMon(max_stream_size=100000, epsilon=0.01, delta=0.01)
        # Pre-populate
        for i in range(1000):
            um.update(f"key_{i}".encode(), float(i))

        def estimate() -> float:
            return um.estimate_entropy()

        benchmark(estimate)

    def test_univmon_heavy_hitters(self, benchmark: Any) -> None:
        """Benchmark UnivMon heavy hitters detection."""
        um = UnivMon(max_stream_size=100000, epsilon=0.01, delta=0.01)
        # Pre-populate
        for i in range(1000):
            um.update(f"key_{i}".encode(), float(i) if i < 10 else 1.0)

        def hh() -> list:  # type: ignore
            return um.heavy_hitters(threshold=10.0)

        benchmark(hh)

    def test_univmon_all_metrics(self, benchmark: Any) -> None:
        """Benchmark UnivMon all metrics computation."""
        um = UnivMon(max_stream_size=100000, epsilon=0.01, delta=0.01)
        # Pre-populate
        for i in range(1000):
            um.update(f"key_{i}".encode(), float(i))

        def all_metrics() -> None:
            um.estimate_l1()
            um.estimate_l2()
            um.estimate_entropy()
            um.heavy_hitters(threshold=10.0)

        benchmark(all_metrics)


# ============================================================================
# LEARNED BLOOM FILTER BENCHMARKS
# ============================================================================


class TestLearnedBloomBenchmarks:
    """Benchmark LearnedBloomFilter operations."""

    def test_learned_bloom_build_small(self, benchmark: Any) -> None:
        """Benchmark LearnedBloomFilter build with small dataset."""
        keys = [f"key_{i}".encode() for i in range(1000)]

        def build() -> None:
            LearnedBloomFilter.new(keys, fpr=0.01)

        benchmark(build)

    def test_learned_bloom_build_large(self, benchmark: Any) -> None:
        """Benchmark LearnedBloomFilter build with large dataset."""
        keys = [f"key_{i}".encode() for i in range(10000)]

        def build() -> None:
            LearnedBloomFilter.new(keys, fpr=0.01)

        benchmark(build)

    def test_learned_bloom_positive_queries(self, benchmark: Any) -> None:
        """Benchmark LearnedBloomFilter positive queries."""
        keys = [f"key_{i}".encode() for i in range(1000)]
        lb = LearnedBloomFilter.new(keys, fpr=0.01)
        query_keys = [f"key_{i}".encode() for i in range(100)]

        def pos_queries() -> None:
            for key in query_keys:
                lb.contains(key)

        benchmark(pos_queries)

    def test_learned_bloom_negative_queries(self, benchmark: Any) -> None:
        """Benchmark LearnedBloomFilter negative queries."""
        keys = [f"key_{i}".encode() for i in range(1000)]
        lb = LearnedBloomFilter.new(keys, fpr=0.01)
        query_keys = [f"notkey_{i}".encode() for i in range(100)]

        def neg_queries() -> None:
            for key in query_keys:
                lb.contains(key)

        benchmark(neg_queries)


# ============================================================================
# COMPARATIVE BENCHMARKS
# ============================================================================


class TestComparativeBenchmarks:
    """Compare performance across multiple sketches."""

    def test_membership_insert_comparison(self, benchmark: Any) -> None:
        """Compare insert throughput across membership structures."""
        vf = VacuumFilter(capacity=10000, fpr=0.01)
        lb = LearnedBloomFilter.new([b"init"], fpr=0.01)

        def compare() -> None:
            for i in range(100):
                vf.insert(f"key_{i}".encode())
                lb.contains(f"key_{i}".encode())

        benchmark(compare)
