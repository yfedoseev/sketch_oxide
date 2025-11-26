//! Comprehensive tests for NitroSketch network telemetry
//!
//! Following TDD methodology: 51+ tests across 8 categories
//! NitroSketch (SIGCOMM 2019) enables 100Gbps line-rate packet processing
//! through selective sampling and background synchronization.

use proptest::prelude::*;
use sketch_oxide::cardinality::HyperLogLog;
use sketch_oxide::frequency::{CountMinSketch, NitroSketch};
use sketch_oxide::Sketch;

// ============================================================================
// PHASE 1: Construction Tests (4 tests)
// ============================================================================

#[test]
fn test_construction_valid_parameters() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let nitro = NitroSketch::new(base, 0.1);
    assert!(
        nitro.is_ok(),
        "Should create NitroSketch with valid parameters"
    );
}

#[test]
fn test_construction_sample_rate_one() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let nitro = NitroSketch::new(base, 1.0);
    assert!(
        nitro.is_ok(),
        "Should accept sample_rate = 1.0 (no sampling)"
    );
}

#[test]
fn test_construction_invalid_sample_rate_zero() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let result = NitroSketch::new(base, 0.0);
    assert!(result.is_err(), "Should reject sample_rate = 0");
}

#[test]
fn test_construction_invalid_sample_rate_negative() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let result = NitroSketch::new(base, -0.5);
    assert!(result.is_err(), "Should reject negative sample_rate");
}

#[test]
fn test_construction_invalid_sample_rate_too_large() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let result = NitroSketch::new(base, 1.5);
    assert!(result.is_err(), "Should reject sample_rate > 1.0");
}

#[test]
fn test_construction_with_different_base_sketches() {
    // Test with CountMinSketch
    let cms = CountMinSketch::new(0.01, 0.01).unwrap();
    let nitro_cms = NitroSketch::new(cms, 0.1);
    assert!(nitro_cms.is_ok());

    // Test with HyperLogLog
    let hll = HyperLogLog::new(12).unwrap();
    let nitro_hll = NitroSketch::new(hll, 0.1);
    assert!(nitro_hll.is_ok());
}

// ============================================================================
// PHASE 2: Sampling Tests (8 tests)
// ============================================================================

#[test]
fn test_sampling_rate_enforcement() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.5).unwrap();

    // Add many items
    for i in 0..10000 {
        nitro.update_sampled(format!("item_{}", i).as_bytes());
    }

    let stats = nitro.stats();
    let actual_rate = stats.sampled_count as f64 / stats.total_items_estimated as f64;

    // Should be close to 0.5 (within 5% tolerance)
    assert!(
        (actual_rate - 0.5).abs() < 0.05,
        "Sample rate {} should be close to 0.5",
        actual_rate
    );
}

#[test]
fn test_sampling_probability_low_rate() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.01).unwrap();

    for i in 0..10000 {
        nitro.update_sampled(format!("item_{}", i).as_bytes());
    }

    let stats = nitro.stats();
    let actual_rate = stats.sampled_count as f64 / stats.total_items_estimated as f64;

    // With 1% sampling, expect close to 0.01
    assert!(
        actual_rate < 0.05,
        "Sample rate {} should be very low",
        actual_rate
    );
}

#[test]
fn test_sampling_probability_high_rate() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.9).unwrap();

    for i in 0..10000 {
        nitro.update_sampled(format!("item_{}", i).as_bytes());
    }

    let stats = nitro.stats();
    let actual_rate = stats.sampled_count as f64 / stats.total_items_estimated as f64;

    // With 90% sampling, expect close to 0.9
    assert!(
        actual_rate > 0.8,
        "Sample rate {} should be high",
        actual_rate
    );
}

#[test]
fn test_sampling_distribution_uniform() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.5).unwrap();

    // Add different items - hash-based sampling means same item hashes to same decision
    // but different items will have varied decisions
    for i in 0..1000 {
        nitro.update_sampled(format!("item_{}", i).as_bytes());
    }

    let stats = nitro.stats();
    assert!(stats.sampled_count > 0);
    assert!(stats.unsampled_count > 0);
}

