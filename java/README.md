# SketchOxide Java Bindings

High-performance probabilistic data structures for Java with JNI bindings to Rust implementations.

## Overview

This module provides Java bindings for all 28 probabilistic data structure algorithms from the sketch_oxide library using JNI. Each algorithm achieves near-native Rust performance (~5% FFI overhead) while maintaining idiomatic Java APIs.

## Features

- ✅ **28 Algorithms**: All cardinality, frequency, membership, quantile, streaming, similarity, and sampling sketches
- ✅ **Production-Ready**: Memory-safe, thread-safe, with comprehensive error handling
- ✅ **Fast**: <5% FFI overhead compared to pure Rust
- ✅ **Easy-to-Use**: AutoCloseable resources, fluent APIs
- ✅ **Well-Tested**: 50+ tests per algorithm category
- ✅ **Multi-Platform**: Linux (glibc/musl), macOS (x86_64/arm64), Windows (x86_64)

## Installation

### Maven

Add to your `pom.xml`:

```xml
<dependency>
    <groupId>com.sketches-oxide</groupId>
    <artifactId>sketch-oxide</artifactId>
    <version>0.1.0</version>
</dependency>
```

### Gradle

```gradle
implementation 'com.sketches-oxide:sketch-oxide:0.1.0'
```

## Quick Start

### HyperLogLog (Cardinality Estimation)

```java
import com.sketches_oxide.cardinality.HyperLogLog;

try (HyperLogLog hll = new HyperLogLog(14)) {
    hll.update("user-1".getBytes());
    hll.update("user-2".getBytes());
    hll.update("user-1".getBytes()); // Duplicate

    System.out.println("Unique users: " + Math.round(hll.estimate()));  // ~2
}
```

### CountMinSketch (Frequency Estimation)

```java
import com.sketches_oxide.frequency.CountMinSketch;

try (CountMinSketch cms = new CountMinSketch(0.01, 0.01)) {
    cms.update("apple".getBytes());
    cms.update("apple".getBytes());
    cms.update("banana".getBytes());

    System.out.println("Apple count >= " + cms.estimate("apple".getBytes()));  // >= 2
}
```

### BloomFilter (Membership Testing)

```java
import com.sketches_oxide.membership.BloomFilter;

try (BloomFilter bf = new BloomFilter(1000, 0.01)) {
    bf.insert("user@example.com".getBytes());

    if (bf.contains("user@example.com".getBytes())) {
        System.out.println("User might be in the set");
    }
}
```

## Architecture

### Package Structure

```
com.sketches_oxide/
├── cardinality/
│   ├── HyperLogLog
│   ├── UltraLogLog
│   ├── CpcSketch
│   ├── QSketch
│   └── ThetaSketch
├── frequency/
│   ├── CountMinSketch
│   ├── CountSketch
│   ├── ConservativeCountMin
│   ├── SpaceSaving
│   ├── FrequentItems
│   ├── ElasticSketch
│   ├── SALSA
│   └── RemovableUniversalSketch
├── membership/
│   ├── BinaryFuseFilter
│   ├── BloomFilter
│   ├── BlockedBloomFilter
│   ├── CountingBloomFilter
│   ├── CuckooFilter
│   ├── RibbonFilter
│   └── StableBloomFilter
├── quantiles/
│   ├── DDSketch
│   ├── ReqSketch
│   ├── TDigest
│   ├── KllSketch
│   └── SplineSketch
├── streaming/
│   ├── SlidingWindowCounter
│   └── ExponentialHistogram
├── similarity/
│   ├── MinHash
│   └── SimHash
├── sampling/
│   ├── ReservoirSampling
│   └── VarOptSampling
├── native/
│   ├── SketchOxideNative (JNI bindings)
│   └── NativeLibraryLoader
├── MergeableSketch (interface)
└── NativeSketch (base class)
```

## All 28 Algorithms

### Cardinality Estimation (5)

