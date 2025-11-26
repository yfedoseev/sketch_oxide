# Benchmark Methodology

This document explains how SketchOxide algorithms are benchmarked against competing implementations.

## Principles

Our benchmarking follows these principles:

1. **Fair Comparison** - Same parameters and conditions for all implementations
2. **Real-World Scenarios** - Benchmarks reflect actual usage patterns
3. **Reproducibility** - All benchmarks include seeds and setup procedures
4. **Transparency** - Methods and results are fully documented

## Languages & Competitors

### Rust (Criterion.rs)
- **Location**: `sketch_oxide/benches/`
- **Competitors**:
  - `pdatastructs` - Popular probabilistic data structures crate
  - `probabilistic-collections` - Alternative implementations
  - `streaming_algorithms` - Stream-based sketches

**Run Rust benchmarks**:
```bash
cd sketch_oxide
cargo bench
```

### Java (OpenJDK JMH)
- **Location**: `java/src/test/java/io/sketchoxide/benchmarks/`
- **Competitors**:
  - Apache DataSketches 3.3.0 - Industry-standard implementation
  - Other candidates: stream-lib, Guava

**Run Java benchmarks**:
```bash
cd java
mvn clean test -Dtest=*Benchmark
# Or with JMH plugin:
mvn jmh:benchmark
```

### Node.js (Benchmark.js)
- **Location**: `nodejs/benchmarks/`
- **Competitors**:
  - `bloom-filters` npm package
  - `hyperloglog` npm package
  - `count-min-sketch` npm package

**Run Node.js benchmarks**:
```bash
cd nodejs
npm run benchmark
```

### C# / .NET (BenchmarkDotNet)
- **Location**: `dotnet/SketchOxide.Benchmarks/`
- **Competitors**:
  - Probably - Popular C# probabilistic structures library
  - BloomFilter.NetCore - Specialized Bloom filter implementation

**Run C# benchmarks**:
```bash
cd dotnet
dotnet run -c Release --project SketchOxide.Benchmarks
```

## Benchmark Categories

### 1. Memory Usage
**What**: Size of serialized sketch in bytes
**Why**: Critical for distributed systems and storage
**How**: Serialize each sketch and measure byte length

### 2. Single Update Performance
**What**: Time to add one element to sketch
**Units**: Microseconds per update
**Why**: Measures the fast-path performance

### 3. Bulk Insert Performance
**What**: Time to add 100 elements
**Units**: Microseconds per batch
**Why**: Typical real-world workload

### 4. Query Performance
**What**: Time to query result (estimate, contains, etc.)
**Units**: Nanoseconds per query
**Why**: How fast clients get results

### 5. Serialization Performance
**What**: Time to serialize sketch to bytes
**Units**: Microseconds
**Why**: Critical for distribution and storage

### 6. Deserialization Performance
**What**: Time to reconstruct sketch from bytes
**Units**: Microseconds
**Why**: Critical for loading cached results

### 7. Merge Performance
**What**: Time to merge two sketches
**Units**: Microseconds
**Why**: Essential for distributed aggregation

### 8. Accuracy vs Memory Trade-off
**What**: Error percentage for given memory budget
**Metric**: (estimated - actual) / actual * 100%
**Why**: Shows quality at different scales

## Test Data

### Data Patterns

1. **Random Bytes** (Default)
   - Completely random 1KB blocks
   - Seed: 12345 for reproducibility

2. **Sequential Numbers**
   - ASCII numbers 1-N
   - Tests hash distribution

3. **Repeated Data**
   - Tests duplicate handling
   - Same element inserted multiple times

4. **Realistic Strings**
   - User IDs, URLs, emails
   - Simulates real-world data

### Dataset Sizes

- **Small**: 1,000 elements
- **Medium**: 100,000 elements
- **Large**: 1,000,000 elements
- **Huge**: 100,000,000 elements (cardinality sketches only)

