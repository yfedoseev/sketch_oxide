# sketch_oxide Cross-Language Test Coverage Harmonization Plan

**Document Version:** 1.0
**Date:** 2025-11-25
**Author:** Architecture Analysis

---

## Executive Summary

This document provides a comprehensive analysis of the current unit test coverage across all language bindings in the sketch_oxide project (C#, Python, Node.js, Java), identifies coverage gaps, and proposes a harmonization strategy to achieve feature parity. The C# test suite serves as the "Gold Standard" reference due to its comprehensive coverage, systematic structure, and adherence to best practices.

**Key Findings:**
- C# has the most comprehensive test coverage with systematic test categories
- Python has good coverage but inconsistent organization between files
- Node.js has reasonable coverage but lacks depth in edge cases
- Java has the largest gaps, missing several critical sketches entirely

**Recommendation:** Implement a **Guided Harmonization Strategy** (hybrid approach) that maintains language-specific idioms while following a unified test specification.

---

## Section 1: Current State Analysis

### 1.1 Test File Inventory

#### C# (Gold Standard) - 15 Test Files
| File | Lines | Sketch | Test Count | Categories |
|------|-------|--------|------------|------------|
| BloomFilterTests.cs | ~465 | BloomFilter | 34 | Constructor, Insert, Contains, FPR, Serialization, Disposal, Large Dataset |
| HyperLogLogTests.cs | ~262 | HyperLogLog | 23 | Constructor, Update, Estimate, Merge, Serialization, Disposal |
| CountMinSketchTests.cs | ~467 | CountMinSketch | 33 | Constructor, Update, Estimate, Merge, Serialization, Disposal, Large Dataset |
| DDSketchTests.cs | ~562 | DDSketch | 35 | Constructor, Update, Quantile, Min/Max/Median, Distribution, Serialization, Disposal |
| MinHashTests.cs | ~590 | MinHash | 37 | Constructor, Update, JaccardSimilarity, Serialization, Disposal, Edge Cases |
| HeavyKeeperTests.cs | ~234 | HeavyKeeper | 17 | Constructor, Update, TopK, Decay, Batch, Disposal |
| RatelessIBLTTests.cs | N/A | RatelessIBLT | ~15 | Constructor, Insert, Decode, Subtract |
| GrafiteTests.cs | ~130 | Grafite/GRF | 10 | Constructor, Queries, FPR |
| MementoFilterTests.cs | N/A | MementoFilter | ~12 | Constructor, Insert, Range Query |
| SlidingHyperLogLogTests.cs | N/A | SlidingHyperLogLog | ~15 | Constructor, Update, Window Estimation, Decay, Merge |
| VacuumFilterTests.cs | ~154 | VacuumFilter | 13 | Constructor, Insert, Delete, Stats |
| GRFTests.cs | ~130 | GRF | 10 | Constructor, Queries, Stats |
| NitroSketchTests.cs | N/A | NitroSketch | ~12 | Constructor, Sampling, Stats |
| LearnedBloomFilterTests.cs | ~153 | LearnedBloomFilter | 11 | Constructor, Contains, Memory |
| UnivMonTests.cs | N/A | UnivMon | ~15 | Constructor, Update, Metrics, ChangeDetection |

**Total: ~388 tests across 15 sketches**

#### Python - 6 Test Files (in tests/)
| File | Sketches Covered | Test Approach |
|------|-----------------|---------------|
| test_basic.py | Basic functionality | Minimal |
| test_new_sketches.py | HeavyKeeper, RatelessIBLT, Grafite, MementoFilter, SlidingHyperLogLog | Pytest classes |
| test_tier2_sketches.py | VacuumFilter, GRF, NitroSketch, UnivMon, LearnedBloomFilter | Comprehensive pytest |
| test_py_bindings.py | CountSketch, SpaceSaving | Manual test runner |
| test_spline_sketch.py | SplineSketch | Specialized |
| test_exponential_histogram.py | ExponentialHistogram | Specialized |

**Additional root-level test files:** test_salsa.py, test_elastic_sketch.py, test_removable_sketch.py, test_qsketch.py, test_all_9_algorithms.py, test_bindings.py

**Total: ~295 tests across ~11 sketches**

#### Node.js - 3 Test Files
| File | Sketches Covered | Test Count |
|------|-----------------|------------|
| hyperloglog.test.ts | HyperLogLog (basic) | ~10 |
| tier1_new_sketches.test.ts | HeavyKeeper, RatelessIBLT, Grafite, MementoFilter, SlidingHyperLogLog | ~55 |
| tier2_sketches.test.ts | VacuumFilter, GRF, NitroSketch, UnivMon, LearnedBloomFilter, CountMinSketch | ~79 |

**Total: ~144 tests across ~11 sketches**

#### Java - 4 Test Files
| File | Sketches Covered | Test Count |
|------|-----------------|------------|
| HyperLogLogTest.java | HyperLogLog | 15 |
| NewSketchesTest.java | HeavyKeeper, RatelessIBLT, Grafite, MementoFilter, SlidingHyperLogLog | ~28 |
| VacuumFilterTest.java | VacuumFilter | 12 |
| Tier2SketchesTest.java | GRF, NitroSketch, UnivMon, LearnedBloomFilter | ~60 |

**Total: ~125 tests across ~9 sketches**

---

### 1.2 Sketch Coverage Matrix

| Sketch | Category | C# | Python | Node.js | Java |
|--------|----------|:--:|:------:|:-------:|:----:|
| **BloomFilter** | Membership | 34 | 0 | 0 | 0 |
| **CuckooFilter** | Membership | 0 | 0 | 0 | 0 |
| **HyperLogLog** | Cardinality | 23 | 5 | 10 | 15 |
| **CountMinSketch** | Frequency | 33 | 15 | 10 | 0 |
| **DDSketch** | Quantiles | 35 | 0 | 0 | 0 |
| **MinHash** | Similarity | 37 | 0 | 0 | 0 |
| **HeavyKeeper** | Frequency | 17 | 20 | 12 | 5 |
| **RatelessIBLT** | Reconciliation | 15 | 10 | 8 | 4 |
| **Grafite/GRF** | Range Filter | 10+10 | 15 | 12 | 15 |
| **MementoFilter** | Range Filter | 12 | 10 | 6 | 3 |
| **SlidingHyperLogLog** | Streaming | 15 | 18 | 10 | 7 |
| **VacuumFilter** | Membership | 13 | 20 | 18 | 12 |
| **NitroSketch** | Network | 12 | 12 | 10 | 15 |
| **UnivMon** | Universal | 15 | 20 | 12 | 15 |
| **LearnedBloomFilter** | ML Membership | 11 | 16 | 12 | 15 |
| **CountSketch** | Frequency | 0 | 10 | 0 | 0 |
| **SpaceSaving** | Frequency | 0 | 10 | 0 | 0 |
| **SplineSketch** | Quantiles | 0 | 10 | 5 | 0 |
| **StableBloomFilter** | Streaming | 0 | 0 | 5 | 0 |
| **ExponentialHistogram** | Streaming | 0 | 10 | 0 | 0 |

**Legend:**
- 0 = Not covered (Gap)
- 5-10 = Light coverage
- 10-20 = Moderate coverage
- 20+ = Comprehensive coverage

---

### 1.3 C# Gold Standard Test Pattern Analysis

The C# tests establish a systematic pattern that should guide harmonization:

#### Test Category Structure (Per Sketch)
```
1. Constructor Tests
   - Valid parameters (boundary values)
   - Invalid parameters (each parameter validated)
   - Theory/parameterized tests for multiple configurations

2. Core Operation Tests
   - Basic operations (Insert/Update/Query)
   - Null handling
   - Empty input handling
   - Duplicate handling

3. Algorithm-Specific Tests
   - Accuracy verification (within error bounds)
   - Statistical guarantees (no false negatives, FPR bounds)
   - Probabilistic behavior validation

4. Merge/Combine Tests (where applicable)
   - Compatible merges
   - Incompatible parameter merges
   - Null merge rejection

5. Serialization Tests
   - Serialize with data
   - Serialize empty sketch
   - Round-trip preservation
   - Invalid data rejection
   - Null data rejection

6. Disposal/Resource Management Tests
   - Prevents further operations
   - Multiple dispose calls (idempotent)
   - Using pattern works

7. ToString Tests
   - Contains key parameters
   - Indicates disposed state

8. Large Dataset / Edge Case Tests
   - Stress testing
   - Unicode/Binary data handling
   - Boundary value testing
```

#### Code Quality Patterns
- `IDisposable` implementation with fixture setup/teardown
- `#region` grouping for test categories
- `[Theory]` / `[InlineData]` for parameterized tests
- Descriptive test names: `MethodName_Scenario_ExpectedBehavior`
- Explicit assertions with meaningful failure messages
- Documentation comments on test classes

---

## Section 2: Gap Analysis

### 2.1 Critical Missing Coverage (By Language)

#### Java - CRITICAL GAPS
1. **BloomFilter** - No tests (entire sketch missing)
2. **CuckooFilter** - No tests (entire sketch missing)
3. **CountMinSketch** - No tests (entire sketch missing)
4. **DDSketch** - No tests (entire sketch missing)
5. **MinHash** - No tests (entire sketch missing)
6. **CountSketch** - No tests
7. **SpaceSaving** - No tests

**Priority:** HIGH - Java is missing 7+ core sketches

#### Node.js - MODERATE GAPS
1. **BloomFilter** - No tests
2. **CuckooFilter** - No tests
3. **DDSketch** - No tests
4. **MinHash** - No tests
5. **CountSketch** - No tests
6. **SpaceSaving** - No tests
7. **ExponentialHistogram** - No tests

**Priority:** MEDIUM - Missing foundational membership and similarity sketches

#### Python - LIGHT GAPS
1. **BloomFilter** - No tests in test directory (may exist in root)
2. **CuckooFilter** - No tests
3. **DDSketch** - No tests
4. **MinHash** - No tests

**Priority:** MEDIUM-LOW - Missing some core sketches, organization needs improvement

#### C# - MINIMAL GAPS
1. **CuckooFilter** - No dedicated tests
2. **CountSketch** - No tests
3. **SpaceSaving** - No tests
4. **SplineSketch** - No tests
5. **ExponentialHistogram** - No tests

**Priority:** LOW - Mostly complete, some newer sketches need coverage

### 2.2 Test Depth Gaps

Even where sketches are covered, depth varies significantly:

| Test Category | C# | Python | Node.js | Java |
|--------------|:--:|:------:|:-------:|:----:|
| Parameter Validation | Complete | Partial | Partial | Partial |
| Error Bounds Verification | Complete | Partial | Light | Partial |
| Serialization Round-Trip | Complete | Partial | Light | Partial |
| Disposal/Resource Mgmt | Complete | N/A | N/A | Partial |
| Edge Cases (Unicode/Binary) | Complete | Light | Light | Light |
| Large Dataset Stress | Complete | Light | Light | Partial |
| Statistical Guarantees | Complete | Partial | Light | Partial |

---

## Section 3: Testing Strategy Recommendations

### 3.1 Strategy Options Analysis

#### Option A: Unified Test Specification
**Description:** Create a JSON/YAML specification that defines all test cases, scenarios, and expected behaviors. Each language implements tests from this specification.

**Pros:**
- Guaranteed feature parity
- Single source of truth
- Easier to audit coverage

**Cons:**
- High initial effort to create spec
- May fight against language idioms
- Maintenance overhead for spec synchronization
- Overkill for current project size

**Verdict:** Over-engineering for this project

#### Option B: Language-Specific Autonomy
**Description:** Each language team writes tests independently following general guidelines.

**Pros:**
- Natural language idioms
- Lower coordination overhead
- Faster initial implementation

**Cons:**
- Coverage drift over time
- Inconsistent depth
- Duplication of effort in test design

**Verdict:** Current state, leading to identified gaps

#### Option C: Guided Harmonization (RECOMMENDED)
**Description:** Use C# as the reference implementation. Create a "Test Coverage Checklist" document that other languages must satisfy, but implementation details are language-specific.

**Pros:**
- Balances consistency with flexibility
- Clear reference for what "complete" means
- Maintains language idioms
- Moderate coordination overhead

**Cons:**
- Requires periodic audits
- C# changes need to propagate

**Verdict:** Best balance of consistency and pragmatism

### 3.2 Shared Test Data Strategy

**Recommendation:** Create shared test data files where beneficial:

```
/test_data/
  sketches/
    bloom_filter/
      insert_items.json          # Common items to insert
      fpr_test_queries.json      # Items to test false positives
      serialized_reference.bin   # Reference serialization (cross-lang)
    hyperloglog/
      cardinality_test_items.json
      expected_estimates.json
    minhash/
      document_pairs.json        # Test documents for similarity
      expected_jaccard.json
```

**When to use shared data:**
- Reproducible accuracy tests
- Cross-language serialization compatibility
- Large dataset tests
- Known edge cases

**When NOT to use:**
- Language-specific edge cases
- Exception message validation
- Resource management tests

---

## Section 4: Implementation Plan

### Phase 1: Java Critical Coverage (Effort: XL)
**Goal:** Bring Java to parity with core sketches

#### 4.1.1 BloomFilter Tests
- [ ] (M) Create BloomFilterTest.java with constructor tests
- [ ] (M) Add insert and contains tests
- [ ] (M) Add FPR verification tests
- [ ] (S) Add serialization tests
- [ ] (S) Add resource management tests
- [ ] (S) Add edge cases (empty, Unicode)

**Acceptance Criteria:** 25+ tests covering all C# categories

#### 4.1.2 CountMinSketch Tests
- [ ] (M) Create CountMinSketchTest.java with constructor tests
- [ ] (M) Add update and estimate tests
- [ ] (M) Add no-underestimate guarantee tests
- [ ] (S) Add merge tests
- [ ] (S) Add serialization tests

**Acceptance Criteria:** 25+ tests

#### 4.1.3 DDSketch Tests
- [ ] (L) Create DDSketchTest.java with constructor tests
- [ ] (L) Add quantile query tests with accuracy verification
- [ ] (M) Add min/max/median tests
- [ ] (M) Add distribution tests (uniform, latency, bimodal)
- [ ] (S) Add serialization tests

**Acceptance Criteria:** 30+ tests

#### 4.1.4 MinHash Tests
- [ ] (L) Create MinHashTest.java with constructor tests
- [ ] (L) Add Jaccard similarity tests (identical, disjoint, partial overlap)
- [ ] (M) Add symmetry and accuracy tests
- [ ] (S) Add serialization tests
- [ ] (S) Add document shingling real-world test

**Acceptance Criteria:** 25+ tests

#### 4.1.5 CuckooFilter Tests (if exposed in Java)
- [ ] (M) Create CuckooFilterTest.java
- [ ] (M) Add insert, contains, delete tests
- [ ] (S) Add capacity tests

**Acceptance Criteria:** 15+ tests

### Phase 2: Node.js Coverage Expansion (Effort: L)

#### 4.2.1 BloomFilter Tests
- [ ] (M) Create bloom_filter.test.ts
- [ ] (M) Port constructor, insert, contains tests from C#
- [ ] (S) Add FPR verification
- [ ] (S) Add serialization tests

**Acceptance Criteria:** 20+ tests

#### 4.2.2 CountMinSketch Enhancement
- [ ] (M) Add merge tests
- [ ] (M) Add accuracy verification tests
- [ ] (S) Add edge cases

**Acceptance Criteria:** Increase from 10 to 25+ tests

#### 4.2.3 DDSketch Tests
- [ ] (L) Create ddsketch.test.ts
- [ ] (L) Port all quantile tests from C#
- [ ] (M) Add distribution tests

**Acceptance Criteria:** 25+ tests

#### 4.2.4 MinHash Tests
- [ ] (L) Create minhash.test.ts
- [ ] (L) Port all similarity tests from C#

**Acceptance Criteria:** 25+ tests

### Phase 3: Python Organization & Enhancement (Effort: M)

#### 4.3.1 Test File Consolidation
- [ ] (M) Move root-level test files to tests/ directory
- [ ] (S) Standardize on pytest classes
- [ ] (S) Add conftest.py with shared fixtures

#### 4.3.2 BloomFilter Tests
- [ ] (M) Create test_bloom_filter.py
- [ ] (M) Add all C# test categories
- [ ] (S) Use pytest.mark.parametrize for parameterized tests

**Acceptance Criteria:** 25+ tests

#### 4.3.3 DDSketch Tests
- [ ] (L) Create test_ddsketch.py
- [ ] (L) Port quantile and distribution tests

**Acceptance Criteria:** 25+ tests

#### 4.3.4 MinHash Tests
- [ ] (L) Create test_minhash.py
- [ ] (L) Port similarity tests

**Acceptance Criteria:** 25+ tests

### Phase 4: C# Completion & Documentation (Effort: S)

#### 4.4.1 Missing Sketch Coverage
- [ ] (M) Create CuckooFilterTests.cs (if exposed)
- [ ] (S) Create CountSketchTests.cs
- [ ] (S) Create SpaceSavingTests.cs

#### 4.4.2 Test Documentation
- [ ] (S) Add XML documentation to all test classes
- [ ] (S) Ensure consistent region grouping
- [ ] (S) Verify all tests have descriptive names

### Phase 5: Cross-Language Validation (Effort: M)

#### 4.5.1 Serialization Compatibility
- [ ] (L) Create cross-language serialization tests
- [ ] (M) Verify sketches serialized in Rust can be deserialized in all languages
- [ ] (S) Document any incompatibilities

#### 4.5.2 Shared Test Data
- [ ] (M) Create shared test data files for reproducible tests
- [ ] (S) Document shared data format

---

## Section 5: Technical Debt Identified

### 5.1 Architecture Debt

**[DEBT:architecture:MEDIUM] Python test file organization**
- Root-level test files (`test_salsa.py`, etc.) should be in `tests/`
- Mix of manual test runners and pytest
- Resolution: Consolidate during Phase 3

**[DEBT:architecture:LOW] C# missing some newer sketches**
- CountSketch, SpaceSaving not covered
- Resolution: Phase 4

### 5.2 Testing Debt

**[DEBT:testing:HIGH] Java missing core sketch tests**
- 7+ sketches have zero test coverage
- Blocks confidence in Java bindings quality
- Resolution: Phase 1 (Priority)

**[DEBT:testing:MEDIUM] Node.js serialization tests incomplete**
- Most sketches lack serialization round-trip tests
- Resolution: Phase 2

**[DEBT:testing:MEDIUM] Inconsistent error message validation**
- Some tests validate exact error messages, others just check exception type
- Consider standardizing on exception type + key content check
- Resolution: Ongoing, not blocking

### 5.3 Documentation Debt

**[DEBT:documentation:LOW] Test naming inconsistency**
- Python uses `test_method_name`, Java uses `testMethodName`, Node.js uses `it('should...')`
- Acceptable - follows language conventions
- No action needed

---

## Section 6: Risk Assessment

### 6.1 Implementation Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|-----------|
| Java bindings have bugs exposed by tests | Medium | Medium | Fix bugs as discovered; tests are valuable |
| Serialization format incompatible cross-language | Low | High | Test early in Phase 5 |
| Test implementation takes longer than estimated | Medium | Low | Phases are independent; prioritize critical gaps |
| C# reference becomes outdated | Low | Medium | Review C# changes; update checklist |

### 6.2 Technical Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|-----------|
| Different FFI behaviors across languages | Low | Medium | Document differences; accept some variance |
| Memory management differences | Low | Medium | Language-specific disposal tests acceptable |
| Statistical test flakiness | Medium | Low | Use wider bounds; document expected variance |

---

## Section 7: Success Metrics

### 7.1 Coverage Targets

| Language | Current Tests | Target Tests | Target Sketches |
|----------|--------------|--------------|-----------------|
| C# | 388 | 420 | 20 (all) |
| Python | 295 | 400 | 18 |
| Node.js | 144 | 300 | 15 |
| Java | 125 | 350 | 15 |

### 7.2 Quality Metrics

- **Test/Code Ratio Target:** 1.5-2.0 per language binding
- **Pass Rate:** 100% on all platforms
- **Flaky Test Rate:** <1%
- **Coverage of C# test categories:** 90%+ per sketch

### 7.3 Audit Checklist (Per Sketch)

```
[ ] Constructor validation tests (valid + invalid parameters)
[ ] Core operation tests (insert/update/query)
[ ] Null/empty input handling
[ ] Algorithm-specific guarantee tests
[ ] Merge tests (if applicable)
[ ] Serialization round-trip
[ ] Resource management / disposal
[ ] toString/repr tests
[ ] At least one large dataset test
[ ] At least one edge case test
```

---

## Section 8: Todo Summary

### Phase 1: Java Critical (Priority: P0)
- [ ] (XL) BloomFilter tests - 25+ tests
- [ ] (L) CountMinSketch tests - 25+ tests
- [ ] (XL) DDSketch tests - 30+ tests
- [ ] (L) MinHash tests - 25+ tests
- [ ] (M) CuckooFilter tests - 15+ tests

### Phase 2: Node.js Expansion (Priority: P1)
- [ ] (L) BloomFilter tests - 20+ tests
- [ ] (M) CountMinSketch enhancement - 15+ additional tests
- [ ] (L) DDSketch tests - 25+ tests
- [ ] (L) MinHash tests - 25+ tests

### Phase 3: Python Organization (Priority: P1)
- [ ] (M) Test file consolidation
- [ ] (L) BloomFilter tests - 25+ tests
- [ ] (L) DDSketch tests - 25+ tests
- [ ] (L) MinHash tests - 25+ tests

### Phase 4: C# Completion (Priority: P2)
- [ ] (M) CuckooFilter tests
- [ ] (S) CountSketch tests
- [ ] (S) SpaceSaving tests
- [ ] (S) Documentation polish

### Phase 5: Cross-Language Validation (Priority: P2)
- [ ] (L) Serialization compatibility tests
- [ ] (M) Shared test data creation

---

## Appendix A: Test Pattern Templates

### A.1 Java Test Template
```java
package com.sketches_oxide.membership;

import org.junit.jupiter.api.*;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;
import static org.junit.jupiter.api.Assertions.*;

@DisplayName("BloomFilter Membership Tests")
public class BloomFilterTest {

    private BloomFilter filter;

    @BeforeEach
    void setUp() {
        filter = new BloomFilter(1000, 0.01);
    }

    @AfterEach
    void tearDown() {
        if (filter != null) {
            filter.close();
        }
    }

    // Region: Constructor Tests

    @Test
    @DisplayName("Constructor with valid parameters should succeed")
    void testConstructorValidParameters() {
        try (BloomFilter bf = new BloomFilter(1000, 0.01)) {
            assertNotNull(bf);
            assertEquals(1000, bf.expectedElements());
        }
    }

    @ParameterizedTest
    @ValueSource(doubles = {0.0, -0.1, 1.0, 1.5})
    @DisplayName("Constructor with invalid FPR should throw")
    void testConstructorInvalidFpr(double fpr) {
        assertThrows(IllegalArgumentException.class,
            () -> new BloomFilter(1000, fpr));
    }

    // ... continue with other test categories
}
```

### A.2 Python Test Template
```python
"""Tests for BloomFilter membership sketch."""
import pytest
from sketch_oxide import BloomFilter


class TestBloomFilterConstruction:
    """Test BloomFilter construction and parameter validation."""

    def test_valid_parameters(self):
        """Test construction with valid parameters."""
        bf = BloomFilter(expected_elements=1000, fpr=0.01)
        assert bf.expected_elements == 1000
        assert bf.fpr == 0.01

    @pytest.mark.parametrize("fpr", [0.0, -0.1, 1.0, 1.5])
    def test_invalid_fpr(self, fpr):
        """Test that invalid FPR raises ValueError."""
        with pytest.raises(ValueError, match="fpr"):
            BloomFilter(expected_elements=1000, fpr=fpr)


class TestBloomFilterOperations:
    """Test BloomFilter insert and query operations."""

    @pytest.fixture
    def filter(self):
        """Create a BloomFilter for testing."""
        return BloomFilter(expected_elements=10000, fpr=0.01)

    def test_insert_and_contains(self, filter):
        """Test basic insert and membership query."""
        filter.insert("test-item")
        assert filter.contains("test-item")

    # ... continue with other tests
```

### A.3 Node.js Test Template
```typescript
import { BloomFilter } from '../index'

describe('BloomFilter', () => {
  describe('constructor', () => {
    it('should create filter with valid parameters', () => {
      const filter = new BloomFilter(1000, 0.01)
      expect(filter).toBeDefined()
      expect(filter.expectedElements()).toBe(1000)
    })

    it.each([0.0, -0.1, 1.0, 1.5])(
      'should throw for invalid FPR: %p',
      (fpr) => {
        expect(() => new BloomFilter(1000, fpr)).toThrow()
      }
    )
  })

  describe('insert and contains', () => {
    let filter: BloomFilter

    beforeEach(() => {
      filter = new BloomFilter(1000, 0.01)
    })

    afterEach(() => {
      filter.dispose?.()
    })

    it('should find inserted items', () => {
      filter.insert(Buffer.from('test-item'))
      expect(filter.contains(Buffer.from('test-item'))).toBe(true)
    })

    // ... continue with other tests
  })
})
```

---

## Appendix B: Sketch Test Category Requirements

### B.1 Membership Filters (BloomFilter, CuckooFilter, VacuumFilter)
- Constructor parameter validation
- Insert/Contains operations
- No false negatives guarantee
- FPR verification (statistical)
- Serialization
- Delete (for Cuckoo/Vacuum)
- Capacity/stats

### B.2 Cardinality Estimators (HyperLogLog, SlidingHyperLogLog)
- Constructor precision validation
- Update operations
- Estimate accuracy within bounds
- Duplicate handling
- Merge operations
- Serialization
- Window estimation (Sliding)

### B.3 Frequency Estimators (CountMinSketch, CountSketch, HeavyKeeper)
- Constructor parameter validation
- Update operations
- Estimate accuracy (no underestimate for CMS)
- Heavy hitter detection
- Merge operations
- Serialization
- Batch updates

### B.4 Quantile Sketches (DDSketch, SplineSketch)
- Constructor accuracy validation
- Update with various value ranges
- Quantile query accuracy
- Min/Max/Median helpers
- Distribution handling (uniform, skewed, bimodal)
- Serialization

### B.5 Similarity (MinHash)
- Constructor permutation validation
- Update operations
- Jaccard similarity accuracy
- Symmetry property
- Permutation count impact on accuracy
- Document shingling use case

### B.6 Range Filters (GRF, MementoFilter)
- Constructor validation
- Point queries
- Range queries
- FPR calculation
- Segment statistics

### B.7 Universal Monitoring (UnivMon)
- Constructor validation
- Update operations
- L1/L2 norm estimation
- Entropy estimation
- Heavy hitter detection
- Change detection
- Merge operations

---

*Document End*