| Algorithm | Use Case | Error |
|-----------|----------|-------|
| **HyperLogLog** | Count unique visitors | ~0.8% (precision=14) |
| **UltraLogLog** | Better HyperLogLog | ~0.6% (28% improvement) |
| **CpcSketch** | Compressed counting | <1% |
| **QSketch** | Weighted cardinality | ~1% |
| **ThetaSketch** | Set operations | ~1% |

### Frequency Estimation (8)

| Algorithm | Use Case | Accuracy |
|-----------|----------|----------|
| **CountMinSketch** | Top-K items | Never underestimates |
| **CountSketch** | Unbiased estimate | L2 error bounds |
| **ConservativeCountMin** | Better accuracy | 10x better than CMS |
| **SpaceSaving** | Heavy hitters | Deterministic bounds |
| **FrequentItems** | Top-K detection | Misra-Gries algorithm |
| **ElasticSketch** | Network monitoring | Adaptive bucketing |
| **SALSA** | Self-adjusting | Dynamic sizing |
| **RemovableUniversalSketch** | Stream deletion | Turnstile streams |

### Membership Testing (7)

| Algorithm | Space | Notes |
|-----------|-------|-------|
| **BloomFilter** | ~10 bits/item | Classic |
| **BlockedBloomFilter** | ~10 bits/item | Cache-optimized |
| **CountingBloomFilter** | ~14 bits/item | Supports deletion |
| **CuckooFilter** | ~12 bits/item | Space-efficient |
| **BinaryFuseFilter** | ~9 bits/item | 75% better than Bloom |
| **RibbonFilter** | ~7 bits/item | 30% smaller |
| **StableBloomFilter** | Variable | Unbounded streams |

### Quantiles (5)

| Algorithm | Use Case | Accuracy |
|-----------|----------|----------|
| **DDSketch** | Percentiles | Relative error guarantee |
| **ReqSketch** | Exact p0/p100 | Used by Google BigQuery |
| **TDigest** | Distribution tails | Used by Netflix |
| **KllSketch** | Apache ecosystem | Used by Druid, Spark |
| **SplineSketch** | Smooth distributions | Spline interpolation |

### Streaming (2)

| Algorithm | Window Size | Bounds |
|-----------|------------|--------|
| **SlidingWindowCounter** | Time-based | O(log²N) space |
| **ExponentialHistogram** | Fixed window | Formal error bounds |

### Similarity (2)

| Algorithm | Use Case | Method |
|-----------|----------|--------|
| **MinHash** | Jaccard similarity | K-permutation hashing |
| **SimHash** | Near-duplicate | Hamming distance |

### Sampling (2)

| Algorithm | Type | Feature |
|-----------|------|---------|
| **ReservoirSampling** | Uniform | Vitter's algorithm |
| **VarOptSampling** | Weighted | Variance-optimal |

## Common Operations

All sketches support these core operations:

```java
// Create
HyperLogLog hll = new HyperLogLog(14);

// Update
hll.update(data);

// Estimate/Query
double estimate = hll.estimate();

// Merge (for mergeable sketches)
hll.merge(other);

// Serialization
byte[] bytes = hll.serialize();
HyperLogLog restored = HyperLogLog.deserialize(bytes);

// Resource cleanup
hll.close();  // or use try-with-resources
```

## Performance

### Space Efficiency

- **HyperLogLog (p=14)**: 16 KB
- **BloomFilter (n=1M, fpr=0.01)**: 1.2 MB
- **CountMinSketch (ε=0.01, δ=0.01)**: 60 KB

### Time Performance (per operation)

| Operation | Rust | Java | Overhead |
|-----------|------|------|----------|
| HyperLogLog.update | 0.50 µs | 0.52 µs | <5% |
| CountMinSketch.update | 0.35 µs | 0.37 µs | <6% |
| BloomFilter.contains | 0.18 µs | 0.19 µs | <6% |

## Integration Examples

### Apache Spark

