//! Comprehensive Benchmarks for GRF (Gorilla Range Filter)
//!
//! This benchmark suite measures performance across various scenarios:
//! - Build performance for different key set sizes
//! - Range query performance for different range widths
//! - Point query performance
//! - FPR measurement (empirical validation)
//! - Comparison with Grafite
//! - Varying bits_per_key configurations
//! - Different key distributions
//! - LSM-tree simulation
//! - Scalability (1K-1M keys)
//! - Memory efficiency

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sketch_oxide::common::RangeFilter;
use sketch_oxide::range_filters::{Grafite, GRF};

// ============================================================================
// 1. Build Performance Benchmarks
// ============================================================================

fn benchmark_build_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("grf_build_small");

    for size in [100, 500, 1000, 5000, 10_000] {
        let keys: Vec<u64> = (0..size).map(|i| i * 10).collect();

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("keys", size), &keys, |b, keys| {
            b.iter(|| GRF::build(black_box(keys), black_box(6)));
        });
    }

    group.finish();
}

fn benchmark_build_large(c: &mut Criterion) {
    let mut group = c.benchmark_group("grf_build_large");
    group.sample_size(20); // Fewer samples for large sizes

    for size in [50_000, 100_000, 500_000, 1_000_000] {
        let keys: Vec<u64> = (0..size).map(|i| i * 10).collect();

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("keys", size), &keys, |b, keys| {
            b.iter(|| GRF::build(black_box(keys), black_box(6)));
        });
    }

    group.finish();
}

fn benchmark_build_varying_bits(c: &mut Criterion) {
    let mut group = c.benchmark_group("grf_build_bits_per_key");

    let keys: Vec<u64> = (0..10_000).map(|i| i * 10).collect();

    for bits in [4, 5, 6, 7, 8] {
        group.throughput(Throughput::Elements(10_000));
        group.bench_with_input(BenchmarkId::new("bits", bits), &bits, |b, &bits| {
            b.iter(|| GRF::build(black_box(&keys), black_box(bits)));
        });
    }

    group.finish();
}

fn benchmark_build_skewed_distribution(c: &mut Criterion) {
    let mut group = c.benchmark_group("grf_build_skewed");

    // Zipf distribution (realistic LSM workload)
    let mut keys = Vec::new();
    keys.extend(vec![1; 5000]); // Heavy key
    keys.extend(vec![2; 2500]); // Medium key
    keys.extend(vec![3; 1250]); // Light key
    keys.extend((4..1000).collect::<Vec<u64>>()); // Tail

    group.throughput(Throughput::Elements(keys.len() as u64));
    group.bench_function("zipf_distribution", |b| {
        b.iter(|| GRF::build(black_box(&keys), black_box(6)));
    });

    group.finish();
}

// ============================================================================
// 2. Range Query Performance Benchmarks
// ============================================================================

fn benchmark_range_query_small_ranges(c: &mut Criterion) {
    let mut group = c.benchmark_group("grf_range_query_small");

    let keys: Vec<u64> = (0..10_000).map(|i| i * 100).collect();
    let filter = GRF::build(&keys, 6).unwrap();

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

fn benchmark_range_query_large_ranges(c: &mut Criterion) {
    let mut group = c.benchmark_group("grf_range_query_large");

    let keys: Vec<u64> = (0..10_000).map(|i| i * 100).collect();
    let filter = GRF::build(&keys, 6).unwrap();

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

fn benchmark_range_query_varying_dataset_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("grf_range_query_dataset_size");

    for size in [1000, 10_000, 100_000, 1_000_000] {
        let keys: Vec<u64> = (0..size).map(|i| i * 100).collect();
        let filter = GRF::build(&keys, 6).unwrap();

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::new("keys", size), &size, |b, _| {
            b.iter(|| filter.may_contain_range(black_box(50_000), black_box(51_000)));
        });
    }

    group.finish();
}

// ============================================================================
// 3. Point Query Performance Benchmarks
// ============================================================================

fn benchmark_point_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("grf_point_query");

    let keys: Vec<u64> = (0..100_000).map(|i| i * 10).collect();
    let filter = GRF::build(&keys, 6).unwrap();

    group.throughput(Throughput::Elements(1));
    group.bench_function("may_contain", |b| {
        b.iter(|| filter.may_contain(black_box(50_000)));
    });

    group.finish();
}

fn benchmark_point_query_hit_vs_miss(c: &mut Criterion) {
    let mut group = c.benchmark_group("grf_point_query_hit_miss");

    let keys: Vec<u64> = (0..100_000).map(|i| i * 100).collect();
    let filter = GRF::build(&keys, 6).unwrap();

    group.throughput(Throughput::Elements(1));

    // Hit (key exists)
    group.bench_function("hit", |b| {
        b.iter(|| filter.may_contain(black_box(5000 * 100)));
    });

    // Miss (key doesn't exist)
    group.bench_function("miss", |b| {
        b.iter(|| filter.may_contain(black_box(5000 * 100 + 50)));
    });

    group.finish();
}

// ============================================================================
// 4. FPR Measurement Benchmarks
// ============================================================================

