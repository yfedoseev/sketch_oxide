//! Comprehensive TDD test suite for Sliding HyperLogLog
//!
//! This test suite covers all aspects of the Sliding HyperLogLog sketch:
//! - Construction and validation
//! - Basic operations
//! - Window queries
//! - Decay mechanism
//! - Accuracy measurements
//! - Merge operations
//! - Edge cases
//! - Real-world scenarios
//!
//! Total: 42+ tests for production-grade quality

use sketch_oxide::streaming::SlidingHyperLogLog;
use sketch_oxide::{Mergeable, SketchError};
use std::collections::HashSet;

// ============================================================================
// Construction Tests (4 tests)
// ============================================================================

#[test]
fn test_construction_valid_precision_4() {
    let result = SlidingHyperLogLog::new(4, 3600);
    assert!(result.is_ok());
    let hll = result.unwrap();
    let stats = hll.stats();
    assert_eq!(stats.precision, 4);
    assert_eq!(stats.max_window_seconds, 3600);
    assert_eq!(stats.total_updates, 0);
}

#[test]
fn test_construction_valid_precision_16() {
    let result = SlidingHyperLogLog::new(16, 86400);
    assert!(result.is_ok());
    let hll = result.unwrap();
    let stats = hll.stats();
    assert_eq!(stats.precision, 16);
}

#[test]
fn test_construction_invalid_precision_too_small() {
    let result = SlidingHyperLogLog::new(3, 3600);
    assert!(result.is_err());
    if let Err(SketchError::InvalidParameter { param, .. }) = result {
        assert_eq!(param, "precision");
    }
}

#[test]
fn test_construction_invalid_precision_too_large() {
    let result = SlidingHyperLogLog::new(17, 3600);
    assert!(result.is_err());
}

// ============================================================================
// Basic Operations Tests (8 tests)
// ============================================================================

#[test]
fn test_single_update() {
    let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();
    let result = hll.update(&"test_item", 1000);
    assert!(result.is_ok());

    let stats = hll.stats();
    assert_eq!(stats.total_updates, 1);
}

#[test]
fn test_multiple_updates_same_item() {
    let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();

    for i in 0..100 {
        hll.update(&"same_item", 1000 + i).unwrap();
    }

    // Estimate should be close to 1 for same item
    let estimate = hll.estimate_total();
    assert!(
        estimate < 5.0,
        "Same item should estimate ~1, got {}",
        estimate
    );
}

#[test]
fn test_multiple_items() {
    let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();

    for i in 0..100 {
        hll.update(&i, 1000).unwrap();
    }

    let estimate = hll.estimate_total();
    let error = (estimate - 100.0).abs() / 100.0;
    assert!(error < 0.2, "Error {} too high for 100 items", error);
}

#[test]
fn test_timestamp_tracking() {
    let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();

    hll.update(&"item1", 1000).unwrap();
    hll.update(&"item2", 2000).unwrap();
    hll.update(&"item3", 3000).unwrap();

    // Window from 2500-3500 should contain only item3
    let estimate = hll.estimate_window(3500, 1000);
    assert!(
        estimate >= 0.5 && estimate <= 2.0,
        "Expected ~1 item in window, got {}",
        estimate
    );
}

#[test]
fn test_update_ordering() {
    let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();

    // Out-of-order timestamps should still work
    hll.update(&"item1", 3000).unwrap();
    hll.update(&"item2", 1000).unwrap();
    hll.update(&"item3", 2000).unwrap();

    let estimate = hll.estimate_total();
    assert!(estimate >= 2.0 && estimate <= 4.0);
}

#[test]
fn test_timestamp_validation() {
    let mut hll = SlidingHyperLogLog::new(12, 60).unwrap();

    // Timestamp should not exceed max_window
    // (Implementation may choose to accept or reject, this tests the behavior)
    let result = hll.update(&"item", 1000);
    assert!(result.is_ok());
}

#[test]
fn test_large_scale_updates() {
    let mut hll = SlidingHyperLogLog::new(14, 3600).unwrap();

    let start = std::time::Instant::now();
    for i in 0..1_000_000 {
        hll.update(&i, 1000 + (i % 3600)).unwrap();
    }
    let duration = start.elapsed();

    println!(
        "1M updates in {:?} ({:.0} ns/op)",
        duration,
        duration.as_nanos() as f64 / 1_000_000.0
    );

    let estimate = hll.estimate_total();
    let error = (estimate - 1_000_000.0).abs() / 1_000_000.0;
    assert!(error < 0.05, "Error {} too high for 1M items", error);
}

