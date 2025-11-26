//! Benchmarks for Rateless IBLT
//!
//! This benchmark suite measures the performance of Rateless IBLT operations
//! across various scenarios and scales.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sketch_oxide::common::Reconcilable;
use sketch_oxide::reconciliation::RatelessIBLT;

/// Benchmark single insert operation
fn bench_single_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("rateless_iblt_insert");

    let mut iblt = RatelessIBLT::new(100, 32).unwrap();

    group.bench_function("single_insert", |b| {
        b.iter(|| {
            iblt.insert(black_box(b"key"), black_box(b"value")).unwrap();
        });
    });

    group.finish();
}

/// Benchmark single delete operation
fn bench_single_delete(c: &mut Criterion) {
    let mut group = c.benchmark_group("rateless_iblt_delete");

    let mut iblt = RatelessIBLT::new(100, 32).unwrap();
    iblt.insert(b"key", b"value").unwrap();

    group.bench_function("single_delete", |b| {
        b.iter(|| {
            iblt.delete(black_box(b"key"), black_box(b"value")).unwrap();
            iblt.insert(b"key", b"value").unwrap(); // Re-insert for next iteration
        });
    });

    group.finish();
}

/// Benchmark bulk insert operations
fn bench_bulk_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("rateless_iblt_bulk_insert");

    for size in [10, 50, 100, 500].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut iblt = RatelessIBLT::new(size * 2, 32).unwrap();
                for i in 0..size {
                    let key = format!("key{}", i);
                    let value = format!("value{}", i);
                    iblt.insert(key.as_bytes(), value.as_bytes()).unwrap();
                }
                black_box(iblt);
            });
        });
    }

    group.finish();
}

/// Benchmark bulk delete operations
fn bench_bulk_delete(c: &mut Criterion) {
    let mut group = c.benchmark_group("rateless_iblt_bulk_delete");

    for size in [10, 50, 100].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    let mut iblt = RatelessIBLT::new(size * 2, 32).unwrap();
                    for i in 0..size {
                        let key = format!("key{}", i);
                        let value = format!("value{}", i);
                        iblt.insert(key.as_bytes(), value.as_bytes()).unwrap();
                    }
                    iblt
                },
                |mut iblt| {
                    for i in 0..size {
                        let key = format!("key{}", i);
                        let value = format!("value{}", i);
                        iblt.delete(key.as_bytes(), value.as_bytes()).unwrap();
                    }
                    black_box(iblt);
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark subtraction operation
fn bench_subtract(c: &mut Criterion) {
    let mut group = c.benchmark_group("rateless_iblt_subtract");

    for size in [10, 50, 100, 500].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut alice = RatelessIBLT::new(size * 2, 32).unwrap();
            let mut bob = RatelessIBLT::new(size * 2, 32).unwrap();

            // Populate both IBLTs
            for i in 0..size {
                let key = format!("key{}", i);
                alice.insert(key.as_bytes(), b"value").unwrap();
                if i < size / 2 {
                    bob.insert(key.as_bytes(), b"value").unwrap();
                }
            }

            b.iter(|| {
                let mut diff = alice.clone();
                diff.subtract(black_box(&bob)).unwrap();
                black_box(diff);
            });
        });
    }

    group.finish();
}

/// Benchmark decode operation
fn bench_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("rateless_iblt_decode");

    for size in [10, 50, 100].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut iblt = RatelessIBLT::new(size * 2, 32).unwrap();

            for i in 0..size {
                let key = format!("key{}", i);
                let value = format!("value{}", i);
                iblt.insert(key.as_bytes(), value.as_bytes()).unwrap();
            }

            b.iter(|| {
                let result = iblt.decode();
                black_box(result);
            });
        });
    }

    group.finish();
}

