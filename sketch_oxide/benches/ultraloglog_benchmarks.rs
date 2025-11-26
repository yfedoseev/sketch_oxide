//! Benchmarks for UltraLogLog cardinality estimation
//!
//! These benchmarks measure the performance of UltraLogLog operations
//! to ensure they meet the sub-microsecond targets.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sketch_oxide::cardinality::UltraLogLog;
use sketch_oxide::{Mergeable, Sketch};

/// Benchmark: Update operation performance at different precisions
fn bench_ultraloglog_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("ultraloglog_update");

    for precision in [8, 10, 12, 14, 16].iter() {
        let mut ull = UltraLogLog::new(*precision).unwrap();

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("precision_{}", precision)),
            precision,
            |b, _| {
                let mut counter = 0u64;
                b.iter(|| {
                    ull.update(black_box(&counter));
                    counter = counter.wrapping_add(1);
                });
            },
        );
    }
    group.finish();
}

/// Benchmark: Estimate computation performance
fn bench_ultraloglog_estimate(c: &mut Criterion) {
    let mut group = c.benchmark_group("ultraloglog_estimate");

    for precision in [8, 12, 16].iter() {
        let mut ull = UltraLogLog::new(*precision).unwrap();

        // Pre-populate with 10,000 items
        for i in 0..10_000 {
            ull.update(&i);
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("precision_{}", precision)),
            precision,
            |b, _| {
                b.iter(|| black_box(ull.estimate()));
            },
        );
    }
    group.finish();
}

/// Benchmark: Merge operation performance
fn bench_ultraloglog_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("ultraloglog_merge");

    for precision in [8, 12, 16].iter() {
        let mut ull1 = UltraLogLog::new(*precision).unwrap();
        let mut ull2 = UltraLogLog::new(*precision).unwrap();

        // Pre-populate both sketches
        for i in 0..5_000 {
            ull1.update(&i);
            ull2.update(&(i + 5_000));
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("precision_{}", precision)),
            precision,
            |b, _| {
                let mut ull1_clone = ull1.clone();
                b.iter(|| {
                    ull1_clone.merge(black_box(&ull2)).unwrap();
                });
            },
        );
    }
    group.finish();
}

/// Benchmark: Serialization performance
fn bench_ultraloglog_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("ultraloglog_serialize");

    for precision in [8, 12, 16].iter() {
        let mut ull = UltraLogLog::new(*precision).unwrap();

        // Pre-populate with 10,000 items
        for i in 0..10_000 {
            ull.update(&i);
        }

        let size = 1 << precision;
        group.throughput(Throughput::Bytes(size as u64 + 1));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("precision_{}", precision)),
            precision,
            |b, _| {
                b.iter(|| black_box(ull.serialize()));
            },
        );
    }
    group.finish();
}

/// Benchmark: Deserialization performance
fn bench_ultraloglog_deserialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("ultraloglog_deserialize");

    for precision in [8, 12, 16].iter() {
        let mut ull = UltraLogLog::new(*precision).unwrap();

        // Pre-populate with 10,000 items
        for i in 0..10_000 {
            ull.update(&i);
        }

        let bytes = ull.serialize();
        let size = bytes.len();
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("precision_{}", precision)),
            precision,
            |b, _| {
                b.iter(|| {
                    let result = UltraLogLog::deserialize(black_box(&bytes));
                    black_box(result)
                });
            },
        );
    }
    group.finish();
}

/// Benchmark: End-to-end workflow (update + estimate)
fn bench_ultraloglog_workflow(c: &mut Criterion) {
    let mut group = c.benchmark_group("ultraloglog_workflow");

    let cardinalities = [100, 1_000, 10_000];

    for &cardinality in &cardinalities {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("cardinality_{}", cardinality)),
            &cardinality,
            |b, &card| {
                b.iter(|| {
                    let mut ull = UltraLogLog::new(12).unwrap();
                    for i in 0..card {
                        ull.update(black_box(&i));
                    }
                    black_box(ull.estimate())
                });
            },
        );
    }
    group.finish();
}

/// Benchmark: Memory allocation overhead
fn bench_ultraloglog_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("ultraloglog_creation");

    for precision in [8, 10, 12, 14, 16].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("precision_{}", precision)),
            precision,
            |b, &p| {
                b.iter(|| {
                    let ull = UltraLogLog::new(p);
                    black_box(ull)
                });
            },
        );
    }
    group.finish();
}

/// Benchmark: Comparison with different cardinalities
fn bench_ultraloglog_accuracy_vs_speed(c: &mut Criterion) {
    let mut group = c.benchmark_group("ultraloglog_accuracy_vs_speed");
    group.sample_size(20); // Fewer samples for longer benchmarks

    let test_cases = vec![
        ("small_100", 100),
        ("medium_10k", 10_000),
        ("large_100k", 100_000),
    ];

    for (name, cardinality) in test_cases {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &cardinality,
            |b, &card| {
                b.iter(|| {
                    let mut ull = UltraLogLog::new(12).unwrap();
                    for i in 0..card {
                        ull.update(black_box(&i));
                    }
                    let estimate = ull.estimate();
                    let error = (estimate - card as f64).abs() / card as f64;
                    black_box(error)
                });
            },
        );
    }
    group.finish();
}

/// Benchmark: Update performance with different data patterns
fn bench_ultraloglog_update_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("ultraloglog_update_patterns");

    // Sequential pattern
    group.bench_function("pattern_sequential", |b| {
        let mut ull = UltraLogLog::new(12).unwrap();
        let mut counter = 0u64;
        b.iter(|| {
            ull.update(black_box(&counter));
            counter += 1;
        });
    });

    // Random-like pattern (using counter XOR with large prime)
    group.bench_function("pattern_pseudo_random", |b| {
        let mut ull = UltraLogLog::new(12).unwrap();
        let mut counter = 0u64;
        let prime = 9223372036854775783u64; // Large prime
        b.iter(|| {
            let value = counter ^ prime;
            ull.update(black_box(&value));
            counter += 1;
        });
    });

    // High collision pattern (many duplicates)
    group.bench_function("pattern_high_duplicates", |b| {
        let mut ull = UltraLogLog::new(12).unwrap();
        let mut counter = 0u64;
        b.iter(|| {
            let value = counter % 100; // Only 100 unique values
            ull.update(black_box(&value));
            counter += 1;
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_ultraloglog_update,
    bench_ultraloglog_estimate,
    bench_ultraloglog_merge,
    bench_ultraloglog_serialize,
    bench_ultraloglog_deserialize,
    bench_ultraloglog_workflow,
    bench_ultraloglog_creation,
    bench_ultraloglog_accuracy_vs_speed,
    bench_ultraloglog_update_patterns,
);

criterion_main!(benches);