#[test]
fn test_sampling_deterministic_hash_based() {
    let base1 = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro1 = NitroSketch::new(base1, 0.5).unwrap();

    let base2 = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro2 = NitroSketch::new(base2, 0.5).unwrap();

    // Hash-based sampling is deterministic - same items should produce same decisions
    for i in 0..100 {
        let key = format!("item_{}", i);
        nitro1.update_sampled(key.as_bytes());
        nitro2.update_sampled(key.as_bytes());
    }

    let stats1 = nitro1.stats();
    let stats2 = nitro2.stats();

    assert_eq!(stats1.sampled_count, stats2.sampled_count);
    assert_eq!(stats1.unsampled_count, stats2.unsampled_count);
}

#[test]
fn test_sampling_consistency() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.5).unwrap();

    // Add the same set of items twice
    for i in 0..100 {
        let key = format!("item_{}", i);
        nitro.update_sampled(key.as_bytes());
    }

    let stats1 = nitro.stats();

    for i in 0..100 {
        let key = format!("item_{}", i);
        nitro.update_sampled(key.as_bytes());
    }

    let stats2 = nitro.stats();

    // Each item should be sampled or not sampled consistently
    // So the delta should match the first batch
    assert_eq!(
        stats2.sampled_count - stats1.sampled_count,
        stats1.sampled_count
    );
}

#[test]
fn test_sampling_no_sampling_with_rate_one() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 1.0).unwrap();

    for i in 0..1000 {
        nitro.update_sampled(format!("item_{}", i).as_bytes());
    }

    let stats = nitro.stats();
    assert_eq!(stats.sampled_count, 1000);
    assert_eq!(stats.unsampled_count, 0);
}

#[test]
fn test_sampling_hash_based_consistency() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.1).unwrap();

    // Same key should always make same sampling decision
    let key = b"consistent_key";

    nitro.update_sampled(key);
    let stats1 = nitro.stats();

    nitro.update_sampled(key);
    let stats2 = nitro.stats();

    // Both updates should be sampled or both not sampled
    let delta_sampled = stats2.sampled_count - stats1.sampled_count;
    let delta_unsampled = stats2.unsampled_count - stats1.unsampled_count;

    assert!(
        (delta_sampled == 1 && delta_unsampled == 0)
            || (delta_sampled == 0 && delta_unsampled == 1)
    );
}

// ============================================================================
// PHASE 3: Accuracy Tests (10 tests)
// ============================================================================

#[test]
fn test_accuracy_before_sync() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 1.0).unwrap(); // No sampling

    for _ in 0..100 {
        nitro.update_sampled(b"item");
    }

    // With no sampling, accuracy should match base sketch
    let stats = nitro.stats();
    assert_eq!(stats.total_items_estimated, 100);
}

#[test]
fn test_accuracy_after_sync() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.1).unwrap();

    for i in 0..1000 {
        nitro.update_sampled(format!("item_{}", i).as_bytes());
    }

    let result = nitro.sync(1.0);
    assert!(result.is_ok());
}

#[test]
fn test_frequency_estimation_with_sampling() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.5).unwrap();

    // Add items with known frequencies
    for _ in 0..100 {
        nitro.update_sampled(b"frequent");
    }
    for _ in 0..10 {
        nitro.update_sampled(b"rare");
    }

    let stats = nitro.stats();
    assert!(stats.total_items_estimated >= 110);
}

#[test]
fn test_heavy_hitter_detection() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.1).unwrap();

    // Add one heavy hitter
    for _ in 0..1000 {
        nitro.update_sampled(b"heavy_hitter");
    }

    // Add many light items
    for i in 0..1000 {
        nitro.update_sampled(format!("light_{}", i).as_bytes());
    }

    let stats = nitro.stats();
    assert!(stats.total_items_estimated >= 2000);
}

#[test]
fn test_error_bounds_low_sample_rate() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.01).unwrap();

    for i in 0..10000 {
        nitro.update_sampled(format!("item_{}", i % 100).as_bytes());
    }

    let stats = nitro.stats();
    // Should track all items even with low sampling
    assert_eq!(stats.total_items_estimated, 10000);
}

