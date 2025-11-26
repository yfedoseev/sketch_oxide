//! Benchmarks for Conservative Count-Min Sketch
//!
//! Compares ConservativeCountMin vs standard CountMinSketch

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sketch_oxide::frequency::{ConservativeCountMin, CountMinSketch};

/// Generate test keys
fn generate_keys(count: usize) -> Vec<String> {
    (0..count).map(|i| format!("key_{}", i)).collect()
}

/// Benchmark insertions
fn bench_insertions(c: &mut Criterion) {
    let mut group = c.benchmark_group("count_min_insert");
    let keys = generate_keys(10000);

    group.throughput(Throughput::Elements(10000));

    // Standard CountMinSketch
    group.bench_function("CountMinSketch", |b| {
        b.iter(|| {
            let mut cms = CountMinSketch::new(0.01, 0.01).unwrap();
            for key in &keys {
                cms.update(black_box(key));
            }
            cms
        })
    });

    // Conservative CountMin
    group.bench_function("ConservativeCountMin", |b| {
        b.iter(|| {
            let mut cms = ConservativeCountMin::new(0.01, 0.01).unwrap();
            for key in &keys {
                cms.update(black_box(key));
            }
            cms
        })
    });

    group.finish();
}

/// Benchmark queries
fn bench_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("count_min_query");
    let keys = generate_keys(10000);
    let query_keys = generate_keys(1000);

    // Pre-populate sketches
    let mut std_cms = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut cons_cms = ConservativeCountMin::new(0.01, 0.01).unwrap();

    for key in &keys {
        std_cms.update(key);
        cons_cms.update(key);
    }

    group.throughput(Throughput::Elements(1000));

    group.bench_function("CountMinSketch", |b| {
        b.iter(|| {
            let mut total = 0u64;
            for key in &query_keys {
                total += std_cms.estimate(black_box(key));
            }
            total
        })
    });

    group.bench_function("ConservativeCountMin", |b| {
        b.iter(|| {
            let mut total = 0u64;
            for key in &query_keys {
                total += cons_cms.estimate(black_box(key));
            }
            total
        })
    });

    group.finish();
}

/// Benchmark accuracy comparison
fn bench_accuracy(c: &mut Criterion) {
    let mut group = c.benchmark_group("count_min_accuracy");

    let epsilon_values = [0.01, 0.05, 0.1];

    for &epsilon in &epsilon_values {
        group.bench_with_input(
            BenchmarkId::new("CountMinSketch", format!("eps={}", epsilon)),
            &epsilon,
            |b, &eps| {
                b.iter(|| {
                    let mut cms = CountMinSketch::new(eps, 0.01).unwrap();

                    // Insert frequent item 100 times
                    for _ in 0..100 {
                        cms.update(&"frequent");
                    }

                    // Insert many other items
                    for i in 0..1000 {
                        cms.update(&format!("item_{}", i));
                    }

                    black_box(cms.estimate(&"frequent"))
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("ConservativeCountMin", format!("eps={}", epsilon)),
            &epsilon,
            |b, &eps| {
                b.iter(|| {
                    let mut cms = ConservativeCountMin::new(eps, 0.01).unwrap();

                    // Insert frequent item 100 times
                    for _ in 0..100 {
                        cms.update(&"frequent");
                    }

                    // Insert many other items
                    for i in 0..1000 {
                        cms.update(&format!("item_{}", i));
                    }

                    black_box(cms.estimate(&"frequent"))
                })
            },
        );
    }

    group.finish();
}

/// Benchmark memory usage
fn bench_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("count_min_memory");

    group.bench_function("ConservativeCountMin", |b| {
        b.iter(|| {
            let cms = ConservativeCountMin::new(0.01, 0.01).unwrap();
            black_box(cms.memory_usage())
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_insertions,
    bench_queries,
    bench_accuracy,
    bench_memory
);
criterion_main!(benches);
