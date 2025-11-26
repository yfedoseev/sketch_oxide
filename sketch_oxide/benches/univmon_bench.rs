//! Comprehensive Benchmark Suite for UnivMon
//!
//! This benchmark suite evaluates UnivMon performance across 15+ scenarios,
//! demonstrating its efficiency for multi-metric streaming analytics.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sketch_oxide::universal::UnivMon;
use sketch_oxide::Mergeable;

// ============================================================================
// Benchmark 1: Update Latency (Single Item)
// ============================================================================

fn bench_update_single(c: &mut Criterion) {
    let mut group = c.benchmark_group("update_single");

    for size in [1_000, 10_000, 100_000, 1_000_000] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let mut univmon = UnivMon::new(size, 0.01, 0.01).unwrap();
            let mut counter = 0u64;

            b.iter(|| {
                let key = format!("item_{}", counter);
                counter += 1;
                black_box(univmon.update(key.as_bytes(), 1.0).unwrap());
            });
        });
    }

    group.finish();
}

// ============================================================================
// Benchmark 2: Update Throughput (Bulk)
// ============================================================================

fn bench_update_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("update_throughput");

    for size in [1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter_batched(
                || UnivMon::new(size as u64, 0.01, 0.01).unwrap(),
                |mut univmon| {
                    for i in 0..size {
                        let key = format!("item_{}", i);
                        univmon.update(key.as_bytes(), 1.0).unwrap();
                    }
                    black_box(univmon);
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

// ============================================================================
// Benchmark 3: L1 Norm Computation
// ============================================================================

fn bench_l1_estimation(c: &mut Criterion) {
    let mut group = c.benchmark_group("l1_estimation");

    for item_count in [100, 1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(item_count),
            &item_count,
            |b, &item_count| {
                let mut univmon = UnivMon::new(100_000, 0.01, 0.01).unwrap();

                // Pre-populate
                for i in 0..item_count {
                    let key = format!("item_{}", i);
                    univmon
                        .update(key.as_bytes(), (i % 100 + 1) as f64)
                        .unwrap();
                }

                b.iter(|| {
                    black_box(univmon.estimate_l1());
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmark 4: L2 Norm Computation
// ============================================================================

fn bench_l2_estimation(c: &mut Criterion) {
    let mut group = c.benchmark_group("l2_estimation");

    for item_count in [100, 1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(item_count),
            &item_count,
            |b, &item_count| {
                let mut univmon = UnivMon::new(100_000, 0.01, 0.01).unwrap();

                // Pre-populate
                for i in 0..item_count {
                    let key = format!("item_{}", i);
                    univmon
                        .update(key.as_bytes(), (i % 100 + 1) as f64)
                        .unwrap();
                }

                b.iter(|| {
                    black_box(univmon.estimate_l2());
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmark 5: Entropy Estimation
// ============================================================================

fn bench_entropy_estimation(c: &mut Criterion) {
    let mut group = c.benchmark_group("entropy_estimation");

    for item_count in [100, 1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(item_count),
            &item_count,
            |b, &item_count| {
                let mut univmon = UnivMon::new(100_000, 0.01, 0.01).unwrap();

                // Pre-populate
                for i in 0..item_count {
                    let key = format!("item_{}", i);
                    univmon.update(key.as_bytes(), 1.0).unwrap();
                }

                b.iter(|| {
                    black_box(univmon.estimate_entropy());
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmark 6: Heavy Hitters Extraction
// ============================================================================

fn bench_heavy_hitters(c: &mut Criterion) {
    let mut group = c.benchmark_group("heavy_hitters");

    for threshold in [0.01, 0.05, 0.1, 0.5] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}%", (threshold * 100.0) as u32)),
            &threshold,
            |b, &threshold| {
                let mut univmon = UnivMon::new(100_000, 0.01, 0.01).unwrap();

                // Create Zipf distribution
                for i in 1..=1000 {
                    let key = format!("item_{}", i);
                    let freq = 10000.0 / (i as f64);
                    univmon.update(key.as_bytes(), freq).unwrap();
                }

                b.iter(|| {
                    black_box(univmon.heavy_hitters(threshold));
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmark 7: Change Detection
// ============================================================================

fn bench_change_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("change_detection");

    for item_count in [100, 1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(item_count),
            &item_count,
            |b, &item_count| {
                let mut univmon1 = UnivMon::new(100_000, 0.01, 0.01).unwrap();
                let mut univmon2 = UnivMon::new(100_000, 0.01, 0.01).unwrap();

                // Create two different distributions
                for i in 0..item_count {
                    let key1 = format!("a_{}", i);
                    let key2 = format!("b_{}", i);
                    univmon1.update(key1.as_bytes(), 1.0).unwrap();
                    univmon2.update(key2.as_bytes(), 1.0).unwrap();
                }

                b.iter(|| {
                    black_box(univmon1.detect_change(&univmon2));
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmark 8: Merge Operation
// ============================================================================

fn bench_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("merge");

    for item_count in [100, 1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(item_count),
            &item_count,
            |b, &item_count| {
                b.iter_batched(
                    || {
                        let mut univmon1 = UnivMon::new(100_000, 0.01, 0.01).unwrap();
                        let mut univmon2 = UnivMon::new(100_000, 0.01, 0.01).unwrap();

                        for i in 0..item_count {
                            let key = format!("item_{}", i);
                            univmon1.update(key.as_bytes(), 1.0).unwrap();
                            univmon2.update(key.as_bytes(), 2.0).unwrap();
                        }

                        (univmon1, univmon2)
                    },
                    |(mut univmon1, univmon2)| {
                        black_box(univmon1.merge(&univmon2).unwrap());
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmark 9: Varying Stream Sizes
// ============================================================================

fn bench_varying_stream_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("varying_stream_sizes");

    for max_size in [1_000, 10_000, 100_000, 1_000_000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(max_size),
            &max_size,
            |b, &max_size| {
                b.iter_batched(
                    || UnivMon::new(max_size, 0.01, 0.01).unwrap(),
                    |mut univmon| {
                        for i in 0..1000 {
                            let key = format!("item_{}", i);
                            univmon.update(key.as_bytes(), 1.0).unwrap();
                        }
                        black_box(univmon);
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmark 10: Varying Distributions
// ============================================================================

fn bench_varying_distributions(c: &mut Criterion) {
    let mut group = c.benchmark_group("varying_distributions");

    // Uniform distribution
    group.bench_with_input(
        BenchmarkId::from_parameter("uniform"),
        &"uniform",
        |b, _| {
            b.iter_batched(
                || UnivMon::new(100_000, 0.01, 0.01).unwrap(),
                |mut univmon| {
                    for i in 0..1000 {
                        let key = format!("item_{}", i);
                        univmon.update(key.as_bytes(), 1.0).unwrap();
                    }
                    black_box(univmon);
                },
                criterion::BatchSize::SmallInput,
            );
        },
    );

    // Zipf distribution
    group.bench_with_input(BenchmarkId::from_parameter("zipf"), &"zipf", |b, _| {
        b.iter_batched(
            || UnivMon::new(100_000, 0.01, 0.01).unwrap(),
            |mut univmon| {
                for i in 0..1000 {
                    let key = format!("item_{}", i);
                    let freq = 1000.0 / (i as f64 + 1.0);
                    univmon.update(key.as_bytes(), freq).unwrap();
                }
                black_box(univmon);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Exponential distribution
    group.bench_with_input(
        BenchmarkId::from_parameter("exponential"),
        &"exponential",
        |b, _| {
            b.iter_batched(
                || UnivMon::new(100_000, 0.01, 0.01).unwrap(),
                |mut univmon| {
                    for i in 0..1000 {
                        let key = format!("item_{}", i);
                        let freq = 1000.0 * 0.9_f64.powi(i as i32);
                        univmon.update(key.as_bytes(), freq).unwrap();
                    }
                    black_box(univmon);
                },
                criterion::BatchSize::SmallInput,
            );
        },
    );

    group.finish();
}

// ============================================================================
// Benchmark 11: Layer Impact Analysis
// ============================================================================

fn bench_layer_impact(c: &mut Criterion) {
    let mut group = c.benchmark_group("layer_impact");

    for max_size in [1_000, 10_000, 100_000, 1_000_000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!(
                "{}layers",
                (max_size as f64).log2().ceil() as usize
            )),
            &max_size,
            |b, &max_size| {
                let mut univmon = UnivMon::new(max_size, 0.01, 0.01).unwrap();

                b.iter(|| {
                    let key = format!("item_{}", black_box(42));
                    black_box(univmon.update(key.as_bytes(), 1.0).unwrap());
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmark 12: Memory Scaling
// ============================================================================

fn bench_memory_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_scaling");

    for max_size in [1_000, 10_000, 100_000, 1_000_000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(max_size),
            &max_size,
            |b, &max_size| {
                b.iter(|| {
                    let univmon = UnivMon::new(max_size, 0.01, 0.01).unwrap();
                    black_box(univmon.stats());
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmark 13: Multi-Metric Simulation
// ============================================================================

fn bench_multi_metric_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_metric_simulation");

    group.bench_function("all_metrics", |b| {
        let mut univmon = UnivMon::new(100_000, 0.01, 0.01).unwrap();

        // Pre-populate
        for i in 0..1000 {
            let key = format!("item_{}", i);
            univmon
                .update(key.as_bytes(), (i % 100 + 1) as f64)
                .unwrap();
        }

        b.iter(|| {
            // Query all 6 metrics
            let l1 = univmon.estimate_l1();
            let l2 = univmon.estimate_l2();
            let entropy = univmon.estimate_entropy();
            let heavy = univmon.heavy_hitters(0.05);
            let other = univmon.clone();
            let change = univmon.detect_change(&other);
            let stats = univmon.stats();

            black_box((l1, l2, entropy, heavy, change, stats));
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark 14: Network Telemetry Workload
// ============================================================================

fn bench_network_telemetry(c: &mut Criterion) {
    let mut group = c.benchmark_group("network_telemetry");

    group.bench_function("realistic_network", |b| {
        b.iter_batched(
            || UnivMon::new(1_000_000, 0.01, 0.01).unwrap(),
            |mut univmon| {
                // Simulate 10K network packets
                for i in 0..10_000 {
                    // Zipf distribution of IPs (realistic)
                    let ip_id = 1 + (i % 1000) / (1 + i / 1000);
                    let ip = format!("192.168.{}.{}", ip_id / 256, ip_id % 256);

                    // Packet size varies
                    let packet_size = 64.0 + (i % 1400) as f64;

                    univmon.update(ip.as_bytes(), packet_size).unwrap();
                }

                // Query metrics
                let total_traffic = univmon.estimate_l1();
                let load_balance = univmon.estimate_l2();
                let ip_diversity = univmon.estimate_entropy();
                let top_talkers = univmon.heavy_hitters(0.01);

                black_box((total_traffic, load_balance, ip_diversity, top_talkers));
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

// ============================================================================
// Benchmark 15: Long-Running Stream
// ============================================================================

fn bench_long_running_stream(c: &mut Criterion) {
    let mut group = c.benchmark_group("long_running_stream");
    group.sample_size(20); // Fewer samples for long benchmark

    for stream_length in [10_000, 50_000, 100_000] {
        group.throughput(Throughput::Elements(stream_length as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(stream_length),
            &stream_length,
            |b, &stream_length| {
                b.iter_batched(
                    || UnivMon::new(1_000_000, 0.01, 0.01).unwrap(),
                    |mut univmon| {
                        for i in 0..stream_length {
                            let key = format!("item_{}", i % 10000);
                            univmon
                                .update(key.as_bytes(), (i % 100 + 1) as f64)
                                .unwrap();
                        }

                        // Query all metrics at end
                        let l1 = univmon.estimate_l1();
                        let l2 = univmon.estimate_l2();
                        let entropy = univmon.estimate_entropy();
                        let heavy = univmon.heavy_hitters(0.01);

                        black_box((univmon, l1, l2, entropy, heavy));
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmark 16: Comparison vs Separate Sketches
// ============================================================================

fn bench_univmon_vs_separate(c: &mut Criterion) {
    let mut group = c.benchmark_group("univmon_vs_separate");

    group.bench_function("univmon_unified", |b| {
        b.iter_batched(
            || UnivMon::new(100_000, 0.01, 0.01).unwrap(),
            |mut univmon| {
                // Update 1000 items
                for i in 0..1000 {
                    let key = format!("item_{}", i);
                    univmon
                        .update(key.as_bytes(), (i % 100 + 1) as f64)
                        .unwrap();
                }

                // Query all metrics
                let l1 = univmon.estimate_l1();
                let l2 = univmon.estimate_l2();
                let entropy = univmon.estimate_entropy();
                let heavy = univmon.heavy_hitters(0.05);

                black_box((univmon, l1, l2, entropy, heavy));
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Note: Would compare against separate CountMinSketch + HyperLogLog + etc.
    // but that would require implementing all separately which defeats the purpose

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    benches,
    bench_update_single,
    bench_update_throughput,
    bench_l1_estimation,
    bench_l2_estimation,
    bench_entropy_estimation,
    bench_heavy_hitters,
    bench_change_detection,
    bench_merge,
    bench_varying_stream_sizes,
    bench_varying_distributions,
    bench_layer_impact,
    bench_memory_scaling,
    bench_multi_metric_simulation,
    bench_network_telemetry,
    bench_long_running_stream,
    bench_univmon_vs_separate,
);

criterion_main!(benches);
