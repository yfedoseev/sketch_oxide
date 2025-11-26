//! "Us vs Them" Comparison Benchmarks
//!
//! Compares sketch_oxide performance against other popular Rust probabilistic
//! data structure libraries:
//! - pdatastructs (HyperLogLog, CountMinSketch, BloomFilter)
//! - probabilistic-collections (BloomFilter, CuckooFilter)
//! - streaming_algorithms (HyperLogLog, CountMinSketch)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

// Our implementations
use sketch_oxide::cardinality::{HyperLogLog, UltraLogLog};
use sketch_oxide::frequency::CountMinSketch;
use sketch_oxide::membership::{BloomFilter, CuckooFilter};
use sketch_oxide::Sketch;

// Comparison implementations
use pdatastructs::countminsketch::CountMinSketch as PdataCMS;
use pdatastructs::hyperloglog::HyperLogLog as PdataHLL;
use probabilistic_collections::bloom::BloomFilter as ProbCollBloom;
use probabilistic_collections::cuckoo::CuckooFilter as ProbCollCuckoo;
use streaming_algorithms::CountMinSketch as StreamingCMS;
use streaming_algorithms::HyperLogLog as StreamingHLL;

/// Generate test data
fn generate_data(count: usize) -> Vec<u64> {
    (0..count as u64).collect()
}

fn generate_string_data(count: usize) -> Vec<String> {
    (0..count).map(|i| format!("item_{}", i)).collect()
}

// =============================================================================
// HYPERLOGLOG BENCHMARKS
// =============================================================================

fn bench_hyperloglog_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("hyperloglog_insert");
    let data = generate_data(100_000);
    group.throughput(Throughput::Elements(data.len() as u64));

    // sketch_oxide HyperLogLog (precision 12)
    group.bench_function("sketch_oxide::HyperLogLog", |b| {
        b.iter(|| {
            let mut hll = HyperLogLog::new(12).unwrap();
            for item in &data {
                hll.update(black_box(item));
            }
            hll.estimate()
        })
    });

    // sketch_oxide UltraLogLog (precision 12) - our SOTA implementation
    group.bench_function("sketch_oxide::UltraLogLog", |b| {
        b.iter(|| {
            let mut ull = UltraLogLog::new(12).unwrap();
            for item in &data {
                ull.update(black_box(item));
            }
            ull.estimate()
        })
    });

    // pdatastructs HyperLogLog (address_bits 12 = 4096 registers)
    group.bench_function("pdatastructs::HyperLogLog", |b| {
        b.iter(|| {
            let mut hll = PdataHLL::<u64>::new(12);
            for item in &data {
                hll.add(black_box(item));
            }
            hll.count()
        })
    });

    // streaming_algorithms HyperLogLog
    group.bench_function("streaming_algorithms::HyperLogLog", |b| {
        b.iter(|| {
            let mut hll = StreamingHLL::new(0.01); // ~1% error
            for item in &data {
                hll.push(black_box(item));
            }
            hll.len()
        })
    });

    group.finish();
}

