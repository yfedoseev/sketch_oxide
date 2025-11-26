use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use sketch_oxide::sampling::{ReservoirSampling, VarOptSampling};

/// Benchmark: ReservoirSampling update operations
fn bench_reservoir_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("reservoir_update");

    for k in [10, 100, 1000] {
        group.bench_with_input(BenchmarkId::new("update", k), &k, |b, &k| {
            let mut reservoir = ReservoirSampling::with_seed(k, 42).unwrap();
            let mut counter = 0u64;
            b.iter(|| {
                reservoir.update(black_box(format!("item_{}", counter)));
                counter += 1;
            });
        });
    }

    group.finish();
}

/// Benchmark: ReservoirSampling with different stream sizes
fn bench_reservoir_stream(c: &mut Criterion) {
    let mut group = c.benchmark_group("reservoir_stream");

    for stream_size in [1000, 10000, 100000] {
        group.bench_with_input(
            BenchmarkId::new("process_stream", stream_size),
            &stream_size,
            |b, &stream_size| {
                b.iter(|| {
                    let mut reservoir = ReservoirSampling::with_seed(100, 42).unwrap();
                    for i in 0..stream_size {
                        reservoir.update(format!("item_{}", i));
                    }
                    black_box(reservoir.sample());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: ReservoirSampling merge operations
fn bench_reservoir_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("reservoir_merge");

    group.bench_function("merge_two_reservoirs", |b| {
        b.iter_batched(
            || {
                let mut r1 = ReservoirSampling::with_seed(100, 42).unwrap();
                let mut r2 = ReservoirSampling::with_seed(100, 43).unwrap();
                for i in 0..1000 {
                    r1.update(format!("r1_item_{}", i));
                    r2.update(format!("r2_item_{}", i));
                }
                (r1, r2)
            },
            |(mut r1, r2)| {
                r1.merge(&r2).unwrap();
                black_box(r1);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark: VarOptSampling update operations
fn bench_varopt_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("varopt_update");

    for k in [10, 100, 1000] {
        group.bench_with_input(BenchmarkId::new("update_uniform", k), &k, |b, &k| {
            let mut sampler = VarOptSampling::with_seed(k, 42).unwrap();
            let mut counter = 0u64;
            b.iter(|| {
                sampler.update(black_box(format!("item_{}", counter)), black_box(1.0));
                counter += 1;
            });
        });

        group.bench_with_input(BenchmarkId::new("update_varied", k), &k, |b, &k| {
            let mut sampler = VarOptSampling::with_seed(k, 42).unwrap();
            let mut counter = 0u64;
            b.iter(|| {
                let weight = ((counter % 100) + 1) as f64;
                sampler.update(black_box(format!("item_{}", counter)), black_box(weight));
                counter += 1;
            });
        });
    }

    group.finish();
}

/// Benchmark: VarOptSampling with heavy hitters
fn bench_varopt_heavy_hitters(c: &mut Criterion) {
    let mut group = c.benchmark_group("varopt_heavy_hitters");

    group.bench_function("mixed_weights", |b| {
        b.iter(|| {
            let mut sampler = VarOptSampling::with_seed(100, 42).unwrap();

            // Add some heavy hitters
            for i in 0..10 {
                sampler.update(format!("heavy_{}", i), 10000.0);
            }

            // Add many light items
            for i in 0..10000 {
                sampler.update(format!("light_{}", i), 1.0);
            }

            black_box(sampler.sample());
        });
    });

    group.bench_function("zipf_distribution", |b| {
        b.iter(|| {
            let mut sampler = VarOptSampling::with_seed(100, 42).unwrap();

            // Simulate Zipf distribution
            for i in 1..=1000 {
                let weight = 1.0 / (i as f64);
                sampler.update(format!("item_{}", i), weight);
            }

            black_box(sampler.sample());
        });
    });

    group.finish();
}

/// Benchmark: VarOptSampling stream processing
fn bench_varopt_stream(c: &mut Criterion) {
    let mut group = c.benchmark_group("varopt_stream");

    for stream_size in [1000, 10000, 100000] {
        group.bench_with_input(
            BenchmarkId::new("process_stream", stream_size),
            &stream_size,
            |b, &stream_size| {
                b.iter(|| {
                    let mut sampler = VarOptSampling::with_seed(100, 42).unwrap();
                    for i in 0..stream_size {
                        let weight = ((i % 100) + 1) as f64;
                        sampler.update(format!("item_{}", i), weight);
                    }
                    black_box(sampler.sample());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Comparison between Reservoir and VarOpt
fn bench_sampling_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("sampling_comparison");
    let stream_size = 10000;
    let k = 100;

    group.bench_function("reservoir_10k_items", |b| {
        b.iter(|| {
            let mut reservoir = ReservoirSampling::with_seed(k, 42).unwrap();
            for i in 0..stream_size {
                reservoir.update(format!("item_{}", i));
            }
            black_box(reservoir.sample());
        });
    });

    group.bench_function("varopt_10k_uniform", |b| {
        b.iter(|| {
            let mut sampler = VarOptSampling::with_seed(k, 42).unwrap();
            for i in 0..stream_size {
                sampler.update(format!("item_{}", i), 1.0);
            }
            black_box(sampler.sample());
        });
    });

    group.bench_function("varopt_10k_weighted", |b| {
        b.iter(|| {
            let mut sampler = VarOptSampling::with_seed(k, 42).unwrap();
            for i in 0..stream_size {
                let weight = ((i % 100) + 1) as f64;
                sampler.update(format!("item_{}", i), weight);
            }
            black_box(sampler.sample());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_reservoir_update,
    bench_reservoir_stream,
    bench_reservoir_merge,
    bench_varopt_update,
    bench_varopt_heavy_hitters,
    bench_varopt_stream,
    bench_sampling_comparison,
);

criterion_main!(benches);