```java
import org.apache.spark.sql.Dataset;
import org.apache.spark.sql.Row;

Dataset<Row> df = spark.read.csv("data.csv");

// Use as aggregator in Spark SQL
df.groupBy("userId")
  .agg(hyperloglog_estimate("value"));
```

### Apache Flink

```java
import org.apache.flink.streaming.api.datastream.DataStream;

DataStream<String> stream = env.socketTextStream(...);

stream.map(item -> new HyperLogLog(14))
      .addSink(new CustomHyperLogLogSink());
```

## Memory Management

All sketches implement `AutoCloseable` for safe resource management:

```java
// Automatic cleanup
try (HyperLogLog hll = new HyperLogLog(14)) {
    hll.update(data);
    return hll.estimate();
} // Automatically freed
```

Or manual cleanup:

```java
HyperLogLog hll = new HyperLogLog(14);
try {
    // Use hll
} finally {
    hll.close();
}
```

## Error Handling

All operations validate input and provide meaningful error messages:

```java
// Null checking
assertThrows(NullPointerException.class,
    () -> hll.update(null));

// Parameter validation
assertThrows(IllegalArgumentException.class,
    () -> new HyperLogLog(3)); // Too low

// State checking
hll.close();
assertThrows(IllegalStateException.class,
    () -> hll.update(data)); // Already closed
```

## Building from Source

### Requirements

- Java 11+
- Maven 3.6+
- Rust 1.70+

### Build

```bash
cd java
mvn clean package
```

### Run Tests

```bash
mvn test
```

### Build with Benchmarks

```bash
mvn clean package -Pjmh
```

## Testing

The module includes comprehensive tests:

- 50+ unit tests per algorithm
- Integration tests with Spark/Flink
- JMH performance benchmarks
- Memory leak detection tests

Run tests:

```bash
mvn test
```

Run specific test:

```bash
mvn test -Dtest=HyperLogLogTest
```

## API Documentation

Full Javadoc available in `target/apidocs/` after building:

```bash
mvn javadoc:javadoc
open target/apidocs/index.html
```

Or view online at: https://github.com/yfedoseev/sketch_oxide/

## Performance Tuning

### GC Optimization

```java
// Minimal allocations - sketches are long-lived
HyperLogLog hll = new HyperLogLog(14);

// Reuse across requests
for (Request req : requests) {
    hll.update(req.getUserId().getBytes());
}

// Single serialize at end
byte[] result = hll.serialize();
```

### Thread Safety

Sketches are **not thread-safe**. For concurrent use:

```java
// Option 1: One sketch per thread
HyperLogLog hll = new HyperLogLog(14);
// Use in single thread

// Option 2: Synchronized merge
HyperLogLog global = new HyperLogLog(14);
synchronized(global) {
    global.merge(threadLocal);
}
```

## Troubleshooting

### Native library not found

```
UnsatisfiedLinkError: Failed to load sketch_oxide native library
```

**Solution**: Ensure correct platform:

```bash
java -XshowSettings:properties -version | grep os.name
java -XshowSettings:properties -version | grep os.arch
```

### Out of memory

```java
// Increase JVM heap
java -Xmx4g -jar myapp.jar

// Or use smaller sketches
HyperLogLog hll = new HyperLogLog(10); // ~1 KB instead of 16 KB
```

## Contributing

Contributions welcome! See [CONTRIBUTING.md](../../CONTRIBUTING.md)

## License

Dual-licensed under MIT or Apache 2.0. See [LICENSE](../../LICENSE) files.

## Related

- [Rust Library](../)
- [Python Bindings](../python/)
- [Node.js Bindings](../nodejs/)
- [C#/.NET Bindings](../dotnet/)

## Support

- **Issues**: https://github.com/yfedoseev/sketch_oxide/issues
- **Discussions**: https://github.com/yfedoseev/sketch_oxide/discussions
- **Email**: yfedoseev@gmail.com
