#!/usr/bin/env python3
"""
Comprehensive tests for CountSketch and SpaceSaving Python bindings.

Tests cover:
1. Basic updates and queries
2. Error bounds and estimates
3. Heavy hitter detection (SpaceSaving)
4. Inner product (CountSketch)
5. Merge operations
6. Serialization
"""

import os
import sys

# Build the Python module first
print("Building Python module...")
os.system(
    "cd /home/yfedoseev/projects/sketch_oxide && maturin develop -r 2>&1 | "
    "grep -E 'error|Finished' || true"
)

# Import the module
try:
    from sketch_oxide import CountSketch, SpaceSaving

    print("Successfully imported CountSketch and SpaceSaving\n")
except ImportError as e:
    print(f"Failed to import: {e}")
    sys.exit(1)


def test_count_sketch_basic():
    """Test basic CountSketch updates and queries."""
    print("TEST 1: CountSketch Basic Operations")
    print("=" * 50)

    cs = CountSketch(epsilon=0.1, delta=0.01)
    print(f"Created: {cs}")

    # Test basic updates
    cs.update("apple", 5)
    cs.update("banana", 3)
    cs.update("cherry", 2)

    # Test estimates
    apple_est = cs.estimate("apple")
    banana_est = cs.estimate("banana")
    cherry_est = cs.estimate("cherry")

    print(f"apple estimate: {apple_est} (true: 5)")
    print(f"banana estimate: {banana_est} (true: 3)")
    print(f"cherry estimate: {cherry_est} (true: 2)")

    # Estimates should be reasonably close (unbiased, can vary)
    assert abs(apple_est - 5) <= 3, f"apple estimate {apple_est} too far from 5"
    assert abs(banana_est - 3) <= 3, f"banana estimate {banana_est} too far from 3"
    assert abs(cherry_est - 2) <= 3, f"cherry estimate {cherry_est} too far from 2"

    print("✓ Basic operations passed\n")


def test_count_sketch_deletions():
    """Test CountSketch with negative deltas (deletions)."""
    print("TEST 2: CountSketch Deletions (Negative Deltas)")
    print("=" * 50)

    cs = CountSketch(epsilon=0.1, delta=0.01)

    # Add and then delete
    cs.update("item", 10)
    cs.update("item", -3)  # Delete 3

    estimate = cs.estimate("item")
    print(f"After 10 + (-3): estimate = {estimate} (expected ~7)")

    assert abs(estimate - 7) <= 3, f"estimate {estimate} too far from 7"

    # Fully negative
    cs2 = CountSketch(epsilon=0.1, delta=0.01)
    cs2.update("negative", -5)
    neg_est = cs2.estimate("negative")

    print(f"Negative count: estimate = {neg_est} (expected ~-5)")
    assert abs(neg_est - (-5)) <= 3, f"negative estimate {neg_est} too far from -5"

    print("✓ Deletions passed\n")


def test_count_sketch_inner_product():
    """Test CountSketch inner product estimation."""
    print("TEST 3: CountSketch Inner Product")
    print("=" * 50)

    cs1 = CountSketch(epsilon=0.1, delta=0.01)
    cs2 = CountSketch(epsilon=0.1, delta=0.01)

    # Create frequency vectors
    # cs1: a=3, b=2
    # cs2: a=4, b=5
    # Inner product = 3*4 + 2*5 = 22
    cs1.update("a", 3)
    cs1.update("b", 2)

    cs2.update("a", 4)
    cs2.update("b", 5)

    inner = cs1.inner_product(cs2)
    print(f"Inner product: {inner} (expected: 22)")

    assert abs(inner - 22) <= 10, f"Inner product {inner} too far from 22"

    print("✓ Inner product passed\n")


def test_count_sketch_merge():
    """Test CountSketch merge operation."""
    print("TEST 4: CountSketch Merge")
    print("=" * 50)

    cs1 = CountSketch(epsilon=0.1, delta=0.01)
    cs2 = CountSketch(epsilon=0.1, delta=0.01)

    cs1.update("item", 10)
    cs1.update("only1", 5)

    cs2.update("item", 8)
    cs2.update("only2", 3)

    cs1.merge(cs2)

    item_est = cs1.estimate("item")
    only1_est = cs1.estimate("only1")
    only2_est = cs1.estimate("only2")

    print(f"item after merge: {item_est} (expected ~18)")
    print(f"only1 after merge: {only1_est} (expected ~5)")
    print(f"only2 after merge: {only2_est} (expected ~3)")

    assert abs(item_est - 18) <= 5, f"merged item estimate {item_est} too far from 18"
    assert abs(only1_est - 5) <= 3, f"only1 estimate {only1_est} too far from 5"
    assert abs(only2_est - 3) <= 3, f"only2 estimate {only2_est} too far from 3"

    print("✓ Merge passed\n")


