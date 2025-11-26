# Cross-Language Test Data and Integration Testing

## Overview

This document outlines the shared test data approach and integration testing patterns for validating consistency across sketch_oxide language bindings.

## Shared Test Datasets

### BloomFilter Test Data

```json
{
  "test_case": "bloom_filter_basic",
  "parameters": {
    "capacity": 10000,
    "false_positive_rate": 0.01
  },
  "operations": [
    {
      "op": "insert",
      "value": "apple"
    },
    {
      "op": "insert",
      "value": "banana"
    },
    {
      "op": "insert",
      "value": "cherry"
    }
  ],
  "expectations": {
    "contains_apple": true,
    "contains_banana": true,
    "contains_cherry": true,
    "contains_date": false,
    "is_empty": false
  }
}
```

### DDSketch Test Data

```json
{
  "test_case": "ddsketch_quantile_estimation",
  "parameters": {
    "relative_accuracy": 0.01
  },
  "operations": [
    {
      "op": "update",
      "values": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    }
  ],
  "expectations": {
    "quantile_0.25": {
      "min": 2.0,
      "max": 3.5,
      "description": "Q1 should be between 2 and 3.5"
    },
    "quantile_0.5": {
      "min": 4.5,
      "max": 6.0,
      "description": "Median should be between 4.5 and 6.0"
    },
    "quantile_0.75": {
      "min": 7.5,
      "max": 8.5,
      "description": "Q3 should be between 7.5 and 8.5"
    },
    "min": 1.0,
    "max": 10.0,
    "count": 10
  }
}
```

### MinHash Test Data

```json
{
  "test_case": "minhash_similarity",
  "parameters": {
    "num_perm": 128
  },
  "sets": [
    {
      "name": "set_A",
      "elements": [1, 2, 3, 4, 5]
    },
    {
      "name": "set_B",
      "elements": [3, 4, 5, 6, 7]
    },
    {
      "name": "set_C",
      "elements": [1, 2, 3]
    }
  ],
  "expectations": {
    "similarity_A_B": {
      "min": 0.35,
      "max": 0.55,
      "theoretical": 0.43,
      "description": "Jaccard(A,B) = 3/7 ≈ 0.43"
    },
    "similarity_A_C": {
      "min": 0.50,
      "max": 0.70,
      "theoretical": 0.60,
      "description": "Jaccard(A,C) = 3/5 = 0.60"
    },
    "similarity_B_A": {
      "min": 0.35,
      "max": 0.55,
      "description": "Should equal similarity_A_B (symmetry)"
    }
  }
}
```

## Cross-Language Integration Test Patterns

### Pattern 1: Serialize in Language A, Verify in Language A

**Purpose**: Baseline test that serialization works within a language

```
Language A:
1. Create sketch with test data
2. Serialize sketch
3. Deserialize sketch
4. Verify behavior matches original
```

### Pattern 2: API Consistency Across Languages

**Purpose**: Verify same operations produce equivalent results

```
All Languages:
1. Create sketch with identical parameters
2. Apply identical sequence of operations
3. Query equivalent results
4. Compare results (within tolerance for statistical sketches)
5. Verify error handling matches
```

### Pattern 3: Merge Operation Consistency

**Purpose**: Verify merging behavior is consistent across languages

```
For each language:
1. Create sketch_1, add items 1-500
2. Create sketch_2, add items 501-1000
3. Merge sketch_2 into sketch_1
4. Verify merged result contains both sets
5. Compare results with reference (sketch with all 1-1000)
```

### Pattern 4: Edge Case Handling

**Purpose**: Ensure consistent behavior for edge cases

```
Test cases to apply in all languages:
- Single item: [42]
- Constant values: [100, 100, 100, ...]
- Very small values: [0.0001, 0.0002, ...]
- Very large values: [1e10, 2e10, ...]
- Negative values: [-100, -50, 0, 50, 100]
- Mixed magnitude: [0.001, 1e8, 1e-5]

Expected: Same behavior pattern across all languages
```

## Serialization Compatibility Testing (Phase 5b)

### Format Specification

Each language maintains serialization specs:

**Java**:
```java
// Serialization format: [MAGIC_BYTES][VERSION][SKETCH_TYPE][PARAMETERS][DATA]
MAGIC_BYTES: 0x534B (SK for SketchOxide)
VERSION: 1 (current)
```

**Node.js**:
```typescript
// Binary buffer format matches Java for compatibility
const serialized = sketch.serialize(); // Returns Buffer
const restored = SketchType.deserialize(serialized);
```

**Python**:
```python
# Bytes format matches Node.js/Java for compatibility
serialized = sketch.to_bytes()  # Returns bytes
restored = SketchType.from_bytes(serialized)
```

**C#**:
```csharp
// Binary serialization compatible with Java format
byte[] serialized = sketch.Serialize();
var restored = SketchType.Deserialize(serialized);
```

### Compatibility Matrix (To Be Validated)

```
        Java   Node.js  Python  C#
Java    ✅     ?        ?       ?
Node.js ?      ✅       ?       ?
Python  ?      ?        ✅      ?
C#      ?      ?        ?       ✅
```

**Legend**:
- ✅ = Can deserialize own format (already implemented)
- ? = To be tested in Phase 5b
- ✗ = Known incompatibility

### Testing Procedure for Cross-Language Serialization

