//! Memento Filter Benchmarks
//!
//! Comprehensive performance benchmarking for the dynamic range filter.
//!
//! Benchmarks:
//! 1. Construction with varying sizes
//! 2. Single insertion latency
//! 3. Batch insertions (100, 1000, 10000)
//! 4. Range expansion overhead
//! 5. Query latency (hit/miss)
//! 6. Quotient filter lookup
//! 7. Throughput tests
//! 8. Memory usage tracking
//! 9. Varying FPR targets

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sketch_oxide::common::RangeFilter;
use sketch_oxide::range_filters::MementoFilter;

// ============================================================================
// Construction Benchmarks
// ============================================================================

fn bench_memento_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("memento_construction");

    for size in [1_000, 10_000, 100_000, 1_000_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let filter = MementoFilter::new(black_box(size), black_box(0.01));
                black_box(filter)
            });
        });
    }

    group.finish();
}

fn bench_memento_construction_varying_fpr(c: &mut Criterion) {
    let mut group = c.benchmark_group("memento_construction_varying_fpr");

    for fpr in [0.001, 0.01, 0.05, 0.1].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(fpr), fpr, |b, &fpr| {
            b.iter(|| {
                let filter = MementoFilter::new(black_box(10_000), black_box(fpr));
                black_box(filter)
            });
        });
    }

    group.finish();
}

// ============================================================================
// Insertion Benchmarks
// ============================================================================

fn bench_memento_single_insertion(c: &mut Criterion) {
    let mut group = c.benchmark_group("memento_single_insertion");
    group.throughput(Throughput::Elements(1));

    for size in [1_000, 10_000, 100_000].iter() {
        let mut filter = MementoFilter::new(*size, 0.01).unwrap();
        let mut counter = 0u64;

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _size| {
            b.iter(|| {
                let key = counter;
                counter += 1;
                filter
                    .insert(black_box(key), black_box(b"benchmark_value"))
                    .unwrap();
            });
        });
    }

    group.finish();
}

fn bench_memento_batch_insertions_100(c: &mut Criterion) {
    let mut group = c.benchmark_group("memento_batch_insertions_100");
    group.throughput(Throughput::Elements(100));

    let mut filter = MementoFilter::new(100_000, 0.01).unwrap();
    let mut start_key = 0u64;

    group.bench_function("batch_100", |b| {
        b.iter(|| {
            for i in 0..100 {
                let key = start_key + i;
                filter.insert(black_box(key), black_box(b"value")).unwrap();
            }
            start_key += 100;
        });
    });

    group.finish();
}

fn bench_memento_batch_insertions_1000(c: &mut Criterion) {
    let mut group = c.benchmark_group("memento_batch_insertions_1000");
    group.throughput(Throughput::Elements(1000));

    let mut filter = MementoFilter::new(100_000, 0.01).unwrap();
    let mut start_key = 0u64;

    group.bench_function("batch_1000", |b| {
        b.iter(|| {
            for i in 0..1000 {
                let key = start_key + i;
                filter.insert(black_box(key), black_box(b"value")).unwrap();
            }
            start_key += 1000;
        });
    });

    group.finish();
}

fn bench_memento_batch_insertions_10000(c: &mut Criterion) {
    let mut group = c.benchmark_group("memento_batch_insertions_10000");
    group.throughput(Throughput::Elements(10000));

    let mut filter = MementoFilter::new(100_000, 0.01).unwrap();
    let mut start_key = 0u64;

    group.bench_function("batch_10000", |b| {
        b.iter(|| {
            for i in 0..10000 {
                let key = start_key + i;
                filter.insert(black_box(key), black_box(b"value")).unwrap();
            }
            start_key += 10000;
        });
    });

    group.finish();
}

// ============================================================================
// Range Expansion Benchmarks
// ============================================================================

fn bench_memento_range_expansion(c: &mut Criterion) {
    let mut group = c.benchmark_group("memento_range_expansion");

    group.bench_function("expansion_overhead", |b| {
        b.iter(|| {
            let mut filter = MementoFilter::new(10_000, 0.01).unwrap();

            // Insert causing expansions
            for i in 0..100 {
                let key = i * 1000; // Causes range expansions
                filter.insert(black_box(key), black_box(b"value")).unwrap();
            }

            black_box(filter.stats().num_expansions)
        });
    });

    group.finish();
}