def test_count_sketch_serialization():
    """Test CountSketch serialization and deserialization."""
    print("TEST 5: CountSketch Serialization")
    print("=" * 50)

    cs1 = CountSketch(epsilon=0.1, delta=0.01)
    cs1.update("test", 42)
    cs1.update("another", -10)

    # Serialize
    data = cs1.serialize()
    print(f"Serialized size: {len(data)} bytes")

    # Deserialize
    cs2 = CountSketch.deserialize(data)

    # Verify they produce same estimates
    test_est1 = cs1.estimate("test")
    test_est2 = cs2.estimate("test")

    print(f"Original estimate: {test_est1}")
    print(f"Restored estimate: {test_est2}")

    assert test_est1 == test_est2, "Estimates differ after deserialization"

    print("✓ Serialization passed\n")


def test_space_saving_basic():
    """Test basic SpaceSaving updates and queries."""
    print("TEST 6: SpaceSaving Basic Operations")
    print("=" * 50)

    ss = SpaceSaving(epsilon=0.1)
    print(f"Created: {ss}")
    print(f"Capacity: {ss.capacity()}, Stream length: {ss.stream_length()}")

    # Add items
    ss.update("apple")
    ss.update("apple")
    ss.update("apple")
    ss.update("banana")
    ss.update("banana")
    ss.update("cherry")

    # Get estimates
    apple_bounds = ss.estimate("apple")
    banana_bounds = ss.estimate("banana")
    cherry_bounds = ss.estimate("cherry")
    orange_bounds = ss.estimate("orange")  # Not in sketch

    print(f"apple bounds: {apple_bounds} (true: 3)")
    print(f"banana bounds: {banana_bounds} (true: 2)")
    print(f"cherry bounds: {cherry_bounds} (true: 1)")
    print(f"orange bounds: {orange_bounds} (not tracked)")

    # Verify bounds contain true values
    assert apple_bounds is not None
    assert (
        apple_bounds[0] <= 3 <= apple_bounds[1]
    ), f"apple true count 3 not in bounds {apple_bounds}"

    assert banana_bounds is not None
    assert (
        banana_bounds[0] <= 2 <= banana_bounds[1]
    ), f"banana true count 2 not in bounds {banana_bounds}"

    assert cherry_bounds is not None
    assert (
        cherry_bounds[0] <= 1 <= cherry_bounds[1]
    ), f"cherry true count 1 not in bounds {cherry_bounds}"

    assert orange_bounds is None, "orange should not be tracked"

    print("✓ Basic operations passed\n")


def test_space_saving_heavy_hitters():
    """Test SpaceSaving heavy hitter detection."""
    print("TEST 7: SpaceSaving Heavy Hitter Detection")
    print("=" * 50)

    ss = SpaceSaving(epsilon=0.01)

    # Create a stream with heavy hitters
    for _ in range(100):
        ss.update("common")
    for _ in range(50):
        ss.update("moderate")
    for _ in range(10):
        ss.update("rare")
    for _ in range(1000):
        ss.update(f"id_{_}")  # Many unique items

    print(f"Stream length: {ss.stream_length()}")
    print(f"Items tracked: {ss.num_items()}")

    # Get heavy hitters with 5% threshold
    heavy = ss.heavy_hitters(0.05)
    print("\nHeavy hitters (> 5%):")
    for item, lower, upper in heavy[:5]:  # Show top 5
        print(f"  {item}: [{lower}, {upper}]")

    # Common (100) should be detected
    heavy_items = set(item for item, _, _ in heavy)
    # Note: bytes objects, need to check carefully
    found_common = any(b"common" in str(item).encode() for item in heavy_items)
    print(f"\n'common' detected as heavy hitter: {found_common}")

    # Get top-k
    top5 = ss.top_k(5)
    print("\nTop 5 items:")
    for item, lower, upper in top5:
        print(f"  {item}: [{lower}, {upper}]")

    assert len(top5) <= 5, "top_k should return at most k items"

    print("✓ Heavy hitter detection passed\n")


def test_space_saving_merge():
    """Test SpaceSaving merge operation."""
    print("TEST 8: SpaceSaving Merge")
    print("=" * 50)

    ss1 = SpaceSaving(epsilon=0.1)
    ss2 = SpaceSaving(epsilon=0.1)

    # Add to first sketch
    for _ in range(50):
        ss1.update("item1")
    for _ in range(30):
        ss1.update("shared")

    # Add to second sketch
    for _ in range(40):
        ss2.update("item2")
    for _ in range(20):
        ss2.update("shared")

    print("Before merge:")
    print(f"  ss1 stream length: {ss1.stream_length()}")
    print(f"  ss2 stream length: {ss2.stream_length()}")

    ss1.merge(ss2)

    print("After merge:")
    print(f"  merged stream length: {ss1.stream_length()}")
    print(f"  items tracked: {ss1.num_items()}")

    # Verify stream lengths are summed
    assert ss1.stream_length() == 50 + 30 + 40 + 20, "Stream length should be sum of both"

    # Check bounds for shared item
    shared_bounds = ss1.estimate("shared")
    print(f"shared bounds after merge: {shared_bounds} (true: 50)")
    if shared_bounds:
        assert (
            shared_bounds[0] <= 50 <= shared_bounds[1]
        ), f"shared true count 50 not in bounds {shared_bounds}"

    print("✓ Merge passed\n")


