//! Benchmarks for membership filter algorithms
//!
//! Compares CountingBloomFilter, CuckooFilter, StableBloomFilter

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sketch_oxide::membership::{BloomFilter, CountingBloomFilter, CuckooFilter, StableBloomFilter};

/// Generate test keys
fn generate_keys(count: usize) -> Vec<Vec<u8>> {
    (0..count)
        .map(|i| format!("key_{}", i).into_bytes())
        .collect()
}

/// Benchmark filter insertions
fn bench_insertions(c: &mut Criterion) {
    let mut group = c.benchmark_group("membership_insert");
    let sizes = [1000, 10_000];
    let keys_10k = generate_keys(10_000);

    for &size in &sizes {
        group.throughput(Throughput::Elements(size as u64));

        // BloomFilter (baseline)
        group.bench_with_input(BenchmarkId::new("BloomFilter", size), &size, |b, &n| {
            b.iter(|| {
                let mut filter = BloomFilter::new(n, 0.01);
                for key in keys_10k.iter().take(n) {
                    filter.insert(black_box(key));
                }
                filter
            })
        });

        // CountingBloomFilter
        group.bench_with_input(
            BenchmarkId::new("CountingBloomFilter", size),
            &size,
            |b, &n| {
                b.iter(|| {
                    let mut filter = CountingBloomFilter::new(n, 0.01);
                    for key in keys_10k.iter().take(n) {
                        filter.insert(black_box(key));
                    }
                    filter
                })
            },
        );

        // CuckooFilter
        group.bench_with_input(BenchmarkId::new("CuckooFilter", size), &size, |b, &n| {
            b.iter(|| {
                let mut filter = CuckooFilter::new(n).unwrap();
                for key in keys_10k.iter().take(n) {
                    let _ = filter.insert(black_box(key));
                }
                filter
            })
        });

        // StableBloomFilter
        group.bench_with_input(
            BenchmarkId::new("StableBloomFilter", size),
            &size,
            |b, &n| {
                b.iter(|| {
                    let mut filter = StableBloomFilter::new(n, 0.01).unwrap();
                    for key in keys_10k.iter().take(n) {
                        filter.insert(black_box(key));
                    }
                    filter
                })
            },
        );
    }

    group.finish();
}

/// Benchmark filter lookups
fn bench_lookups(c: &mut Criterion) {
    let mut group = c.benchmark_group("membership_lookup");
    let n = 10_000;
    let keys = generate_keys(n);
    let lookup_keys = generate_keys(1000);

    group.throughput(Throughput::Elements(1000));

    // BloomFilter
    {
        let mut filter = BloomFilter::new(n, 0.01);
        for key in &keys {
            filter.insert(key);
        }
        group.bench_function("BloomFilter", |b| {
            b.iter(|| {
                let mut found = 0;
                for key in &lookup_keys {
                    if filter.contains(black_box(key)) {
                        found += 1;
                    }
                }
                found
            })
        });
    }

    // CountingBloomFilter
    {
        let mut filter = CountingBloomFilter::new(n, 0.01);
        for key in &keys {
            filter.insert(key);
        }
        group.bench_function("CountingBloomFilter", |b| {
            b.iter(|| {
                let mut found = 0;
                for key in &lookup_keys {
                    if filter.contains(black_box(key)) {
                        found += 1;
                    }
                }
                found
            })
        });
    }

    // CuckooFilter
    {
        let mut filter = CuckooFilter::new(n).unwrap();
        for key in &keys {
            let _ = filter.insert(key);
        }
        group.bench_function("CuckooFilter", |b| {
            b.iter(|| {
                let mut found = 0;
                for key in &lookup_keys {
                    if filter.contains(black_box(key)) {
                        found += 1;
                    }
                }
                found
            })
        });
    }

    // StableBloomFilter
    {
        let mut filter = StableBloomFilter::new(n, 0.01).unwrap();
        for key in &keys {
            filter.insert(key);
        }
        group.bench_function("StableBloomFilter", |b| {
            b.iter(|| {
                let mut found = 0;
                for key in &lookup_keys {
                    if filter.contains(black_box(key)) {
                        found += 1;
                    }
                }
                found
            })
        });
    }

    group.finish();
}

/// Benchmark deletions for deletable filters
fn bench_deletions(c: &mut Criterion) {
    let mut group = c.benchmark_group("membership_delete");
    let n = 10_000;
    let keys = generate_keys(n);
    let delete_keys: Vec<_> = keys.iter().take(1000).cloned().collect();

    group.throughput(Throughput::Elements(1000));

    // CountingBloomFilter
    group.bench_function("CountingBloomFilter", |b| {
        b.iter_batched(
            || {
                let mut filter = CountingBloomFilter::new(n, 0.01);
                for key in &keys {
                    filter.insert(key);
                }
                filter
            },
            |mut filter| {
                for key in &delete_keys {
                    filter.remove(black_box(key));
                }
                filter
            },
            criterion::BatchSize::SmallInput,
        )
    });

    // CuckooFilter
    group.bench_function("CuckooFilter", |b| {
        b.iter_batched(
            || {
                let mut filter = CuckooFilter::new(n).unwrap();
                for key in &keys {
                    let _ = filter.insert(key);
                }
                filter
            },
            |mut filter| {
                for key in &delete_keys {
                    filter.remove(black_box(key));
                }
                filter
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

/// Benchmark memory usage
fn bench_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("membership_memory");
    let n = 10_000;
    let keys = generate_keys(n);

    group.bench_function("BloomFilter_memory", |b| {
        b.iter(|| {
            let mut filter = BloomFilter::new(n, 0.01);
            for key in &keys {
                filter.insert(key);
            }
            black_box(filter.memory_usage())
        })
    });

    group.bench_function("CountingBloomFilter_memory", |b| {
        b.iter(|| {
            let mut filter = CountingBloomFilter::new(n, 0.01);
            for key in &keys {
                filter.insert(key);
            }
            black_box(filter.memory_usage())
        })
    });

    group.bench_function("CuckooFilter_memory", |b| {
        b.iter(|| {
            let mut filter = CuckooFilter::new(n).unwrap();
            for key in &keys {
                let _ = filter.insert(key);
            }
            black_box(filter.memory_usage())
        })
    });

    group.bench_function("StableBloomFilter_memory", |b| {
        b.iter(|| {
            let mut filter = StableBloomFilter::new(n, 0.01).unwrap();
            for key in &keys {
                filter.insert(key);
            }
            black_box(filter.memory_usage())
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_insertions,
    bench_lookups,
    bench_deletions,
    bench_memory
);
criterion_main!(benches);