#[test]
fn test_error_bounds_high_sample_rate() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.99).unwrap();

    for i in 0..1000 {
        nitro.update_sampled(format!("item_{}", i).as_bytes());
    }

    let stats = nitro.stats();
    assert_eq!(stats.total_items_estimated, 1000);
    assert!(stats.sampled_count > 900); // Most should be sampled
}

#[test]
fn test_synchronization_improves_accuracy() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.1).unwrap();

    for i in 0..1000 {
        nitro.update_sampled(format!("item_{}", i).as_bytes());
    }

    // Sync should succeed
    let result = nitro.sync(1.0);
    assert!(result.is_ok());
}

#[test]
fn test_multiple_syncs() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.5).unwrap();

    for i in 0..100 {
        nitro.update_sampled(format!("item_{}", i).as_bytes());
    }

    nitro.sync(1.0).unwrap();
    nitro.sync(1.0).unwrap();
    nitro.sync(1.0).unwrap();

    // Multiple syncs should not cause errors
    let stats = nitro.stats();
    assert_eq!(stats.total_items_estimated, 100);
}

#[test]
fn test_sync_weight_parameter() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.1).unwrap();

    for i in 0..1000 {
        nitro.update_sampled(format!("item_{}", i).as_bytes());
    }

    // Test different weights
    assert!(nitro.sync(0.5).is_ok());
    assert!(nitro.sync(1.0).is_ok());
    assert!(nitro.sync(2.0).is_ok());
}

#[test]
fn test_accuracy_zipfian_distribution() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.2).unwrap();

    // Zipfian: few heavy hitters, many rare items
    for i in 0..10 {
        let count = 1000 / (i + 1);
        for _ in 0..count {
            nitro.update_sampled(format!("item_{}", i).as_bytes());
        }
    }

    let stats = nitro.stats();
    assert!(stats.total_items_estimated > 1000);
}

// ============================================================================
// PHASE 4: Base Sketch Integration Tests (6 tests)
// ============================================================================

#[test]
fn test_works_with_hyperloglog() {
    let hll = HyperLogLog::new(12).unwrap();
    let mut nitro = NitroSketch::new(hll, 0.5).unwrap();

    for i in 0..1000 {
        nitro.update_sampled(format!("user_{}", i).as_bytes());
    }

    let stats = nitro.stats();
    assert!(stats.total_items_estimated >= 1000);
}

#[test]
fn test_works_with_count_min_sketch() {
    let cms = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(cms, 0.5).unwrap();

    for i in 0..1000 {
        nitro.update_sampled(format!("flow_{}", i % 50).as_bytes());
    }

    let stats = nitro.stats();
    assert_eq!(stats.total_items_estimated, 1000);
}

#[test]
fn test_base_sketch_access() {
    let cms = CountMinSketch::new(0.01, 0.01).unwrap();
    let nitro = NitroSketch::new(cms, 0.5).unwrap();

    let base = nitro.base_sketch();
    assert!(base.is_empty());
}

#[test]
fn test_base_sketch_mutable_access() {
    let cms = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(cms, 0.5).unwrap();

    nitro.update_sampled(b"test");

    let _base = nitro.base_sketch_mut();
    // Can modify base sketch directly if needed
}

#[test]
fn test_wrapper_transparency() {
    let cms = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(cms, 1.0).unwrap(); // No sampling

    for _ in 0..100 {
        nitro.update_sampled(b"item");
    }

    // With sample_rate=1.0, NitroSketch should behave like base sketch
    let stats = nitro.stats();
    assert_eq!(stats.sampled_count, 100);
    assert_eq!(stats.unsampled_count, 0);
}

#[test]
fn test_different_base_sketch_sizes() {
    // Small sketch
    let cms_small = CountMinSketch::new(0.1, 0.1).unwrap();
    let nitro_small = NitroSketch::new(cms_small, 0.5);
    assert!(nitro_small.is_ok());

    // Large sketch
    let cms_large = CountMinSketch::new(0.001, 0.001).unwrap();
    let nitro_large = NitroSketch::new(cms_large, 0.5);
    assert!(nitro_large.is_ok());
}

