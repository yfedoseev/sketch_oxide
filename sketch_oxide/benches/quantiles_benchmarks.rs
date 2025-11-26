use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use sketch_oxide::quantiles::{KllSketch, TDigest};

/// Benchmark: T-Digest update operations
fn bench_tdigest_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("tdigest_update");

    for compression in [50.0, 100.0, 200.0] {
        group.bench_with_input(
            BenchmarkId::new("update", compression as u64),
            &compression,
            |b, &c| {
                let mut td = TDigest::new(c);
                let mut counter = 0.0f64;
                b.iter(|| {
                    td.update(black_box(counter));
                    counter += 1.0;
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: T-Digest quantile operations
fn bench_tdigest_quantile(c: &mut Criterion) {
    let mut group = c.benchmark_group("tdigest_quantile");

    for n in [1000, 10000, 100000] {
        let mut td = TDigest::new(100.0);
        for i in 0..n {
            td.update(i as f64);
        }

        group.bench_with_input(BenchmarkId::new("quantile", n), &(), |b, _| {
            b.iter(|| {
                black_box(td.quantile(0.5));
                black_box(td.quantile(0.99));
                black_box(td.quantile(0.999));
            });
        });
    }

    group.finish();
}

/// Benchmark: KLL update operations
fn bench_kll_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("kll_update");

    for k in [100, 200, 500] {
        group.bench_with_input(BenchmarkId::new("update", k), &k, |b, &k| {
            let mut kll = KllSketch::new(k).unwrap();
            let mut counter = 0.0f64;
            b.iter(|| {
                kll.update(black_box(counter));
                counter += 1.0;
            });
        });
    }

    group.finish();
}

/// Benchmark: KLL quantile operations
fn bench_kll_quantile(c: &mut Criterion) {
    let mut group = c.benchmark_group("kll_quantile");

    for n in [1000, 10000, 100000] {
        let mut kll = KllSketch::new(200).unwrap();
        for i in 0..n {
            kll.update(i as f64);
        }

        group.bench_with_input(BenchmarkId::new("quantile", n), &(), |b, _| {
            b.iter(|| {
                black_box(kll.quantile(0.5));
                black_box(kll.quantile(0.99));
            });
        });
    }

    group.finish();
}

/// Benchmark: Comparison between T-Digest and KLL
fn bench_quantile_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("quantile_comparison");
    let n = 10000;

    group.bench_function("tdigest_pipeline", |b| {
        b.iter(|| {
            let mut td = TDigest::new(100.0);
            for i in 0..n {
                td.update(i as f64);
            }
            black_box(td.quantile(0.99))
        });
    });

    group.bench_function("kll_pipeline", |b| {
        b.iter(|| {
            let mut kll = KllSketch::new(200).unwrap();
            for i in 0..n {
                kll.update(i as f64);
            }
            black_box(kll.quantile(0.99))
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_tdigest_update,
    bench_tdigest_quantile,
    bench_kll_update,
    bench_kll_quantile,
    bench_quantile_comparison,
);

criterion_main!(benches);