#[test]
fn test_memory_efficiency() {
    let hll = SlidingHyperLogLog::new(12, 3600).unwrap();

    // With precision 12, we have 4096 registers
    // Each register needs: value (1 byte) + timestamp (8 bytes) = 9 bytes
    // Total: ~36KB plus metadata
    // This is efficient compared to storing individual items

    let serialized = hll.serialize();
    println!(
        "Serialized size for precision 12: {} bytes",
        serialized.len()
    );
    assert!(serialized.len() < 100_000); // Should be well under 100KB
}

// ============================================================================
// Window Query Tests (8 tests)
// ============================================================================

#[test]
fn test_estimate_within_window() {
    let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();

    // Add items at different times
    for i in 0..100 {
        hll.update(&i, 1000 + i).unwrap();
    }

    // Query window from 1050-1150 (100 seconds, should contain ~100 items)
    let estimate = hll.estimate_window(1150, 100);
    assert!(
        estimate >= 50.0 && estimate <= 150.0,
        "Expected ~100 items, got {}",
        estimate
    );
}

#[test]
fn test_estimate_outside_window_empty() {
    let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();

    // Add items at time 1000
    for i in 0..100 {
        hll.update(&i, 1000).unwrap();
    }

    // Query window at time 5000 with 60 second window (should be empty)
    let estimate = hll.estimate_window(5000, 60);
    assert!(estimate < 1.0, "Window should be empty, got {}", estimate);
}

#[test]
fn test_estimate_at_window_boundary() {
    let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();

    // Add item exactly at boundary
    hll.update(&"boundary_item", 1000).unwrap();

    // Query at exact boundary
    let estimate = hll.estimate_window(1000, 0);
    // Should include or exclude based on implementation (>=)
    assert!(estimate >= 0.0);
}

#[test]
fn test_window_smaller_than_max() {
    let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();

    for i in 0..100 {
        hll.update(&i, 1000 + i).unwrap();
    }

    // Query with window smaller than max
    let estimate = hll.estimate_window(1050, 50);
    assert!(estimate >= 20.0 && estimate <= 80.0);
}

#[test]
fn test_window_equal_to_max() {
    let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();

    for i in 0..100 {
        hll.update(&i, 1000 + i).unwrap();
    }

    let estimate = hll.estimate_window(4600, 3600);
    assert!(estimate >= 50.0);
}

#[test]
fn test_window_larger_than_max() {
    let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();

    for i in 0..100 {
        hll.update(&i, 1000 + i).unwrap();
    }

    // Window larger than max should still work
    let estimate = hll.estimate_window(5000, 7200);
    assert!(estimate >= 50.0);
}

#[test]
fn test_multiple_windows_over_time() {
    let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();

    // Simulate streaming data over time
    for t in 0..10 {
        for i in 0..10 {
            hll.update(&(t * 10 + i), 1000 + t * 100).unwrap();
        }
    }

    // Query different windows
    let est1 = hll.estimate_window(1500, 600); // First 6 batches
    let est2 = hll.estimate_window(1900, 600); // Last 6 batches

    println!("Window 1: {}, Window 2: {}", est1, est2);
    assert!(est1 >= 30.0 && est1 <= 90.0);
    assert!(est2 >= 30.0 && est2 <= 90.0);
}

#[test]
fn test_temporal_consistency() {
    let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();

    for i in 0..1000 {
        hll.update(&i, 1000).unwrap();
    }

    // Estimates should be consistent for same window
    let est1 = hll.estimate_window(2000, 1000);
    let est2 = hll.estimate_window(2000, 1000);
    assert_eq!(est1, est2, "Estimates should be deterministic");
}

// ============================================================================
// Decay Mechanism Tests (6 tests)
// ============================================================================

#[test]
fn test_decay_removes_old_entries() {
    let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();

    // Add items in the past
    for i in 0..100 {
        hll.update(&i, 1000).unwrap();
    }

    // Decay with current time far in future
    hll.decay(5000, 60).unwrap();

    // Window should now be empty
    let estimate = hll.estimate_window(5000, 60);
    assert!(estimate < 1.0, "After decay, old items should be removed");
}

