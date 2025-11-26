//! Comprehensive Benchmarks for Grafite Range Filter
//!
//! This benchmark suite measures performance across various scenarios:
//! - Build performance for different key set sizes
//! - Range query performance for different range widths
//! - Point query performance
//! - FPR measurement (empirical validation)
//! - Throughput testing
//! - Comparison with different bits_per_key values

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sketch_oxide::common::RangeFilter;
use sketch_oxide::range_filters::Grafite;

// ============================================================================
// Build Performance Benchmarks
// ============================================================================

fn benchmark_build_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("grafite_build_small");

    for size in [100, 500, 1000, 5000, 10_000] {
        let keys: Vec<u64> = (0..size).map(|i| i * 10).collect();

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("keys", size), &keys, |b, keys| {
            b.iter(|| Grafite::build(black_box(keys), black_box(6)));
        });
    }

    group.finish();
}

fn benchmark_build_large(c: &mut Criterion) {
    let mut group = c.benchmark_group("grafite_build_large");
    group.sample_size(20); // Fewer samples for large sizes

    for size in [50_000, 100_000, 500_000, 1_000_000] {
        let keys: Vec<u64> = (0..size).map(|i| i * 10).collect();

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("keys", size), &keys, |b, keys| {
            b.iter(|| Grafite::build(black_box(keys), black_box(6)));
        });
    }

    group.finish();
}

fn benchmark_build_varying_bits(c: &mut Criterion) {
    let mut group = c.benchmark_group("grafite_build_bits_per_key");

    let keys: Vec<u64> = (0..10_000).map(|i| i * 10).collect();

    for bits in [4, 5, 6, 7, 8] {
        group.throughput(Throughput::Elements(10_000));
        group.bench_with_input(BenchmarkId::new("bits", bits), &bits, |b, &bits| {
            b.iter(|| Grafite::build(black_box(&keys), black_box(bits)));
        });
    }

    group.finish();
}

// ============================================================================
// Range Query Performance Benchmarks
// ============================================================================

fn benchmark_range_query_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("grafite_range_query_small");

    let keys: Vec<u64> = (0..10_000).map(|i| i * 100).collect();
    let filter = Grafite::build(&keys, 6).unwrap();

    for width in [10, 50, 100, 500, 1000] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("range_width", width),
            &width,
            |b, &width| {
                b.iter(|| filter.may_contain_range(black_box(50_000), black_box(50_000 + width)));
            },
        );
    }

    group.finish();
}

fn benchmark_range_query_large(c: &mut Criterion) {
    let mut group = c.benchmark_group("grafite_range_query_large");

    let keys: Vec<u64> = (0..10_000).map(|i| i * 100).collect();
    let filter = Grafite::build(&keys, 6).unwrap();

    for width in [5_000, 10_000, 50_000, 100_000] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("range_width", width),
            &width,
            |b, &width| {
                b.iter(|| filter.may_contain_range(black_box(50_000), black_box(50_000 + width)));
            },
        );
    }

    group.finish();
}

fn benchmark_range_query_hit(c: &mut Criterion) {
    let mut group = c.benchmark_group("grafite_range_query_hit");

    let keys: Vec<u64> = (0..10_000).map(|i| i * 100).collect();
    let filter = Grafite::build(&keys, 6).unwrap();

    group.throughput(Throughput::Elements(1));
    group.bench_function("contains_keys", |b| {
        b.iter(|| {
            // Query range that contains keys
            filter.may_contain_range(black_box(5_000), black_box(5_500))
        });
    });

    group.finish();
}

fn benchmark_range_query_miss(c: &mut Criterion) {
    let mut group = c.benchmark_group("grafite_range_query_miss");

    let keys: Vec<u64> = (0..10_000).map(|i| i * 100).collect();
    let filter = Grafite::build(&keys, 6).unwrap();

    group.throughput(Throughput::Elements(1));
    group.bench_function("no_keys", |b| {
        b.iter(|| {
            // Query range with no keys
            filter.may_contain_range(black_box(2_000_000), black_box(2_000_500))
        });
    });

    group.finish();
}

// ============================================================================
// Point Query Performance Benchmarks
// ============================================================================

fn benchmark_point_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("grafite_point_query");

    let keys: Vec<u64> = (0..10_000).map(|i| i * 100).collect();
    let filter = Grafite::build(&keys, 6).unwrap();

    group.throughput(Throughput::Elements(1));

    group.bench_function("existing_key", |b| {
        b.iter(|| filter.may_contain(black_box(50_000)));
    });

    group.bench_function("missing_key", |b| {
        b.iter(|| filter.may_contain(black_box(50_001)));
    });

    group.finish();
}

// ============================================================================
// Throughput Benchmarks
// ============================================================================

fn benchmark_throughput_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("grafite_throughput");

    let keys: Vec<u64> = (0..10_000).map(|i| i * 100).collect();
    let filter = Grafite::build(&keys, 6).unwrap();

    group.throughput(Throughput::Elements(1000));
    group.bench_function("1000_range_queries", |b| {
        b.iter(|| {
            for i in 0..1000 {
                let low = i * 1000;
                let high = low + 100;
                black_box(filter.may_contain_range(low, high));
            }
        });
    });

    group.throughput(Throughput::Elements(1000));
    group.bench_function("1000_point_queries", |b| {
        b.iter(|| {
            for i in 0..1000 {
                black_box(filter.may_contain(i * 100));
            }
        });
    });

    group.finish();
}

// ============================================================================
// FPR Measurement Benchmarks (Empirical)
// ============================================================================

