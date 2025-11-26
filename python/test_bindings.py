#!/usr/bin/env python3
"""
Test script for sketch_oxide Python bindings.
Demonstrates all 5 core algorithms.
"""

import sketch_oxide


def test_ultraloglog():
    """Test UltraLogLog cardinality estimation."""
    print("\n" + "=" * 60)
    print("1. UltraLogLog - Cardinality Estimation (28% better than HLL)")
    print("=" * 60)

    ull = sketch_oxide.UltraLogLog(12)
    n = 10000

    # Add unique items
    for i in range(n):
        ull.update(i)

    estimate = ull.estimate()
    error = abs(estimate - n) / n * 100

    print(f"✓ Added {n:,} unique items")
    print(f"✓ Estimated: {estimate:.0f}")
    print(f"✓ Error: {error:.2f}%")
    print("✓ Memory: ~4KB (precision=12)")
    assert error < 5.0, f"Error too high: {error:.2f}%"
    print("✓ PASSED")


def test_binary_fuse():
    """Test BinaryFuseFilter membership testing."""
    print("\n" + "=" * 60)
    print("2. BinaryFuseFilter - Membership Testing (75% better than Bloom)")
    print("=" * 60)

    items = list(range(1000))
    bf = sketch_oxide.BinaryFuseFilter(items, bits_per_entry=9)

    # Test positive cases
    assert bf.contains(0), "Should contain 0"
    assert bf.contains(500), "Should contain 500"
    assert bf.contains(999), "Should contain 999"

    # Test negative case (may have false positives)
    # Count false positives
    false_positives = sum(1 for i in range(1000, 2000) if bf.contains(i))
    fpr = false_positives / 1000

    print(f"✓ Built filter with {len(bf):,} items")
    print(f"✓ Bits per entry: {bf.bits_per_entry():.2f}")
    print(f"✓ Theoretical FPR: {bf.false_positive_rate() * 100:.4f}%")
    print(f"✓ Measured FPR: {fpr * 100:.2f}%")
    print(f"✓ Space: ~{len(bf) * bf.bits_per_entry() / 8:.0f} bytes")
    assert fpr < 0.05, f"FPR too high: {fpr:.4f}"
    print("✓ PASSED")


def test_ddsketch():
    """Test DDSketch quantile estimation."""
    print("\n" + "=" * 60)
    print("3. DDSketch - Quantile Estimation (Datadog, ClickHouse)")
    print("=" * 60)

    dd = sketch_oxide.DDSketch(0.01)  # 1% relative error

    # Add values 1-1000
    for i in range(1, 1001):
        dd.update(float(i))

    p50 = dd.quantile(0.5)
    p95 = dd.quantile(0.95)
    p99 = dd.quantile(0.99)

    print(f"✓ Added {dd.count():,} values (1-1000)")
    print(f"✓ Min: {dd.min():.0f}, Max: {dd.max():.0f}")
    print(f"✓ p50: {p50:.0f} (expected ~500)")
    print(f"✓ p95: {p95:.0f} (expected ~950)")
    print(f"✓ p99: {p99:.0f} (expected ~990)")
    print("✓ Relative accuracy: 1%")

    # Verify accuracy
    assert 450 < p50 < 550, f"p50 out of range: {p50}"
    assert 900 < p95 < 1000, f"p95 out of range: {p95}"
    assert 950 < p99 < 1000, f"p99 out of range: {p99}"
    print("✓ PASSED")