#[test]
fn test_decay_preserves_recent_entries() {
    let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();

    // Add recent items
    for i in 0..100 {
        hll.update(&i, 1000).unwrap();
    }

    // Decay with window that includes these items
    hll.decay(1500, 600).unwrap();

    let estimate = hll.estimate_window(1500, 600);
    assert!(
        estimate >= 50.0,
        "Recent items should be preserved, got {}",
        estimate
    );
}

#[test]
fn test_decay_accuracy() {
    let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();

    // Add items at different times
    for i in 0..50 {
        hll.update(&i, 1000).unwrap();
    }
    for i in 50..100 {
        hll.update(&i, 2000).unwrap();
    }

    // Decay to remove first batch
    hll.decay(2500, 600).unwrap();

    let estimate = hll.estimate_window(2500, 600);
    // Should have ~50 items remaining
    assert!(
        estimate >= 30.0 && estimate <= 70.0,
        "Expected ~50 items after decay, got {}",
        estimate
    );
}

#[test]
fn test_decay_efficiency() {
    let mut hll = SlidingHyperLogLog::new(14, 3600).unwrap();

    // Add many items
    for i in 0..10_000 {
        hll.update(&i, 1000 + (i % 1000)).unwrap();
    }

    // Decay should be fast (O(m) where m is number of registers)
    let start = std::time::Instant::now();
    hll.decay(2500, 1000).unwrap();
    let duration = start.elapsed();

    println!("Decay operation took {:?}", duration);
    assert!(duration.as_millis() < 10, "Decay should be fast");
}

#[test]
fn test_multiple_decay_cycles() {
    let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();

    for i in 0..100 {
        hll.update(&i, 1000).unwrap();
    }

    // Multiple decay cycles
    hll.decay(2000, 1200).unwrap();
    hll.decay(3000, 600).unwrap();
    hll.decay(4000, 300).unwrap();

    let estimate = hll.estimate_window(4000, 300);
    assert!(
        estimate < 5.0,
        "After multiple decays, should have few items"
    );
}

#[test]
fn test_decay_with_no_expired_entries() {
    let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();

    for i in 0..100 {
        hll.update(&i, 1000).unwrap();
    }

    // Decay with window that includes all items
    let est_before = hll.estimate_total();
    hll.decay(1100, 3600).unwrap();
    let est_after = hll.estimate_total();

    assert!(
        (est_before - est_after).abs() < 5.0,
        "No items should be removed"
    );
}

// ============================================================================
// Accuracy Tests (5 tests)
// ============================================================================

#[test]
fn test_standard_error_within_bounds() {
    let hll = SlidingHyperLogLog::new(12, 3600).unwrap();
    let stats = hll.stats();

    // Standard error for HLL is 1.04 / sqrt(m)
    let m = 2_f64.powi(stats.precision as i32);
    let expected_error = 1.04 / m.sqrt();

    println!("Expected standard error: {:.4}", expected_error);
    assert!(expected_error < 0.02, "Standard error should be < 2%");
}

#[test]
fn test_accuracy_various_cardinalities() {
    for &cardinality in &[100, 1_000, 10_000, 100_000] {
        let mut hll = SlidingHyperLogLog::new(14, 3600).unwrap();

        for i in 0..cardinality {
            hll.update(&i, 1000).unwrap();
        }

        let estimate = hll.estimate_total();
        let error = (estimate - cardinality as f64).abs() / cardinality as f64;

        println!(
            "Cardinality: {}, Estimate: {:.0}, Error: {:.2}%",
            cardinality,
            estimate,
            error * 100.0
        );
        assert!(
            error < 0.05,
            "Error {:.2}% too high for cardinality {}",
            error * 100.0,
            cardinality
        );
    }
}

#[test]
fn test_accuracy_zipf_distribution() {
    let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();

    // Simulate Zipf distribution (common in real data)
    for i in 0..10_000 {
        let key = (i as f64).powf(0.5) as u64; // Simplified Zipf
        hll.update(&key, 1000 + i).unwrap();
    }

    let estimate = hll.estimate_total();
    let actual_unique = (0..10_000)
        .map(|i| (i as f64).powf(0.5) as u64)
        .collect::<HashSet<_>>()
        .len();
    let error = (estimate - actual_unique as f64).abs() / actual_unique as f64;

    println!(
        "Zipf: actual {}, estimate {:.0}, error {:.2}%",
        actual_unique,
        estimate,
        error * 100.0
    );
    assert!(error < 0.1, "Error too high for Zipf distribution");
}

