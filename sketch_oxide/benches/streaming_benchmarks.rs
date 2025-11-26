//! Benchmarks for streaming algorithms
//!
//! Compares SlidingWindowCounter performance

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sketch_oxide::streaming::SlidingWindowCounter;

/// Benchmark insertions
fn bench_insertions(c: &mut Criterion) {
    let mut group = c.benchmark_group("sliding_window_insert");
    let epsilons = [0.01, 0.05, 0.1];

    for &epsilon in &epsilons {
        group.throughput(Throughput::Elements(10000));

        group.bench_with_input(
            BenchmarkId::new("SlidingWindowCounter", format!("eps={}", epsilon)),
            &epsilon,
            |b, &eps| {
                b.iter(|| {
                    let mut counter = SlidingWindowCounter::new(10000, eps).unwrap();
                    for i in 0..10000u64 {
                        counter.increment(black_box(i));
                    }
                    counter
                })
            },
        );
    }

    group.finish();
}

/// Benchmark queries
fn bench_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("sliding_window_query");

    // Pre-populate counter
    let mut counter = SlidingWindowCounter::new(10000, 0.1).unwrap();
    for i in 0..10000u64 {
        counter.increment(i);
    }

    group.throughput(Throughput::Elements(1000));

    group.bench_function("count", |b| {
        b.iter(|| {
            let mut total = 0u64;
            for t in 9000..10000u64 {
                total += counter.count(black_box(t));
            }
            total
        })
    });

    group.bench_function("count_range", |b| {
        b.iter(|| {
            let mut total = 0u64;
            for t in 0..1000u64 {
                total += counter.count_range(black_box(t * 10), black_box(t * 10 + 100));
            }
            total
        })
    });

    group.finish();
}

/// Benchmark memory efficiency at different error bounds
fn bench_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("sliding_window_memory");
    let epsilons = [0.01, 0.05, 0.1, 0.2];

    for &epsilon in &epsilons {
        group.bench_with_input(
            BenchmarkId::new("memory_usage", format!("eps={}", epsilon)),
            &epsilon,
            |b, &eps| {
                b.iter(|| {
                    let mut counter = SlidingWindowCounter::new(10000, eps).unwrap();
                    for i in 0..5000u64 {
                        counter.increment(i);
                    }
                    black_box(counter.memory_usage())
                })
            },
        );
    }

    group.finish();
}

/// Benchmark expire operation
fn bench_expire(c: &mut Criterion) {
    let mut group = c.benchmark_group("sliding_window_expire");

    group.bench_function("expire", |b| {
        b.iter_batched(
            || {
                let mut counter = SlidingWindowCounter::new(1000, 0.1).unwrap();
                for i in 0..5000u64 {
                    counter.increment(i);
                }
                counter
            },
            |mut counter| {
                counter.expire(black_box(6000));
                counter
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_insertions,
    bench_queries,
    bench_memory,
    bench_expire
);
criterion_main!(benches);
