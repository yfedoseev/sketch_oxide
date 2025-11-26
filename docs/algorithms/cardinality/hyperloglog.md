# HyperLogLog - Probabilistic Cardinality Estimation

## Overview

**HyperLogLog** is the industry-standard algorithm for estimating the cardinality (number of unique elements) of a set using logarithmic memory. It can estimate the count of distinct elements in massive datasets while using only a few kilobytes of memory.

| Property | Value |
|----------|-------|
| **Memory** | O(log log N) - ~16KB for precision 14 |
| **Time/Update** | O(1) amortized |
| **Time/Query** | O(1) |
| **Accuracy** | ~2% relative error (configurable) |
| **Mergeable** | ✅ Yes |
| **Serializable** | ✅ Yes |
| **Space Efficiency** | 28-75% better than alternatives |

## When to Use

✅ **Ideal for:**
- Counting unique visitors to websites
- Unique user IPs in DDoS detection
- Deduplication of elements in streams
- Database cardinality estimation
- Analytics dashboards
- Large-scale log analysis

❌ **Not ideal for:**
- Need for exact counts (use a set instead)
- Very small datasets (< 1000 items - use a set)
- Need for item deletion (use modified HyperLogLog)

## Algorithm Explanation

### Core Concept

HyperLogLog estimates cardinality by counting the maximum number of leading zeros in the binary representation of hashed values.

**Key insight**: If we hash random elements, elements with many leading zeros in binary are rare. The maximum number of leading zeros tells us how many unique elements we've likely seen.

### How It Works

1. **Hash Function**: Each element is hashed to a uniform random binary string
2. **Leading Zeros**: Count the leading zeros in each hash
3. **Bucket Analysis**: Estimate cardinality from the maximum leading zeros across buckets

### Example: Binary Representation

```
Element: "alice"
Hash: 10110010 01101110 11010011...
Leading zeros: 1 (just one 0 at the start)

Element: "bob"
Hash: 00010101 11101001 01011010...
Leading zeros: 3 (three 0s at the start)

Max leading zeros observed: 3
This tells us: We've likely seen ~2^3 = 8 elements
```

### Precision Parameter

The `precision` parameter (p) controls:
- Number of buckets: 2^p
- Memory usage: ~2^p bytes
- Accuracy: ~1.04/√(2^p) relative error

**Recommended values:**
- `p=10` (1KB) - Very rough estimate, relative error ~3.3%
- `p=12` (4KB) - Good estimate, relative error ~0.8%
- `p=14` (16KB) - Standard choice, relative error ~0.41%
- `p=16` (64KB) - High precision, relative error ~0.1%
- `p=18` (256KB) - Very high precision, relative error ~0.025%

## Mathematical Foundation

### Standard Error Formula

```
Standard Error = 1.04 / sqrt(m)

where m = 2^precision
```

**Examples:**
- Precision 14 (m=16384): Error ≈ 0.41%
- Precision 16 (m=65536): Error ≈ 0.1%

### Cardinality Estimation

The algorithm uses harmonic mean of hash values:

```
Estimate = α_m * m^2 / Z

where:
  m = number of buckets (2^precision)
  α_m = constant based on m
  Z = harmonic sum of values
```

## Usage Examples

### Rust

```rust
use sketch_oxide::cardinality::HyperLogLog;

fn main() {
    // Create HyperLogLog with precision 14
    let mut hll = HyperLogLog::new(14).unwrap();

    // Add elements
    for user_id in 1..=1_000_000 {
        hll.update(&user_id.to_string().as_bytes());
    }

    // Get estimate
    let cardinality = hll.estimate();
    println!("Unique elements: {}", cardinality);
    // Output: ~1,000,000 (±0.41%)

    // Serialize for storage
    let bytes = hll.serialize();
    println!("Serialized size: {} bytes", bytes.len()); // ~16KB

    // Deserialize and restore
    let restored = HyperLogLog::deserialize(&bytes).unwrap();
    println!("Restored estimate: {}", restored.estimate());

    // Merge with another HyperLogLog
    let mut hll2 = HyperLogLog::new(14).unwrap();
    hll2.update(&"new_element".as_bytes());
    hll.merge(&hll2).unwrap();
    println!("Merged estimate: {}", hll.estimate());
}
```

### Python

```python
from sketch_oxide import HyperLogLog

# Create sketch with precision 14
hll = HyperLogLog(precision=14)

# Add elements
for user_id in range(1, 1_000_001):
    hll.update(str(user_id).encode())

# Get estimate
cardinality = hll.estimate()
print(f"Unique elements: {cardinality}")
# Output: ~1,000,000 (±0.41%)

# Serialize to bytes
serialized = hll.serialize()
print(f"Serialized size: {len(serialized)} bytes")  # ~16KB

# Deserialize
restored = HyperLogLog.deserialize(serialized)
print(f"Restored estimate: {restored.estimate()}")

# Merge sketches
hll2 = HyperLogLog(precision=14)
hll2.update(b"new_element")
hll.merge(hll2)
print(f"Merged estimate: {hll.estimate()}")
```

