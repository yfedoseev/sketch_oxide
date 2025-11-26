use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use sketch_oxide::cardinality::CpcSketch;
use sketch_oxide::common::{Mergeable, Sketch};

fn bench_cpc_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("cpc_update");

    for lg_k in [10, 11, 12] {
        group.bench_with_input(BenchmarkId::new("update", lg_k), &lg_k, |b, &lg_k| {
            let mut cpc = CpcSketch::new(lg_k).unwrap();
            let mut counter = 0u64;
            b.iter(|| {
                cpc.update(black_box(&counter));
                counter += 1;
            });
        });
    }

    group.finish();
}

fn bench_cpc_estimate(c: &mut Criterion) {
    let mut group = c.benchmark_group("cpc_estimate");

    for size in [100, 1000, 10000] {
        group.bench_with_input(BenchmarkId::new("estimate", size), &size, |b, &size| {
            let mut cpc = CpcSketch::new(11).unwrap();
            for i in 0..size {
                cpc.update(&i);
            }
            b.iter(|| {
                black_box(cpc.estimate());
            });
        });
    }

    group.finish();
}

fn bench_cpc_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("cpc_merge");

    for size in [100, 1000] {
        group.bench_with_input(BenchmarkId::new("merge", size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let mut cpc1 = CpcSketch::new(11).unwrap();
                    let mut cpc2 = CpcSketch::new(11).unwrap();
                    for i in 0..size {
                        cpc1.update(&i);
                        cpc2.update(&(i + size));
                    }
                    (cpc1, cpc2)
                },
                |(mut cpc1, cpc2)| {
                    cpc1.merge(&cpc2).unwrap();
                    black_box(cpc1);
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn bench_cpc_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("cpc_serialize");

    for size in [100, 1000, 10000] {
        group.bench_with_input(BenchmarkId::new("serialize", size), &size, |b, &size| {
            let mut cpc = CpcSketch::new(11).unwrap();
            for i in 0..size {
                cpc.update(&i);
            }
            b.iter(|| {
                black_box(cpc.serialize());
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_cpc_update,
    bench_cpc_estimate,
    bench_cpc_merge,
    bench_cpc_serialize
);
criterion_main!(benches);
