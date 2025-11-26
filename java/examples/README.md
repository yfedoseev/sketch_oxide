# Java Examples for sketch_oxide

Production-ready examples demonstrating probabilistic data structures in Java.

## Quick Start

```bash
# Build the library first
cd ..
mvn clean package

# Compile examples
cd examples
javac -cp "../target/*" BasicUsage.java
java -cp ".:../target/*" BasicUsage
```

## Examples

1. **BasicUsage.java** - BloomFilter and HyperLogLog fundamentals
2. **DDoSDetection.java** - HeavyKeeper for network security
3. **CardinalityEstimation.java** - Analytics dashboard with HyperLogLog
4. **FrequencyAnalysis.java** - CountMinSketch for stream processing
5. **SetReconciliation.java** - RatelessIBLT for blockchain sync
6. **RangeFiltering.java** - GRF for LSM-tree optimization
7. **NetworkMonitoring.java** - UnivMon for multi-metric telemetry
8. **PerformanceComparison.java** - Benchmark all sketches

## Common Patterns

### Resource Management
```java
// Use try-with-resources
try (HyperLogLog hll = new HyperLogLog(14)) {
    hll.update("item".getBytes());
    System.out.println(hll.estimate());
} // Automatically cleaned up
```

### Serialization
```java
HyperLogLog hll = new HyperLogLog(14);
byte[] data = hll.serialize();
HyperLogLog restored = HyperLogLog.deserialize(data);
```

### Error Handling
```java
try {
    HyperLogLog hll = new HyperLogLog(20); // Invalid
} catch (IllegalArgumentException e) {
    System.err.println("Precision must be 4-16");
}
```

## Performance Tips

1. Reuse byte arrays to reduce GC pressure
2. Use ByteBuffer for zero-copy operations
3. Close sketches in finally blocks or try-with-resources
4. Batch updates before querying

## Dependencies

```xml
<dependency>
    <groupId>com.sketches-oxide</groupId>
    <artifactId>sketch-oxide</artifactId>
    <version>0.1.0</version>
</dependency>
```
