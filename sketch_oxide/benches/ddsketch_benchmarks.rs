//! Benchmarks for DDSketch (VLDB 2019)
//!
//! Measures performance of:
//! - Add operations (different accuracy levels)
//! - Quantile queries
//! - Merge operations
//! - Memory usage

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sketch_oxide::common::Mergeable;
use sketch_oxide::quantiles::DDSketch;

/// Benchmark add operations with different accuracy levels
fn bench_ddsketch_add(c: &mut Criterion) {
    let mut group = c.benchmark_group("ddsketch_add");

    for accuracy in [0.001, 0.01, 0.05].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("accuracy_{}", accuracy)),
            accuracy,
            |b, &acc| {
                let mut dd = DDSketch::new(acc).unwrap();
                let mut counter = 1.0;
                b.iter(|| {
                    dd.add(black_box(counter));
                    counter += 1.0;
                });
            },
        );
    }
    group.finish();
}

/// Benchmark add operations with different value ranges
fn bench_ddsketch_add_ranges(c: &mut Criterion) {
    let mut group = c.benchmark_group("ddsketch_add_ranges");

    let test_cases = vec![
        ("small_values", 1.0, 100.0),
        ("medium_values", 1.0, 10000.0),
        ("large_values", 1.0, 1000000.0),
    ];

    for (name, min, max) in test_cases {
        group.bench_function(name, |b| {
            let mut dd = DDSketch::new(0.01).unwrap();
            let mut counter = min;
            b.iter(|| {
                dd.add(black_box(counter));
                counter = (counter + 1.0).min(max);
                if counter >= max {
                    counter = min;
                }
            });
        });
    }
    group.finish();
}

/// Benchmark quantile queries on sketches of different sizes
fn bench_ddsketch_quantile(c: &mut Criterion) {
    let mut group = c.benchmark_group("ddsketch_quantile");

    for size in [100, 1000, 10000, 100000].iter() {
        let mut dd = DDSketch::new(0.01).unwrap();
        for i in 1..=*size {
            dd.add(i as f64);
        }

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                black_box(dd.quantile(black_box(0.99)));
            });
        });
    }
    group.finish();
}

/// Benchmark different quantile queries
fn bench_ddsketch_quantile_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("ddsketch_quantile_types");

    let mut dd = DDSketch::new(0.01).unwrap();
    for i in 1..=10000 {
        dd.add(i as f64);
    }

    let quantiles = vec![
        ("p50", 0.50),
        ("p90", 0.90),
        ("p95", 0.95),
        ("p99", 0.99),
        ("p999", 0.999),
    ];

    for (name, q) in quantiles {
        group.bench_function(name, |b| {
            b.iter(|| {
                black_box(dd.quantile(black_box(q)));
            });
        });
    }
    group.finish();
}

/// Benchmark merge operations with different sketch sizes
fn bench_ddsketch_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("ddsketch_merge");

    for size in [100, 1000, 10000].iter() {
        let mut dd1 = DDSketch::new(0.01).unwrap();
        let mut dd2 = DDSketch::new(0.01).unwrap();

        for i in 1..=*size {
            dd1.add(i as f64);
            dd2.add((i + size) as f64);
        }

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let mut dd_copy = dd1.clone();
                dd_copy.merge(black_box(&dd2)).unwrap();
                black_box(dd_copy);
            });
        });
    }
    group.finish();
}

/// Benchmark merge operations with different value distributions
fn bench_ddsketch_merge_distributions(c: &mut Criterion) {
    let mut group = c.benchmark_group("ddsketch_merge_distributions");

    // Disjoint ranges
    let mut dd1_disjoint = DDSketch::new(0.01).unwrap();
    let mut dd2_disjoint = DDSketch::new(0.01).unwrap();
    for i in 1..=5000 {
        dd1_disjoint.add(i as f64);
        dd2_disjoint.add((i + 5000) as f64);
    }

    group.bench_function("disjoint_ranges", |b| {
        b.iter(|| {
            let mut dd_copy = dd1_disjoint.clone();
            dd_copy.merge(black_box(&dd2_disjoint)).unwrap();
        });
    });

    // Overlapping ranges
    let mut dd1_overlap = DDSketch::new(0.01).unwrap();
    let mut dd2_overlap = DDSketch::new(0.01).unwrap();
    for i in 1..=5000 {
        dd1_overlap.add(i as f64);
    }
    for i in 2500..=7500 {
        dd2_overlap.add(i as f64);
    }

    group.bench_function("overlapping_ranges", |b| {
        b.iter(|| {
            let mut dd_copy = dd1_overlap.clone();
            dd_copy.merge(black_box(&dd2_overlap)).unwrap();
        });
    });

    group.finish();
}

/// Benchmark full workflow: create, add many values, query multiple quantiles
fn bench_ddsketch_full_workflow(c: &mut Criterion) {
    let mut group = c.benchmark_group("ddsketch_full_workflow");
    group.throughput(Throughput::Elements(10000));

    group.bench_function("workflow_10k_values", |b| {
        b.iter(|| {
            let mut dd = DDSketch::new(0.01).unwrap();

            // Add 10k values
            for i in 1..=10000 {
                dd.add(i as f64);
            }

            // Query multiple quantiles
            let _p50 = dd.quantile(0.50).unwrap();
            let _p90 = dd.quantile(0.90).unwrap();
            let _p95 = dd.quantile(0.95).unwrap();
            let _p99 = dd.quantile(0.99).unwrap();
            let _p999 = dd.quantile(0.999).unwrap();

            black_box(dd);
        });
    });

    group.finish();
}

/// Benchmark with realistic latency distributions
fn bench_ddsketch_realistic_latencies(c: &mut Criterion) {
    let mut group = c.benchmark_group("ddsketch_realistic");

    // Simulate realistic latency distribution (mostly low, some high outliers)
    group.bench_function("latency_distribution", |b| {
        let mut dd = DDSketch::new(0.01).unwrap();
        let mut i = 0;

        b.iter(|| {
            // 90% low latency (1-100ms)
            // 9% medium latency (100-1000ms)
            // 1% high latency (1000-10000ms)
            let value = if i % 100 < 90 {
                ((i % 100) + 1) as f64
            } else if i % 100 < 99 {
                (100 + (i % 900)) as f64
            } else {
                (1000 + (i % 9000)) as f64
            };

            dd.add(black_box(value));
            i += 1;
        });
    });

    group.finish();
}

/// Benchmark min/max queries
fn bench_ddsketch_min_max(c: &mut Criterion) {
    let mut group = c.benchmark_group("ddsketch_min_max");

    let mut dd = DDSketch::new(0.01).unwrap();
    for i in 1..=10000 {
        dd.add(i as f64);
    }

    group.bench_function("min", |b| {
        b.iter(|| {
            black_box(dd.min());
        });
    });

    group.bench_function("max", |b| {
        b.iter(|| {
            black_box(dd.max());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_ddsketch_add,
    bench_ddsketch_add_ranges,
    bench_ddsketch_quantile,
    bench_ddsketch_quantile_types,
    bench_ddsketch_merge,
    bench_ddsketch_merge_distributions,
    bench_ddsketch_full_workflow,
    bench_ddsketch_realistic_latencies,
    bench_ddsketch_min_max
);
criterion_main!(benches);