fn bench_memento_expansion_vs_no_expansion(c: &mut Criterion) {
    let mut group = c.benchmark_group("memento_expansion_comparison");

    // Benchmark without expansion (sequential inserts)
    group.bench_function("sequential_no_expansion", |b| {
        b.iter(|| {
            let mut filter = MementoFilter::new(10_000, 0.01).unwrap();
            for i in 0..1000 {
                filter.insert(black_box(i), black_box(b"value")).unwrap();
            }
            black_box(filter)
        });
    });

    // Benchmark with expansion (sparse inserts)
    group.bench_function("sparse_with_expansion", |b| {
        b.iter(|| {
            let mut filter = MementoFilter::new(10_000, 0.01).unwrap();
            for i in 0..1000 {
                let key = i * 100; // Causes expansions
                filter.insert(black_box(key), black_box(b"value")).unwrap();
            }
            black_box(filter)
        });
    });

    group.finish();
}

// ============================================================================
// Query Benchmarks
// ============================================================================

fn bench_memento_query_hit(c: &mut Criterion) {
    let mut group = c.benchmark_group("memento_query_hit");
    group.throughput(Throughput::Elements(1));

    for size in [1_000, 10_000, 100_000].iter() {
        let mut filter = MementoFilter::new(*size, 0.01).unwrap();

        // Insert elements
        for i in 0..*size {
            filter.insert(i as u64, b"value").unwrap();
        }

        let mut query_idx = 0;

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let key = (query_idx % size) as u64;
                query_idx += 1;
                black_box(filter.may_contain_range(key, key))
            });
        });
    }

    group.finish();
}

fn bench_memento_query_miss(c: &mut Criterion) {
    let mut group = c.benchmark_group("memento_query_miss");
    group.throughput(Throughput::Elements(1));

    for size in [1_000, 10_000, 100_000].iter() {
        let mut filter = MementoFilter::new(*size, 0.01).unwrap();

        // Insert elements in even positions
        for i in 0..(*size / 2) {
            filter.insert((i * 2) as u64, b"value").unwrap();
        }

        let mut query_idx = 0;

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let key = ((query_idx % (size / 2)) * 2 + 1) as u64; // Odd keys (not inserted)
                query_idx += 1;
                black_box(filter.may_contain_range(key, key))
            });
        });
    }

    group.finish();
}

fn bench_memento_range_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("memento_range_query");
    group.throughput(Throughput::Elements(1));

    let mut filter = MementoFilter::new(100_000, 0.01).unwrap();

    // Insert 10,000 elements
    for i in 0..10_000 {
        filter.insert(i * 10, b"value").unwrap();
    }

    group.bench_function("range_query_small", |b| {
        let mut start = 0;
        b.iter(|| {
            let result = filter.may_contain_range(black_box(start), black_box(start + 100));
            start += 100;
            black_box(result)
        });
    });

    group.bench_function("range_query_medium", |b| {
        let mut start = 0;
        b.iter(|| {
            let result = filter.may_contain_range(black_box(start), black_box(start + 1000));
            start += 1000;
            black_box(result)
        });
    });

    group.bench_function("range_query_large", |b| {
        let mut start = 0;
        b.iter(|| {
            let result = filter.may_contain_range(black_box(start), black_box(start + 10000));
            start += 10000;
            black_box(result)
        });
    });

    group.finish();
}

// ============================================================================
// Quotient Filter Layer Benchmarks
// ============================================================================

fn bench_memento_quotient_filter_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("memento_quotient_filter_lookup");
    group.throughput(Throughput::Elements(1));

    let mut filter = MementoFilter::new(10_000, 0.01).unwrap();

    // Insert with different values for same keys (forces QF usage)
    for i in 0..1000 {
        filter.insert(i, format!("value_{}", i).as_bytes()).unwrap();
    }

    let mut query_idx = 0;

    group.bench_function("qf_lookup", |b| {
        b.iter(|| {
            let key = query_idx % 1000;
            query_idx += 1;
            black_box(filter.may_contain_range(key, key))
        });
    });

    group.finish();
}

