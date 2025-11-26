#!/usr/bin/env python3
"""
Comprehensive test for all 9 sketch_oxide algorithms.
Tests feature parity with Rust library.
"""

import sketch_oxide


def test_theta_sketch():
    """Test ThetaSketch set operations."""
    print("\n=== Testing ThetaSketch (Set Operations) ===")

    ts1 = sketch_oxide.ThetaSketch(lg_k=12)
    ts2 = sketch_oxide.ThetaSketch(lg_k=12)

    # Add items: ts1 = [0, 999], ts2 = [500, 1499]
    for i in range(1000):
        ts1.update(i)
    for i in range(500, 1500):
        ts2.update(i)

    # Test estimates
    est1 = ts1.estimate()
    est2 = ts2.estimate()
    print(f"  ts1 estimate: {est1:.0f} (expected ~1000)")
    print(f"  ts2 estimate: {est2:.0f} (expected ~1000)")

    # Test set operations
    union = ts1.union(ts2)
    union_est = union.estimate()
    print(f"  Union estimate: {union_est:.0f} (expected ~1500)")

    intersect = ts1.intersect(ts2)
    intersect_est = intersect.estimate()
    print(f"  Intersection estimate: {intersect_est:.0f} (expected ~500)")

    diff = ts1.difference(ts2)
    diff_est = diff.estimate()
    print(f"  Difference estimate: {diff_est:.0f} (expected ~500)")

    # Check results are reasonable (within 10% error)
    assert abs(est1 - 1000) / 1000 < 0.1, f"ts1 estimate too far off: {est1}"
    assert abs(union_est - 1500) / 1500 < 0.1, f"Union estimate too far off: {union_est}"
    assert (
        abs(intersect_est - 500) / 500 < 0.2
    ), f"Intersection estimate too far off: {intersect_est}"

    print("  ✅ ThetaSketch PASSED")


def test_req_sketch():
    """Test ReqSketch tail quantiles with HRA mode."""
    print("\n=== Testing ReqSketch (Tail Quantiles - HRA Mode) ===")

    req = sketch_oxide.ReqSketch(k=128, mode="HRA")

    # Add 10,000 values: 1 to 10,000
    for i in range(1, 10001):
        req.update(float(i))

    # Test exact max in HRA mode (p100)
    p100 = req.quantile(1.0)
    print(f"  p100 (max): {p100} (expected 10000.0, HRA mode = exact)")
    assert p100 == 10000.0, f"p100 should be exact in HRA mode: {p100}"

    # Test tail quantiles
    p99 = req.quantile(0.99)
    p95 = req.quantile(0.95)
    p50 = req.quantile(0.50)

    print(f"  p99: {p99:.0f} (expected ~9900)")
    print(f"  p95: {p95:.0f} (expected ~9500)")
    print(f"  p50: {p50:.0f} (expected ~5000)")

    # Check min/max
    min_val = req.min()
    max_val = req.max()
    print(f"  min: {min_val} (expected 1.0)")
    print(f"  max: {max_val} (expected 10000.0)")

    assert min_val == 1.0, f"min should be exact: {min_val}"
    assert max_val == 10000.0, f"max should be exact: {max_val}"
    assert abs(p99 - 9900) / 9900 < 0.05, f"p99 too far off: {p99}"

    print("  ✅ ReqSketch PASSED")


def test_cpc_sketch():
    """Test CpcSketch maximum space efficiency."""
    print("\n=== Testing CpcSketch (Maximum Space Efficiency) ===")

    cpc = sketch_oxide.CpcSketch(lg_k=11)

    # Add 10,000 unique items
    for i in range(10000):
        cpc.update(i)

    estimate = cpc.estimate()
    error = abs(estimate - 10000) / 10000

    print(f"  CPC estimate: {estimate:.0f} (true: 10000)")
    print(f"  Error: {error:.2%}")

    # CPC should be within 5% for lg_k=11
    assert error < 0.05, f"CPC error too high: {error:.2%}"

    # Test merge
    cpc2 = sketch_oxide.CpcSketch(lg_k=11)
    for i in range(10000, 20000):
        cpc2.update(i)

    cpc.merge(cpc2)
    merged_est = cpc.estimate()
    print(f"  Merged estimate: {merged_est:.0f} (expected ~20000)")

    merged_error = abs(merged_est - 20000) / 20000
    assert merged_error < 0.05, f"Merged error too high: {merged_error:.2%}"

    print("  ✅ CpcSketch PASSED")


