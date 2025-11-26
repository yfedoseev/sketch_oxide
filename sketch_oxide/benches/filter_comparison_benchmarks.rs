use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sketch_oxide::membership::{BlockedBloomFilter, BloomFilter, RibbonFilter};

/// Benchmark filter construction time
fn bench_filter_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter_construction");

    for n in [1_000, 10_000, 100_000].iter() {
        let fpr = 0.01;

        group.bench_with_input(BenchmarkId::new("bloom", n), n, |b, &n| {
            b.iter(|| BloomFilter::new(black_box(n), black_box(fpr)));
        });

        group.bench_with_input(BenchmarkId::new("blocked_bloom", n), n, |b, &n| {
            b.iter(|| BlockedBloomFilter::new(black_box(n), black_box(fpr)));
        });

        group.bench_with_input(BenchmarkId::new("ribbon", n), n, |b, &n| {
            b.iter(|| RibbonFilter::new(black_box(n), black_box(fpr)));
        });
    }

    group.finish();
}

/// Benchmark filter query performance (contains - hit case)
fn bench_filter_contains_hit(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter_contains_hit");

    for n in [1_000, 10_000, 100_000].iter() {
        let fpr = 0.01;
        let keys: Vec<Vec<u8>> = (0..*n).map(|i| format!("key{}", i).into_bytes()).collect();

        // Bloom Filter
        let mut bloom = BloomFilter::new(*n, fpr);
        for key in &keys {
            bloom.insert(key);
        }

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::new("bloom", n), &keys, |b, keys| {
            let mut idx = 0;
            b.iter(|| {
                let result = bloom.contains(black_box(&keys[idx % keys.len()]));
                idx += 1;
                black_box(result)
            });
        });

        // Blocked Bloom Filter
        let mut blocked_bloom = BlockedBloomFilter::new(*n, fpr);
        for key in &keys {
            blocked_bloom.insert(key);
        }

        group.bench_with_input(BenchmarkId::new("blocked_bloom", n), &keys, |b, keys| {
            let mut idx = 0;
            b.iter(|| {
                let result = blocked_bloom.contains(black_box(&keys[idx % keys.len()]));
                idx += 1;
                black_box(result)
            });
        });

        // Ribbon Filter
        let mut ribbon = RibbonFilter::new(*n, fpr);
        for key in &keys {
            ribbon.insert(key);
        }
        ribbon.finalize();

        group.bench_with_input(BenchmarkId::new("ribbon", n), &keys, |b, keys| {
            let mut idx = 0;
            b.iter(|| {
                let result = ribbon.contains(black_box(&keys[idx % keys.len()]));
                idx += 1;
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark filter query performance (contains - miss case)
fn bench_filter_contains_miss(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter_contains_miss");

    for n in [1_000, 10_000, 100_000].iter() {
        let fpr = 0.01;
        let keys: Vec<Vec<u8>> = (0..*n).map(|i| format!("key{}", i).into_bytes()).collect();

        // Create miss keys (different from inserted keys)
        let miss_keys: Vec<Vec<u8>> = (0..*n).map(|i| format!("miss{}", i).into_bytes()).collect();

        // Bloom Filter
        let mut bloom = BloomFilter::new(*n, fpr);
        for key in &keys {
            bloom.insert(key);
        }

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::new("bloom", n), &miss_keys, |b, miss_keys| {
            let mut idx = 0;
            b.iter(|| {
                let result = bloom.contains(black_box(&miss_keys[idx % miss_keys.len()]));
                idx += 1;
                black_box(result)
            });
        });

        // Blocked Bloom Filter
        let mut blocked_bloom = BlockedBloomFilter::new(*n, fpr);
        for key in &keys {
            blocked_bloom.insert(key);
        }

        group.bench_with_input(
            BenchmarkId::new("blocked_bloom", n),
            &miss_keys,
            |b, miss_keys| {
                let mut idx = 0;
                b.iter(|| {
                    let result =
                        blocked_bloom.contains(black_box(&miss_keys[idx % miss_keys.len()]));
                    idx += 1;
                    black_box(result)
                });
            },
        );

        // Ribbon Filter
        let mut ribbon = RibbonFilter::new(*n, fpr);
        for key in &keys {
            ribbon.insert(key);
        }
        ribbon.finalize();

        group.bench_with_input(BenchmarkId::new("ribbon", n), &miss_keys, |b, miss_keys| {
            let mut idx = 0;
            b.iter(|| {
                let result = ribbon.contains(black_box(&miss_keys[idx % miss_keys.len()]));
                idx += 1;
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark filter memory usage calculation
fn bench_filter_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter_memory_usage");

    for n in [1_000, 10_000, 100_000].iter() {
        let fpr = 0.01;
        let keys: Vec<Vec<u8>> = (0..*n).map(|i| format!("key{}", i).into_bytes()).collect();

        // Bloom Filter
        let mut bloom = BloomFilter::new(*n, fpr);
        for key in &keys {
            bloom.insert(key);
        }

        group.bench_with_input(BenchmarkId::new("bloom", n), &bloom, |b, filter| {
            b.iter(|| {
                let memory = filter.memory_usage();
                black_box(memory)
            });
        });

        // Blocked Bloom Filter
        let mut blocked_bloom = BlockedBloomFilter::new(*n, fpr);
        for key in &keys {
            blocked_bloom.insert(key);
        }

        group.bench_with_input(
            BenchmarkId::new("blocked_bloom", n),
            &blocked_bloom,
            |b, filter| {
                b.iter(|| {
                    let memory = filter.memory_usage();
                    black_box(memory)
                });
            },
        );

        // Ribbon Filter
        let mut ribbon = RibbonFilter::new(*n, fpr);
        for key in &keys {
            ribbon.insert(key);
        }
        ribbon.finalize();

        group.bench_with_input(BenchmarkId::new("ribbon", n), &ribbon, |b, filter| {
            b.iter(|| {
                let memory = filter.memory_usage();
                black_box(memory)
            });
        });
    }

    group.finish();
}

/// Benchmark filter serialization
fn bench_filter_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter_serialization");

    let n = 10_000;
    let fpr = 0.01;
    let keys: Vec<Vec<u8>> = (0..n).map(|i| format!("key{}", i).into_bytes()).collect();

    // Bloom Filter
    let mut bloom = BloomFilter::new(n, fpr);
    for key in &keys {
        bloom.insert(key);
    }

    group.bench_function("bloom_to_bytes", |b| {
        b.iter(|| {
            let bytes = bloom.to_bytes();
            black_box(bytes)
        });
    });

    let bloom_bytes = bloom.to_bytes();
    group.bench_function("bloom_from_bytes", |b| {
        b.iter(|| {
            let filter = BloomFilter::from_bytes(black_box(&bloom_bytes)).unwrap();
            black_box(filter)
        });
    });

    // Blocked Bloom Filter
    let mut blocked_bloom = BlockedBloomFilter::new(n, fpr);
    for key in &keys {
        blocked_bloom.insert(key);
    }

    group.bench_function("blocked_bloom_to_bytes", |b| {
        b.iter(|| {
            let bytes = blocked_bloom.to_bytes();
            black_box(bytes)
        });
    });

    let blocked_bloom_bytes = blocked_bloom.to_bytes();
    group.bench_function("blocked_bloom_from_bytes", |b| {
        b.iter(|| {
            let filter = BlockedBloomFilter::from_bytes(black_box(&blocked_bloom_bytes)).unwrap();
            black_box(filter)
        });
    });

    // Ribbon Filter
    let mut ribbon = RibbonFilter::new(n, fpr);
    for key in &keys {
        ribbon.insert(key);
    }
    ribbon.finalize();

    group.bench_function("ribbon_to_bytes", |b| {
        b.iter(|| {
            let bytes = ribbon.to_bytes();
            black_box(bytes)
        });
    });

    let ribbon_bytes = ribbon.to_bytes();
    group.bench_function("ribbon_from_bytes", |b| {
        b.iter(|| {
            let filter = RibbonFilter::from_bytes(black_box(&ribbon_bytes)).unwrap();
            black_box(filter)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_filter_construction,
    bench_filter_contains_hit,
    bench_filter_contains_miss,
    bench_filter_memory_usage,
    bench_filter_serialization
);
criterion_main!(benches);