fn benchmark_fpr_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("grf_fpr_calculation");

    let keys: Vec<u64> = (0..10_000).map(|i| i * 10).collect();
    let filter = GRF::build(&keys, 6).unwrap();

    group.throughput(Throughput::Elements(1));
    group.bench_function("expected_fpr", |b| {
        b.iter(|| filter.expected_fpr(black_box(1000)));
    });

    group.finish();
}

fn benchmark_fpr_empirical_measurement(c: &mut Criterion) {
    let mut group = c.benchmark_group("grf_fpr_empirical");
    group.sample_size(10);

    let keys: Vec<u64> = (0..10_000).map(|i| i * 100).collect();
    let filter = GRF::build(&keys, 6).unwrap();

    group.throughput(Throughput::Elements(1000));
    group.bench_function("measure_1000_queries", |b| {
        b.iter(|| {
            let mut fp_count = 0;
            for i in 0..1000 {
                let start = 50 + i * 100; // Between keys
                if filter.may_contain_range(start, start + 10) {
                    fp_count += 1;
                }
            }
            fp_count
        });
    });

    group.finish();
}

// ============================================================================
// 5. Comparison with Grafite Benchmarks
// ============================================================================

fn benchmark_comparison_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("comparison_build");

    let keys: Vec<u64> = (0..100_000).map(|i| i * 10).collect();

    group.throughput(Throughput::Elements(100_000));

    group.bench_function("grf_build", |b| {
        b.iter(|| GRF::build(black_box(&keys), black_box(6)));
    });

    group.bench_function("grafite_build", |b| {
        b.iter(|| Grafite::build(black_box(&keys), black_box(6)));
    });

    group.finish();
}

fn benchmark_comparison_range_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("comparison_range_query");

    let keys: Vec<u64> = (0..100_000).map(|i| i * 10).collect();
    let grf = GRF::build(&keys, 6).unwrap();
    let grafite = Grafite::build(&keys, 6).unwrap();

    group.throughput(Throughput::Elements(1));

    group.bench_function("grf_range_query", |b| {
        b.iter(|| grf.may_contain_range(black_box(50_000), black_box(51_000)));
    });

    group.bench_function("grafite_range_query", |b| {
        b.iter(|| grafite.may_contain_range(black_box(50_000), black_box(51_000)));
    });

    group.finish();
}

fn benchmark_comparison_skewed_data(c: &mut Criterion) {
    let mut group = c.benchmark_group("comparison_skewed_data");

    // Zipf distribution
    let mut keys = Vec::new();
    keys.extend(vec![1; 5000]);
    keys.extend(vec![2; 2500]);
    keys.extend(vec![3; 1250]);
    keys.extend((4..1000).collect::<Vec<u64>>());

    let grf = GRF::build(&keys, 6).unwrap();
    let grafite = Grafite::build(&keys, 6).unwrap();

    group.throughput(Throughput::Elements(1));

    group.bench_function("grf_skewed_query", |b| {
        b.iter(|| grf.may_contain_range(black_box(1), black_box(100)));
    });

    group.bench_function("grafite_skewed_query", |b| {
        b.iter(|| grafite.may_contain_range(black_box(1), black_box(100)));
    });

    group.finish();
}

// ============================================================================
// 6. Varying Bits Per Key Benchmarks
// ============================================================================

fn benchmark_varying_bits_query_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("varying_bits_query");

    let keys: Vec<u64> = (0..10_000).map(|i| i * 10).collect();

    for bits in [4, 6, 8] {
        let filter = GRF::build(&keys, bits).unwrap();

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::new("bits", bits), &bits, |b, _| {
            b.iter(|| filter.may_contain_range(black_box(50_000), black_box(51_000)));
        });
    }

    group.finish();
}

// ============================================================================
// 7. Different Key Distribution Benchmarks
// ============================================================================

fn benchmark_distribution_uniform(c: &mut Criterion) {
    let mut group = c.benchmark_group("distribution_uniform");

    let keys: Vec<u64> = (0..10_000).collect();
    let filter = GRF::build(&keys, 6).unwrap();

    group.throughput(Throughput::Elements(1));
    group.bench_function("uniform_query", |b| {
        b.iter(|| filter.may_contain_range(black_box(5_000), black_box(5_100)));
    });

    group.finish();
}

fn benchmark_distribution_sparse(c: &mut Criterion) {
    let mut group = c.benchmark_group("distribution_sparse");

    let keys: Vec<u64> = (0..10_000).map(|i| i * 1000).collect();
    let filter = GRF::build(&keys, 6).unwrap();

    group.throughput(Throughput::Elements(1));
    group.bench_function("sparse_query", |b| {
        b.iter(|| filter.may_contain_range(black_box(5_000_000), black_box(5_100_000)));
    });

    group.finish();
}