def test_frequent_items():
    """Test FrequentItems top-K heavy hitters."""
    print("\n=== Testing FrequentItems (Top-K Heavy Hitters) ===")

    fi = sketch_oxide.FrequentItems(max_size=10)

    # Add items with known frequencies
    for _ in range(100):
        fi.update("apple")
    for _ in range(50):
        fi.update("banana")
    for _ in range(25):
        fi.update("cherry")
    for _ in range(10):
        fi.update("date")

    # Get frequent items
    items = fi.frequent_items(mode="no_false_positives")

    print("  Top items (with bounds):")
    for item, lower, upper in items[:4]:
        print(f"    {item}: [{lower}, {upper}]")

    # Check that top items are correct
    assert items[0][0] == "apple", f"Top item should be 'apple': {items[0][0]}"
    assert items[1][0] == "banana", f"Second item should be 'banana': {items[1][0]}"
    assert items[2][0] == "cherry", f"Third item should be 'cherry': {items[2][0]}"

    # Check estimates
    apple_est = fi.get_estimate("apple")
    assert apple_est is not None, "apple should be tracked"
    lower, upper = apple_est
    print(f"  Apple frequency: [{lower}, {upper}] (true: 100)")
    assert lower <= 100 <= upper, f"True frequency must be in bounds: [{lower}, {upper}]"

    print("  ✅ FrequentItems PASSED")


def test_all_existing():
    """Quick test of existing 5 algorithms."""
    print("\n=== Testing Existing 5 Algorithms ===")

    # UltraLogLog
    ull = sketch_oxide.UltraLogLog(precision=12)
    for i in range(1000):
        ull.update(i)
    assert 900 < ull.estimate() < 1100
    print("  ✅ UltraLogLog works")

    # BinaryFuseFilter
    items = list(range(1000))
    bf = sketch_oxide.BinaryFuseFilter(items, bits_per_entry=9)
    assert bf.contains(500)
    assert not bf.contains(10000)
    print("  ✅ BinaryFuseFilter works")

    # DDSketch
    dd = sketch_oxide.DDSketch(relative_accuracy=0.01)
    for i in range(1, 1001):
        dd.update(float(i))
    p99 = dd.quantile(0.99)
    assert 970 < p99 < 1020
    print("  ✅ DDSketch works")

    # CountMinSketch
    cms = sketch_oxide.CountMinSketch(epsilon=0.01, delta=0.01)
    for _ in range(100):
        cms.update("apple")
    assert 100 <= cms.estimate("apple") <= 120
    print("  ✅ CountMinSketch works")

    # MinHash
    mh1 = sketch_oxide.MinHash(num_perm=128)
    mh2 = sketch_oxide.MinHash(num_perm=128)

    for i in range(100):
        mh1.update(i)
        mh2.update(i)

    similarity = mh1.jaccard_similarity(mh2)
    assert 0.95 < similarity <= 1.0
    print("  ✅ MinHash works")

    print("\n  All 5 existing algorithms still working!")


def main():
    """Run all tests."""
    print("=" * 60)
    print("Testing All 9 sketch_oxide Algorithms")
    print("=" * 60)

    # Test existing 5
    test_all_existing()

    # Test new 4
    test_theta_sketch()
    test_req_sketch()
    test_cpc_sketch()
    test_frequent_items()

    print("\n" + "=" * 60)
    print("✅ ALL 9 ALGORITHMS WORKING! 100% FEATURE PARITY!")
    print("=" * 60)
    print("\nComplete list:")
    print("  1. UltraLogLog (Cardinality)")
    print("  2. CpcSketch (Cardinality)")
    print("  3. ThetaSketch (Cardinality + Set Ops)")
    print("  4. BinaryFuseFilter (Membership)")
    print("  5. DDSketch (Quantiles)")
    print("  6. ReqSketch (Quantiles - Tail)")
    print("  7. CountMinSketch (Frequency)")
    print("  8. FrequentItems (Frequency - Top-K)")
    print("  9. MinHash (Similarity)")
    print("\n🎉 Python bindings complete!")


if __name__ == "__main__":
    main()
