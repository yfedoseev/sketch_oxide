//! Benchmarks for Sliding HyperLogLog
//!
//! Tests performance characteristics of time-windowed cardinality estimation

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sketch_oxide::streaming::SlidingHyperLogLog;
use sketch_oxide::Mergeable;

/// Benchmark single update operations
fn bench_single_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("sliding_hll_single_update");
    let precisions = [8, 10, 12, 14];

    for &precision in &precisions {
        group.bench_with_input(
            BenchmarkId::new("update", format!("p={}", precision)),
            &precision,
            |b, &p| {
                let mut hll = SlidingHyperLogLog::new(p, 3600).unwrap();
                let mut counter = 0u64;
                b.iter(|| {
                    hll.update(&black_box(counter), black_box(1000)).unwrap();
                    counter += 1;
                })
            },
        );
    }

    group.finish();
}

/// Benchmark batch update operations
fn bench_batch_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("sliding_hll_batch_updates");
    let batch_sizes = [100, 1000, 10000];

    for &batch_size in &batch_sizes {
        group.throughput(Throughput::Elements(batch_size));

        group.bench_with_input(
            BenchmarkId::new("batch", batch_size),
            &batch_size,
            |b, &size| {
                b.iter(|| {
                    let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();
                    for i in 0..size {
                        hll.update(&black_box(i), black_box(1000 + i)).unwrap();
                    }
                    hll
                })
            },
        );
    }

    group.finish();
}

/// Benchmark window query performance
fn bench_window_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("sliding_hll_window_query");

    // Pre-populate with data
    let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();
    for i in 0..10000 {
        hll.update(&i, 1000 + (i % 3600)).unwrap();
    }

    group.bench_function("estimate_window", |b| {
        b.iter(|| hll.estimate_window(black_box(2500), black_box(600)))
    });

    group.bench_function("estimate_total", |b| b.iter(|| hll.estimate_total()));

    group.finish();
}

/// Benchmark decay operation
fn bench_decay(c: &mut Criterion) {
    let mut group = c.benchmark_group("sliding_hll_decay");
    let precisions = [8, 10, 12, 14];

    for &precision in &precisions {
        group.bench_with_input(
            BenchmarkId::new("decay", format!("p={}", precision)),
            &precision,
            |b, &p| {
                b.iter_batched(
                    || {
                        let mut hll = SlidingHyperLogLog::new(p, 3600).unwrap();
                        for i in 0..10000 {
                            hll.update(&i, 1000 + (i % 3600)).unwrap();
                        }
                        hll
                    },
                    |mut hll| {
                        hll.decay(black_box(2500), black_box(1000)).unwrap();
                        hll
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

/// Benchmark merge operation
fn bench_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("sliding_hll_merge");
    let precisions = [8, 10, 12, 14];

    for &precision in &precisions {
        group.bench_with_input(
            BenchmarkId::new("merge", format!("p={}", precision)),
            &precision,
            |b, &p| {
                b.iter_batched(
                    || {
                        let mut hll1 = SlidingHyperLogLog::new(p, 3600).unwrap();
                        let mut hll2 = SlidingHyperLogLog::new(p, 3600).unwrap();

                        for i in 0..5000 {
                            hll1.update(&i, 1000).unwrap();
                        }
                        for i in 2500..7500 {
                            hll2.update(&i, 1000).unwrap();
                        }

                        (hll1, hll2)
                    },
                    |(mut hll1, hll2)| {
                        hll1.merge(&hll2).unwrap();
                        hll1
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

/// Benchmark accuracy measurement overhead
fn bench_accuracy(c: &mut Criterion) {
    let mut group = c.benchmark_group("sliding_hll_accuracy");
    let cardinalities = [1000, 10000, 100000];

    for &cardinality in &cardinalities {
        group.bench_with_input(
            BenchmarkId::new("accuracy_test", cardinality),
            &cardinality,
            |b, &card| {
                b.iter(|| {
                    let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();
                    for i in 0..card {
                        hll.update(&i, 1000).unwrap();
                    }
                    let estimate = hll.estimate_total();
                    let error = (estimate - card as f64).abs() / card as f64;
                    black_box((estimate, error))
                })
            },
        );
    }

    group.finish();
}

/// Benchmark throughput
fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("sliding_hll_throughput");
    group.throughput(Throughput::Elements(1_000_000));

    group.bench_function("1M_updates", |b| {
        b.iter(|| {
            let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();
            for i in 0..1_000_000 {
                hll.update(&black_box(i), black_box(1000 + (i % 3600)))
                    .unwrap();
            }
            hll
        })
    });

    group.finish();
}

/// Benchmark with varying precision
fn bench_precision_impact(c: &mut Criterion) {
    let mut group = c.benchmark_group("sliding_hll_precision_impact");
    let precisions = [4, 8, 10, 12, 14, 16];

    for &precision in &precisions {
        group.throughput(Throughput::Elements(10000));

        group.bench_with_input(
            BenchmarkId::new("precision", precision),
            &precision,
            |b, &p| {
                b.iter(|| {
                    let mut hll = SlidingHyperLogLog::new(p, 3600).unwrap();
                    for i in 0..10000 {
                        hll.update(&black_box(i), black_box(1000)).unwrap();
                    }
                    hll.estimate_total()
                })
            },
        );
    }

    group.finish();
}

/// Benchmark with varying window sizes
fn bench_window_size_impact(c: &mut Criterion) {
    let mut group = c.benchmark_group("sliding_hll_window_size");
    let window_sizes = [60, 300, 3600, 86400]; // 1 min, 5 min, 1 hour, 1 day

    for &window_size in &window_sizes {
        group.bench_with_input(
            BenchmarkId::new("window", window_size),
            &window_size,
            |b, &ws| {
                b.iter_batched(
                    || {
                        let mut hll = SlidingHyperLogLog::new(12, ws).unwrap();
                        for i in 0..10000 {
                            hll.update(&i, 1000 + (i % ws)).unwrap();
                        }
                        hll
                    },
                    |hll| hll.estimate_window(black_box(2000), black_box(ws / 2)),
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

/// Benchmark large-scale test simulating production workload
fn bench_large_scale(c: &mut Criterion) {
    let mut group = c.benchmark_group("sliding_hll_large_scale");
    group.sample_size(10); // Fewer samples for large test
    group.throughput(Throughput::Elements(1_000_000));

    group.bench_function("production_simulation", |b| {
        b.iter(|| {
            let mut hll = SlidingHyperLogLog::new(14, 3600).unwrap();

            // Simulate 1M events over 1 hour
            for i in 0..1_000_000 {
                hll.update(&black_box(i), black_box(1000 + (i % 3600)))
                    .unwrap();
            }

            // Simulate periodic queries
            let mut total = 0.0;
            for window_end in (1600..4600).step_by(300) {
                total += hll.estimate_window(window_end, 300);
            }

            // Simulate decay
            hll.decay(4600, 1800).unwrap();

            black_box((hll, total))
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_single_update,
    bench_batch_updates,
    bench_window_query,
    bench_decay,
    bench_merge,
    bench_accuracy,
    bench_throughput,
    bench_precision_impact,
    bench_window_size_impact,
    bench_large_scale
);
criterion_main!(benches);