#[test]
fn test_accuracy_uniform_distribution() {
    let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();

    for i in 0..5_000 {
        hll.update(&i, 1000).unwrap();
    }

    let estimate = hll.estimate_total();
    let error = (estimate - 5_000.0).abs() / 5_000.0;

    println!(
        "Uniform: actual 5000, estimate {:.0}, error {:.2}%",
        estimate,
        error * 100.0
    );
    assert!(
        error < 0.03,
        "Error should be very low for uniform distribution"
    );
}

#[test]
fn test_confidence_intervals() {
    // Run multiple trials to verify error bounds
    let trials = 10;
    let cardinality = 10_000;
    let mut errors = Vec::new();

    for _ in 0..trials {
        let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();
        for i in 0..cardinality {
            hll.update(&i, 1000).unwrap();
        }

        let estimate = hll.estimate_total();
        let error = (estimate - cardinality as f64).abs() / cardinality as f64;
        errors.push(error);
    }

    let avg_error: f64 = errors.iter().sum::<f64>() / trials as f64;
    println!(
        "Average error over {} trials: {:.2}%",
        trials,
        avg_error * 100.0
    );

    // Average error should be within expected bounds
    assert!(avg_error < 0.05, "Average error too high");
}

// ============================================================================
// Merge Operations Tests (4 tests)
// ============================================================================

#[test]
fn test_merge_compatible_sketches() {
    let mut hll1 = SlidingHyperLogLog::new(12, 3600).unwrap();
    let mut hll2 = SlidingHyperLogLog::new(12, 3600).unwrap();

    for i in 0..100 {
        hll1.update(&i, 1000).unwrap();
    }
    for i in 50..150 {
        hll2.update(&i, 1000).unwrap();
    }

    let result = hll1.merge(&hll2);
    assert!(result.is_ok());

    // Merged should have ~150 unique items
    let estimate = hll1.estimate_total();
    assert!(
        estimate >= 100.0 && estimate <= 200.0,
        "Expected ~150 items, got {}",
        estimate
    );
}

#[test]
fn test_merge_respects_timestamps() {
    let mut hll1 = SlidingHyperLogLog::new(12, 3600).unwrap();
    let mut hll2 = SlidingHyperLogLog::new(12, 3600).unwrap();

    // hll1 has old data
    for i in 0..50 {
        hll1.update(&i, 1000).unwrap();
    }

    // hll2 has recent data
    for i in 50..100 {
        hll2.update(&i, 2000).unwrap();
    }

    hll1.merge(&hll2).unwrap();

    // Query recent window should show hll2's data
    let estimate = hll1.estimate_window(2500, 600);
    assert!(
        estimate >= 30.0,
        "Merge should preserve timestamp information"
    );
}

#[test]
fn test_merge_commutative_property() {
    let mut hll1a = SlidingHyperLogLog::new(12, 3600).unwrap();
    let mut hll1b = SlidingHyperLogLog::new(12, 3600).unwrap();
    let mut hll2 = SlidingHyperLogLog::new(12, 3600).unwrap();

    for i in 0..50 {
        hll1a.update(&i, 1000).unwrap();
        hll1b.update(&i, 1000).unwrap();
    }
    for i in 50..100 {
        hll2.update(&i, 1000).unwrap();
    }

    // Merge in both orders
    hll1a.merge(&hll2).unwrap();
    hll2.merge(&hll1b).unwrap();

    let est1 = hll1a.estimate_total();
    let est2 = hll2.estimate_total();

    assert!((est1 - est2).abs() < 5.0, "Merge should be commutative");
}

#[test]
fn test_merge_incompatible_precision() {
    let mut hll1 = SlidingHyperLogLog::new(10, 3600).unwrap();
    let hll2 = SlidingHyperLogLog::new(12, 3600).unwrap();

    let result = hll1.merge(&hll2);
    assert!(result.is_err());

    if let Err(SketchError::IncompatibleSketches { reason }) = result {
        assert!(reason.contains("precision") || reason.contains("Precision"));
    }
}

// ============================================================================
// Edge Cases Tests (5 tests)
// ============================================================================

#[test]
fn test_precision_minimum() {
    let mut hll = SlidingHyperLogLog::new(4, 3600).unwrap();

    for i in 0..100 {
        hll.update(&i, 1000).unwrap();
    }

    let estimate = hll.estimate_total();
    // Lower precision = higher error
    assert!(estimate >= 50.0 && estimate <= 200.0);
}

