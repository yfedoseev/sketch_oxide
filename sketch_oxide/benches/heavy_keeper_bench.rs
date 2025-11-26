//! Benchmarks for HeavyKeeper Heavy Hitter Detection
//!
//! This benchmark suite evaluates HeavyKeeper performance across different operations:
//! 1. Single update latency
//! 2. Top-k query performance
//! 3. Estimate operation
//! 4. Merge operation
//! 5. Decay operation
//! 6. Throughput under load

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sketch_oxide::frequency::HeavyKeeper;

/// Benchmark 1: Single item update latency
fn benchmark_update_single(c: &mut Criterion) {
    let mut group = c.benchmark_group("heavy_keeper_update");

    // Test with different k values
    for k in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("single_update", k), k, |b, &k| {
            let mut hk = HeavyKeeper::new(k, 0.001, 0.01).unwrap();
            let item = black_box(b"test_item");
            b.iter(|| {
                hk.update(item);
            });
        });
    }

    group.finish();
}

/// Benchmark 2: Top-k query performance
fn benchmark_top_k_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("heavy_keeper_top_k");

    // Prepare sketches with different sizes
    for k in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("query", k), k, |b, &k| {
            let mut hk = HeavyKeeper::new(k, 0.001, 0.01).unwrap();

            // Pre-populate with items
            for i in 0..10000 {
                hk.update(format!("item_{}", i % 100).as_bytes());
            }

            b.iter(|| {
                black_box(hk.top_k());
            });
        });
    }

    group.finish();
}

/// Benchmark 3: Estimate operation performance
fn benchmark_estimate(c: &mut Criterion) {
    let mut group = c.benchmark_group("heavy_keeper_estimate");

    let mut hk = HeavyKeeper::new(100, 0.001, 0.01).unwrap();

    // Pre-populate
    for i in 0..10000 {
        hk.update(format!("item_{}", i % 100).as_bytes());
    }

    let item = b"item_50";

    group.bench_function("estimate_count", |b| {
        b.iter(|| {
            black_box(hk.estimate(item));
        });
    });

    group.finish();
}

/// Benchmark 4: Merge operation performance
fn benchmark_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("heavy_keeper_merge");

    for num_items in [1000, 10000, 100000].iter() {
        group.bench_with_input(BenchmarkId::new("merge", num_items), num_items, |b, &n| {
            b.iter_batched(
                || {
                    let mut hk1 = HeavyKeeper::new(100, 0.001, 0.01).unwrap();
                    let mut hk2 = HeavyKeeper::new(100, 0.001, 0.01).unwrap();

                    for i in 0..n {
                        hk1.update(format!("item_{}", i % 100).as_bytes());
                        hk2.update(format!("item_{}", i % 100 + 50).as_bytes());
                    }

                    (hk1, hk2)
                },
                |(mut hk1, hk2)| {
                    hk1.merge(&hk2).unwrap();
                    black_box(());
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark 5: Decay operation performance
fn benchmark_decay(c: &mut Criterion) {
    let mut group = c.benchmark_group("heavy_keeper_decay");

    for k in [100, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("decay", k), k, |b, &k| {
            let mut hk = HeavyKeeper::new(k, 0.001, 0.01).unwrap();

            // Pre-populate
            for i in 0..10000 {
                hk.update(format!("item_{}", i % 100).as_bytes());
            }

            b.iter(|| {
                hk.decay();
            });
        });
    }

    group.finish();
}

/// Benchmark 6: Throughput test - many updates
fn benchmark_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("heavy_keeper_throughput");
    group.throughput(Throughput::Elements(1000));

    group.bench_function("1000_updates", |b| {
        let mut hk = HeavyKeeper::new(100, 0.001, 0.01).unwrap();
        b.iter(|| {
            for i in 0..1000 {
                hk.update(black_box(format!("item_{}", i % 100).as_bytes()));
            }
        });
    });

    group.finish();
}

/// Benchmark 7: Comparison with different epsilon values
fn benchmark_epsilon_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("heavy_keeper_epsilon");

    for epsilon in [0.0001, 0.001, 0.01, 0.1].iter() {
        group.bench_with_input(
            BenchmarkId::new("update_with_epsilon", epsilon),
            epsilon,
            |b, &eps| {
                let mut hk = HeavyKeeper::new(100, eps, 0.01).unwrap();
                let item = black_box(b"test_item");
                b.iter(|| {
                    hk.update(item);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark 8: Heavy hitter detection in skewed distribution
fn benchmark_skewed_distribution(c: &mut Criterion) {
    let mut group = c.benchmark_group("heavy_keeper_skewed");

    group.bench_function("zipf_distribution", |b| {
        b.iter(|| {
            let mut hk = HeavyKeeper::new(20, 0.001, 0.01).unwrap();

            // Zipf-like distribution
            for i in 1..=100 {
                let freq = (1000.0 / (i as f64).powf(1.5)) as usize;
                for _ in 0..freq {
                    hk.update(black_box(format!("item_{}", i).as_bytes()));
                }
            }

            black_box(hk.top_k());
        });
    });

    group.finish();
}

/// Benchmark 9: Comparison with varying k values
fn benchmark_varying_k(c: &mut Criterion) {
    let mut group = c.benchmark_group("heavy_keeper_varying_k");

    for k in [1, 10, 50, 100, 500, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("k_value", k), k, |b, &k| {
            let mut hk = HeavyKeeper::new(k, 0.001, 0.01).unwrap();

            b.iter(|| {
                for i in 0..1000 {
                    hk.update(black_box(format!("item_{}", i % 100).as_bytes()));
                }
            });
        });
    }

    group.finish();
}

/// Benchmark 10: Large scale stress test
fn benchmark_large_scale(c: &mut Criterion) {
    let mut group = c.benchmark_group("heavy_keeper_large_scale");
    group.sample_size(10); // Fewer samples for large test

    group.bench_function("1M_updates_10k_items", |b| {
        b.iter(|| {
            let mut hk = HeavyKeeper::new(100, 0.001, 0.01).unwrap();

            for i in 0..1_000_000 {
                hk.update(black_box(format!("item_{}", i % 10_000).as_bytes()));
            }

            black_box(hk.top_k());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_update_single,
    benchmark_top_k_query,
    benchmark_estimate,
    benchmark_merge,
    benchmark_decay,
    benchmark_throughput,
    benchmark_epsilon_comparison,
    benchmark_skewed_distribution,
    benchmark_varying_k,
    benchmark_large_scale,
);

criterion_main!(benches);
