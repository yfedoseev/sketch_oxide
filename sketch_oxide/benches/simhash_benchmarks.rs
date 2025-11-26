use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use sketch_oxide::similarity::SimHash;

/// Benchmark: Update operations
fn bench_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("simhash_update");

    group.bench_function("single_feature", |b| {
        let mut sh = SimHash::new();
        let mut counter = 0;
        b.iter(|| {
            sh.update(black_box(&format!("word_{}", counter)));
            counter += 1;
        });
    });

    group.bench_function("weighted_feature", |b| {
        let mut sh = SimHash::new();
        let mut counter = 0;
        b.iter(|| {
            sh.update_weighted(black_box(&format!("word_{}", counter)), black_box(5));
            counter += 1;
        });
    });

    group.finish();
}

/// Benchmark: Fingerprint computation
fn bench_fingerprint(c: &mut Criterion) {
    let mut group = c.benchmark_group("simhash_fingerprint");

    for size in [10, 100, 1000] {
        let mut sh = SimHash::new();
        for i in 0..size {
            sh.update(&format!("word_{}", i));
        }

        group.bench_with_input(BenchmarkId::new("compute", size), &size, |b, _| {
            b.iter(|| {
                let mut sh_clone = sh.clone();
                black_box(sh_clone.fingerprint());
            });
        });
    }

    group.finish();
}

/// Benchmark: Hamming distance computation
fn bench_hamming_distance(c: &mut Criterion) {
    let mut group = c.benchmark_group("simhash_hamming");

    let mut sh1 = SimHash::new();
    let mut sh2 = SimHash::new();

    for i in 0..100 {
        sh1.update(&format!("word_{}", i));
        sh2.update(&format!("word_{}", i + 50));
    }

    group.bench_function("distance", |b| {
        let mut sh1_clone = sh1.clone();
        let mut sh2_clone = sh2.clone();
        b.iter(|| {
            black_box(sh1_clone.hamming_distance(&mut sh2_clone));
        });
    });

    group.bench_function("similarity", |b| {
        let mut sh1_clone = sh1.clone();
        let mut sh2_clone = sh2.clone();
        b.iter(|| {
            black_box(sh1_clone.similarity(&mut sh2_clone));
        });
    });

    group.finish();
}

/// Benchmark: Full document processing
fn bench_document_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("simhash_document");

    let document = "The quick brown fox jumps over the lazy dog. \
                    This is a sample document for testing SimHash performance. \
                    SimHash is used for near-duplicate detection in web crawling.";

    group.bench_function("process_document", |b| {
        b.iter(|| {
            let mut sh = SimHash::new();
            for word in document.split_whitespace() {
                sh.update(black_box(word));
            }
            black_box(sh.fingerprint());
        });
    });

    group.finish();
}

/// Benchmark: Near-duplicate comparison
fn bench_near_duplicate(c: &mut Criterion) {
    let mut group = c.benchmark_group("simhash_near_duplicate");

    group.bench_function("compare_1000_docs", |b| {
        // Pre-compute fingerprints for 1000 documents
        let fingerprints: Vec<u64> = (0..1000)
            .map(|i| {
                let mut sh = SimHash::new();
                for j in 0..50 {
                    sh.update(&format!("doc_{}_word_{}", i, j));
                }
                sh.fingerprint()
            })
            .collect();

        b.iter(|| {
            // Find all pairs with Hamming distance <= 3
            let mut near_duplicates = 0;
            for i in 0..fingerprints.len() {
                for j in (i + 1)..fingerprints.len() {
                    let distance = SimHash::hamming_distance_from_fingerprints(
                        fingerprints[i],
                        fingerprints[j],
                    );
                    if distance <= 3 {
                        near_duplicates += 1;
                    }
                }
            }
            black_box(near_duplicates);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_update,
    bench_fingerprint,
    bench_hamming_distance,
    bench_document_processing,
    bench_near_duplicate,
);

criterion_main!(benches);
