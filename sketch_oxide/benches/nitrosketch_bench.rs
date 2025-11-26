//! Benchmarks for NitroSketch
//!
//! Performance targets (SIGCOMM 2019):
//! - Update latency: <100ns (sub-microsecond)
//! - Throughput: >100K updates/sec
//! - Memory: Same as base sketch
//! - Achieves 100Gbps line rate in DPDK

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sketch_oxide::cardinality::HyperLogLog;
use sketch_oxide::frequency::{CountMinSketch, NitroSketch};

// ============================================================================
// Update Benchmarks
// ============================================================================

fn bench_update_high_sample_rate(c: &mut Criterion) {
    let mut group = c.benchmark_group("nitrosketch_update_high_rate");
    group.throughput(Throughput::Elements(1));

    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.9).unwrap();

    group.bench_function("sample_rate_0.9", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            let key = format!("flow_{}", counter);
            nitro.update_sampled(key.as_bytes());
            counter = counter.wrapping_add(1);
        });
    });

    group.finish();
}

fn bench_update_low_sample_rate(c: &mut Criterion) {
    let mut group = c.benchmark_group("nitrosketch_update_low_rate");
    group.throughput(Throughput::Elements(1));

    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.01).unwrap();

    group.bench_function("sample_rate_0.01", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            let key = format!("flow_{}", counter);
            nitro.update_sampled(key.as_bytes());
            counter = counter.wrapping_add(1);
        });
    });

    group.finish();
}

fn bench_update_various_rates(c: &mut Criterion) {
    let mut group = c.benchmark_group("nitrosketch_update_rates");
    group.throughput(Throughput::Elements(1));

    for &sample_rate in &[0.01, 0.1, 0.5, 0.9, 1.0] {
        let base = CountMinSketch::new(0.01, 0.01).unwrap();
        let mut nitro = NitroSketch::new(base, sample_rate).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("rate_{}", sample_rate)),
            &sample_rate,
            |b, _| {
                let mut counter = 0u64;
                b.iter(|| {
                    let key = format!("flow_{}", counter);
                    nitro.update_sampled(key.as_bytes());
                    counter = counter.wrapping_add(1);
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Throughput Benchmarks
// ============================================================================

fn bench_throughput_100k_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("nitrosketch_throughput");
    group.sample_size(10);

    for &sample_rate in &[0.01, 0.1, 0.5] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("100k_rate_{}", sample_rate)),
            &sample_rate,
            |b, &rate| {
                b.iter(|| {
                    let base = CountMinSketch::new(0.01, 0.01).unwrap();
                    let mut nitro = NitroSketch::new(base, rate).unwrap();

                    for i in 0..100_000 {
                        let key = format!("flow_{}", i % 1000);
                        nitro.update_sampled(key.as_bytes());
                    }
                    black_box(nitro);
                });
            },
        );
    }

    group.finish();
}

fn bench_throughput_sustained_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("nitrosketch_sustained");
    group.sample_size(10);

    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.1).unwrap();

    group.bench_function("1M_updates", |b| {
        b.iter(|| {
            for i in 0..1_000_000 {
                let key = format!("flow_{}", i % 10000);
                nitro.update_sampled(key.as_bytes());
            }
        });
    });

    group.finish();
}

// ============================================================================
// Query Benchmarks
// ============================================================================

fn bench_query_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("nitrosketch_query");
    group.throughput(Throughput::Elements(1));

    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.1).unwrap();

    // Populate with data
    for i in 0..10000 {
        nitro.update_sampled(format!("flow_{}", i % 100).as_bytes());
    }

    group.bench_function("query", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            let key = format!("flow_{}", counter % 100);
            let result = nitro.query(key.as_bytes());
            counter = counter.wrapping_add(1);
            black_box(result);
        });
    });

    group.finish();
}

// ============================================================================
// Synchronization Benchmarks
// ============================================================================

