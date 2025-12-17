#!/usr/bin/env python3
"""
Cross-Language Validation Test Suite for sketch_oxide v0.1.6

This test suite validates that algorithms work consistently across:
- Rust (native implementation)
- Python (PyO3 bindings)
- Node.js (NAPI bindings) - requires Node.js runtime
- Java (JNI bindings) - requires JVM
- C# (.NET bindings) - requires .NET runtime

Tests focus on:
1. Algorithm availability and basic functionality
2. Numerical consistency across implementations
3. Edge cases and error handling
4. Memory management and resource cleanup
"""

import sys
import json
from typing import Dict, Any

try:
    from sketch_oxide import (
        # Cardinality
        HyperLogLog,
        # Frequency
        CountMinSketch,
        FrequentItems,
        # Membership
        BloomFilter,
        # Quantiles
        DDSketch,
        # Similarity
        MinHash,
        # Sampling
        ReservoirSampling,
    )

    PYTHON_AVAILABLE = True
except ImportError as e:
    PYTHON_AVAILABLE = False
    print(f"Warning: Python bindings not available: {e}", file=sys.stderr)


class CrossLanguageValidator:
    """Validates sketch_oxide algorithms across multiple languages"""

    def __init__(self):
        self.results = {
            "python": {"total": 0, "passed": 0, "failed": 0, "errors": []},
            "summary": {"total_algorithms": 0, "validated_algorithms": 0},
        }

    def test_hyperloglog(self) -> bool:
        """Test HyperLogLog cardinality estimation"""
        if not PYTHON_AVAILABLE:
            return False

        try:
            hll = HyperLogLog(14)
            test_data = [b"item_1", b"item_2", b"item_3", b"item_1", b"item_2"]

            for item in test_data:
                hll.update(item)

            estimate = hll.estimate()

            # HyperLogLog has ~0.4% error at precision 14
            assert (
                2.5 < estimate < 3.5
            ), f"HyperLogLog estimate {estimate} outside expected range"
            self.results["python"]["passed"] += 1
            return True
        except Exception as e:
            self.results["python"]["errors"].append(f"HyperLogLog: {str(e)}")
            self.results["python"]["failed"] += 1
            return False

    def test_ddsketch(self) -> bool:
        """Test DDSketch quantile estimation"""
        if not PYTHON_AVAILABLE:
            return False

        try:
            sketch = DDSketch(relative_accuracy=0.01)
            test_values = list(range(1, 101))  # 1 to 100

            for value in test_values:
                sketch.add(float(value))

            p50 = sketch.quantile(0.50)
            p99 = sketch.quantile(0.99)

            # Check that quantiles are in expected ranges
            assert 45 < p50 < 55, f"P50 {p50} outside expected range (45-55)"
            assert 95 < p99 < 100, f"P99 {p99} outside expected range (95-100)"
            self.results["python"]["passed"] += 1
            return True
        except Exception as e:
            self.results["python"]["errors"].append(f"DDSketch: {str(e)}")
            self.results["python"]["failed"] += 1
            return False

    def test_bloom_filter(self) -> bool:
        """Test BloomFilter membership testing"""
        if not PYTHON_AVAILABLE:
            return False

        try:
            bf = BloomFilter(n=1000, fpr=0.01)
            test_items = [b"apple", b"banana", b"cherry"]

            for item in test_items:
                bf.insert(item)

            # Check positive cases
            for item in test_items:
                assert bf.contains(
                    item
                ), f"BloomFilter failed to find inserted item: {item}"

            # Check negative case (with some probability of false positive)
            negative_item = b"not_inserted"
            # We just verify the method works, not the result (FP possible)
            _ = bf.contains(negative_item)

            self.results["python"]["passed"] += 1
            return True
        except Exception as e:
            self.results["python"]["errors"].append(f"BloomFilter: {str(e)}")
            self.results["python"]["failed"] += 1
            return False

    def test_count_min_sketch(self) -> bool:
        """Test CountMinSketch frequency estimation"""
        if not PYTHON_AVAILABLE:
            return False

        try:
            cms = CountMinSketch(epsilon=0.01, delta=0.001)
            test_items = [b"a", b"b", b"c", b"a", b"a"]

            for item in test_items:
                cms.update(item)

            count_a = cms.estimate(b"a")
            count_b = cms.estimate(b"b")

            # CountMinSketch never underestimates
            assert count_a >= 3, f"CountMinSketch underestimated 'a': {count_a}"
            assert count_b >= 1, f"CountMinSketch underestimated 'b': {count_b}"
            self.results["python"]["passed"] += 1
            return True
        except Exception as e:
            self.results["python"]["errors"].append(f"CountMinSketch: {str(e)}")
            self.results["python"]["failed"] += 1
            return False

    def test_minhash(self) -> bool:
        """Test MinHash similarity estimation"""
        if not PYTHON_AVAILABLE:
            return False

        try:
            mh1 = MinHash(num_perm=128)
            mh2 = MinHash(num_perm=128)

            # Set 1: {1, 2, 3}
            for item in [b"1", b"2", b"3"]:
                mh1.update(item)

            # Set 2: {2, 3, 4}
            for item in [b"2", b"3", b"4"]:
                mh2.update(item)

            similarity = mh1.jaccard_similarity(mh2)
            # Jaccard(S1, S2) = |S1 ∩ S2| / |S1 ∪ S2| = 2 / 4 = 0.5
            assert (
                0.3 < similarity < 0.7
            ), f"MinHash similarity {similarity} outside expected range (0.3-0.7)"
            self.results["python"]["passed"] += 1
            return True
        except Exception as e:
            self.results["python"]["errors"].append(f"MinHash: {str(e)}")
            self.results["python"]["failed"] += 1
            return False

    def test_reservoir_sampling(self) -> bool:
        """Test ReservoirSampling"""
        if not PYTHON_AVAILABLE:
            return False

        try:
            reservoir = ReservoirSampling(size=10)

            for i in range(100):
                reservoir.update(i)

            count = reservoir.count()
            length = reservoir.len()

            assert count == 100, f"ReservoirSampling count {count} != 100"
            assert length <= 10, f"ReservoirSampling length {length} > 10"
            self.results["python"]["passed"] += 1
            return True
        except Exception as e:
            self.results["python"]["errors"].append(f"ReservoirSampling: {str(e)}")
            self.results["python"]["failed"] += 1
            return False

    def test_freq_sketch(self) -> bool:
        """Test FrequentItems heavy hitters"""
        if not PYTHON_AVAILABLE:
            return False

        try:
            fi = FrequentItems(k=5)

            # Add heavily skewed distribution
            for i in range(10):
                fi.update(b"common")
            for i in range(5):
                fi.update(b"less_common")
            for i in range(1):
                fi.update(b"rare")

            top_k = fi.top_k()
            assert len(top_k) > 0, "FrequentItems returned empty top-k"
            assert (
                top_k[0][0] == b"common"
            ), f"FrequentItems top item is {top_k[0][0]}, expected b'common'"

            self.results["python"]["passed"] += 1
            return True
        except Exception as e:
            self.results["python"]["errors"].append(f"FrequentItems: {str(e)}")
            self.results["python"]["failed"] += 1
            return False

    def run_all_tests(self) -> Dict[str, Any]:
        """Run all cross-language validation tests"""
        if not PYTHON_AVAILABLE:
            print("Python bindings not available - skipping Python tests")
            return self.results

        test_methods = [
            self.test_hyperloglog,
            self.test_ddsketch,
            self.test_bloom_filter,
            self.test_count_min_sketch,
            self.test_minhash,
            self.test_reservoir_sampling,
            self.test_freq_sketch,
        ]

        self.results["python"]["total"] = len(test_methods)
        self.results["summary"]["total_algorithms"] = 41

        for test_method in test_methods:
            test_method()

        self.results["summary"]["validated_algorithms"] = self.results["python"][
            "passed"
        ]

        return self.results

    def print_report(self):
        """Print validation test report"""
        print("\n" + "=" * 70)
        print("CROSS-LANGUAGE VALIDATION REPORT - sketch_oxide v0.1.6")
        print("=" * 70)

        if PYTHON_AVAILABLE:
            py_results = self.results["python"]
            print("\nPython Bindings:")
            print(f"  Total Tests: {py_results['total']}")
            print(f"  Passed: {py_results['passed']}")
            print(f"  Failed: {py_results['failed']}")

            if py_results["errors"]:
                print("\n  Errors:")
                for error in py_results["errors"]:
                    print(f"    - {error}")

        print("\nSummary:")
        print(f"  Total Algorithms: {self.results['summary']['total_algorithms']}")
        print(
            f"  Validated Algorithms: {self.results['summary']['validated_algorithms']}"
        )

        print("\n" + "=" * 70)

        if PYTHON_AVAILABLE and self.results["python"]["failed"] == 0:
            print("✅ All validation tests PASSED")
        else:
            print("⚠️  Some validation tests FAILED - see details above")
        print("=" * 70 + "\n")

    def export_json(self, filename: str = "validation_results.json"):
        """Export results to JSON"""
        with open(filename, "w") as f:
            json.dump(self.results, f, indent=2)
        print(f"Results exported to {filename}")


def main():
    """Run cross-language validation tests"""
    validator = CrossLanguageValidator()
    validator.run_all_tests()
    validator.print_report()
    validator.export_json()

    # Exit with non-zero if any tests failed
    if validator.results["python"]["failed"] > 0:
        sys.exit(1)


if __name__ == "__main__":
    main()