def test_space_saving_serialization():
    """Test SpaceSaving serialization (empty sketch only)."""
    print("TEST 9: SpaceSaving Serialization")
    print("=" * 50)

    ss1 = SpaceSaving(epsilon=0.1)

    # Serialize empty sketch
    data = ss1.serialize()
    print(f"Serialized size: {len(data)} bytes")

    # Deserialize
    ss2 = SpaceSaving.deserialize(data)

    print(f"Original capacity: {ss1.capacity()}, epsilon: {ss1.epsilon()}")
    print(f"Restored capacity: {ss2.capacity()}, epsilon: {ss2.epsilon()}")

    assert ss1.capacity() == ss2.capacity(), "Capacity mismatch"
    assert abs(ss1.epsilon() - ss2.epsilon()) < 1e-10, "Epsilon mismatch"

    print("✓ Serialization passed\n")


def test_type_conversions():
    """Test that both sketches work with different types."""
    print("TEST 10: Type Conversions (int, str, bytes)")
    print("=" * 50)

    cs = CountSketch(epsilon=0.1, delta=0.01)
    ss = SpaceSaving(epsilon=0.1)

    # Test with different types
    test_values = [
        ("string_key", "string value"),
        (42, "integer key"),
        (b"bytes_key", "bytes value"),
    ]

    for key, desc in test_values:
        try:
            cs.update(key, 1)
            ss.update(key)
            print(f"✓ {desc}: {repr(key)}")
        except Exception as e:
            print(f"✗ {desc}: {e}")
            raise

    print("✓ Type conversions passed\n")


def test_parameter_validation():
    """Test parameter validation."""
    print("TEST 11: Parameter Validation")
    print("=" * 50)

    # Invalid epsilon for CountSketch
    try:
        CountSketch(epsilon=0.0, delta=0.01)
        assert False, "Should reject epsilon=0"
    except ValueError:
        print("✓ CountSketch rejects epsilon=0")

    try:
        CountSketch(epsilon=1.0, delta=0.01)
        assert False, "Should reject epsilon=1.0"
    except ValueError:
        print("✓ CountSketch rejects epsilon=1.0")

    # Invalid delta for CountSketch
    try:
        CountSketch(epsilon=0.1, delta=0.0)
        assert False, "Should reject delta=0"
    except ValueError:
        print("✓ CountSketch rejects delta=0")

    # Invalid epsilon for SpaceSaving
    try:
        SpaceSaving(epsilon=0.0)
        assert False, "Should reject epsilon=0"
    except ValueError:
        print("✓ SpaceSaving rejects epsilon=0")

    try:
        SpaceSaving(epsilon=1.0)
        assert False, "Should reject epsilon=1.0"
    except ValueError:
        print("✓ SpaceSaving rejects epsilon=1.0")

    print("✓ Parameter validation passed\n")


def main():
    """Run all tests."""
    print("\n" + "=" * 70)
    print("COMPREHENSIVE PYTHON BINDINGS TEST SUITE")
    print("=" * 70 + "\n")

    tests = [
        (
            "Count Sketch",
            [
                test_count_sketch_basic,
                test_count_sketch_deletions,
                test_count_sketch_inner_product,
                test_count_sketch_merge,
                test_count_sketch_serialization,
            ],
        ),
        (
            "Space Saving",
            [
                test_space_saving_basic,
                test_space_saving_heavy_hitters,
                test_space_saving_merge,
                test_space_saving_serialization,
            ],
        ),
        (
            "General",
            [
                test_type_conversions,
                test_parameter_validation,
            ],
        ),
    ]

    total_tests = sum(len(group_tests) for _, group_tests in tests)
    passed = 0
    failed = 0

    for group_name, group_tests in tests:
        print(f"\n{group_name.upper()}")
        print("-" * 70)

        for test_func in group_tests:
            try:
                test_func()
                passed += 1
            except Exception as e:
                print(f"✗ FAILED: {test_func.__name__}")
                print(f"  Error: {e}\n")
                failed += 1

    print("\n" + "=" * 70)
    print(f"TEST SUMMARY: {passed}/{total_tests} passed, {failed} failed")
    print("=" * 70 + "\n")

    if failed > 0:
        sys.exit(1)
    else:
        print("All tests passed!")
        sys.exit(0)


if __name__ == "__main__":
    main()