fn bench_hyperloglog_accuracy(c: &mut Criterion) {
    let mut group = c.benchmark_group("hyperloglog_accuracy");

    for &size in &[10_000usize, 100_000, 1_000_000] {
        let data = generate_data(size);

        group.bench_with_input(
            BenchmarkId::new("sketch_oxide::HyperLogLog", size),
            &data,
            |b, data| {
                b.iter(|| {
                    let mut hll = HyperLogLog::new(14).unwrap();
                    for item in data {
                        hll.update(item);
                    }
                    let estimate = hll.estimate();
                    let error = (estimate - data.len() as f64).abs() / data.len() as f64;
                    black_box((estimate, error))
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("sketch_oxide::UltraLogLog", size),
            &data,
            |b, data| {
                b.iter(|| {
                    let mut ull = UltraLogLog::new(14).unwrap();
                    for item in data {
                        ull.update(item);
                    }
                    let estimate = ull.estimate();
                    let error = (estimate - data.len() as f64).abs() / data.len() as f64;
                    black_box((estimate, error))
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("pdatastructs::HyperLogLog", size),
            &data,
            |b, data| {
                b.iter(|| {
                    let mut hll = PdataHLL::<u64>::new(14);
                    for item in data {
                        hll.add(item);
                    }
                    let estimate = hll.count() as f64;
                    let error = (estimate - data.len() as f64).abs() / data.len() as f64;
                    black_box((estimate as u64, error))
                })
            },
        );
    }

    group.finish();
}

// =============================================================================
// COUNT-MIN SKETCH BENCHMARKS
// =============================================================================

fn bench_countmin_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("countmin_insert");
    let data = generate_string_data(100_000);
    group.throughput(Throughput::Elements(data.len() as u64));

    // sketch_oxide CountMinSketch
    group.bench_function("sketch_oxide::CountMinSketch", |b| {
        b.iter(|| {
            let mut cms = CountMinSketch::new(0.001, 0.01).unwrap();
            for item in &data {
                cms.update(black_box(item));
            }
            cms
        })
    });

    // pdatastructs CountMinSketch (with_params: width, depth)
    group.bench_function("pdatastructs::CountMinSketch", |b| {
        b.iter(|| {
            let mut cms = PdataCMS::<String, u64>::with_params(2000, 7);
            for item in &data {
                cms.add(black_box(item));
            }
            cms
        })
    });

    // streaming_algorithms CountMinSketch
    // Note: streaming_algorithms uses (probability, tolerance) where:
    // - probability = success probability (1 - delta)
    // - tolerance = epsilon (error bound)
    // For fair comparison with epsilon=0.001, delta=0.01:
    // We need probability=0.99 (1-0.01), tolerance=0.001
    group.bench_function("streaming_algorithms::CountMinSketch", |b| {
        b.iter(|| {
            let mut cms: StreamingCMS<String, u64> = StreamingCMS::new(0.99, 0.001, ());
            for item in &data {
                cms.push(black_box(item), &1u64);
            }
            cms
        })
    });

    group.finish();
}

fn bench_countmin_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("countmin_query");
    let data = generate_string_data(100_000);
    let queries = generate_string_data(10_000);
    group.throughput(Throughput::Elements(queries.len() as u64));

    // Pre-populate sketches
    let mut our_cms = CountMinSketch::new(0.001, 0.01).unwrap();
    let mut pdata_cms = PdataCMS::<String, u64>::with_params(2000, 7);
    // Fair comparison: probability=0.99 (1-delta), tolerance=0.001 (epsilon)
    let mut stream_cms: StreamingCMS<String, u64> = StreamingCMS::new(0.99, 0.001, ());

    for item in &data {
        our_cms.update(item);
        pdata_cms.add(item);
        stream_cms.push(item, &1u64);
    }

    group.bench_function("sketch_oxide::CountMinSketch", |b| {
        b.iter(|| {
            let mut total = 0u64;
            for query in &queries {
                total += our_cms.estimate(black_box(query));
            }
            total
        })
    });

    group.bench_function("pdatastructs::CountMinSketch", |b| {
        b.iter(|| {
            let mut total = 0u64;
            for query in &queries {
                total += pdata_cms.query_point(black_box(query));
            }
            total
        })
    });

    group.bench_function("streaming_algorithms::CountMinSketch", |b| {
        b.iter(|| {
            let mut total = 0u64;
            for query in &queries {
                total += stream_cms.get(black_box(query));
            }
            total
        })
    });

    group.finish();
}

// =============================================================================
// BLOOM FILTER BENCHMARKS
// =============================================================================

fn bench_bloom_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("bloom_insert");
    let data = generate_string_data(100_000);
    group.throughput(Throughput::Elements(data.len() as u64));

    // sketch_oxide BloomFilter (1% false positive rate)
    group.bench_function("sketch_oxide::BloomFilter", |b| {
        b.iter(|| {
            let mut bloom = BloomFilter::new(100_000, 0.01);
            for item in &data {
                bloom.insert(black_box(item.as_bytes()));
            }
            bloom
        })
    });

    // probabilistic-collections BloomFilter
    group.bench_function("probabilistic_collections::BloomFilter", |b| {
        b.iter(|| {
            let mut bloom = ProbCollBloom::<String>::new(100_000, 0.01);
            for item in &data {
                bloom.insert(black_box(item));
            }
            bloom
        })
    });

    group.finish();
}

fn bench_bloom_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("bloom_query");
    let data = generate_string_data(100_000);
    let queries = generate_string_data(10_000);
    group.throughput(Throughput::Elements(queries.len() as u64));

    // Pre-populate filters
    let mut our_bloom = BloomFilter::new(100_000, 0.01);
    let mut prob_bloom = ProbCollBloom::<String>::new(100_000, 0.01);

    for item in &data {
        our_bloom.insert(item.as_bytes());
        prob_bloom.insert(item);
    }

    group.bench_function("sketch_oxide::BloomFilter", |b| {
        b.iter(|| {
            let mut hits = 0u64;
            for query in &queries {
                if our_bloom.contains(black_box(query.as_bytes())) {
                    hits += 1;
                }
            }
            hits
        })
    });

    group.bench_function("probabilistic_collections::BloomFilter", |b| {
        b.iter(|| {
            let mut hits = 0u64;
            for query in &queries {
                if prob_bloom.contains(black_box(query)) {
                    hits += 1;
                }
            }
            hits
        })
    });

    group.finish();
}

// =============================================================================
// CUCKOO FILTER BENCHMARKS
// =============================================================================

fn bench_cuckoo_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("cuckoo_insert");
    let data = generate_string_data(50_000); // Smaller for cuckoo to avoid capacity issues
    group.throughput(Throughput::Elements(data.len() as u64));

    // sketch_oxide CuckooFilter
    group.bench_function("sketch_oxide::CuckooFilter", |b| {
        b.iter(|| {
            let mut cuckoo = CuckooFilter::new(100_000).unwrap();
            for item in &data {
                let _ = cuckoo.insert(black_box(item.as_bytes()));
            }
            cuckoo
        })
    });

    // probabilistic-collections CuckooFilter
    group.bench_function("probabilistic_collections::CuckooFilter", |b| {
        b.iter(|| {
            let mut cuckoo = ProbCollCuckoo::<String>::new(100_000);
            for item in &data {
                cuckoo.insert(black_box(item));
            }
            cuckoo
        })
    });

    group.finish();
}

fn bench_cuckoo_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("cuckoo_query");
    let data = generate_string_data(50_000);
    let queries = generate_string_data(10_000);
    group.throughput(Throughput::Elements(queries.len() as u64));

    // Pre-populate filters
    let mut our_cuckoo = CuckooFilter::new(100_000).unwrap();
    let mut prob_cuckoo = ProbCollCuckoo::<String>::new(100_000);

    for item in &data {
        let _ = our_cuckoo.insert(item.as_bytes());
        prob_cuckoo.insert(item);
    }

    group.bench_function("sketch_oxide::CuckooFilter", |b| {
        b.iter(|| {
            let mut hits = 0u64;
            for query in &queries {
                if our_cuckoo.contains(black_box(query.as_bytes())) {
                    hits += 1;
                }
            }
            hits
        })
    });

    group.bench_function("probabilistic_collections::CuckooFilter", |b| {
        b.iter(|| {
            let mut hits = 0u64;
            for query in &queries {
                if prob_cuckoo.contains(black_box(query)) {
                    hits += 1;
                }
            }
            hits
        })
    });

    group.finish();
}