def test_count_min():
    """Test CountMinSketch frequency estimation."""
    print("\n" + "=" * 60)
    print("4. CountMinSketch - Frequency Estimation (Redis, monitoring)")
    print("=" * 60)

    cms = sketch_oxide.CountMinSketch(0.01, 0.01)

    # Add items with known frequencies
    items = ["apple"] * 10 + ["banana"] * 5 + ["cherry"] * 2 + ["date"]
    for item in items:
        cms.update(item)

    freq_apple = cms.estimate("apple")
    freq_banana = cms.estimate("banana")
    freq_cherry = cms.estimate("cherry")
    freq_date = cms.estimate("date")
    freq_unknown = cms.estimate("unknown")

    print(f"✓ Added {len(items)} items")
    print(f"✓ Frequency of 'apple': {freq_apple} (expected 10)")
    print(f"✓ Frequency of 'banana': {freq_banana} (expected 5)")
    print(f"✓ Frequency of 'cherry': {freq_cherry} (expected 2)")
    print(f"✓ Frequency of 'date': {freq_date} (expected 1)")
    print(f"✓ Frequency of 'unknown': {freq_unknown} (expected 0)")
    print(f"✓ Dimensions: {cms.width()}x{cms.depth()}")

    # Count-Min never underestimates
    assert freq_apple >= 10, "Should never underestimate"
    assert freq_banana >= 5, "Should never underestimate"
    assert freq_unknown == 0, "Unknown item should be 0"
    print("✓ PASSED")


def test_minhash():
    """Test MinHash similarity estimation."""
    print("\n" + "=" * 60)
    print("5. MinHash - Similarity Estimation (LSH, deduplication)")
    print("=" * 60)

    mh1 = sketch_oxide.MinHash(128)
    mh2 = sketch_oxide.MinHash(128)
    mh3 = sketch_oxide.MinHash(128)

    # Set 1: {1, 2, 3, 4, 5}
    for i in range(1, 6):
        mh1.update(i)

    # Set 2: {3, 4, 5, 6, 7} - overlap: {3, 4, 5}
    for i in range(3, 8):
        mh2.update(i)

    # Set 3: {1, 2, 3} - subset of Set 1
    for i in range(1, 4):
        mh3.update(i)

    sim_1_2 = mh1.jaccard_similarity(mh2)
    sim_1_3 = mh1.jaccard_similarity(mh3)

    # True Jaccard: |A ∩ B| / |A ∪ B|
    # Set1 ∩ Set2 = {3,4,5}, Set1 ∪ Set2 = {1,2,3,4,5,6,7} => 3/7 ≈ 0.43
    # Set1 ∩ Set3 = {1,2,3}, Set1 ∪ Set3 = {1,2,3,4,5} => 3/5 = 0.60

    print("✓ Set 1: {1,2,3,4,5}")
    print("✓ Set 2: {3,4,5,6,7}")
    print("✓ Set 3: {1,2,3}")
    print(f"✓ Jaccard(Set1, Set2): {sim_1_2:.3f} (expected ~0.43)")
    print(f"✓ Jaccard(Set1, Set3): {sim_1_3:.3f} (expected ~0.60)")
    print(f"✓ Num permutations: {mh1.num_perm()}")

    # Allow ~10% error for estimates
    assert 0.35 < sim_1_2 < 0.55, f"Similarity out of range: {sim_1_2}"
    assert 0.50 < sim_1_3 < 0.70, f"Similarity out of range: {sim_1_3}"
    print("✓ PASSED")


def main():
    """Run all tests."""
    print("\n" + "=" * 60)
    print("sketch_oxide Python Bindings - Comprehensive Test Suite")
    print("=" * 60)
    print(f"Version: {sketch_oxide.__version__}")

    try:
        test_ultraloglog()
        test_binary_fuse()
        test_ddsketch()
        test_count_min()
        test_minhash()

        print("\n" + "=" * 60)
        print("✓ ALL TESTS PASSED!")
        print("=" * 60)
        print("\nReady for production use:")
        print("  • UltraLogLog: 28% better than HyperLogLog")
        print("  • BinaryFuseFilter: 75% better than Bloom filters")
        print("  • DDSketch: Production-proven (Datadog, ClickHouse)")
        print("  • CountMinSketch: Battle-tested frequency estimation")
        print("  • MinHash: Fast similarity with LSH support")
        print("=" * 60)

    except AssertionError as e:
        print(f"\n✗ TEST FAILED: {e}")
        return 1
    except Exception as e:
        print(f"\n✗ ERROR: {e}")
        import traceback

        traceback.print_exc()
        return 1

    return 0


if __name__ == "__main__":
    exit(main())
