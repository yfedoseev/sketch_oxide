use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use sketch_oxide::common::hash::{murmur3_hash, xxhash};

fn bench_murmur3(c: &mut Criterion) {
    let mut group = c.benchmark_group("murmur3");

    for size in [8, 64, 512, 4096].iter() {
        let data: Vec<u8> = (0..*size).map(|i| i as u8).collect();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| murmur3_hash(black_box(&data), black_box(0)));
        });
    }

    group.finish();
}

fn bench_xxhash(c: &mut Criterion) {
    let mut group = c.benchmark_group("xxhash");

    for size in [8, 64, 512, 4096].iter() {
        let data: Vec<u8> = (0..*size).map(|i| i as u8).collect();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| xxhash(black_box(&data), black_box(0)));
        });
    }

    group.finish();
}

criterion_group!(benches, bench_murmur3, bench_xxhash);
criterion_main!(benches);