fn benchmark_distribution_fibonacci(c: &mut Criterion) {
    let mut group = c.benchmark_group("distribution_fibonacci");

    // Generate Fibonacci-like sequence
    let mut keys = vec![1, 2];
    for i in 2..20 {
        let next = keys[i - 1] + keys[i - 2];
        keys.push(next);
    }

    let filter = GRF::build(&keys, 6).unwrap();

    group.throughput(Throughput::Elements(1));
    group.bench_function("fibonacci_query", |b| {
        b.iter(|| filter.may_contain_range(black_box(10), black_box(100)));
    });

    group.finish();
}

// ============================================================================
// 8. LSM-Tree Simulation Benchmarks
// ============================================================================

fn benchmark_lsm_multi_level_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("lsm_multi_level");

    // Create multiple levels
    let level0: Vec<u64> = (0..1000).collect();
    let level1: Vec<u64> = (10_000..11_000).collect();
    let level2: Vec<u64> = (20_000..21_000).collect();

    let filter0 = GRF::build(&level0, 6).unwrap();
    let filter1 = GRF::build(&level1, 6).unwrap();
    let filter2 = GRF::build(&level2, 6).unwrap();

    group.throughput(Throughput::Elements(3)); // Query 3 levels

    group.bench_function("multi_level_query", |b| {
        b.iter(|| {
            let _ = filter0.may_contain_range(black_box(500), black_box(600));
            let _ = filter1.may_contain_range(black_box(10_500), black_box(10_600));
            let _ = filter2.may_contain_range(black_box(20_500), black_box(20_600));
        });
    });

    group.finish();
}

fn benchmark_lsm_compaction(c: &mut Criterion) {
    let mut group = c.benchmark_group("lsm_compaction");
    group.sample_size(20);

    // Simulate compaction: merge two sorted runs
    let run1: Vec<u64> = (0..50_000).map(|i| i * 2).collect();
    let run2: Vec<u64> = (0..50_000).map(|i| i * 2 + 1).collect();

    group.throughput(Throughput::Elements(100_000));
    group.bench_function("merge_and_build", |b| {
        b.iter(|| {
            let mut merged = run1.clone();
            merged.extend(run2.clone());
            merged.sort_unstable();
            GRF::build(&merged, 6)
        });
    });

    group.finish();
}

// ============================================================================
// 9. Scalability Benchmarks (1K-1M keys)
// ============================================================================

fn benchmark_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("grf_scalability");
    group.sample_size(10);

    for size in [1_000, 10_000, 100_000, 1_000_000] {
        let keys: Vec<u64> = (0..size).map(|i| i * 10).collect();

        group.throughput(Throughput::Elements(size as u64));

        // Build time
        group.bench_with_input(BenchmarkId::new("build", size), &keys, |b, keys| {
            b.iter(|| GRF::build(black_box(keys), black_box(6)));
        });

        // Query time
        let filter = GRF::build(&keys, 6).unwrap();
        group.bench_with_input(BenchmarkId::new("query", size), &size, |b, _| {
            b.iter(|| filter.may_contain_range(black_box(size / 2), black_box(size / 2 + 1000)));
        });
    }

    group.finish();
}

// ============================================================================
// 10. Memory Efficiency Benchmarks
// ============================================================================

fn benchmark_memory_stats(c: &mut Criterion) {
    let mut group = c.benchmark_group("grf_memory_stats");

    let keys: Vec<u64> = (0..10_000).map(|i| i * 10).collect();
    let filter = GRF::build(&keys, 6).unwrap();

    group.throughput(Throughput::Elements(1));
    group.bench_function("stats", |b| {
        b.iter(|| filter.stats());
    });

    group.finish();
}

fn benchmark_memory_varying_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("grf_memory_size");

    for size in [1000, 10_000, 100_000] {
        let keys: Vec<u64> = (0..size).collect();
        let filter = GRF::build(&keys, 6).unwrap();
        let stats = filter.stats();

        println!(
            "Size: {}, Memory: {} bytes, Segments: {}",
            size, stats.memory_bytes, stats.segment_count
        );
    }

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    benches,
    // Build benchmarks
    benchmark_build_small,
    benchmark_build_large,
    benchmark_build_varying_bits,
    benchmark_build_skewed_distribution,
    // Range query benchmarks
    benchmark_range_query_small_ranges,
    benchmark_range_query_large_ranges,
    benchmark_range_query_varying_dataset_size,
    // Point query benchmarks
    benchmark_point_query,
    benchmark_point_query_hit_vs_miss,
    // FPR benchmarks
    benchmark_fpr_calculation,
    benchmark_fpr_empirical_measurement,
    // Comparison benchmarks
    benchmark_comparison_build,
    benchmark_comparison_range_query,
    benchmark_comparison_skewed_data,
    // Varying bits benchmarks
    benchmark_varying_bits_query_performance,
    // Distribution benchmarks
    benchmark_distribution_uniform,
    benchmark_distribution_sparse,
    benchmark_distribution_fibonacci,
    // LSM-tree benchmarks
    benchmark_lsm_multi_level_query,
    benchmark_lsm_compaction,
    // Scalability benchmarks
    benchmark_scalability,
    // Memory benchmarks
    benchmark_memory_stats,
    benchmark_memory_varying_sizes,
);

criterion_main!(benches);
