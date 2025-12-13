# Java and C# FFI Completion Plan - v0.1.6

**Strategic Goal:** Complete multi-language support with all 41 algorithms available across all 5 supported languages (Rust, Python, Node.js, Java, C#)

---

## Current State Analysis

### Java Bindings (JNI)
**File:** `java/src/lib.rs`
**Current Count:** 9 algorithms
**Status:** Partial implementation with JNI wrappers

**Currently Implemented (9):**
1. HyperLogLog (Cardinality)
2. CountMinSketch (Frequency)
3. NitroSketch (Frequency)
4. BloomFilter (Membership)
5. LearnedBloomFilter (Membership)
6. VacuumFilter (Membership)
7. GRF (Range Filter)
8. UnivMon (Universal)
9. RangeFilter trait (interface)

**Missing (32):**
- Cardinality: UltraLogLog, CpcSketch, QSketch, ThetaSketch
- Membership: BlockedBloomFilter, BinaryFuseFilter, CountingBloomFilter, CuckooFilter, RibbonFilter, StableBloomFilter
- Quantiles: DDSketch, ReqSketch, TDigest, KllSketch, SplineSketch
- Frequency: CountSketch, ConservativeCountMin, SpaceSaving, ElasticSketch, SALSA, RemovableUniversalSketch, FrequentItems, HeavyKeeper
- Similarity: MinHash, SimHash
- Sampling: ReservoirSampling, VarOptSampling
- Streaming: SlidingWindowCounter, ExponentialHistogram, SlidingHyperLogLog
- Reconciliation: RatelessIBLT
- Range Filters: MementoFilter, Grafite

### C# Bindings (.NET P/Invoke)
**File:** `dotnet/csharp-ffi/src/lib.rs`
**Current Count:** 1 algorithm
**Status:** Minimal implementation

**Currently Implemented (1):**
1. HyperLogLog (Cardinality)

**Missing (40):**
All algorithms except HyperLogLog

---

## Implementation Strategy

### Phase 1: Java FFI Expansion (32 algorithms)

#### Step 1.1: Establish Standard JNI Pattern
- Study existing implementations (HyperLogLog, CountMinSketch, BloomFilter)
- Document standard pattern for:
  - Constructor wrappers (`new_*` functions)
  - Update/insert operations
  - Query/estimate operations
  - Serialization/deserialization
  - Cleanup (`drop_*` functions)

#### Step 1.2: Add Missing Cardinality Algorithms (4)
**Algorithms:** UltraLogLog, CpcSketch, QSketch, ThetaSketch

**Per Algorithm (JNI wrapper):**
```rust
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ALGORITHM_new(...) -> jlong
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ALGORITHM_update(...)
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ALGORITHM_query(...)
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ALGORITHM_serialize(...)
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ALGORITHM_deserialize(...)
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ALGORITHM_drop(...)
```

**Java Side (per algorithm):**
```java
public class ALGORITHM {
    private long nativePtr;

    public ALGORITHM(int param1) {
        nativePtr = new(...);
    }

    public void update(byte[] item) {
        update(nativePtr, item);
    }

    public Object query() {
        return query(nativePtr);
    }

    public void close() {
        drop(nativePtr);
        nativePtr = 0;
    }

    // Native method declarations
    private native long new(...);
    private native void update(long ptr, byte[] item);
    private native Object query(long ptr);
    private native void drop(long ptr);
}
```

#### Step 1.3: Add Missing Membership Algorithms (6)
**Algorithms:** BlockedBloomFilter, BinaryFuseFilter, CountingBloomFilter, CuckooFilter, RibbonFilter, StableBloomFilter

**Template:** Same pattern as Step 1.2

#### Step 1.4: Add Missing Quantile Algorithms (5)
**Algorithms:** DDSketch, ReqSketch, TDigest, KllSketch, SplineSketch

**Special Handling:** Quantile queries return double/float values

#### Step 1.5: Add Missing Frequency Algorithms (8)
**Algorithms:** CountSketch, ConservativeCountMin, SpaceSaving, ElasticSketch, SALSA, RemovableUniversalSketch, FrequentItems, HeavyKeeper

**Special Handling:** Some return complex types (HashMap, List of items)

#### Step 1.6: Add Missing Specialized Algorithms (9)
**Algorithms:** MinHash, SimHash, ReservoirSampling, VarOptSampling, SlidingWindowCounter, ExponentialHistogram, SlidingHyperLogLog, RatelessIBLT, MementoFilter, Grafite

---

### Phase 2: C# FFI Expansion (40 algorithms)

#### Step 2.1: Establish Standard P/Invoke Pattern
- Study existing HyperLogLog implementation
- Document standard pattern for C FFI exports
- Establish naming conventions for P/Invoke signatures

#### Step 2.2: Create C FFI for All 41 Algorithms
**Rust FFI Layer (`dotnet/csharp-ffi/src/lib.rs`):**

```rust
#[no_mangle]
pub unsafe extern "C" fn algorithm_new(...) -> *mut ALGORITHM { ... }
#[no_mangle]
pub unsafe extern "C" fn algorithm_update(ptr: *mut ALGORITHM, data: *const u8, len: usize) { ... }
#[no_mangle]
pub unsafe extern "C" fn algorithm_query(ptr: *mut ALGORITHM) -> ResultType { ... }
#[no_mangle]
pub unsafe extern "C" fn algorithm_serialize(ptr: *mut ALGORITHM, out: *mut *const u8, len: *mut usize) { ... }
#[no_mangle]
pub unsafe extern "C" fn algorithm_free(ptr: *mut ALGORITHM) { ... }
```

#### Step 2.3: Create C# P/Invoke Wrappers
**C# Pattern (per algorithm):**

```csharp
public class Algorithm : IDisposable
{
    [DllImport("sketch_oxide_dotnet", CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr algorithm_new(int param);

    [DllImport("sketch_oxide_dotnet", CallingConvention = CallingConvention.Cdecl)]
    private static extern void algorithm_update(IntPtr ptr, byte[] data, int len);

    [DllImport("sketch_oxide_dotnet", CallingConvention = CallingConvention.Cdecl)]
    private static extern double algorithm_query(IntPtr ptr);

    [DllImport("sketch_oxide_dotnet", CallingConvention = CallingConvention.Cdecl)]
    private static extern void algorithm_free(IntPtr ptr);

    private IntPtr _ptr;

    public Algorithm(int param) {
        _ptr = algorithm_new(param);
    }

    public void Update(byte[] item) {
        algorithm_update(_ptr, item, item.Length);
    }

    public double Query() {
        return algorithm_query(_ptr);
    }

    public void Dispose() {
        if (_ptr != IntPtr.Zero) {
            algorithm_free(_ptr);
            _ptr = IntPtr.Zero;
        }
    }
}
```

#### Step 2.4: Address by Category
1. **Cardinality (5):** HyperLogLog, UltraLogLog, CpcSketch, QSketch, ThetaSketch
2. **Membership (9):** All 9 variants
3. **Quantiles (5):** DDSketch, ReqSketch, TDigest, KllSketch, SplineSketch
4. **Frequency (10):** All frequency algorithms
5. **Similarity (2):** MinHash, SimHash
6. **Sampling (2):** ReservoirSampling, VarOptSampling
7. **Streaming (3):** SlidingWindowCounter, ExponentialHistogram, SlidingHyperLogLog
8. **Reconciliation (1):** RatelessIBLT
9. **Range Filters (3):** GRF, MementoFilter, Grafite
10. **Universal (1):** UnivMon

---

## Testing Strategy

### Java Testing
1. Create JUnit 5 tests for each algorithm
2. Test all standard operations:
   - Construction
   - Update/insert
   - Query/estimate
   - Serialization/deserialization
   - Memory cleanup
3. Cross-language validation:
   - Same seed → same results as Rust
   - Same test data → same estimates

### C# Testing
1. Create xUnit tests for each algorithm
2. Test all standard operations (same as Java)
3. Cross-language validation
4. P/Invoke interop verification

---

## Build and Publishing

### Java
- Maven build configuration
- JAR artifact generation
- Maven Central publishing (credentials needed)
- Multi-platform native library support

### C#
- .NET project file configuration
- NuGet package generation
- NuGet publishing (credentials needed)
- Windows/Linux/macOS native library support

---

## Estimated Effort

### Java FFI (32 algorithms)
- **Per algorithm:** ~30 minutes (JNI wrapper + Java class)
- **32 algorithms × 30 min = 16 hours**
- **Testing:** 8 hours
- **Documentation:** 2 hours
- **Total Java:** ~26 hours

### C# FFI (40 algorithms)
- **Per algorithm:** ~20 minutes (C FFI + C# wrapper)
- **40 algorithms × 20 min = 13 hours**
- **Testing:** 6 hours
- **Documentation:** 2 hours
- **Total C#:** ~21 hours

### Total v0.1.6 Language Binding Work
- **~47 hours** for complete multi-language support
- **~5-6 day sprint** for one developer

---

## Success Criteria

✅ **Java:**
- [ ] All 41 algorithms exposed via JNI
- [ ] Each algorithm has ≥5 unit tests
- [ ] Cross-language validation tests passing
- [ ] Maven build succeeds
- [ ] JAR artifact generated

✅ **C#:**
- [ ] All 41 algorithms exposed via P/Invoke
- [ ] Each algorithm has ≥5 unit tests
- [ ] Cross-language validation tests passing
- [ ] .NET project builds successfully
- [ ] NuGet package generated

✅ **Overall:**
- [ ] All 5 languages have 41/41 algorithms
- [ ] Documentation updated
- [ ] README shows 41/41 across all languages
- [ ] ROADMAP updated with completion status
- [ ] v0.1.6 release ready

---

## Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| JNI pointer management bugs | Template-based code generation, extensive testing |
| P/Invoke calling convention issues | Platform-specific testing (Windows, Linux, macOS) |
| Memory leaks in native code | Comprehensive cleanup tests, valgrind/ASAN |
| Type conversion mismatches | Cross-language validation tests |
| Build system complexity | Use established build tools (Maven, .NET SDK) |
| Credential/publishing issues | Pre-test publishing pipeline on test registries |

---

## Dependencies & Prerequisites

- Java Development Kit (JDK) 11+
- Maven 3.8+
- .NET SDK 6.0+
- Cross-platform testing environment (Windows, Linux, macOS)
- JUnit 5 (Java testing)
- xUnit (C# testing)

---

## Timeline

**Week 1:**
- Days 1-2: Java Phase 1.1-1.2 (Cardinality algorithms)
- Days 3-4: Java Phase 1.3-1.4 (Membership + Quantiles)
- Days 5: Java Phase 1.5-1.6 (Remaining algorithms)

**Week 2:**
- Days 1-2: Java testing & documentation
- Days 3-4: C# Phase 2.1-2.3 (All algorithms)
- Days 5: C# testing & documentation

**Week 3:**
- Full v0.1.6 testing suite
- Documentation finalization
- Release preparation

---

## Next Steps

1. **Approve Plan:** Get confirmation to proceed with this timeline
2. **Setup Infrastructure:** Configure Maven/C# build systems
3. **Code Generation:** Create templates for bulk algorithm wrapper generation
4. **Phase 1:** Begin Java FFI expansion
5. **Phase 2:** Begin C# FFI expansion
6. **Testing:** Comprehensive cross-language validation
7. **Release:** Package and publish v0.1.6 with complete multi-language support

---

**Document Version:** v0.1
**Created:** 2025-12-13
**Status:** Ready for implementation
**Next Review:** After Week 1 progress check
