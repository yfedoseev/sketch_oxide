use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use sketch_oxide::frequency::frequent::{ErrorType, FrequentItems};

/// Benchmark: Update operations
fn bench_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("frequent_items_update");

    for max_size in [10, 100, 1000] {
        group.bench_with_input(
            BenchmarkId::new("single", max_size),
            &max_size,
            |b, &size| {
                let mut sketch: FrequentItems<String> = FrequentItems::new(size).unwrap();
                let mut counter = 0;
                b.iter(|| {
                    sketch.update(black_box(format!("item_{}", counter % 100)));
                    counter += 1;
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Update with count
fn bench_update_by(c: &mut Criterion) {
    let mut group = c.benchmark_group("frequent_items_update_by");

    for max_size in [10, 100, 1000] {
        group.bench_with_input(
            BenchmarkId::new("batch", max_size),
            &max_size,
            |b, &size| {
                let mut sketch: FrequentItems<String> = FrequentItems::new(size).unwrap();
                let mut counter = 0;
                b.iter(|| {
                    sketch.update_by(black_box(format!("item_{}", counter % 100)), black_box(10));
                    counter += 1;
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Get estimate (point query)
fn bench_get_estimate(c: &mut Criterion) {
    let mut group = c.benchmark_group("frequent_items_get_estimate");

    for max_size in [10, 100, 1000] {
        let mut sketch: FrequentItems<String> = FrequentItems::new(max_size).unwrap();

        // Populate sketch
        for i in 0..max_size * 2 {
            sketch.update(format!("item_{}", i % max_size));
        }

        group.bench_with_input(
            BenchmarkId::new("query", max_size),
            &max_size,
            |b, &size| {
                let mut counter = 0;
                b.iter(|| {
                    let item = format!("item_{}", counter % size);
                    black_box(sketch.get_estimate(&item));
                    counter += 1;
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Frequent items query (top-K)
fn bench_frequent_items(c: &mut Criterion) {
    let mut group = c.benchmark_group("frequent_items_top_k");

    for max_size in [10, 100, 1000] {
        let mut sketch: FrequentItems<String> = FrequentItems::new(max_size).unwrap();

        // Populate sketch with Zipf distribution
        for rank in 1..=max_size {
            let freq = 1000 / rank;
            for _ in 0..freq {
                sketch.update(format!("item_{}", rank));
            }
        }

        group.bench_with_input(
            BenchmarkId::new("no_false_positives", max_size),
            &sketch,
            |b, sketch: &FrequentItems<String>| {
                b.iter(|| {
                    black_box(sketch.frequent_items(ErrorType::NoFalsePositives));
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("no_false_negatives", max_size),
            &sketch,
            |b, sketch: &FrequentItems<String>| {
                b.iter(|| {
                    black_box(sketch.frequent_items(ErrorType::NoFalseNegatives));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Merge operations
fn bench_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("frequent_items_merge");

    for max_size in [10, 100, 1000] {
        group.bench_with_input(
            BenchmarkId::new("merge", max_size),
            &max_size,
            |b, &size| {
                b.iter_batched(
                    || {
                        let mut sketch1: FrequentItems<String> = FrequentItems::new(size).unwrap();
                        let mut sketch2: FrequentItems<String> = FrequentItems::new(size).unwrap();

                        // Populate both sketches
                        for i in 0..size {
                            sketch1.update(format!("item_{}", i));
                            sketch2.update(format!("item_{}", i + size / 2));
                        }

                        (sketch1, sketch2)
                    },
                    |(mut sketch1, sketch2)| {
                        sketch1.merge(&sketch2).unwrap();
                        black_box(());
                        sketch1
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

/// Benchmark: Heavy hitter workload
fn bench_heavy_hitter_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("frequent_items_heavy_hitter");

    group.bench_function("zipf_stream", |b| {
        b.iter(|| {
            let mut sketch: FrequentItems<String> = FrequentItems::new(black_box(100)).unwrap();

            // Simulate Zipf distribution (80-20 rule)
            for rank in 1..=100 {
                let freq = 1000 / rank;
                for _ in 0..freq {
                    sketch.update(format!("item_{}", rank));
                }
            }

            // Query top-10
            let items = sketch.frequent_items(ErrorType::NoFalsePositives);
            black_box(items.into_iter().take(10).collect::<Vec<_>>());
        });
    });

    group.finish();
}

/// Benchmark: Purge operations (worst case)
fn bench_purge_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("frequent_items_purge");

    group.bench_function("trigger_purge", |b| {
        b.iter(|| {
            let mut sketch: FrequentItems<String> = FrequentItems::new(black_box(10)).unwrap();

            // Add items to trigger multiple purges
            for i in 0..100 {
                sketch.update(format!("item_{}", i));
            }

            black_box(sketch);
        });
    });

    group.finish();
}

/// Benchmark: Realistic workload
fn bench_realistic_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("frequent_items_realistic");

    group.bench_function("web_analytics", |b| {
        b.iter(|| {
            let mut sketch: FrequentItems<String> = FrequentItems::new(black_box(1000)).unwrap();
            let mut seed = 12345u32;

            // Simple LCG random number generator
            let mut next_random = |max: u32| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                seed % max
            };

            // Simulate web analytics: top URLs
            // 80% traffic to 20% of URLs
            for _ in 0..8000 {
                let url = format!("/page_{}", next_random(200));
                sketch.update(url);
            }

            // 20% traffic to remaining 80% of URLs
            for _ in 0..2000 {
                let url = format!("/page_{}", 200 + next_random(800));
                sketch.update(url);
            }

            // Query top-20 pages
            let items = sketch.frequent_items(ErrorType::NoFalsePositives);
            black_box(items.into_iter().take(20).collect::<Vec<_>>());
        });
    });

    group.finish();
}

/// Benchmark: Integer vs String items
fn bench_item_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("frequent_items_types");

    group.bench_function("integers", |b| {
        let mut sketch: FrequentItems<u64> = FrequentItems::new(100).unwrap();
        let mut counter = 0u64;
        b.iter(|| {
            sketch.update(black_box(counter % 100));
            counter += 1;
        });
    });

    group.bench_function("strings", |b| {
        let mut sketch: FrequentItems<String> = FrequentItems::new(100).unwrap();
        let mut counter = 0;
        b.iter(|| {
            sketch.update(black_box(format!("item_{}", counter % 100)));
            counter += 1;
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_update,
    bench_update_by,
    bench_get_estimate,
    bench_frequent_items,
    bench_merge,
    bench_heavy_hitter_workload,
    bench_purge_operations,
    bench_realistic_workload,
    bench_item_types,
);

criterion_main!(benches);