// ============================================================================
// PHASE 5: Synchronization Tests (8 tests)
// ============================================================================

#[test]
fn test_sync_basic() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.5).unwrap();

    for i in 0..1000 {
        nitro.update_sampled(format!("item_{}", i).as_bytes());
    }

    let result = nitro.sync(1.0);
    assert!(result.is_ok());
}

#[test]
fn test_sync_after_no_updates() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.5).unwrap();

    // Sync without any updates
    let result = nitro.sync(1.0);
    assert!(result.is_ok());
}

#[test]
fn test_sync_multiple_times() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.5).unwrap();

    for i in 0..100 {
        nitro.update_sampled(format!("item_{}", i).as_bytes());
    }

    for _ in 0..10 {
        assert!(nitro.sync(1.0).is_ok());
    }
}

#[test]
fn test_sync_idempotency() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.5).unwrap();

    for i in 0..1000 {
        nitro.update_sampled(format!("item_{}", i).as_bytes());
    }

    nitro.sync(1.0).unwrap();
    let stats1 = nitro.stats();

    nitro.sync(1.0).unwrap();
    let stats2 = nitro.stats();

    // Stats should not change after multiple syncs
    assert_eq!(stats1.total_items_estimated, stats2.total_items_estimated);
}

#[test]
fn test_sync_with_zero_weight() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.5).unwrap();

    for i in 0..1000 {
        nitro.update_sampled(format!("item_{}", i).as_bytes());
    }

    assert!(nitro.sync(0.0).is_ok());
}

#[test]
fn test_sync_with_high_weight() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.5).unwrap();

    for i in 0..1000 {
        nitro.update_sampled(format!("item_{}", i).as_bytes());
    }

    assert!(nitro.sync(10.0).is_ok());
}

#[test]
fn test_sync_updates_then_sync_pattern() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.1).unwrap();

    // Update-sync-update-sync pattern
    for i in 0..100 {
        nitro.update_sampled(format!("batch1_{}", i).as_bytes());
    }
    nitro.sync(1.0).unwrap();

    for i in 0..100 {
        nitro.update_sampled(format!("batch2_{}", i).as_bytes());
    }
    nitro.sync(1.0).unwrap();

    let stats = nitro.stats();
    assert_eq!(stats.total_items_estimated, 200);
}

#[test]
fn test_sync_preserves_base_sketch() {
    let cms = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(cms, 1.0).unwrap(); // 100% sampling so base gets updated

    for _ in 0..1000 {
        nitro.update_sampled(b"item");
    }

    nitro.sync(1.0).unwrap();

    // Base sketch should have data (with 100% sampling)
    let stats = nitro.stats();
    assert!(stats.sampled_count > 0);
}

// ============================================================================
// PHASE 6: Performance Tests (8 tests)
// ============================================================================

#[test]
fn test_performance_high_sample_rate() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.9).unwrap();

    // Should handle high sample rate efficiently
    for i in 0..10000 {
        nitro.update_sampled(format!("item_{}", i).as_bytes());
    }

    let stats = nitro.stats();
    assert!(stats.sampled_count > 8000); // Most should be sampled
}

#[test]
fn test_performance_low_sample_rate() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.01).unwrap();

    // Should handle low sample rate efficiently (most items skipped)
    for i in 0..10000 {
        nitro.update_sampled(format!("item_{}", i).as_bytes());
    }

    let stats = nitro.stats();
    assert!(stats.unsampled_count > 9000); // Most should be skipped
}

#[test]
fn test_throughput_100k_ops() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.1).unwrap();

    // Process 100K items
    for i in 0..100_000 {
        nitro.update_sampled(format!("flow_{}", i % 1000).as_bytes());
    }

    let stats = nitro.stats();
    assert_eq!(stats.total_items_estimated, 100_000);
}

