use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sketch_oxide::membership::BloomFilter;

fn bench_bloom_new(c: &mut Criterion) {
    let mut group = c.benchmark_group("bloom_new");

    for n in [1_000, 10_000, 100_000, 1_000_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(n), n, |b, &n| {
            b.iter(|| BloomFilter::new(black_box(n), black_box(0.01)));
        });
    }

    group.finish();
}

fn bench_bloom_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("bloom_insert");

    for n in [1_000, 10_000, 100_000].iter() {
        let mut filter = BloomFilter::new(*n, 0.01);
        let keys: Vec<Vec<u8>> = (0..*n).map(|i| format!("key{}", i).into_bytes()).collect();

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(n), &keys, |b, keys| {
            let mut idx = 0;
            b.iter(|| {
                filter.insert(black_box(&keys[idx % keys.len()]));
                idx += 1;
            });
        });
    }

    group.finish();
}

fn bench_bloom_contains_hit(c: &mut Criterion) {
    let mut group = c.benchmark_group("bloom_contains_hit");

    for n in [1_000, 10_000, 100_000].iter() {
        let mut filter = BloomFilter::new(*n, 0.01);
        let keys: Vec<Vec<u8>> = (0..*n).map(|i| format!("key{}", i).into_bytes()).collect();

        // Insert all keys
        for key in &keys {
            filter.insert(key);
        }

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(n), &keys, |b, keys| {
            let mut idx = 0;
            b.iter(|| {
                let result = filter.contains(black_box(&keys[idx % keys.len()]));
                idx += 1;
                black_box(result)
            });
        });
    }

    group.finish();
}

fn bench_bloom_contains_miss(c: &mut Criterion) {
    let mut group = c.benchmark_group("bloom_contains_miss");

    for n in [1_000, 10_000, 100_000].iter() {
        let mut filter = BloomFilter::new(*n, 0.01);
        let keys: Vec<Vec<u8>> = (0..*n).map(|i| format!("key{}", i).into_bytes()).collect();

        // Insert all keys
        for key in &keys {
            filter.insert(key);
        }

        // Create non-existent keys
        let miss_keys: Vec<Vec<u8>> = (0..*n).map(|i| format!("miss{}", i).into_bytes()).collect();

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::from_parameter(n),
            &miss_keys,
            |b, miss_keys| {
                let mut idx = 0;
                b.iter(|| {
                    let result = filter.contains(black_box(&miss_keys[idx % miss_keys.len()]));
                    idx += 1;
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

fn bench_bloom_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("bloom_serialization");

    for n in [1_000, 10_000, 100_000].iter() {
        let mut filter = BloomFilter::new(*n, 0.01);
        let keys: Vec<Vec<u8>> = (0..*n).map(|i| format!("key{}", i).into_bytes()).collect();

        // Insert all keys
        for key in &keys {
            filter.insert(key);
        }

        group.bench_with_input(BenchmarkId::new("to_bytes", n), n, |b, _| {
            b.iter(|| {
                let bytes = filter.to_bytes();
                black_box(bytes)
            });
        });

        let bytes = filter.to_bytes();
        group.bench_with_input(BenchmarkId::new("from_bytes", n), &bytes, |b, bytes| {
            b.iter(|| {
                let deserialized = BloomFilter::from_bytes(black_box(bytes)).unwrap();
                black_box(deserialized)
            });
        });
    }

    group.finish();
}

fn bench_bloom_false_positive_rate(c: &mut Criterion) {
    let mut group = c.benchmark_group("bloom_fpr");

    for fpr in [0.001, 0.01, 0.05].iter() {
        let n = 10_000;
        let mut filter = BloomFilter::new(n, *fpr);
        let keys: Vec<Vec<u8>> = (0..n).map(|i| format!("key{}", i).into_bytes()).collect();

        // Insert all keys
        for key in &keys {
            filter.insert(key);
        }

        let label = format!("{:.3}", fpr);
        group.bench_with_input(BenchmarkId::from_parameter(&label), &filter, |b, filter| {
            b.iter(|| {
                let rate = filter.false_positive_rate();
                black_box(rate)
            });
        });
    }

    group.finish();
}

fn bench_bloom_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("bloom_memory");

    for n in [1_000, 10_000, 100_000, 1_000_000].iter() {
        let filter = BloomFilter::new(*n, 0.01);

        group.bench_with_input(BenchmarkId::from_parameter(n), &filter, |b, filter| {
            b.iter(|| {
                let memory = filter.memory_usage();
                black_box(memory)
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_bloom_new,
    bench_bloom_insert,
    bench_bloom_contains_hit,
    bench_bloom_contains_miss,
    bench_bloom_serialization,
    bench_bloom_false_positive_rate,
    bench_bloom_memory_usage
);
criterion_main!(benches);