/// Benchmark throughput for different operations
fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("rateless_iblt_throughput");

    let sizes = [100, 500, 1000];

    for &size in &sizes {
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("insert_throughput", size),
            &size,
            |b, &size| {
                b.iter(|| {
                    let mut iblt = RatelessIBLT::new(size * 2, 32).unwrap();
                    for i in 0..size {
                        let key = format!("key{}", i);
                        iblt.insert(key.as_bytes(), b"value").unwrap();
                    }
                    black_box(iblt);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark large-scale difference detection
fn bench_large_scale_diff(c: &mut Criterion) {
    let mut group = c.benchmark_group("rateless_iblt_large_scale");

    let test_cases = vec![
        ("small_diff_100", 100, 10),
        ("medium_diff_500", 500, 50),
        ("large_diff_1000", 1000, 100),
    ];

    for (name, total_size, diff_size) in test_cases {
        group.throughput(Throughput::Elements(diff_size as u64));

        group.bench_function(name, |b| {
            // Setup
            let mut alice = RatelessIBLT::new(diff_size * 2, 64).unwrap();
            let mut bob = RatelessIBLT::new(diff_size * 2, 64).unwrap();

            // Shared items
            for i in 0..total_size - diff_size {
                let key = format!("shared{}", i);
                alice.insert(key.as_bytes(), b"value").unwrap();
                bob.insert(key.as_bytes(), b"value").unwrap();
            }

            // Different items
            for i in 0..diff_size / 2 {
                alice
                    .insert(format!("alice{}", i).as_bytes(), b"a")
                    .unwrap();
                bob.insert(format!("bob{}", i).as_bytes(), b"b").unwrap();
            }

            b.iter(|| {
                let mut diff = alice.clone();
                diff.subtract(&bob).unwrap();
                let result = diff.decode();
                black_box(result);
            });
        });
    }

    group.finish();
}

/// Benchmark varying cell sizes
fn bench_varying_cell_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("rateless_iblt_cell_sizes");

    let cell_sizes = [16, 32, 64, 128, 256];

    for &cell_size in &cell_sizes {
        group.bench_with_input(
            BenchmarkId::new("insert_decode", cell_size),
            &cell_size,
            |b, &cell_size| {
                b.iter(|| {
                    let mut iblt = RatelessIBLT::new(100, cell_size).unwrap();
                    for i in 0..50 {
                        let key = format!("key{}", i);
                        iblt.insert(key.as_bytes(), b"value").unwrap();
                    }
                    let result = iblt.decode();
                    black_box(result);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark varying item sizes
fn bench_varying_item_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("rateless_iblt_item_sizes");

    let item_sizes = [10, 50, 100, 500, 1024];

    for &item_size in &item_sizes {
        group.throughput(Throughput::Bytes(item_size as u64 * 50));

        group.bench_with_input(
            BenchmarkId::new("insert_items", item_size),
            &item_size,
            |b, &item_size| {
                let value = vec![0x42u8; item_size];

                b.iter(|| {
                    let mut iblt = RatelessIBLT::new(100, item_size * 2).unwrap();
                    for i in 0..50 {
                        let key = format!("key{}", i);
                        iblt.insert(key.as_bytes(), &value).unwrap();
                    }
                    black_box(iblt);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark full reconciliation workflow
fn bench_reconciliation_workflow(c: &mut Criterion) {
    let mut group = c.benchmark_group("rateless_iblt_reconciliation");

    group.bench_function("complete_workflow_100", |b| {
        b.iter(|| {
            // Create two IBLTs
            let mut alice = RatelessIBLT::new(200, 64).unwrap();
            let mut bob = RatelessIBLT::new(200, 64).unwrap();

            // Populate with shared and unique items
            for i in 0..80 {
                let key = format!("shared{}", i);
                alice.insert(key.as_bytes(), b"value").unwrap();
                bob.insert(key.as_bytes(), b"value").unwrap();
            }

            for i in 0..10 {
                alice
                    .insert(format!("alice{}", i).as_bytes(), b"a")
                    .unwrap();
                bob.insert(format!("bob{}", i).as_bytes(), b"b").unwrap();
            }

            // Compute difference
            let mut diff = alice.clone();
            diff.subtract(&bob).unwrap();

            // Decode
            let result = diff.decode();

            black_box(result);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_single_insert,
    bench_single_delete,
    bench_bulk_insert,
    bench_bulk_delete,
    bench_subtract,
    bench_decode,
    bench_throughput,
    bench_large_scale_diff,
    bench_varying_cell_sizes,
    bench_varying_item_sizes,
    bench_reconciliation_workflow,
);

criterion_main!(benches);