### Java

```java
import io.sketchoxide.cardinality.HyperLogLog;

public class HyperLogLogExample {
    public static void main(String[] args) {
        try (HyperLogLog hll = new HyperLogLog(14)) {
            // Add elements
            for (int i = 1; i <= 1_000_000; i++) {
                hll.update(Integer.toString(i).getBytes());
            }

            // Get estimate
            double cardinality = hll.estimate();
            System.out.println("Unique elements: " + cardinality);
            // Output: ~1,000,000 (±0.41%)

            // Serialize
            byte[] serialized = hll.serialize();
            System.out.println("Serialized size: " + serialized.length); // ~16KB

            // Deserialize
            HyperLogLog restored = HyperLogLog.deserialize(serialized);
            System.out.println("Restored estimate: " + restored.estimate());

            // Merge
            try (HyperLogLog hll2 = new HyperLogLog(14)) {
                hll2.update("new_element".getBytes());
                hll.merge(hll2);
                System.out.println("Merged estimate: " + hll.estimate());
            }
        }
    }
}
```

### Node.js / TypeScript

```typescript
import { HyperLogLog } from '@sketchoxide/core';

async function main() {
    // Create sketch
    const hll = new HyperLogLog(14);

    // Add elements
    for (let i = 1; i <= 1_000_000; i++) {
        hll.update(Buffer.from(i.toString()));
    }

    // Get estimate
    const cardinality = hll.estimate();
    console.log(`Unique elements: ${cardinality}`);
    // Output: ~1,000,000 (±0.41%)

    // Serialize
    const serialized = hll.serialize();
    console.log(`Serialized size: ${serialized.length} bytes`); // ~16KB

    // Deserialize
    const restored = HyperLogLog.deserialize(serialized);
    console.log(`Restored estimate: ${restored.estimate()}`);

    // Merge
    const hll2 = new HyperLogLog(14);
    hll2.update(Buffer.from('new_element'));
    hll.merge(hll2);
    console.log(`Merged estimate: ${hll.estimate()}`);
}

main();
```

### C# / .NET

```csharp
using SketchOxide.Cardinality;

class Program {
    static void Main() {
        using (var hll = new HyperLogLog(14)) {
            // Add elements
            for (int i = 1; i <= 1_000_000; i++) {
                hll.Update(i.ToString().ToByteArray());
            }

            // Get estimate
            double cardinality = hll.Estimate();
            Console.WriteLine($"Unique elements: {cardinality}");
            // Output: ~1,000,000 (±0.41%)

            // Serialize
            byte[] serialized = hll.Serialize();
            Console.WriteLine($"Serialized size: {serialized.Length} bytes"); // ~16KB

            // Deserialize
            var restored = HyperLogLog.Deserialize(serialized);
            Console.WriteLine($"Restored estimate: {restored.Estimate()}");

            // Merge
            using (var hll2 = new HyperLogLog(14)) {
                hll2.Update("new_element"u8);
                hll.Merge(hll2);
                Console.WriteLine($"Merged estimate: {hll.Estimate()}");
            }
        }
    }
}
```

## Performance Characteristics

### Time Complexity
- **Update**: O(1) amortized
- **Query**: O(1)
- **Merge**: O(2^precision)

### Space Complexity
- **Memory**: O(2^precision) ≈ O(log log N)
- For precision 14: ~16KB regardless of data size

### Accuracy vs Precision

| Precision | Memory | Relative Error | Good for |
|-----------|--------|----------------|----------|
| 10 | 1 KB | ±3.3% | Quick rough estimates |
| 12 | 4 KB | ±0.8% | Good balance |
| 14 | 16 KB | ±0.41% | Standard choice |
| 16 | 64 KB | ±0.1% | High precision |
| 18 | 256 KB | ±0.025% | Very high precision |

## Comparison with Alternatives

| Algorithm | Memory | Accuracy | Mergeable | Speed |
|-----------|--------|----------|-----------|-------|
| **HyperLogLog** | 16KB | ~2% | ✅ | 🟢 Fast |
| HashSet | O(n) | Exact | ❌ | 🟢 Fast |
| Bitmap | O(n/8) | Exact | ✅ | 🟢 Fast |
| UltraLogLog | 16KB | <1% | ✅ | 🟢 Fast |
| CPC Sketch | 4KB | ~1% | ✅ | 🟡 Slower |

