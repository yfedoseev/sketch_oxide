use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sketch_oxide::quantiles::req::{ReqMode, ReqSketch};

fn bench_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("req_update");

    for k in [32, 128, 256] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::new("hra", k), &k, |b, &k| {
            let mut sketch = ReqSketch::new(k, ReqMode::HighRankAccuracy).unwrap();
            let mut counter = 0.0;
            b.iter(|| {
                counter += 1.0;
                sketch.update(black_box(counter));
            });
        });

        group.bench_with_input(BenchmarkId::new("lra", k), &k, |b, &k| {
            let mut sketch = ReqSketch::new(k, ReqMode::LowRankAccuracy).unwrap();
            let mut counter = 0.0;
            b.iter(|| {
                counter += 1.0;
                sketch.update(black_box(counter));
            });
        });
    }
    group.finish();
}

fn bench_quantile(c: &mut Criterion) {
    let mut group = c.benchmark_group("req_quantile");

    for n in [1000, 10000, 100000] {
        let mut sketch = ReqSketch::new(128, ReqMode::HighRankAccuracy).unwrap();
        for i in 1..=n {
            sketch.update(i as f64);
        }

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::new("p50", n), &sketch, |b, sketch| {
            b.iter(|| sketch.quantile(black_box(0.5)));
        });

        group.bench_with_input(BenchmarkId::new("p99", n), &sketch, |b, sketch| {
            b.iter(|| sketch.quantile(black_box(0.99)));
        });

        group.bench_with_input(BenchmarkId::new("p100", n), &sketch, |b, sketch| {
            b.iter(|| sketch.quantile(black_box(1.0)));
        });
    }
    group.finish();
}

fn bench_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("req_merge");

    for n in [1000, 10000] {
        let mut sketch1 = ReqSketch::new(128, ReqMode::HighRankAccuracy).unwrap();
        let mut sketch2 = ReqSketch::new(128, ReqMode::HighRankAccuracy).unwrap();

        for i in 1..=n {
            sketch1.update(i as f64);
            sketch2.update((i + n) as f64);
        }

        group.throughput(Throughput::Elements(n as u64 * 2));
        group.bench_with_input(
            BenchmarkId::from_parameter(n),
            &(&sketch1, &sketch2),
            |b, (s1, s2)| {
                b.iter(|| {
                    black_box(s1.merge(s2).unwrap());
                });
            },
        );
    }
    group.finish();
}

fn bench_end_to_end(c: &mut Criterion) {
    let mut group = c.benchmark_group("req_end_to_end");

    for n in [1000, 10000, 100000] {
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                let mut sketch = ReqSketch::new(128, ReqMode::HighRankAccuracy).unwrap();
                for i in 1..=n {
                    sketch.update(i as f64);
                }
                let _p50 = sketch.quantile(0.5);
                let _p99 = sketch.quantile(0.99);
                let _p100 = sketch.quantile(1.0);
                black_box(sketch);
            });
        });
    }
    group.finish();
}

fn bench_compaction_heavy(c: &mut Criterion) {
    let mut group = c.benchmark_group("req_compaction_heavy");
    group.sample_size(10);

    // Small k forces many compactions
    for k in [8, 16, 32] {
        group.throughput(Throughput::Elements(100000));
        group.bench_with_input(BenchmarkId::from_parameter(k), &k, |b, &k| {
            b.iter(|| {
                let mut sketch = ReqSketch::new(k, ReqMode::HighRankAccuracy).unwrap();
                for i in 1..=100000 {
                    sketch.update(i as f64);
                }
                black_box(sketch);
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_update,
    bench_quantile,
    bench_merge,
    bench_end_to_end,
    bench_compaction_heavy
);
criterion_main!(benches);
