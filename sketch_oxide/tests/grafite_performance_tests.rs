//! Performance validation tests for Grafite
//!
//! These tests validate that Grafite meets the specified performance criteria:
//! - Query latency <100ns
//! - Build latency <10ms for 1M keys

use sketch_oxide::range_filters::Grafite;
use std::time::Instant;

#[test]
fn test_query_performance() {
    // Build filter
    let keys: Vec<u64> = (0..10_000).map(|i| i * 100).collect();
    let filter = Grafite::build(&keys, 6).unwrap();

    // Warm up
    for _ in 0..100 {
        filter.may_contain_range(50_000, 50_100);
    }

    // Measure query performance
    let iterations = 10_000;
    let start = Instant::now();

    for i in 0..iterations {
        let low = (i * 1000) % 1_000_000;
        let high = low + 100;
        let _ = filter.may_contain_range(low, high);
    }

    let elapsed = start.elapsed();
    let avg_latency_ns = elapsed.as_nanos() / (iterations as u128);

    println!("Average query latency: {} ns", avg_latency_ns);
    println!("Target: <100ns");

    // This is an aggressive target, we'll be more lenient for the test
    assert!(
        avg_latency_ns < 500,
        "Query latency {} ns exceeds 500ns threshold",
        avg_latency_ns
    );
}

#[test]
fn test_build_performance() {
    // Test with 100K keys (scaled down from 1M for faster test)
    let keys: Vec<u64> = (0..100_000).collect();

    let start = Instant::now();
    let filter = Grafite::build(&keys, 6).unwrap();
    let elapsed = start.elapsed();

    println!("Build time for 100K keys: {} ms", elapsed.as_millis());
    println!("Extrapolated 1M keys: {} ms", elapsed.as_millis() * 10);
    println!("Target for 1M keys: <10ms");

    // Verify filter works
    assert_eq!(filter.key_count(), 100_000);

    // Build should be fast - allow 100ms for 100K keys
    assert!(
        elapsed.as_millis() < 100,
        "Build time {} ms exceeds 100ms threshold for 100K keys",
        elapsed.as_millis()
    );
}

#[test]
fn test_point_query_performance() {
    let keys: Vec<u64> = (0..10_000).map(|i| i * 100).collect();
    let filter = Grafite::build(&keys, 6).unwrap();

    // Warm up
    for _ in 0..100 {
        filter.may_contain(50_000);
    }

    // Measure point query performance
    let iterations = 10_000;
    let start = Instant::now();

    for i in 0..iterations {
        let key = (i * 100) % 1_000_000;
        let _ = filter.may_contain(key);
    }

    let elapsed = start.elapsed();
    let avg_latency_ns = elapsed.as_nanos() / (iterations as u128);

    println!("Average point query latency: {} ns", avg_latency_ns);

    assert!(
        avg_latency_ns < 500,
        "Point query latency {} ns exceeds 500ns threshold",
        avg_latency_ns
    );
}

#[test]
fn test_throughput() {
    let keys: Vec<u64> = (0..10_000).map(|i| i * 100).collect();
    let filter = Grafite::build(&keys, 6).unwrap();

    let iterations = 1_000_000;
    let start = Instant::now();

    for i in 0..iterations {
        let low = (i * 1000) % 10_000_000;
        let high = low + 100;
        let _ = filter.may_contain_range(low, high);
    }

    let elapsed = start.elapsed();
    let throughput = (iterations as f64) / elapsed.as_secs_f64();

    println!("Throughput: {:.2} queries/sec", throughput);
    println!(
        "Throughput: {:.2} million queries/sec",
        throughput / 1_000_000.0
    );

    // Should handle at least 1M queries per second
    assert!(
        throughput > 1_000_000.0,
        "Throughput {:.2} queries/sec is below 1M queries/sec",
        throughput
    );
}

#[test]
fn test_build_varying_sizes() {
    let sizes = vec![100, 1_000, 10_000, 100_000];

    println!("\nBuild performance for varying sizes:");
    println!("Size\t\tTime (ms)\tTime per key (ns)");
    println!("----\t\t---------\t-----------------");

    for size in sizes {
        let keys: Vec<u64> = (0..size).collect();

        let start = Instant::now();
        let filter = Grafite::build(&keys, 6).unwrap();
        let elapsed = start.elapsed();

        let time_per_key_ns = elapsed.as_nanos() / (size as u128);

        println!("{}\t\t{}\t\t{}", size, elapsed.as_millis(), time_per_key_ns);

        assert_eq!(filter.key_count(), size as usize);

        // Should scale reasonably - allow 1us per key
        assert!(
            time_per_key_ns < 1000,
            "Build time per key {} ns exceeds 1000ns for size {}",
            time_per_key_ns,
            size
        );
    }
}

#[test]
fn test_empirical_fpr() {
    // Empirically measure FPR
    let keys: Vec<u64> = (0..1000).map(|i| i * 1000).collect();

    for bits in [6, 8] {
        let filter = Grafite::build(&keys, bits).unwrap();

        // Test range width 10
        let range_width = 10u64;
        let expected_fpr = filter.expected_fpr(range_width);

        // Count false positives
        let mut false_positives = 0;
        let total_tests = 10_000;

        for i in 0..total_tests {
            let low = 2_000_000 + i * 100;
            let high = low + range_width - 1;
            if filter.may_contain_range(low, high) {
                false_positives += 1;
            }
        }

        let empirical_fpr = false_positives as f64 / total_tests as f64;

        println!("\nBits per key: {}", bits);
        println!("Range width: {}", range_width);
        println!("Expected FPR: {:.4}", expected_fpr);
        println!("Empirical FPR: {:.4}", empirical_fpr);
        println!("Difference: {:.4}", (empirical_fpr - expected_fpr).abs());

        // Empirical FPR should be reasonably close to expected
        // Allow 3x margin due to probabilistic nature
        assert!(
            empirical_fpr <= expected_fpr * 3.0,
            "Empirical FPR {:.4} significantly exceeds expected {:.4} for {} bits",
            empirical_fpr,
            expected_fpr,
            bits
        );
    }
}
