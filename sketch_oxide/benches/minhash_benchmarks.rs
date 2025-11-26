//! Benchmarks for MinHash
//!
//! Performance targets:
//! - Update: <100ns per item (k hash operations)
//! - Jaccard similarity: <1µs (k comparisons)
//! - Merge: <1µs (k min operations)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sketch_oxide::similarity::MinHash;
use sketch_oxide::Mergeable;

fn bench_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("minhash_construction");

    for &num_perm in &[64, 128, 256, 512] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("k={}", num_perm)),
            &num_perm,
            |b, &k| {
                b.iter(|| {
                    let mh = MinHash::new(k).unwrap();
                    black_box(mh);
                });
            },
        );
    }

    group.finish();
}

fn bench_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("minhash_update");
    group.throughput(Throughput::Elements(1));

    // Benchmark with different num_perm values
    for &num_perm in &[64, 128, 256, 512] {
        let mut mh = MinHash::new(num_perm).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("k={}", num_perm)),
            &num_perm,
            |b, _| {
                let mut counter = 0u64;
                b.iter(|| {
                    mh.update(&counter);
                    counter = counter.wrapping_add(1);
                });
            },
        );
    }

    group.finish();
}

fn bench_update_different_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("minhash_update_types");
    group.throughput(Throughput::Elements(1));

    let mut mh = MinHash::new(128).unwrap();

    group.bench_function("u64", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            mh.update(&counter);
            counter = counter.wrapping_add(1);
        });
    });

    group.bench_function("string", |b| {
        let strings: Vec<String> = (0..1000).map(|i| format!("item_{}", i)).collect();
        let mut idx = 0;
        b.iter(|| {
            mh.update(&strings[idx % strings.len()]);
            idx = idx.wrapping_add(1);
        });
    });

    group.bench_function("str_ref", |b| {
        let mut counter = 0usize;
        b.iter(|| {
            let s = format!("item_{}", counter % 1000);
            mh.update(&s.as_str());
            counter = counter.wrapping_add(1);
        });
    });

    group.bench_function("vec", |b| {
        let mut counter = 0u64;
        b.iter(|| {
            let v = vec![counter, counter + 1, counter + 2];
            mh.update(&v);
            counter = counter.wrapping_add(1);
        });
    });

    group.finish();
}

fn bench_jaccard_similarity(c: &mut Criterion) {
    let mut group = c.benchmark_group("minhash_jaccard_similarity");

    for &num_perm in &[64, 128, 256, 512] {
        let mut mh1 = MinHash::new(num_perm).unwrap();
        let mut mh2 = MinHash::new(num_perm).unwrap();

        // Populate with some data
        for i in 0..100 {
            mh1.update(&i);
        }
        for i in 50..150 {
            mh2.update(&i);
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("k={}", num_perm)),
            &num_perm,
            |b, _| {
                b.iter(|| {
                    let sim = mh1.jaccard_similarity(&mh2).unwrap();
                    black_box(sim);
                });
            },
        );
    }

    group.finish();
}

fn bench_jaccard_similarity_varying_overlap(c: &mut Criterion) {
    let mut group = c.benchmark_group("minhash_similarity_overlap");
    let num_perm = 128;

    // Test with different overlap scenarios
    for &overlap_percent in &[0, 25, 50, 75, 100] {
        let mut mh1 = MinHash::new(num_perm).unwrap();
        let mut mh2 = MinHash::new(num_perm).unwrap();

        // Create sets with specified overlap
        let set_size = 1000;
        let overlap_size = (set_size * overlap_percent) / 100;

        for i in 0..set_size {
            mh1.update(&i);
        }
        for i in (set_size - overlap_size)..((set_size - overlap_size) + set_size) {
            mh2.update(&i);
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}% overlap", overlap_percent)),
            &overlap_percent,
            |b, _| {
                b.iter(|| {
                    let sim = mh1.jaccard_similarity(&mh2).unwrap();
                    black_box(sim);
                });
            },
        );
    }

    group.finish();
}

fn bench_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("minhash_merge");

    for &num_perm in &[64, 128, 256, 512] {
        let mut mh1 = MinHash::new(num_perm).unwrap();
        let mut mh2 = MinHash::new(num_perm).unwrap();

        // Populate with some data
        for i in 0..100 {
            mh1.update(&i);
        }
        for i in 50..150 {
            mh2.update(&i);
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("k={}", num_perm)),
            &num_perm,
            |b, _| {
                b.iter(|| {
                    let mut mh_copy = mh1.clone();
                    mh_copy.merge(&mh2).unwrap();
                    black_box(mh_copy);
                });
            },
        );
    }

    group.finish();
}

fn bench_batch_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("minhash_batch_update");

    for &num_perm in &[64, 128, 256] {
        for &batch_size in &[100, 1000, 10000] {
            let mut mh = MinHash::new(num_perm).unwrap();

            group.throughput(Throughput::Elements(batch_size));
            group.bench_with_input(
                BenchmarkId::from_parameter(format!("k={}_n={}", num_perm, batch_size)),
                &(num_perm, batch_size),
                |b, &(_, n)| {
                    b.iter(|| {
                        for i in 0..n {
                            mh.update(&i);
                        }
                    });
                },
            );
        }
    }

    group.finish();
}

fn bench_realistic_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("minhash_realistic");

    // Simulate document shingling
    let documents = vec![
        "the quick brown fox jumps over the lazy dog",
        "the fast brown fox leaps over the sleepy cat",
        "completely different text with no overlap",
    ];

    // Generate trigrams for each document
    let mut shingles: Vec<Vec<Vec<u8>>> = Vec::new();
    for doc in &documents {
        let doc_shingles: Vec<Vec<u8>> = doc.as_bytes().windows(3).map(|w| w.to_vec()).collect();
        shingles.push(doc_shingles);
    }

    group.bench_function("build_sketch", |b| {
        b.iter(|| {
            let mut mh = MinHash::new(128).unwrap();
            for shingle in &shingles[0] {
                mh.update(shingle);
            }
            black_box(mh);
        });
    });

    group.bench_function("compare_documents", |b| {
        let mut mh1 = MinHash::new(128).unwrap();
        let mut mh2 = MinHash::new(128).unwrap();

        for shingle in &shingles[0] {
            mh1.update(shingle);
        }
        for shingle in &shingles[1] {
            mh2.update(shingle);
        }

        b.iter(|| {
            let sim = mh1.jaccard_similarity(&mh2).unwrap();
            black_box(sim);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_construction,
    bench_update,
    bench_update_different_types,
    bench_jaccard_similarity,
    bench_jaccard_similarity_varying_overlap,
    bench_merge,
    bench_batch_update,
    bench_realistic_workload,
);
criterion_main!(benches);