fn benchmark_fpr_measurement(c: &mut Criterion) {
    let mut group = c.benchmark_group("grafite_fpr_measurement");
    group.sample_size(20);

    let keys: Vec<u64> = (0..1000).map(|i| i * 1000).collect();

    for bits in [4, 6, 8] {
        let filter = Grafite::build(&keys, bits).unwrap();

        group.bench_with_input(
            BenchmarkId::new("measure_fpr_bits", bits),
            &filter,
            |b, filter| {
                b.iter(|| {
                    let mut false_positives = 0;
                    let total_tests = 1000;

                    // Test ranges that don't contain keys
                    for i in 0..total_tests {
                        let low = 2_000_000 + i * 100;
                        let high = low + 10;
                        if filter.may_contain_range(low, high) {
                            false_positives += 1;
                        }
                    }

                    black_box(false_positives)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Varying Bits Per Key Performance
// ============================================================================

fn benchmark_query_varying_bits(c: &mut Criterion) {
    let mut group = c.benchmark_group("grafite_query_bits_per_key");

    let keys: Vec<u64> = (0..10_000).map(|i| i * 100).collect();

    for bits in [4, 5, 6, 7, 8] {
        let filter = Grafite::build(&keys, bits).unwrap();

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::new("bits", bits), &filter, |b, filter| {
            b.iter(|| filter.may_contain_range(black_box(50_000), black_box(50_100)));
        });
    }

    group.finish();
}

// ============================================================================
// Database-Like Workload Benchmarks
// ============================================================================

fn benchmark_db_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("grafite_db_workload");

    // Simulate LSM-tree keys (sorted, sparse)
    let keys: Vec<u64> = (0..100_000).map(|i| i * 100).collect();
    let filter = Grafite::build(&keys, 6).unwrap();

    group.throughput(Throughput::Elements(1));

    // Small range scan (typical DB query)
    group.bench_function("small_range_scan", |b| {
        b.iter(|| filter.may_contain_range(black_box(500_000), black_box(500_100)));
    });

    // Medium range scan
    group.bench_function("medium_range_scan", |b| {
        b.iter(|| filter.may_contain_range(black_box(500_000), black_box(510_000)));
    });

    // Large range scan
    group.bench_function("large_range_scan", |b| {
        b.iter(|| filter.may_contain_range(black_box(500_000), black_box(600_000)));
    });

    // Point lookup (primary key)
    group.bench_function("point_lookup", |b| {
        b.iter(|| filter.may_contain(black_box(500_000)));
    });

    group.finish();
}

// ============================================================================
// Memory Efficiency Benchmarks
// ============================================================================

fn benchmark_memory_stats(c: &mut Criterion) {
    let mut group = c.benchmark_group("grafite_memory_stats");

    let keys: Vec<u64> = (0..10_000).collect();
    let filter = Grafite::build(&keys, 6).unwrap();

    group.bench_function("stats_access", |b| {
        b.iter(|| black_box(filter.stats()));
    });

    group.bench_function("expected_fpr", |b| {
        b.iter(|| black_box(filter.expected_fpr(100)));
    });

    group.finish();
}

// ============================================================================
// Filter Construction Patterns
// ============================================================================

fn benchmark_construction_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("grafite_construction_patterns");

    // Sequential keys
    let sequential: Vec<u64> = (0..10_000).collect();
    group.bench_function("sequential_keys", |b| {
        b.iter(|| Grafite::build(black_box(&sequential), black_box(6)));
    });

    // Sparse keys
    let sparse: Vec<u64> = (0..10_000).map(|i| i * 1000).collect();
    group.bench_function("sparse_keys", |b| {
        b.iter(|| Grafite::build(black_box(&sparse), black_box(6)));
    });

    // Random keys (pre-sorted)
    let mut random: Vec<u64> = (0..10_000).map(|i| i * 37 % 1_000_000).collect();
    random.sort_unstable();
    random.dedup();
    group.bench_function("random_keys", |b| {
        b.iter(|| Grafite::build(black_box(&random), black_box(6)));
    });

    group.finish();
}

// ============================================================================
// Trait Method Performance
// ============================================================================

fn benchmark_trait_methods(c: &mut Criterion) {
    let mut group = c.benchmark_group("grafite_trait_methods");

    let keys: Vec<u64> = (0..10_000).map(|i| i * 100).collect();
    let filter = Grafite::build(&keys, 6).unwrap();

    group.throughput(Throughput::Elements(1));

    // Direct method call
    group.bench_function("direct_method", |b| {
        b.iter(|| filter.may_contain_range(black_box(50_000), black_box(50_100)));
    });

    // Through RangeFilter trait
    group.bench_function("trait_method", |b| {
        b.iter(|| RangeFilter::may_contain_range(&filter, black_box(50_000), black_box(50_100)));
    });

    group.finish();
}

criterion_group!(
    build_benches,
    benchmark_build_small,
    benchmark_build_large,
    benchmark_build_varying_bits,
);

criterion_group!(
    query_benches,
    benchmark_range_query_small,
    benchmark_range_query_large,
    benchmark_range_query_hit,
    benchmark_range_query_miss,
    benchmark_point_query,
);

criterion_group!(throughput_benches, benchmark_throughput_queries,);

criterion_group!(
    fpr_benches,
    benchmark_fpr_measurement,
    benchmark_query_varying_bits,
);

criterion_group!(
    workload_benches,
    benchmark_db_workload,
    benchmark_construction_patterns,
);

criterion_group!(
    misc_benches,
    benchmark_memory_stats,
    benchmark_trait_methods,
);

criterion_main!(
    build_benches,
    query_benches,
    throughput_benches,
    fpr_benches,
    workload_benches,
    misc_benches,
);
