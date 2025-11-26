#!/usr/bin/env python3
"""
Phase 3 FFI Optimization Benchmarks - Python Batch APIs

Measures performance improvements from batch API implementations across
multiple sketch categories: membership, frequency, and quantiles.
"""

import time

import numpy as np

import sketch_oxide


def benchmark_bloom_filter():
    """Benchmark BloomFilter batch operations"""
    print("\n=== BloomFilter Batch API Benchmarks ===")

    # Generate test data
    keys = [f"key_{i}".encode() for i in range(1000)]

    # Individual inserts
    bf_individual = sketch_oxide.BloomFilter(1000)
    start = time.perf_counter()
    for key in keys:
        bf_individual.insert(key)
    individual_time = time.perf_counter() - start

    # Batch inserts
    bf_batch = sketch_oxide.BloomFilter(1000)
    start = time.perf_counter()
    bf_batch.insert_batch(keys)
    batch_time = time.perf_counter() - start

    speedup = individual_time / batch_time
    print("Insert 1000 items:")
    print(f"  Individual: {individual_time*1000:.2f}ms")
    print(f"  Batch:      {batch_time*1000:.2f}ms")
    print(f"  Speedup:    {speedup:.2f}x")

    # Batch contains
    start = time.perf_counter()
    for key in keys:
        bf_individual.contains(key)
    individual_time = time.perf_counter() - start

    start = time.perf_counter()
    bf_batch.contains_batch(keys)
    batch_time = time.perf_counter() - start

    speedup = individual_time / batch_time
    print("\nContains check 1000 items:")
    print(f"  Individual: {individual_time*1000:.2f}ms")
    print(f"  Batch:      {batch_time*1000:.2f}ms")
    print(f"  Speedup:    {speedup:.2f}x")


def benchmark_count_min_sketch():
    """Benchmark CountMinSketch batch operations"""
    print("\n=== CountMinSketch Batch API Benchmarks ===")

    # Generate test data
    items = [f"item_{i}".encode() for i in range(1000)]

    # Individual updates
    cms_individual = sketch_oxide.CountMinSketch(epsilon=0.01, delta=0.01)
    start = time.perf_counter()
    for item in items:
        cms_individual.update(item)
    individual_time = time.perf_counter() - start

    # Batch updates
    cms_batch = sketch_oxide.CountMinSketch(epsilon=0.01, delta=0.01)
    start = time.perf_counter()
    cms_batch.update_batch(items)
    batch_time = time.perf_counter() - start

    speedup = individual_time / batch_time
    print("Update 1000 items:")
    print(f"  Individual: {individual_time*1000:.2f}ms")
    print(f"  Batch:      {batch_time*1000:.2f}ms")
    print(f"  Speedup:    {speedup:.2f}x")

    # Batch estimate
    start = time.perf_counter()
    for item in items:
        cms_individual.estimate(item)
    individual_time = time.perf_counter() - start

    start = time.perf_counter()
    cms_batch.estimate_batch(items)
    batch_time = time.perf_counter() - start

    speedup = individual_time / batch_time
    print("\nEstimate 1000 items:")
    print(f"  Individual: {individual_time*1000:.2f}ms")
    print(f"  Batch:      {batch_time*1000:.2f}ms")
    print(f"  Speedup:    {speedup:.2f}x")


def benchmark_cuckoo_filter():
    """Benchmark CuckooFilter batch operations"""
    print("\n=== CuckooFilter Batch API Benchmarks ===")

    # Generate test data
    keys = [f"key_{i}".encode() for i in range(500)]  # Smaller for CuckooFilter

    try:
        # Individual inserts
        cf_individual = sketch_oxide.CuckooFilter(500)
        start = time.perf_counter()
        for key in keys:
            cf_individual.insert(key)
        individual_time = time.perf_counter() - start

        # Batch inserts
        cf_batch = sketch_oxide.CuckooFilter(500)
        start = time.perf_counter()
        cf_batch.insert_batch(keys)
        batch_time = time.perf_counter() - start

        speedup = individual_time / batch_time
        print("Insert 500 items:")
        print(f"  Individual: {individual_time*1000:.2f}ms")
        print(f"  Batch:      {batch_time*1000:.2f}ms")
        print(f"  Speedup:    {speedup:.2f}x")

        # Batch contains
        start = time.perf_counter()
        for key in keys:
            cf_individual.contains(key)
        individual_time = time.perf_counter() - start

        start = time.perf_counter()
        cf_batch.contains_batch(keys)
        batch_time = time.perf_counter() - start

        speedup = individual_time / batch_time
        print("\nContains check 500 items:")
        print(f"  Individual: {individual_time*1000:.2f}ms")
        print(f"  Batch:      {batch_time*1000:.2f}ms")
        print(f"  Speedup:    {speedup:.2f}x")
    except Exception as e:
        print(f"CuckooFilter benchmark skipped: {e}")


