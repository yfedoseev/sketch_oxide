//! Quick performance validation for Vacuum Filter
//! These tests verify that performance targets are met

use sketch_oxide::membership::VacuumFilter;
use std::time::Instant;

#[test]
fn test_insert_performance() {
    let mut filter = VacuumFilter::new(10000, 0.01).unwrap();

    let start = Instant::now();
    for i in 0u32..10000 {
        filter.insert(&i.to_le_bytes()).unwrap();
    }
    let duration = start.elapsed();

    let ops_per_sec = 10000.0 / duration.as_secs_f64();
    let ns_per_op = duration.as_nanos() as f64 / 10000.0;

    println!("Insert: {:.0} ops/sec, {:.2} ns/op", ops_per_sec, ns_per_op);

    // Should be able to do at least 50k inserts per second (< 20 microseconds per op)
    assert!(ns_per_op < 20000.0, "Insert too slow: {:.2} ns", ns_per_op);
}

#[test]
fn test_query_performance() {
    let mut filter = VacuumFilter::new(10000, 0.01).unwrap();
    for i in 0u32..5000 {
        filter.insert(&i.to_le_bytes()).unwrap();
    }

    let start = Instant::now();
    for i in 0u32..10000 {
        let _ = filter.contains(&i.to_le_bytes());
    }
    let duration = start.elapsed();

    let ops_per_sec = 10000.0 / duration.as_secs_f64();
    let ns_per_op = duration.as_nanos() as f64 / 10000.0;

    println!("Query: {:.0} ops/sec, {:.2} ns/op", ops_per_sec, ns_per_op);

    // Should be able to do at least 100k queries per second (< 10 microseconds per op)
    assert!(ns_per_op < 10000.0, "Query too slow: {:.2} ns", ns_per_op);
}

#[test]
fn test_delete_performance() {
    let mut filter = VacuumFilter::new(10000, 0.01).unwrap();
    for i in 0u32..5000 {
        filter.insert(&i.to_le_bytes()).unwrap();
    }

    let start = Instant::now();
    for i in 0u32..1000 {
        filter.delete(&i.to_le_bytes()).unwrap();
    }
    let duration = start.elapsed();

    let ops_per_sec = 1000.0 / duration.as_secs_f64();
    let ns_per_op = duration.as_nanos() as f64 / 1000.0;

    println!("Delete: {:.0} ops/sec, {:.2} ns/op", ops_per_sec, ns_per_op);

    // Should be able to do at least 50k deletes per second (< 20 microseconds per op)
    assert!(ns_per_op < 20000.0, "Delete too slow: {:.2} ns", ns_per_op);
}

#[test]
fn test_space_efficiency_comparison() {
    // Compare Vacuum vs Cuckoo filter space efficiency
    use sketch_oxide::membership::CuckooFilter;

    let mut vacuum = VacuumFilter::new(10000, 0.01).unwrap();
    let mut cuckoo = CuckooFilter::new(10000).unwrap();

    for i in 0u32..10000 {
        vacuum.insert(&i.to_le_bytes()).unwrap();
        cuckoo.insert(&i.to_le_bytes()).unwrap();
    }

    let vacuum_stats = vacuum.stats();
    let vacuum_bits_per_item = vacuum_stats.memory_bits as f64 / vacuum_stats.num_items as f64;

    let cuckoo_bytes = cuckoo.memory_usage();
    let cuckoo_bits_per_item = (cuckoo_bytes * 8) as f64 / 10000.0;

    println!("Vacuum: {:.2} bits/item", vacuum_bits_per_item);
    println!("Cuckoo: {:.2} bits/item", cuckoo_bits_per_item);

    // Both should be reasonably space-efficient
    assert!(vacuum_bits_per_item < 100.0);
    assert!(cuckoo_bits_per_item < 100.0);
}

#[test]
fn test_no_false_negatives_stress() {
    let mut filter = VacuumFilter::new(5000, 0.01).unwrap();

    // Insert 5000 items
    for i in 0u32..5000 {
        filter.insert(&i.to_le_bytes()).unwrap();
    }

    // Verify no false negatives
    let mut false_negatives = 0;
    for i in 0u32..5000 {
        if !filter.contains(&i.to_le_bytes()) {
            false_negatives += 1;
        }
    }

    assert_eq!(
        false_negatives, 0,
        "Found {} false negatives",
        false_negatives
    );
}

#[test]
fn test_fpr_validation_large() {
    let mut filter = VacuumFilter::new(10000, 0.01).unwrap();

    // Insert 5000 items
    for i in 0u32..5000 {
        filter.insert(&i.to_le_bytes()).unwrap();
    }

    // Test 10000 non-existent items
    let mut false_positives = 0;
    for i in 10000u32..20000 {
        if filter.contains(&i.to_le_bytes()) {
            false_positives += 1;
        }
    }

    let measured_fpr = false_positives as f64 / 10000.0;

    println!("Measured FPR: {:.4} (target: 0.01)", measured_fpr);

    // FPR should be within reasonable bounds (allow 5x margin for small sample)
    assert!(measured_fpr < 0.05, "FPR too high: {:.4}", measured_fpr);
}