```
1. Java Test Suite:
   - Serialize 10 test sketches → test-data/java-serialized/

2. Node.js Test Suite:
   - Load Java serialized files
   - Attempt deserialization
   - Verify behavior matches original
   - Serialize Node.js sketches → test-data/nodejs-serialized/

3. Python Test Suite:
   - Load Java and Node.js serialized files
   - Attempt deserialization
   - Serialize Python sketches → test-data/python-serialized/

4. C# Test Suite:
   - Load all other language serialized files
   - Attempt deserialization
   - Verify behavior matches

5. Documentation:
   - Update compatibility matrix
   - Document version compatibility
   - Note any format differences
```

## Test Data Organization

Proposed directory structure:

```
docs/
├── test-data/
│   ├── bloom-filter/
│   │   ├── basic.json
│   │   ├── large-dataset.json
│   │   └── edge-cases.json
│   ├── ddsketch/
│   │   ├── uniform-distribution.json
│   │   ├── skewed-distribution.json
│   │   └── merge-test.json
│   ├── minhash/
│   │   ├── overlap-scenarios.json
│   │   ├── symmetry-test.json
│   │   └── large-sets.json
│   └── serialized/
│       ├── java/
│       │   ├── bloom-filter-001.bin
│       │   ├── ddsketch-001.bin
│       │   └── minhash-001.bin
│       ├── nodejs/
│       │   ├── bloom-filter-001.bin
│       │   └── ...
│       ├── python/
│       │   └── ...
│       └── dotnet/
│           └── ...
```

## Integration Test Examples

### Example: BloomFilter Consistency

```python
# Python: test_cross_language_bloom_filter.py
import json

def load_test_data(filename):
    with open(f'docs/test-data/bloom-filter/{filename}') as f:
        return json.load(f)

def test_bloom_filter_api_consistency():
    """Verify BloomFilter behaves consistently across languages"""
    data = load_test_data('basic.json')

    bf = sketch_oxide.BloomFilter(
        data['parameters']['capacity'],
        data['parameters']['false_positive_rate']
    )

    # Apply operations
    for op in data['operations']:
        if op['op'] == 'insert':
            bf.insert(op['value'].encode())

    # Verify expectations
    assert bf.contains(b'apple')
    assert bf.contains(b'banana')
    assert not bf.contains(b'date')
    assert not bf.is_empty()
```

```typescript
// Node.js: cross-language.test.ts
describe('Cross-Language Consistency', () => {
  it('should handle BloomFilter operations consistently', () => {
    const testData = require('../docs/test-data/bloom-filter/basic.json');

    const bf = new BloomFilter(
      testData.parameters.capacity,
      testData.parameters.false_positive_rate
    );

    testData.operations.forEach(op => {
      if (op.op === 'insert') {
        bf.insert(Buffer.from(op.value));
      }
    });

    expect(bf.contains(Buffer.from('apple'))).toBe(true);
    expect(bf.contains(Buffer.from('banana'))).toBe(true);
    expect(bf.contains(Buffer.from('date'))).toBe(false);
    expect(bf.is_empty()).toBe(false);
  });
});
```

```java
// Java: CrossLanguageTest.java
public class CrossLanguageTest {
  @Test
  public void testBloomFilterConsistency() throws IOException {
    String testDataPath = "docs/test-data/bloom-filter/basic.json";
    JSONObject testData = new JSONObject(
        new String(Files.readAllBytes(Paths.get(testDataPath)))
    );

    BloomFilter bf = new BloomFilter(
        testData.getJSONObject("parameters").getInt("capacity"),
        testData.getJSONObject("parameters").getDouble("false_positive_rate")
    );

    JSONArray ops = testData.getJSONArray("operations");
    for (int i = 0; i < ops.length(); i++) {
      JSONObject op = ops.getJSONObject(i);
      if ("insert".equals(op.getString("op"))) {
        bf.insert(op.getString("value").getBytes());
      }
    }

    assertTrue(bf.contains("apple".getBytes()));
    assertTrue(bf.contains("banana".getBytes()));
    assertFalse(bf.contains("date".getBytes()));
    assertFalse(bf.isEmpty());
  }
}
```

## Success Criteria

✅ **Phase 5a Complete**:
- Cross-language validation documentation created
- Test data structure defined
- Integration test patterns documented
- Compatibility matrix created

📋 **Phase 5b Goals**:
- Serialized test data files created in each language
- Cross-language deserialization tested
- Compatibility matrix filled in
- Breaking change procedures documented

## Known Issues and Limitations

1. **Binary Format**: Serialization formats may differ by language
   - Java uses Kryo/custom serializers
   - C# uses .NET BinaryFormatter
   - Solution: Define canonical binary format or use language-agnostic (JSON/protobuf)

2. **Floating Point Precision**: Different languages may have slight variations
   - Solution: Use tolerance ranges in comparisons (±0.01% for quantile tests)

3. **Collection Types**: Each language has different collection implementations
   - Solution: Test only on observable behavior, not implementation details

## References

- Cross-Language Validation Strategy: `/docs/CROSS_LANGUAGE_VALIDATION.md`
- Phase 3 Test Coverage: `/docs/test_coverage_harmonization_plan.md`
- Language-Specific Test Files: See per-language directories

---

**Last Updated**: November 25, 2024
**Phase**: 5a (Documentation), 5b (Implementation Pending)
**Status**: Phase 5a Complete, Phase 5b Ready for Implementation