// ============================================================================
// Throughput Benchmarks
// ============================================================================

fn bench_memento_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("memento_throughput");

    group.bench_function("insert_throughput_1M", |b| {
        b.iter(|| {
            let mut filter = MementoFilter::new(1_000_000, 0.01).unwrap();

            for i in 0..1_000_000 {
                filter.insert(black_box(i), black_box(b"value")).unwrap();
            }

            black_box(filter)
        });
    });

    group.bench_function("query_throughput_1M", |b| {
        let mut filter = MementoFilter::new(1_000_000, 0.01).unwrap();

        // Pre-fill
        for i in 0..100_000 {
            filter.insert(i, b"value").unwrap();
        }

        b.iter(|| {
            for i in 0..1_000_000 {
                black_box(filter.may_contain_range(i, i));
            }
        });
    });

    group.finish();
}

// ============================================================================
// Memory Usage Benchmarks
// ============================================================================

fn bench_memento_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memento_memory_usage");

    for size in [1_000, 10_000, 100_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut filter = MementoFilter::new(size, 0.01).unwrap();

                // Fill filter
                for i in 0..size {
                    filter.insert(i as u64, b"value").unwrap();
                }

                black_box(filter.stats())
            });
        });
    }

    group.finish();
}

// ============================================================================
// FPR Target Benchmarks
// ============================================================================

fn bench_memento_varying_fpr(c: &mut Criterion) {
    let mut group = c.benchmark_group("memento_varying_fpr");

    for fpr in [0.001, 0.01, 0.05, 0.1].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(fpr), fpr, |b, &fpr| {
            b.iter(|| {
                let mut filter = MementoFilter::new(10_000, fpr).unwrap();

                for i in 0..1000 {
                    filter.insert(black_box(i), black_box(b"value")).unwrap();
                }

                black_box(filter)
            });
        });
    }

    group.finish();
}

// ============================================================================
// Comparison Benchmarks
// ============================================================================

fn bench_memento_vs_static_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("memento_comparison");

    // Memento filter with dynamic insertions
    group.bench_function("memento_dynamic", |b| {
        b.iter(|| {
            let mut filter = MementoFilter::new(10_000, 0.01).unwrap();

            for i in 0..1000 {
                filter
                    .insert(black_box(i * 10), black_box(b"value"))
                    .unwrap();
            }

            // Query
            for i in 0..1000 {
                black_box(filter.may_contain_range(i * 10, i * 10));
            }
        });
    });

    group.finish();
}

// ============================================================================
// Stats Collection Benchmarks
// ============================================================================

fn bench_memento_stats_collection(c: &mut Criterion) {
    let mut group = c.benchmark_group("memento_stats");

    let mut filter = MementoFilter::new(10_000, 0.01).unwrap();

    for i in 0..1000 {
        filter.insert(i, b"value").unwrap();
    }

    group.bench_function("stats_collection", |b| {
        b.iter(|| {
            let stats = filter.stats();
            black_box(stats)
        });
    });

    group.bench_function("range_collection", |b| {
        b.iter(|| {
            let range = filter.range();
            black_box(range)
        });
    });

    group.finish();
}

// ============================================================================
// Criterion Groups
// ============================================================================

criterion_group!(
    benches,
    bench_memento_construction,
    bench_memento_construction_varying_fpr,
    bench_memento_single_insertion,
    bench_memento_batch_insertions_100,
    bench_memento_batch_insertions_1000,
    bench_memento_batch_insertions_10000,
    bench_memento_range_expansion,
    bench_memento_expansion_vs_no_expansion,
    bench_memento_query_hit,
    bench_memento_query_miss,
    bench_memento_range_query,
    bench_memento_quotient_filter_lookup,
    bench_memento_throughput,
    bench_memento_memory_usage,
    bench_memento_varying_fpr,
    bench_memento_vs_static_filter,
    bench_memento_stats_collection,
);

criterion_main!(benches);
