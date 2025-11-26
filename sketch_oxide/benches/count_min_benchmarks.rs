//! Benchmarks for Count-Min Sketch
//!
//! Performance targets:
//! - Update: <200ns (k hash operations)
//! - Estimate: <100ns (k lookups + min)
//! - Merge: <1ms

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sketch_oxide::frequency::CountMinSketch;
use sketch_oxide::Mergeable;

fn bench_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("count_min_construction");

    for &(epsilon, delta) in &[(0.1, 0.1), (0.01, 0.01), (0.001, 0.001)] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("ε={},δ={}", epsilon, delta)),
            &(epsilon, delta),
            |b, &(eps, dlt)| {
                b.iter(|| {
                    let cms = CountMinSketch::new(eps, dlt).unwrap();
                    black_box(cms);
                });
            },
        );
    }

    group.finish();
}

fn bench_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("count_min_update");
    group.throughput(Throughput::Elements(1));

    // Benchmark with different accuracy parameters
    for &(epsilon, delta) in &[(0.1, 0.1), (0.01, 0.01), (0.001, 0.001)] {
        let mut cms = CountMinSketch::new(epsilon, delta).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("ε={},δ={}", epsilon, delta)),
            &epsilon,
            |b, _| {
                let mut counter = 0u64;
                b.iter(|| {
                    cms.update(&counter);
                    counter = counter.wrapping_add(1);
                });
            },
        );
    }

    group.finish();
}

fn bench_update_different_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("count_min_update_types");
    group.throughput(Throughput::Elements(1));

    let mut cms = CountMinSketch::new(0.01, 0.01).unwrap();

    group.bench_function("u64", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            cms.update(&counter);
            counter = counter.wrapping_add(1);
        });
    });

    group.bench_function("string", |b| {
        let strings: Vec<String> = (0..1000).map(|i| format!("item_{}", i)).collect();
        let mut idx = 0;
        b.iter(|| {
            cms.update(&strings[idx % strings.len()]);
            idx = idx.wrapping_add(1);
        });
    });

    group.bench_function("str_ref", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            cms.update(&"static_string");
            counter = counter.wrapping_add(1);
        });
    });

    group.finish();
}

fn bench_estimate(c: &mut Criterion) {
    let mut group = c.benchmark_group("count_min_estimate");
    group.throughput(Throughput::Elements(1));

    // Benchmark with different accuracy parameters
    for &(epsilon, delta) in &[(0.1, 0.1), (0.01, 0.01), (0.001, 0.001)] {
        let mut cms = CountMinSketch::new(epsilon, delta).unwrap();

        // Pre-populate with some data
        for i in 0..1000 {
            cms.update(&i);
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("ε={},δ={}", epsilon, delta)),
            &epsilon,
            |b, _| {
                let mut counter = 0u64;
                b.iter(|| {
                    let estimate = cms.estimate(&counter);
                    counter = (counter + 1) % 1000;
                    black_box(estimate);
                });
            },
        );
    }

    group.finish();
}

fn bench_update_and_estimate(c: &mut Criterion) {
    let mut group = c.benchmark_group("count_min_update_and_estimate");
    group.throughput(Throughput::Elements(2)); // One update + one estimate

    let mut cms = CountMinSketch::new(0.01, 0.01).unwrap();

    group.bench_function("interleaved", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            cms.update(&counter);
            let estimate = cms.estimate(&counter);
            counter = counter.wrapping_add(1);
            black_box(estimate);
        });
    });

    group.finish();
}

fn bench_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("count_min_merge");

    // Benchmark merge with different sketch sizes
    for &(epsilon, delta) in &[(0.1, 0.1), (0.01, 0.01), (0.001, 0.001)] {
        let mut cms1 = CountMinSketch::new(epsilon, delta).unwrap();
        let mut cms2 = CountMinSketch::new(epsilon, delta).unwrap();

        // Populate both sketches
        for i in 0..1000 {
            cms1.update(&i);
            cms2.update(&(i + 1000));
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("ε={},δ={}", epsilon, delta)),
            &epsilon,
            |b, _| {
                b.iter(|| {
                    let mut cms1_copy = cms1.clone();
                    cms1_copy.merge(&cms2).unwrap();
                    black_box(cms1_copy);
                });
            },
        );
    }

    group.finish();
}

fn bench_heavy_hitter_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("count_min_heavy_hitter");
    group.throughput(Throughput::Elements(100));

    let mut cms = CountMinSketch::new(0.01, 0.01).unwrap();

    group.bench_function("80_20_distribution", |b| {
        b.iter(|| {
            // 80% of traffic goes to 20% of items (Pareto principle)
            for i in 0..100 {
                let item = if i < 80 {
                    i % 20 // Heavy hitters (20 items get 80% of traffic)
                } else {
                    20 + i // Long tail (80 items get 20% of traffic)
                };
                cms.update(&item);
            }
        });
    });

    group.finish();
}

fn bench_realistic_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("count_min_realistic");
    group.throughput(Throughput::Elements(1000));

    let mut cms = CountMinSketch::new(0.01, 0.01).unwrap();

    group.bench_function("mixed_updates_queries", |b| {
        b.iter(|| {
            // Realistic pattern: 90% updates, 10% queries
            for i in 0..1000 {
                if i % 10 == 0 {
                    // 10% queries
                    let estimate = cms.estimate(&(i / 10));
                    black_box(estimate);
                } else {
                    // 90% updates
                    cms.update(&i);
                }
            }
        });
    });

    group.finish();
}

fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("count_min_serialization");

    for &(epsilon, delta) in &[(0.1, 0.1), (0.01, 0.01), (0.001, 0.001)] {
        let mut cms = CountMinSketch::new(epsilon, delta).unwrap();

        // Populate sketch
        for i in 0..1000 {
            cms.update(&i);
        }

        group.bench_with_input(
            BenchmarkId::new("serialize", format!("ε={},δ={}", epsilon, delta)),
            &epsilon,
            |b, _| {
                b.iter(|| {
                    use sketch_oxide::Sketch;
                    let bytes = cms.serialize();
                    black_box(bytes);
                });
            },
        );

        // Benchmark deserialization
        use sketch_oxide::Sketch;
        let serialized = cms.serialize();

        group.bench_with_input(
            BenchmarkId::new("deserialize", format!("ε={},δ={}", epsilon, delta)),
            &epsilon,
            |b, _| {
                b.iter(|| {
                    let cms2 = CountMinSketch::deserialize(&serialized).unwrap();
                    black_box(cms2);
                });
            },
        );
    }

    group.finish();
}

fn bench_memory_footprint(c: &mut Criterion) {
    let mut group = c.benchmark_group("count_min_memory");

    for &(epsilon, delta) in &[(0.1, 0.1), (0.01, 0.01), (0.001, 0.001)] {
        let cms = CountMinSketch::new(epsilon, delta).unwrap();

        let width = cms.width();
        let depth = cms.depth();
        let bytes = width * depth * 8; // 8 bytes per u64 counter

        group.bench_with_input(
            BenchmarkId::from_parameter(format!(
                "ε={},δ={} ({}x{}, {} KB)",
                epsilon,
                delta,
                width,
                depth,
                bytes / 1024
            )),
            &epsilon,
            |b, _| {
                b.iter(|| {
                    black_box(&cms);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_construction,
    bench_update,
    bench_update_different_types,
    bench_estimate,
    bench_update_and_estimate,
    bench_merge,
    bench_heavy_hitter_workload,
    bench_realistic_workload,
    bench_serialization,
    bench_memory_footprint,
);
criterion_main!(benches);
