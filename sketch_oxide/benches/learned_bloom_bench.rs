//! Learned Bloom Filter Benchmarks
//!
//! Benchmarking ML-enhanced Bloom filter vs standard Bloom filter
//! across various scenarios:
//!
//! 1. Training time (1K to 100K keys)
//! 2. Query latency (model + backup)
//! 3. Memory comparison
//! 4. Varying FPR targets
//! 5. Different data distributions

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use sketch_oxide::membership::{BloomFilter, LearnedBloomFilter};

/// Benchmark training time for different dataset sizes
fn bench_training_time(c: &mut Criterion) {
    let mut group = c.benchmark_group("learned_bloom/training");

    for n in [1_000, 10_000, 50_000, 100_000] {
        group.throughput(Throughput::Elements(n as u64));
        group.bench_function(format!("train_{}_keys", n), |b| {
            b.iter_batched(
                || {
                    (0..n)
                        .map(|i| format!("key{:08}", i).into_bytes())
                        .collect::<Vec<Vec<u8>>>()
                },
                |training_keys| black_box(LearnedBloomFilter::new(&training_keys, 0.01).unwrap()),
                BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

/// Benchmark query latency
fn bench_query_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("learned_bloom/query");

    // Setup filter
    let training_keys: Vec<Vec<u8>> = (0..10_000)
        .map(|i| format!("key{:08}", i).into_bytes())
        .collect();

    let learned_filter = LearnedBloomFilter::new(&training_keys, 0.01).unwrap();
    let standard_filter = {
        let mut f = BloomFilter::new(10_000, 0.01);
        for key in &training_keys {
            f.insert(key);
        }
        f
    };

    // Benchmark: Query for present keys (hits)
    group.bench_function("learned_filter_hit", |b| {
        b.iter(|| {
            let key = b"key00005000";
            black_box(learned_filter.contains(key))
        })
    });

    group.bench_function("standard_filter_hit", |b| {
        b.iter(|| {
            let key = b"key00005000";
            black_box(standard_filter.contains(key))
        })
    });

    // Benchmark: Query for absent keys (misses)
    group.bench_function("learned_filter_miss", |b| {
        b.iter(|| {
            let key = b"absent_key_123456";
            black_box(learned_filter.contains(key))
        })
    });

    group.bench_function("standard_filter_miss", |b| {
        b.iter(|| {
            let key = b"absent_key_123456";
            black_box(standard_filter.contains(key))
        })
    });

    group.finish();
}

/// Benchmark memory efficiency
fn bench_memory_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("learned_bloom/memory");

    for n in [5_000, 10_000, 50_000] {
        group.bench_function(format!("memory_learned_{}_keys", n), |b| {
            b.iter_batched(
                || {
                    let training_keys: Vec<Vec<u8>> = (0..n)
                        .map(|i| format!("key{:08}", i).into_bytes())
                        .collect();
                    LearnedBloomFilter::new(&training_keys, 0.01).unwrap()
                },
                |filter| {
                    let mem = filter.memory_usage();
                    black_box(mem);
                    println!("Learned filter ({} keys): {} bytes", n, mem);
                    mem
                },
                BatchSize::SmallInput,
            )
        });

        group.bench_function(format!("memory_standard_{}_keys", n), |b| {
            b.iter(|| {
                let filter = BloomFilter::new(n, 0.01);
                let mem = filter.memory_usage();
                black_box(mem);
                println!("Standard filter ({} keys): {} bytes", n, mem);
                mem
            })
        });
    }

    group.finish();
}

/// Benchmark varying FPR targets
fn bench_varying_fpr(c: &mut Criterion) {
    let mut group = c.benchmark_group("learned_bloom/fpr_targets");

    let training_keys: Vec<Vec<u8>> = (0..10_000)
        .map(|i| format!("key{:08}", i).into_bytes())
        .collect();

    for fpr in [0.001, 0.01, 0.1] {
        group.bench_function(format!("learned_fpr_{:.3}", fpr), |b| {
            b.iter(|| black_box(LearnedBloomFilter::new(&training_keys, fpr).unwrap()))
        });
    }

    group.finish();
}

/// Benchmark different data distributions
fn bench_data_distributions(c: &mut Criterion) {
    let mut group = c.benchmark_group("learned_bloom/distributions");

    let n = 10_000;

    // Uniform distribution
    group.bench_function("uniform_distribution", |b| {
        b.iter_batched(
            || {
                (0..n)
                    .map(|i| {
                        let mut key = vec![0u8; 8];
                        key[..8].copy_from_slice(&(i as u64).to_le_bytes());
                        key
                    })
                    .collect::<Vec<Vec<u8>>>()
            },
            |training_keys| black_box(LearnedBloomFilter::new(&training_keys, 0.01).unwrap()),
            BatchSize::SmallInput,
        )
    });

    // Sequential strings
    group.bench_function("sequential_strings", |b| {
        b.iter_batched(
            || {
                (0..n)
                    .map(|i| format!("{:08}", i).into_bytes())
                    .collect::<Vec<Vec<u8>>>()
            },
            |training_keys| black_box(LearnedBloomFilter::new(&training_keys, 0.01).unwrap()),
            BatchSize::SmallInput,
        )
    });

    // Skewed distribution (Zipf-like)
    group.bench_function("skewed_distribution", |b| {
        b.iter_batched(
            || {
                let mut keys = Vec::new();
                // Hot keys (repeated)
                for _ in 0..n / 2 {
                    keys.push(b"hot_key_1".to_vec());
                    keys.push(b"hot_key_2".to_vec());
                }
                // Cold keys (unique)
                for i in 0..n / 2 {
                    keys.push(format!("cold_{}", i).into_bytes());
                }
                keys
            },
            |training_keys| black_box(LearnedBloomFilter::new(&training_keys, 0.01).unwrap()),
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

/// Benchmark batch queries
fn bench_batch_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("learned_bloom/batch");

    let n = 10_000;
    let training_keys: Vec<Vec<u8>> = (0..n)
        .map(|i| format!("key{:08}", i).into_bytes())
        .collect();

    let filter = LearnedBloomFilter::new(&training_keys, 0.01).unwrap();

    // Benchmark: 1000 queries
    let query_keys: Vec<Vec<u8>> = (0..1000)
        .map(|i| format!("query{:08}", i).into_bytes())
        .collect();

    group.throughput(Throughput::Elements(query_keys.len() as u64));
    group.bench_function("batch_1000_queries", |b| {
        b.iter(|| {
            for key in &query_keys {
                black_box(filter.contains(key));
            }
        })
    });

    group.finish();
}

/// Benchmark construction + query pipeline
fn bench_end_to_end(c: &mut Criterion) {
    let mut group = c.benchmark_group("learned_bloom/end_to_end");

    group.bench_function("full_pipeline", |b| {
        b.iter_batched(
            || {
                let training_keys: Vec<Vec<u8>> = (0..5_000)
                    .map(|i| format!("key{:08}", i).into_bytes())
                    .collect();
                training_keys
            },
            |training_keys| {
                // Train
                let filter = LearnedBloomFilter::new(&training_keys, 0.01).unwrap();

                // Query
                let mut hits = 0;
                for i in 0..1000 {
                    let key = format!("key{:08}", i).into_bytes();
                    if filter.contains(&key) {
                        hits += 1;
                    }
                }

                black_box(hits)
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_training_time,
    bench_query_latency,
    bench_memory_comparison,
    bench_varying_fpr,
    bench_data_distributions,
    bench_batch_queries,
    bench_end_to_end,
);
criterion_main!(benches);