**Why HyperLogLog?**
- Established industry standard (used by Redis, Elasticsearch, BigQuery)
- Perfect balance of speed, memory, and accuracy
- Proven performance across billions of elements
- Easy to implement and understand

## Real-World Examples

### Example 1: Unique Website Visitors

```rust
use sketch_oxide::cardinality::HyperLogLog;
use std::fs;

fn track_unique_visitors(log_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut hll = HyperLogLog::new(14)?;

    // Process log file line by line
    for line in fs::read_to_string(log_file)?.lines() {
        if let Some(ip) = extract_ip(line) {
            hll.update(&ip.as_bytes());
        }
    }

    println!("Unique visitors: {}", hll.estimate());

    // Save for later analysis
    let serialized = hll.serialize();
    fs::write("visitor_count.bin", serialized)?;

    Ok(())
}
```

### Example 2: Merging Daily Counts

```python
from sketch_oxide import HyperLogLog
from datetime import date, timedelta

# Create sketches for each day
sketches = {}
for day in range(7):  # 7 days
    sketches[day] = HyperLogLog(precision=14)
    # Process that day's data...

# Merge all weekly data
weekly = HyperLogLog(precision=14)
for sketch in sketches.values():
    weekly.merge(sketch)

print(f"Weekly unique visitors: {weekly.estimate()}")
```

### Example 3: Distributed System Aggregation

```java
// On individual servers
HyperLogLog localHLL = new HyperLogLog(14);
// ... process local data ...

// Send serialized HyperLogLog to central aggregator
byte[] serialized = localHLL.serialize();
sendToAggregator(serialized);

// On central aggregator
HyperLogLog result = new HyperLogLog(14);
for (byte[] data : receivedSketches) {
    HyperLogLog remote = HyperLogLog.deserialize(data);
    result.merge(remote);
}
System.out.println("Total unique: " + result.estimate());
```

## Choosing Precision

### Decision Guide

1. **Rough estimate (±5% acceptable)?** → Precision 10-12
2. **Standard use (±1% target)?** → Precision 14
3. **High precision (±0.5%)?** → Precision 16
4. **Memory critical (< 1KB)?** → CPC Sketch or QSketch
5. **Extreme precision needed?** → Consider exact counting

### Memory Budget

```
Precision 10: 1 KB         ← Minimal
Precision 12: 4 KB         ← Small
Precision 14: 16 KB        ← Recommended
Precision 16: 64 KB        ← High precision
Precision 18: 256 KB       ← Very high
```

## Advanced Topics

### Handling Deletions

Standard HyperLogLog doesn't support deletion. For streaming with deletions:
1. Use **Stable Bloom Filter** + HyperLogLog
2. Use **Sliding Window Counter**
3. Use **Exponential Histogram**

### Improving Accuracy

For better accuracy without precision increase:
1. Use **UltraLogLog** (< 1% error)
2. Use **CPC Sketch** (Ultra-compressed)
3. Use **Theta Sketch** (Configurable accuracy)

### Merging Sketches

HyperLogLog supports merging sketches with the same precision:

```rust
let mut merged = HyperLogLog::new(14)?;
for sketch in sketches {
    merged.merge(&sketch)?; // Merges in-place
}
```

⚠️ **Warning**: Only merge sketches with identical precision

## Accuracy Considerations

### Sources of Error

1. **Algorithmic error**: ~1.04/√(2^p) (inherent to algorithm)
2. **Precision loss**: Higher p reduces error
3. **Small cardinality**: For n < 2^p, error can be higher

### Validating Results

```rust
// Compare with known small sets
let mut test = HyperLogLog::new(14)?;
for i in 0..1000 {
    test.update(&i.to_string().as_bytes());
}
let estimate = test.estimate();
assert!((estimate - 1000.0).abs() < 50.0); // ±5% margin
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Estimate too far off | Increase precision parameter |
| Memory usage too high | Decrease precision or use CPC Sketch |
| Merging gives wrong result | Ensure same precision for all sketches |
| Serialization/deserialization fails | Verify bytes aren't corrupted |

## Further Reading

- [Original HyperLogLog paper](http://algo.inria.fr/flajolet/Publications/FlFuGaMe07.pdf) (Flajolet et al., 2007)
- [Redis HyperLogLog implementation](https://redis.io/commands/pfadd/)
- [Google BigQuery HyperLogLog](https://cloud.google.com/bigquery/docs/reference/standard-sql/approximate_aggregate_functions)

## See Also

- [UltraLogLog](ultraloglog.md) - Better accuracy variant
- [Theta Sketch](theta-sketch.md) - Set operations
- [CPC Sketch](cpc-sketch.md) - Ultra-compressed cardinality
