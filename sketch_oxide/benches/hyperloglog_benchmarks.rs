use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use sketch_oxide::cardinality::HyperLogLog;
use sketch_oxide::{Mergeable, Sketch};

/// Benchmark: Update operations
fn bench_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("hyperloglog_update");

    for precision in [10, 12, 14] {
        group.bench_with_input(
            BenchmarkId::new("update", precision),
            &precision,
            |b, &p| {
                let mut hll = HyperLogLog::new(p).unwrap();
                let mut counter = 0u64;
                b.iter(|| {
                    hll.update(black_box(&counter));
                    counter += 1;
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Estimate operations
fn bench_estimate(c: &mut Criterion) {
    let mut group = c.benchmark_group("hyperloglog_estimate");

    for (n, precision) in [(1000, 12), (10000, 12), (100000, 14)] {
        let mut hll = HyperLogLog::new(precision).unwrap();
        for i in 0..n {
            hll.update(&i);
        }

        group.bench_with_input(
            BenchmarkId::new("estimate", format!("n={}_p={}", n, precision)),
            &(),
            |b, _| {
                b.iter(|| black_box(hll.estimate()));
            },
        );
    }

    group.finish();
}

/// Benchmark: Merge operations
fn bench_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("hyperloglog_merge");

    group.bench_function("merge_two_hlls", |b| {
        b.iter_batched(
            || {
                let mut hll1 = HyperLogLog::new(12).unwrap();
                let mut hll2 = HyperLogLog::new(12).unwrap();
                for i in 0..5000 {
                    hll1.update(&i);
                    hll2.update(&(i + 5000));
                }
                (hll1, hll2)
            },
            |(mut hll1, hll2)| {
                hll1.merge(&hll2).unwrap();
                black_box(hll1);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark: Serialization
fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("hyperloglog_serialization");

    let mut hll = HyperLogLog::new(12).unwrap();
    for i in 0..10000 {
        hll.update(&i);
    }

    group.bench_function("to_bytes", |b| {
        b.iter(|| black_box(hll.to_bytes()));
    });

    let bytes = hll.to_bytes();
    group.bench_function("from_bytes", |b| {
        b.iter(|| black_box(HyperLogLog::from_bytes(&bytes).unwrap()));
    });

    group.finish();
}

/// Benchmark: Full pipeline
fn bench_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("hyperloglog_pipeline");

    for n in [1000, 10000, 100000] {
        group.bench_with_input(BenchmarkId::new("full_pipeline", n), &n, |b, &n| {
            b.iter(|| {
                let mut hll = HyperLogLog::new(12).unwrap();
                for i in 0..n {
                    hll.update(&i);
                }
                black_box(hll.estimate())
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_update,
    bench_estimate,
    bench_merge,
    bench_serialization,
    bench_full_pipeline,
);

criterion_main!(benches);