// =============================================================================
// MEMORY USAGE COMPARISON
// =============================================================================

fn bench_memory_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_comparison");

    // Compare memory usage at similar accuracy levels
    group.bench_function("sketch_oxide::HyperLogLog_p12", |b| {
        b.iter(|| {
            let hll = HyperLogLog::new(12).unwrap();
            black_box(hll.to_bytes().len())
        })
    });

    group.bench_function("sketch_oxide::UltraLogLog_p12", |b| {
        b.iter(|| {
            let ull = UltraLogLog::new(12).unwrap();
            black_box(std::mem::size_of_val(&ull))
        })
    });

    group.bench_function("pdatastructs::HyperLogLog_b12", |b| {
        b.iter(|| {
            let hll = PdataHLL::<u64>::new(12);
            black_box(std::mem::size_of_val(&hll))
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    // HyperLogLog comparisons
    bench_hyperloglog_insert,
    bench_hyperloglog_accuracy,
    // Count-Min Sketch comparisons
    bench_countmin_insert,
    bench_countmin_query,
    // Bloom Filter comparisons
    bench_bloom_insert,
    bench_bloom_query,
    // Cuckoo Filter comparisons
    bench_cuckoo_insert,
    bench_cuckoo_query,
    // Memory comparison
    bench_memory_comparison,
);
criterion_main!(benches);