def benchmark_kll_sketch():
    """Benchmark KllSketch batch operations"""
    print("\n=== KllSketch Batch API Benchmarks ===")

    # Generate test data
    values = np.random.randn(1000).tolist()

    # Individual updates
    kll_individual = sketch_oxide.KllSketch(200)
    start = time.perf_counter()
    for value in values:
        kll_individual.update(value)
    individual_time = time.perf_counter() - start

    # Batch updates
    kll_batch = sketch_oxide.KllSketch(200)
    start = time.perf_counter()
    kll_batch.update_batch(values)
    batch_time = time.perf_counter() - start

    speedup = individual_time / batch_time
    print("Update 1000 values:")
    print(f"  Individual: {individual_time*1000:.2f}ms")
    print(f"  Batch:      {batch_time*1000:.2f}ms")
    print(f"  Speedup:    {speedup:.2f}x")

    # Batch quantile queries
    quantiles = [0.25, 0.5, 0.75, 0.95, 0.99]
    start = time.perf_counter()
    for q in quantiles:
        kll_individual.quantile(q)
    individual_time = time.perf_counter() - start

    start = time.perf_counter()
    kll_batch.quantile_batch(quantiles)
    batch_time = time.perf_counter() - start

    speedup = individual_time / batch_time
    print("\nQuantile queries (5 queries):")
    print(f"  Individual: {individual_time*1000:.2f}ms")
    print(f"  Batch:      {batch_time*1000:.2f}ms")
    print(f"  Speedup:    {speedup:.2f}x")


def benchmark_hyperloglog():
    """Benchmark HyperLogLog batch operations"""
    print("\n=== HyperLogLog Batch API Benchmarks ===")

    # Generate test data
    items = [f"item_{i}".encode() for i in range(1000)]

    # Individual updates
    hll_individual = sketch_oxide.HyperLogLog(14)
    start = time.perf_counter()
    for item in items:
        hll_individual.update(item)
    individual_time = time.perf_counter() - start

    # Batch updates
    hll_batch = sketch_oxide.HyperLogLog(14)
    start = time.perf_counter()
    hll_batch.update_batch(items)
    batch_time = time.perf_counter() - start

    speedup = individual_time / batch_time
    print("Update 1000 items:")
    print(f"  Individual: {individual_time*1000:.2f}ms")
    print(f"  Batch:      {batch_time*1000:.2f}ms")
    print(f"  Speedup:    {speedup:.2f}x")


def benchmark_conservative_count_min():
    """Benchmark ConservativeCountMin batch operations"""
    print("\n=== ConservativeCountMin Batch API Benchmarks ===")

    # Generate test data
    items = [f"item_{i}".encode() for i in range(1000)]

    # Individual updates
    ccm_individual = sketch_oxide.ConservativeCountMin(epsilon=0.01, delta=0.01)
    start = time.perf_counter()
    for item in items:
        ccm_individual.update(item)
    individual_time = time.perf_counter() - start

    # Batch updates
    ccm_batch = sketch_oxide.ConservativeCountMin(epsilon=0.01, delta=0.01)
    start = time.perf_counter()
    ccm_batch.update_batch(items)
    batch_time = time.perf_counter() - start

    speedup = individual_time / batch_time
    print("Update 1000 items:")
    print(f"  Individual: {individual_time*1000:.2f}ms")
    print(f"  Batch:      {batch_time*1000:.2f}ms")
    print(f"  Speedup:    {speedup:.2f}x")

    # Batch estimate
    start = time.perf_counter()
    for item in items:
        ccm_individual.estimate(item)
    individual_time = time.perf_counter() - start

    start = time.perf_counter()
    ccm_batch.estimate_batch(items)
    batch_time = time.perf_counter() - start

    speedup = individual_time / batch_time
    print("\nEstimate 1000 items:")
    print(f"  Individual: {individual_time*1000:.2f}ms")
    print(f"  Batch:      {batch_time*1000:.2f}ms")
    print(f"  Speedup:    {speedup:.2f}x")


if __name__ == "__main__":
    print("=" * 60)
    print("Phase 3 FFI Optimization Benchmarks - Python")
    print("=" * 60)

    benchmark_bloom_filter()
    benchmark_count_min_sketch()
    benchmark_hyperloglog()
    benchmark_conservative_count_min()
    benchmark_cuckoo_filter()
    benchmark_kll_sketch()

    print("\n" + "=" * 60)
    print("Benchmarks Complete")
    print("=" * 60)
