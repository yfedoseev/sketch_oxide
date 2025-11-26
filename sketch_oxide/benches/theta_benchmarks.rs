//! Theta Sketch Performance Benchmarks
//!
//! Measures performance of Theta Sketch operations:
//! - Update: Hash + HashSet insert
//! - Union: HashSet union operation
//! - Intersection: HashSet intersection operation
//! - Difference: HashSet difference operation
//! - Estimate: Cardinality estimation
//!
//! Performance Targets (2024 hardware):
//! - Update: <150ns per item
//! - Union: <10µs for typical sketches
//! - Intersection: <10µs for typical sketches
//! - Difference: <10µs for typical sketches
//! - Estimate: <1µs

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sketch_oxide::cardinality::ThetaSketch;

// ============================================================================
// Update Operation Benchmarks
// ============================================================================

fn bench_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("theta_update");

    // Benchmark single update
    group.bench_function("single", |b| {
        let mut sketch = ThetaSketch::new(12).unwrap();
        let mut i = 0u64;
        b.iter(|| {
            sketch.update(black_box(&i));
            i += 1;
        });
    });

    // Benchmark update throughput for different cardinalities
    for n in [100, 1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(n));
        group.bench_with_input(BenchmarkId::new("throughput", n), &n, |b, &n| {
            b.iter(|| {
                let mut sketch = ThetaSketch::new(14).unwrap();
                for i in 0..n {
                    sketch.update(black_box(&i));
                }
                black_box(sketch)
            });
        });
    }

    group.finish();
}

// ============================================================================
// Union Operation Benchmarks
// ============================================================================

fn bench_union(c: &mut Criterion) {
    let mut group = c.benchmark_group("theta_union");

    // Test different sketch sizes
    for lg_k in [10, 12, 14] {
        let k = 1_usize << lg_k;

        // Create two sketches with 50% overlap
        let mut sketch_a = ThetaSketch::new(lg_k).unwrap();
        let mut sketch_b = ThetaSketch::new(lg_k).unwrap();

        for i in 0..k / 2 {
            sketch_a.update(&i);
        }
        for i in k / 4..3 * k / 4 {
            sketch_b.update(&i);
        }

        group.bench_with_input(
            BenchmarkId::new("overlap_50%", format!("lg_k={}", lg_k)),
            &(sketch_a, sketch_b),
            |bencher, (a, b)| {
                bencher.iter(|| black_box(a.union(black_box(b)).unwrap()));
            },
        );
    }

    group.finish();
}

// ============================================================================
// Intersection Operation Benchmarks
// ============================================================================

fn bench_intersect(c: &mut Criterion) {
    let mut group = c.benchmark_group("theta_intersect");

    // Test different overlap percentages
    for overlap_pct in [10, 50, 90] {
        let lg_k = 12;
        let k = 1_usize << lg_k;

        let mut sketch_a = ThetaSketch::new(lg_k).unwrap();
        let mut sketch_b = ThetaSketch::new(lg_k).unwrap();

        // Fill sketch_a
        for i in 0..k {
            sketch_a.update(&i);
        }

        // Fill sketch_b with specified overlap
        let overlap_count = (k * overlap_pct) / 100;
        for i in 0..overlap_count {
            sketch_b.update(&i);
        }
        for i in k..k + (k - overlap_count) {
            sketch_b.update(&i);
        }

        group.bench_with_input(
            BenchmarkId::new("overlap", format!("{}%", overlap_pct)),
            &(sketch_a, sketch_b),
            |bencher, (a, b)| {
                bencher.iter(|| black_box(a.intersect(black_box(b)).unwrap()));
            },
        );
    }

    group.finish();
}

// ============================================================================
// Difference Operation Benchmarks
// ============================================================================

fn bench_difference(c: &mut Criterion) {
    let mut group = c.benchmark_group("theta_difference");

    // Test different overlap percentages
    for overlap_pct in [10, 50, 90] {
        let lg_k = 12;
        let k = 1_usize << lg_k;

        let mut sketch_a = ThetaSketch::new(lg_k).unwrap();
        let mut sketch_b = ThetaSketch::new(lg_k).unwrap();

        // Fill sketch_a
        for i in 0..k {
            sketch_a.update(&i);
        }

        // Fill sketch_b with specified overlap
        let overlap_count = (k * overlap_pct) / 100;
        for i in 0..overlap_count {
            sketch_b.update(&i);
        }
        for i in k..k + (k - overlap_count) {
            sketch_b.update(&i);
        }

        group.bench_with_input(
            BenchmarkId::new("overlap", format!("{}%", overlap_pct)),
            &(sketch_a, sketch_b),
            |bencher, (a, b)| {
                bencher.iter(|| black_box(a.difference(black_box(b)).unwrap()));
            },
        );
    }

    group.finish();
}

// ============================================================================
// Estimate Operation Benchmarks
// ============================================================================

fn bench_estimate(c: &mut Criterion) {
    let mut group = c.benchmark_group("theta_estimate");

    for lg_k in [10, 12, 14] {
        let mut sketch = ThetaSketch::new(lg_k).unwrap();
        let k = 1_usize << lg_k;

        // Fill sketch
        for i in 0..k {
            sketch.update(&i);
        }

        group.bench_with_input(
            BenchmarkId::new("exact_mode", format!("lg_k={}", lg_k)),
            &sketch,
            |b, sketch| {
                b.iter(|| black_box(sketch.estimate()));
            },
        );
    }

    // Test estimate in sampling mode
    let mut sketch = ThetaSketch::new(8).unwrap(); // Small k
    for i in 0..100_000 {
        sketch.update(&i);
    }

    group.bench_function("sampling_mode", |b| {
        b.iter(|| black_box(sketch.estimate()));
    });

    group.finish();
}

// ============================================================================
// End-to-End Workflow Benchmarks
// ============================================================================

fn bench_workflow_set_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("theta_workflow");

    // Real-world scenario: compute |A∪B|, |A∩B|, |A-B|
    group.bench_function("all_set_operations", |b| {
        let mut sketch_a = ThetaSketch::new(12).unwrap();
        let mut sketch_b = ThetaSketch::new(12).unwrap();

        for i in 0..1000 {
            sketch_a.update(&i);
        }
        for i in 500..1500 {
            sketch_b.update(&i);
        }

        b.iter(|| {
            let union = sketch_a.union(black_box(&sketch_b)).unwrap();
            let intersection = sketch_a.intersect(black_box(&sketch_b)).unwrap();
            let difference = sketch_a.difference(black_box(&sketch_b)).unwrap();

            black_box((
                union.estimate(),
                intersection.estimate(),
                difference.estimate(),
            ))
        });
    });

    group.finish();
}

// ============================================================================
// Memory Usage Benchmarks
// ============================================================================

fn bench_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("theta_memory");

    // Measure allocation time for different sizes
    for lg_k in [8, 10, 12, 14, 16] {
        group.bench_with_input(
            BenchmarkId::new("new", format!("lg_k={}", lg_k)),
            &lg_k,
            |b, &lg_k| {
                b.iter(|| black_box(ThetaSketch::new(lg_k).unwrap()));
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_update,
    bench_union,
    bench_intersect,
    bench_difference,
    bench_estimate,
    bench_workflow_set_operations,
    bench_memory,
);

criterion_main!(benches);