## Statistical Requirements

### Sample Size
- Minimum 5 warmup iterations
- Minimum 10 measurement iterations
- Multiple fork runs for variance estimation

### Reporting
- Report mean, median, and standard deviation
- Show 95% confidence interval
- Flag outliers (>3σ)

## Hardware Specifications

Benchmarks should report:

```
CPU:      [CPU model and count]
Memory:   [RAM size]
JVM:      [Version for Java]
Node:     [Version for Node.js]
.NET:     [Version for C#]
OS:       [OS and version]
```

### Recommended Test Environment
- Modern CPU (2020+)
- 8GB+ RAM
- SSD storage
- Isolated, dedicated hardware
- No other processes running

## Accuracy Benchmarks

### Method
1. Create sketch with precision P
2. Insert N known unique elements
3. Get estimate E
4. Calculate error: (E - N) / N * 100%

### Precision Levels Tested
- Precision 10 (1KB)
- Precision 12 (4KB)
- Precision 14 (16KB)
- Precision 16 (64KB)
- Precision 18 (256KB)

### Cardinalities Tested
- 100, 1K, 10K, 100K, 1M, 10M, 100M

## Results Reporting

### Format

```
Algorithm: HyperLogLog
Comparison: SketchOxide vs Apache DataSketches
Date: 2025-11-23
Hardware: [specs]

| Operation | SketchOxide | Apache | Difference | Winner |
|-----------|------------|--------|------------|--------|
| Update (µs) | 0.5 | 0.6 | -17% | SO |
| Query (ns) | 10 | 15 | -33% | SO |
| Memory (bytes) | 16384 | 16384 | 0% | Tie |
| Serialize (µs) | 2 | 3 | -33% | SO |
```

### Performance Tiers

- 🏆 **Tier 1**: >20% faster
- 🥈 **Tier 2**: 5-20% faster
- 🥉 **Tier 3**: Within 5% margin (statistical tie)
- ⚠️  **Slower**: Statistically slower

## Validation & Correctness

### Functional Tests
All benchmarks must also verify correctness:
- Estimates are within expected error bounds
- Merged sketches give correct combined estimates
- Serialization/deserialization roundtrip preserves state

### Example (HyperLogLog)
```
Insert 1M unique items with precision 14
Expected error: ±0.41%
Measured error must be: ±1% to ±2%
If outside bounds → benchmark is invalid
```

## Re-running Benchmarks

To ensure reproducibility:

1. Use same seed values
2. Same hardware or document differences
3. Report statistical measures, not single runs
4. Include JVM/runtime flags used
5. State date and version of each library

### Variance Factors

- CPU frequency scaling (disable if possible)
- Background processes (kill or minimize)
- Memory pressure (use > 4x heap size needed)
- Thermal conditions (let system cool)
- Java: Warmup iterations critical

## Continuous Benchmarking

### What to Track
- Performance regression over versions
- Accuracy degradation
- Memory bloat
- Serialization size growth

### Frequency
- Before each release
- For significant code changes
- Monthly for continuous monitoring

## Reporting Results

When publishing results:

1. **Link to benchmark code** - Reproducibility
2. **Hardware specs** - Context for comparisons
3. **Date run** - Versions matter
4. **All measurements** - Not just wins
5. **Confidence intervals** - Show uncertainty
6. **Test data patterns** - Real vs synthetic

## Caveats & Disclaimers

- Results are specific to test environment
- Different hardware may show different results
- Language implementations vary (JVM vs native)
- Memory allocators affect performance
- Benchmarks don't account for everything (cache effects, etc.)

## Next Steps

See language-specific benchmark documentation:
- [Rust Benchmarks](rust-benchmarks.md)
- [Java Benchmarks](java-benchmarks.md)
- [Node.js Benchmarks](nodejs-benchmarks.md)
- [C# Benchmarks](dotnet-benchmarks.md)