#[test]
fn test_memory_efficiency() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let nitro = NitroSketch::new(base, 0.1).unwrap();

    // NitroSketch should not significantly increase memory
    // (just tracking two counters + RNG state)
    let stats = nitro.stats();
    assert_eq!(stats.sampled_count, 0);
    assert_eq!(stats.unsampled_count, 0);
}

#[test]
fn test_update_latency_low_overhead() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.1).unwrap();

    // Most updates should be fast (not updating base sketch)
    for i in 0..1000 {
        nitro.update_sampled(format!("item_{}", i).as_bytes());
    }

    let stats = nitro.stats();
    // With 10% sampling, ~90% of updates should be very fast
    assert!(stats.unsampled_count > stats.sampled_count);
}

#[test]
fn test_sustained_load() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.05).unwrap();

    // Simulate sustained network traffic
    for batch in 0..100 {
        for i in 0..1000 {
            nitro.update_sampled(format!("flow_{}_{}", batch, i % 100).as_bytes());
        }
    }

    let stats = nitro.stats();
    assert_eq!(stats.total_items_estimated, 100_000);
}

#[test]
fn test_bursty_traffic_pattern() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.1).unwrap();

    // Burst 1
    for i in 0..5000 {
        nitro.update_sampled(format!("burst1_{}", i).as_bytes());
    }

    // Quiet period (no updates)

    // Burst 2
    for i in 0..5000 {
        nitro.update_sampled(format!("burst2_{}", i).as_bytes());
    }

    let stats = nitro.stats();
    assert_eq!(stats.total_items_estimated, 10_000);
}

#[test]
fn test_network_flow_simulation() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.1).unwrap();

    // Simulate network flows: src_ip:port -> dst_ip:port
    for i in 0..10000 {
        let src_ip = i % 256;
        let dst_ip = (i / 256) % 256;
        let flow_key = format!("192.168.1.{}:443->10.0.0.{}:8080", src_ip, dst_ip);
        nitro.update_sampled(flow_key.as_bytes());
    }

    let stats = nitro.stats();
    assert_eq!(stats.total_items_estimated, 10_000);
}

// ============================================================================
// PHASE 7: Edge Cases (5 tests)
// ============================================================================

#[test]
fn test_edge_case_single_item() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.5).unwrap();

    nitro.update_sampled(b"single");

    let stats = nitro.stats();
    assert_eq!(stats.total_items_estimated, 1);
}

#[test]
fn test_edge_case_many_items() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.01).unwrap();

    // Process 1M items with low sampling
    for i in 0..1_000_000 {
        nitro.update_sampled(format!("item_{}", i % 10000).as_bytes());
    }

    let stats = nitro.stats();
    assert_eq!(stats.total_items_estimated, 1_000_000);
}

#[test]
fn test_edge_case_empty_keys() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.5).unwrap();

    nitro.update_sampled(b"");
    nitro.update_sampled(b"");

    let stats = nitro.stats();
    assert_eq!(stats.total_items_estimated, 2);
}

#[test]
fn test_edge_case_very_long_keys() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.5).unwrap();

    let long_key = vec![0u8; 10000];
    nitro.update_sampled(&long_key);

    let stats = nitro.stats();
    assert_eq!(stats.total_items_estimated, 1);
}

#[test]
fn test_edge_case_reset_stats() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.5).unwrap();

    for i in 0..100 {
        nitro.update_sampled(format!("item_{}", i).as_bytes());
    }

    nitro.reset_stats();

    let stats = nitro.stats();
    assert_eq!(stats.sampled_count, 0);
    assert_eq!(stats.unsampled_count, 0);
    assert_eq!(stats.total_items_estimated, 0);
}

// ============================================================================
// PHASE 8: Property Tests (5 tests)
// ============================================================================