fn bench_sync_operation(c: &mut Criterion) {
    let mut group = c.benchmark_group("nitrosketch_sync");

    for &sample_rate in &[0.01, 0.1, 0.5] {
        let base = CountMinSketch::new(0.01, 0.01).unwrap();
        let mut nitro = NitroSketch::new(base, sample_rate).unwrap();

        // Populate
        for i in 0..10000 {
            nitro.update_sampled(format!("flow_{}", i).as_bytes());
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("rate_{}", sample_rate)),
            &sample_rate,
            |b, _| {
                b.iter(|| {
                    nitro.sync(1.0).unwrap();
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Memory Efficiency Benchmarks
// ============================================================================

fn bench_memory_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("nitrosketch_memory");
    group.sample_size(10);

    // Baseline: raw CountMinSketch
    group.bench_function("baseline_cms", |b| {
        b.iter(|| {
            let mut cms = CountMinSketch::new(0.01, 0.01).unwrap();
            for i in 0..10000 {
                cms.update(&format!("flow_{}", i % 100));
            }
            black_box(cms);
        });
    });

    // NitroSketch wrapper
    group.bench_function("nitrosketch_wrapper", |b| {
        b.iter(|| {
            let base = CountMinSketch::new(0.01, 0.01).unwrap();
            let mut nitro = NitroSketch::new(base, 0.1).unwrap();
            for i in 0..10000 {
                nitro.update_sampled(format!("flow_{}", i % 100).as_bytes());
            }
            black_box(nitro);
        });
    });

    group.finish();
}

// ============================================================================
// Base Sketch Comparison Benchmarks
// ============================================================================

fn bench_different_base_sketches(c: &mut Criterion) {
    let mut group = c.benchmark_group("nitrosketch_base_sketches");
    group.throughput(Throughput::Elements(1));

    // With CountMinSketch
    {
        let base = CountMinSketch::new(0.01, 0.01).unwrap();
        let mut nitro = NitroSketch::new(base, 0.1).unwrap();

        group.bench_function("count_min_sketch", |b| {
            let mut counter = 0u64;
            b.iter(|| {
                let key = format!("flow_{}", counter);
                nitro.update_sampled(key.as_bytes());
                counter = counter.wrapping_add(1);
            });
        });
    }

    // With HyperLogLog
    {
        let base = HyperLogLog::new(12).unwrap();
        let mut nitro = NitroSketch::new(base, 0.1).unwrap();

        group.bench_function("hyperloglog", |b| {
            let mut counter = 0u64;
            b.iter(|| {
                let key = format!("user_{}", counter);
                nitro.update_sampled(key.as_bytes());
                counter = counter.wrapping_add(1);
            });
        });
    }

    group.finish();
}

// ============================================================================
// Network Traffic Pattern Benchmarks
// ============================================================================

fn bench_bursty_traffic(c: &mut Criterion) {
    let mut group = c.benchmark_group("nitrosketch_bursty");
    group.sample_size(10);

    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.1).unwrap();

    group.bench_function("burst_pattern", |b| {
        b.iter(|| {
            // Burst 1: 10K items
            for i in 0..10000 {
                nitro.update_sampled(format!("burst1_{}", i).as_bytes());
            }

            // Burst 2: 10K items
            for i in 0..10000 {
                nitro.update_sampled(format!("burst2_{}", i).as_bytes());
            }
        });
    });

    group.finish();
}

fn bench_network_flow_keys(c: &mut Criterion) {
    let mut group = c.benchmark_group("nitrosketch_network_flows");
    group.throughput(Throughput::Elements(1));

    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.1).unwrap();

    group.bench_function("flow_5tuple", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            let src_ip = counter % 256;
            let dst_ip = (counter / 256) % 256;
            let src_port = (counter % 65536) as u16;
            let dst_port = ((counter / 65536) % 65536) as u16;

            // Simulate 5-tuple flow key: src_ip:src_port -> dst_ip:dst_port + proto
            let flow_key = format!(
                "192.168.{}.{}:{}->10.0.{}.{}:{}:TCP",
                src_ip / 256,
                src_ip % 256,
                src_port,
                dst_ip / 256,
                dst_ip % 256,
                dst_port
            );

            nitro.update_sampled(flow_key.as_bytes());
            counter = counter.wrapping_add(1);
        });
    });

    group.finish();
}

// ============================================================================
// Accuracy vs Sample Rate Benchmarks
// ============================================================================

fn bench_accuracy_vs_sample_rate(c: &mut Criterion) {
    let mut group = c.benchmark_group("nitrosketch_accuracy");
    group.sample_size(10);

    for &sample_rate in &[0.01, 0.05, 0.1, 0.2, 0.5, 1.0] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("rate_{}", sample_rate)),
            &sample_rate,
            |b, &rate| {
                b.iter(|| {
                    let base = CountMinSketch::new(0.01, 0.01).unwrap();
                    let mut nitro = NitroSketch::new(base, rate).unwrap();

                    // Add 10K items
                    for i in 0..10000 {
                        nitro.update_sampled(format!("item_{}", i % 100).as_bytes());
                    }

                    nitro.sync(1.0).unwrap();
                    black_box(nitro);
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Construction Benchmarks
// ============================================================================

fn bench_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("nitrosketch_construction");

    for &sample_rate in &[0.01, 0.1, 0.5, 1.0] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("rate_{}", sample_rate)),
            &sample_rate,
            |b, &rate| {
                b.iter(|| {
                    let base = CountMinSketch::new(0.01, 0.01).unwrap();
                    let nitro = NitroSketch::new(base, rate).unwrap();
                    black_box(nitro);
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Stats Collection Benchmarks
// ============================================================================

fn bench_stats_collection(c: &mut Criterion) {
    let mut group = c.benchmark_group("nitrosketch_stats");

    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.1).unwrap();

    // Populate
    for i in 0..10000 {
        nitro.update_sampled(format!("flow_{}", i).as_bytes());
    }

    group.bench_function("get_stats", |b| {
        b.iter(|| {
            let stats = nitro.stats();
            black_box(stats);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_update_high_sample_rate,
    bench_update_low_sample_rate,
    bench_update_various_rates,
    bench_throughput_100k_ops,
    bench_throughput_sustained_load,
    bench_query_latency,
    bench_sync_operation,
    bench_memory_comparison,
    bench_different_base_sketches,
    bench_bursty_traffic,
    bench_network_flow_keys,
    bench_accuracy_vs_sample_rate,
    bench_construction,
    bench_stats_collection,
);

criterion_main!(benches);