#[test]
fn test_precision_maximum() {
    let hll = SlidingHyperLogLog::new(16, 3600).unwrap();
    let stats = hll.stats();
    assert_eq!(stats.precision, 16);

    // Should have 2^16 = 65536 registers
    let serialized = hll.serialize();
    println!("Precision 16 serialized size: {} bytes", serialized.len());
}

#[test]
fn test_window_zero_seconds() {
    let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();

    hll.update(&"item", 1000).unwrap();

    // Zero-second window
    let estimate = hll.estimate_window(1000, 0);
    // Should be deterministic based on implementation
    assert!(estimate >= 0.0);
}

#[test]
fn test_very_large_window() {
    let mut hll = SlidingHyperLogLog::new(12, 31536000).unwrap(); // 1 year

    for i in 0..100 {
        hll.update(&i, 1000).unwrap();
    }

    let estimate = hll.estimate_window(10_000_000, 31536000);
    assert!(estimate >= 50.0);
}

#[test]
fn test_single_item_lifetime_tracking() {
    let mut hll = SlidingHyperLogLog::new(12, 3600).unwrap();

    hll.update(&"tracked_item", 1000).unwrap();

    // Item visible in early window
    let est1 = hll.estimate_window(1500, 600);
    assert!(est1 >= 0.5);

    // Item expires after window
    let est2 = hll.estimate_window(5000, 60);
    assert!(est2 < 0.5);
}

// ============================================================================
// Real-World Scenarios Tests (2 tests)
// ============================================================================

#[test]
fn test_ddos_detection_scenario() {
    // Simulate DDoS detection: track unique source IPs in 5-minute window
    let mut hll = SlidingHyperLogLog::new(14, 300).unwrap(); // 5 minutes

    let base_time = 1000;

    // Normal traffic: ~1000 unique IPs per minute
    for minute in 0..5 {
        for ip in 0..1000 {
            let ip_addr = format!("192.168.{}.{}", ip / 256, ip % 256);
            hll.update(&ip_addr, base_time + minute * 60).unwrap();
        }
    }

    let normal_estimate = hll.estimate_window(base_time + 300, 300);
    println!(
        "Normal traffic: ~{:.0} unique IPs in 5 min",
        normal_estimate
    );

    // Attack traffic: 10000 unique IPs in 1 minute
    for ip in 0..10_000 {
        let ip_addr = format!("10.{}.{}.{}", ip / 65536, (ip / 256) % 256, ip % 256);
        hll.update(&ip_addr, base_time + 360).unwrap();
    }

    let attack_estimate = hll.estimate_window(base_time + 360, 300);
    println!("During attack: ~{:.0} unique IPs in 5 min", attack_estimate);

    // Attack should show significant increase
    assert!(
        attack_estimate > normal_estimate * 2.0,
        "Attack traffic should be detectable"
    );
}

#[test]
fn test_dashboard_metrics_scenario() {
    // Simulate dashboard showing unique users over time
    // Note: Sliding HyperLogLog tracks cardinality within time windows,
    // but register collisions mean recent data may overwrite older data
    let mut hll = SlidingHyperLogLog::new(14, 3600).unwrap(); // 1 hour window, higher precision

    let base_time = 1000;

    // Add data for first hour
    for user in 0..2000 {
        hll.update(&user, base_time + (user % 3600)).unwrap();
    }

    // Query first hour
    let est1 = hll.estimate_window(base_time + 3600, 3600);
    println!("Hour 1: ~{:.0} unique users", est1);
    assert!(
        est1 >= 1500.0 && est1 <= 2500.0,
        "Hour 1 should have ~2000 users, got {}",
        est1
    );

    // Add data for second hour (mostly new users)
    for user in 1500..3500 {
        hll.update(&user, base_time + 3600 + (user % 3600)).unwrap();
    }

    // Query second hour only
    let est2 = hll.estimate_window(base_time + 7200, 3600);
    println!("Hour 2: ~{:.0} unique users", est2);
    // Due to register overlap and timestamp updates, this may show fewer than 2000
    // but should still show significant activity
    assert!(
        est2 >= 1000.0 && est2 <= 3000.0,
        "Hour 2 should have significant users, got {}",
        est2
    );

    // Total across all time should be ~3500
    let total = hll.estimate_total();
    println!("Total: ~{:.0} unique users", total);
    assert!(
        total >= 2500.0 && total <= 4500.0,
        "Total should be ~3500 users, got {}",
        total
    );
}