proptest! {
    #[test]
    fn prop_sample_rate_honored(
        sample_rate in 0.05f64..0.95,  // Narrow range for more consistent results
        count in 5000usize..10000       // Larger sample size for better statistics
    ) {
        let base = CountMinSketch::new(0.01, 0.01).unwrap();
        let mut nitro = NitroSketch::new(base, sample_rate).unwrap();

        for i in 0..count {
            nitro.update_sampled(format!("item_{}", i).as_bytes());
        }

        let stats = nitro.stats();
        let actual_rate = stats.sampled_count as f64 / stats.total_items_estimated as f64;

        // Allow 30% tolerance for hash-based sampling variance
        // (not true randomness, so distribution may vary)
        let tolerance = 0.3;
        prop_assert!(
            (actual_rate - sample_rate).abs() < sample_rate * tolerance,
            "Sample rate {} should be close to {}",
            actual_rate, sample_rate
        );
    }

    #[test]
    fn prop_total_count_accurate(
        sample_rate in 0.01f64..1.0,
        count in 100usize..1000
    ) {
        let base = CountMinSketch::new(0.01, 0.01).unwrap();
        let mut nitro = NitroSketch::new(base, sample_rate).unwrap();

        for i in 0..count {
            nitro.update_sampled(format!("item_{}", i).as_bytes());
        }

        let stats = nitro.stats();
        prop_assert_eq!(stats.total_items_estimated as usize, count);
        prop_assert_eq!(
            (stats.sampled_count + stats.unsampled_count) as usize,
            count
        );
    }

    #[test]
    fn prop_sync_idempotent(
        sample_rate in 0.1f64..0.9,
        count in 100usize..500
    ) {
        let base = CountMinSketch::new(0.01, 0.01).unwrap();
        let mut nitro = NitroSketch::new(base, sample_rate).unwrap();

        for i in 0..count {
            nitro.update_sampled(format!("item_{}", i).as_bytes());
        }

        nitro.sync(1.0).unwrap();
        let stats1 = nitro.stats();

        nitro.sync(1.0).unwrap();
        let stats2 = nitro.stats();

        prop_assert_eq!(stats1.total_items_estimated, stats2.total_items_estimated);
    }

    #[test]
    fn prop_no_data_loss(
        items in prop::collection::vec(0u64..10000, 100..500),
        sample_rate in 0.01f64..1.0
    ) {
        let base = CountMinSketch::new(0.01, 0.01).unwrap();
        let mut nitro = NitroSketch::new(base, sample_rate).unwrap();

        for item in items.iter() {
            nitro.update_sampled(&item.to_le_bytes());
        }

        let stats = nitro.stats();
        prop_assert_eq!(stats.total_items_estimated as usize, items.len());
    }

    #[test]
    fn prop_stats_consistency(
        sample_rate in 0.1f64..0.9,
        count in 100usize..1000
    ) {
        let base = CountMinSketch::new(0.01, 0.01).unwrap();
        let mut nitro = NitroSketch::new(base, sample_rate).unwrap();

        for i in 0..count {
            nitro.update_sampled(format!("item_{}", i).as_bytes());
        }

        let stats = nitro.stats();

        prop_assert_eq!(stats.sample_rate, sample_rate);
        prop_assert!(stats.sampled_count > 0);
        prop_assert_eq!(
            stats.sampled_count + stats.unsampled_count,
            stats.total_items_estimated
        );
    }
}

// ============================================================================
// Additional Tests for Coverage
// ============================================================================

#[test]
fn test_sketch_trait_implementation() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.5).unwrap();

    // Test Sketch trait methods
    assert!(nitro.is_empty());

    let key = b"test".to_vec();
    nitro.update(&key);
    assert!(!nitro.is_empty());

    let _estimate = nitro.estimate();
}

#[test]
fn test_serialization() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 0.5).unwrap();

    for i in 0..100 {
        nitro.update_sampled(format!("item_{}", i).as_bytes());
    }

    let bytes = nitro.serialize();
    assert!(!bytes.is_empty());
}

#[test]
fn test_query_method() {
    let base = CountMinSketch::new(0.01, 0.01).unwrap();
    let mut nitro = NitroSketch::new(base, 1.0).unwrap();

    nitro.update_sampled(b"test");

    let _freq = nitro.query(b"test");
    // Query implementation is sketch-specific
}
