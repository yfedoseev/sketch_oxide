//! Comprehensive benchmark suite for Vacuum Filter
//!
//! Performance targets:
//! - Insert: <100ns amortized
//! - Query: <50ns
//! - Delete: <100ns
//! - Space: <15 bits/item at 1% FPR

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sketch_oxide::membership::{CuckooFilter, VacuumFilter};

// ============================================================================
// Insertion Benchmarks
// ============================================================================

fn bench_insert_single(c: &mut Criterion) {
    let mut group = c.benchmark_group("vacuum_insert_single");

    for size in [1000, 10000, 100000] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter_batched(
                || VacuumFilter::new(size, 0.01).unwrap(),
                |mut filter| {
                    filter.insert(black_box(b"test_key")).unwrap();
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn bench_insert_bulk(c: &mut Criterion) {
    let mut group = c.benchmark_group("vacuum_insert_bulk");
    group.throughput(Throughput::Elements(1000));

    for fpr in [0.001, 0.01, 0.1] {
        group.bench_with_input(BenchmarkId::from_parameter(fpr), &fpr, |b, &fpr| {
            b.iter_batched(
                || VacuumFilter::new(10000, fpr).unwrap(),
                |mut filter| {
                    for i in 0u32..1000 {
                        filter.insert(black_box(&i.to_le_bytes())).unwrap();
                    }
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn bench_insert_with_rehashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("vacuum_insert_rehashing");

    group.bench_function("fill_and_rehash", |b| {
        b.iter_batched(
            || VacuumFilter::new(100, 0.01).unwrap(),
            |mut filter| {
                // Fill beyond initial capacity to trigger rehashing
                for i in 0u32..500 {
                    filter.insert(black_box(&i.to_le_bytes())).unwrap();
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

// ============================================================================
// Query Benchmarks
// ============================================================================

fn bench_query_positive(c: &mut Criterion) {
    let mut group = c.benchmark_group("vacuum_query_positive");

    for size in [1000, 10000, 100000] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let mut filter = VacuumFilter::new(size, 0.01).unwrap();
            for i in 0u32..(size as u32 / 2) {
                filter.insert(&i.to_le_bytes()).unwrap();
            }

            b.iter(|| {
                let key = 100u32;
                black_box(filter.contains(black_box(&key.to_le_bytes())));
            });
        });
    }

    group.finish();
}

fn bench_query_negative(c: &mut Criterion) {
    let mut group = c.benchmark_group("vacuum_query_negative");

    for size in [1000, 10000, 100000] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let mut filter = VacuumFilter::new(size, 0.01).unwrap();
            for i in 0u32..(size as u32 / 2) {
                filter.insert(&i.to_le_bytes()).unwrap();
            }

            b.iter(|| {
                let key = 999999u32;
                black_box(filter.contains(black_box(&key.to_le_bytes())));
            });
        });
    }

    group.finish();
}

fn bench_query_mixed(c: &mut Criterion) {
    let mut group = c.benchmark_group("vacuum_query_mixed");
    group.throughput(Throughput::Elements(100));

    group.bench_function("50_50_hit_miss", |b| {
        let mut filter = VacuumFilter::new(10000, 0.01).unwrap();
        for i in 0u32..5000 {
            filter.insert(&i.to_le_bytes()).unwrap();
        }

        b.iter(|| {
            for i in 0u32..100 {
                // 50% hits, 50% misses
                let key = if i % 2 == 0 { i * 10 } else { i + 10000 };
                black_box(filter.contains(black_box(&key.to_le_bytes())));
            }
        });
    });

    group.finish();
}

// ============================================================================
// Deletion Benchmarks
// ============================================================================

fn bench_delete_single(c: &mut Criterion) {
    let mut group = c.benchmark_group("vacuum_delete_single");

    for size in [1000, 10000, 100000] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let mut filter = VacuumFilter::new(size, 0.01).unwrap();
                    for i in 0u32..(size as u32 / 2) {
                        filter.insert(&i.to_le_bytes()).unwrap();
                    }
                    filter
                },
                |mut filter| {
                    filter.delete(black_box(&100u32.to_le_bytes())).unwrap();
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn bench_delete_bulk(c: &mut Criterion) {
    let mut group = c.benchmark_group("vacuum_delete_bulk");
    group.throughput(Throughput::Elements(100));

    group.bench_function("delete_100_items", |b| {
        b.iter_batched(
            || {
                let mut filter = VacuumFilter::new(10000, 0.01).unwrap();
                for i in 0u32..5000 {
                    filter.insert(&i.to_le_bytes()).unwrap();
                }
                filter
            },
            |mut filter| {
                for i in 0u32..100 {
                    filter.delete(black_box(&i.to_le_bytes())).unwrap();
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

// ============================================================================
// Throughput Benchmarks
// ============================================================================

fn bench_throughput_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("vacuum_throughput");
    group.throughput(Throughput::Elements(10000));

    group.bench_function("10k_mixed_ops", |b| {
        b.iter_batched(
            || VacuumFilter::new(20000, 0.01).unwrap(),
            |mut filter| {
                // Mix of insertions, queries, and deletions
                for i in 0u32..10000 {
                    match i % 3 {
                        0 => {
                            filter.insert(black_box(&i.to_le_bytes())).unwrap();
                        }
                        1 => {
                            black_box(filter.contains(black_box(&i.to_le_bytes())));
                        }
                        2 => {
                            filter.delete(black_box(&i.to_le_bytes())).unwrap();
                        }
                        _ => unreachable!(),
                    }
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

// ============================================================================
// Memory Comparison Benchmarks
// ============================================================================

fn bench_memory_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_comparison");

    for size in [1000, 10000, 100000] {
        group.bench_with_input(BenchmarkId::new("vacuum", size), &size, |b, &size| {
            b.iter(|| {
                let filter = VacuumFilter::new(black_box(size), 0.01).unwrap();
                black_box(filter.memory_usage());
            });
        });

        group.bench_with_input(BenchmarkId::new("cuckoo", size), &size, |b, &size| {
            b.iter(|| {
                let filter = CuckooFilter::new(black_box(size)).unwrap();
                black_box(filter.memory_usage());
            });
        });
    }

    group.finish();
}

// ============================================================================
// Fingerprint Bits Variation
// ============================================================================

fn bench_varying_fingerprint_bits(c: &mut Criterion) {
    let mut group = c.benchmark_group("vacuum_fingerprint_bits");
    group.throughput(Throughput::Elements(1000));

    for fpr in [0.001, 0.01, 0.05, 0.1, 0.5] {
        group.bench_with_input(BenchmarkId::from_parameter(fpr), &fpr, |b, &fpr| {
            b.iter_batched(
                || VacuumFilter::new(10000, fpr).unwrap(),
                |mut filter| {
                    for i in 0u32..1000 {
                        filter.insert(black_box(&i.to_le_bytes())).unwrap();
                    }
                    for i in 0u32..1000 {
                        black_box(filter.contains(black_box(&i.to_le_bytes())));
                    }
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

// ============================================================================
// Load Factor Variation
// ============================================================================

fn bench_varying_load_factors(c: &mut Criterion) {
    let mut group = c.benchmark_group("vacuum_load_factors");

    for max_lf in [0.7, 0.8, 0.9, 0.95] {
        group.bench_with_input(
            BenchmarkId::from_parameter(max_lf),
            &max_lf,
            |b, &max_lf| {
                b.iter_batched(
                    || VacuumFilter::with_load_factor(10000, 0.01, max_lf).unwrap(),
                    |mut filter| {
                        for i in 0u32..5000 {
                            filter.insert(black_box(&i.to_le_bytes())).unwrap();
                        }
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

// ============================================================================
// Space Efficiency Analysis
// ============================================================================

fn bench_space_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("space_efficiency");

    group.bench_function("vacuum_bits_per_item", |b| {
        b.iter(|| {
            let mut filter = VacuumFilter::new(black_box(10000), 0.01).unwrap();
            for i in 0u32..10000 {
                filter.insert(&i.to_le_bytes()).unwrap();
            }
            let stats = filter.stats();
            let bits_per_item = stats.memory_bits as f64 / stats.num_items as f64;
            black_box(bits_per_item);
        });
    });

    group.bench_function("cuckoo_bits_per_item", |b| {
        b.iter(|| {
            let mut filter = CuckooFilter::new(black_box(10000)).unwrap();
            for i in 0u32..10000 {
                filter.insert(&i.to_le_bytes()).unwrap();
            }
            let bytes = filter.memory_usage();
            let bits_per_item = (bytes * 8) as f64 / 10000.0;
            black_box(bits_per_item);
        });
    });

    group.finish();
}

// ============================================================================
// Cache Performance
// ============================================================================

fn bench_cache_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("vacuum_cache_performance");

    group.bench_function("sequential_queries", |b| {
        let mut filter = VacuumFilter::new(100000, 0.01).unwrap();
        for i in 0u32..50000 {
            filter.insert(&i.to_le_bytes()).unwrap();
        }

        b.iter(|| {
            for i in 0u32..1000 {
                black_box(filter.contains(black_box(&i.to_le_bytes())));
            }
        });
    });

    group.bench_function("random_queries", |b| {
        let mut filter = VacuumFilter::new(100000, 0.01).unwrap();
        for i in 0u32..50000 {
            filter.insert(&i.to_le_bytes()).unwrap();
        }

        b.iter(|| {
            for i in (0u32..1000).step_by(37) {
                // Pseudo-random access pattern
                let key = (i * 7919) % 100000;
                black_box(filter.contains(black_box(&key.to_le_bytes())));
            }
        });
    });

    group.finish();
}

// ============================================================================
// Comparison: Vacuum vs Cuckoo
// ============================================================================

fn bench_vacuum_vs_cuckoo(c: &mut Criterion) {
    let mut group = c.benchmark_group("vacuum_vs_cuckoo");
    group.throughput(Throughput::Elements(1000));

    // Insert comparison
    group.bench_function("vacuum_insert_1k", |b| {
        b.iter_batched(
            || VacuumFilter::new(10000, 0.01).unwrap(),
            |mut filter| {
                for i in 0u32..1000 {
                    filter.insert(black_box(&i.to_le_bytes())).unwrap();
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("cuckoo_insert_1k", |b| {
        b.iter_batched(
            || CuckooFilter::new(10000).unwrap(),
            |mut filter| {
                for i in 0u32..1000 {
                    filter.insert(black_box(&i.to_le_bytes())).unwrap();
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Query comparison
    group.bench_function("vacuum_query_1k", |b| {
        let mut filter = VacuumFilter::new(10000, 0.01).unwrap();
        for i in 0u32..5000 {
            filter.insert(&i.to_le_bytes()).unwrap();
        }

        b.iter(|| {
            for i in 0u32..1000 {
                black_box(filter.contains(black_box(&i.to_le_bytes())));
            }
        });
    });

    group.bench_function("cuckoo_query_1k", |b| {
        let mut filter = CuckooFilter::new(10000).unwrap();
        for i in 0u32..5000 {
            filter.insert(&i.to_le_bytes()).unwrap();
        }

        b.iter(|| {
            for i in 0u32..1000 {
                black_box(filter.contains(black_box(&i.to_le_bytes())));
            }
        });
    });

    // Delete comparison
    group.bench_function("vacuum_delete_1k", |b| {
        b.iter_batched(
            || {
                let mut filter = VacuumFilter::new(10000, 0.01).unwrap();
                for i in 0u32..5000 {
                    filter.insert(&i.to_le_bytes()).unwrap();
                }
                filter
            },
            |mut filter| {
                for i in 0u32..1000 {
                    filter.delete(black_box(&i.to_le_bytes())).unwrap();
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("cuckoo_delete_1k", |b| {
        b.iter_batched(
            || {
                let mut filter = CuckooFilter::new(10000).unwrap();
                for i in 0u32..5000 {
                    filter.insert(&i.to_le_bytes()).unwrap();
                }
                filter
            },
            |mut filter| {
                for i in 0u32..1000 {
                    filter.remove(black_box(&i.to_le_bytes()));
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_insert_single,
    bench_insert_bulk,
    bench_insert_with_rehashing,
    bench_query_positive,
    bench_query_negative,
    bench_query_mixed,
    bench_delete_single,
    bench_delete_bulk,
    bench_throughput_operations,
    bench_memory_comparison,
    bench_varying_fingerprint_bits,
    bench_varying_load_factors,
    bench_space_efficiency,
    bench_cache_performance,
    bench_vacuum_vs_cuckoo,
);

criterion_main!(benches);
