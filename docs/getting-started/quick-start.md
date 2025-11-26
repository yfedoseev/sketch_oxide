# Quick Start Guide

Get started with SketchOxide in 5 minutes. Choose your language:

## Rust

```rust
use sketch_oxide::cardinality::HyperLogLog;

fn main() {
    // Create a sketch with precision 14 (uses ~16KB memory)
    let mut sketch = HyperLogLog::new(14).unwrap();

    // Add some items
    sketch.update(&"alice".as_bytes());
    sketch.update(&"bob".as_bytes());
    sketch.update(&"charlie".as_bytes());
    sketch.update(&"alice".as_bytes()); // Duplicates are ignored

    // Get the cardinality estimate
    println!("Unique users: {}", sketch.estimate());
    // Output: Unique users: 3
}
```

## Python

```python
from sketch_oxide import HyperLogLog

# Create a sketch
sketch = HyperLogLog(precision=14)

# Add items
sketch.update(b"alice")
sketch.update(b"bob")
sketch.update(b"charlie")
sketch.update(b"alice")  # Duplicates are ignored

# Get cardinality estimate
print(f"Unique users: {sketch.estimate()}")
# Output: Unique users: 3
```

## Java

```java
import io.sketchoxide.cardinality.HyperLogLog;

public class QuickStart {
    public static void main(String[] args) {
        // Create a sketch (try-with-resources for automatic cleanup)
        try (HyperLogLog sketch = new HyperLogLog(14)) {
            // Add items
            sketch.update("alice".getBytes());
            sketch.update("bob".getBytes());
            sketch.update("charlie".getBytes());
            sketch.update("alice".getBytes()); // Duplicates ignored

            // Get cardinality estimate
            System.out.println("Unique users: " + sketch.estimate());
            // Output: Unique users: 3
        }
    }
}
```

## Node.js / TypeScript

```typescript
import { HyperLogLog } from '@sketchoxide/core';

// Create a sketch
const sketch = new HyperLogLog(14);

// Add items
sketch.update(Buffer.from('alice'));
sketch.update(Buffer.from('bob'));
sketch.update(Buffer.from('charlie'));
sketch.update(Buffer.from('alice')); // Duplicates ignored

// Get cardinality estimate
console.log(`Unique users: ${sketch.estimate()}`);
// Output: Unique users: 3
```

## C# / .NET

```csharp
using SketchOxide.Cardinality;

class Program {
    static void Main() {
        // Create a sketch (using for automatic cleanup)
        using (var sketch = new HyperLogLog(14)) {
            // Add items
            sketch.Update("alice"u8);
            sketch.Update("bob"u8);
            sketch.Update("charlie"u8);
            sketch.Update("alice"u8); // Duplicates ignored

            // Get cardinality estimate
            Console.WriteLine($"Unique users: {sketch.Estimate()}");
            // Output: Unique users: 3
        }
    }
}
```

## Common Patterns

### Memory-Efficient Counting

HyperLogLog with precision 14 uses only 16KB to count billions of unique items:

```rust
let mut sketch = HyperLogLog::new(14)?; // 16KB memory

// Add a million items
for i in 0..1_000_000 {
    sketch.update(&i.to_string().as_bytes());
}

println!("Cardinality: {}", sketch.estimate()); // ~1,000,000
```

### Merging Sketches

Combine results from multiple sources:

```rust
let mut sketch1 = HyperLogLog::new(14)?;
let mut sketch2 = HyperLogLog::new(14)?;

sketch1.update(&"alice".as_bytes());
sketch1.update(&"bob".as_bytes());

sketch2.update(&"bob".as_bytes());
sketch2.update(&"charlie".as_bytes());

// Merge sketches
sketch1.merge(&sketch2)?;

println!("Total unique: {}", sketch1.estimate()); // ~3
```

### Serialization for Storage

Save and load sketches:

```rust
let mut sketch = HyperLogLog::new(14)?;
sketch.update(&"data".as_bytes());

// Serialize to bytes
let bytes = sketch.serialize();

// Save to file or database
std::fs::write("sketch.bin", &bytes)?;

// Later, deserialize
let data = std::fs::read("sketch.bin")?;
let restored = HyperLogLog::deserialize(&data)?;
```

## More Examples

### Bloom Filters for Membership Testing

```rust
use sketch_oxide::membership::BloomFilter;

let mut filter = BloomFilter::new(1000, 0.01); // 1000 items, 1% FPR

filter.insert(&"alice".as_bytes());
filter.insert(&"bob".as_bytes());

assert!(filter.contains(&"alice".as_bytes()));
assert!(!filter.contains(&"charlie".as_bytes())); // Probably false
```

### Count-Min Sketch for Frequency Estimation

```rust
use sketch_oxide::frequency::CountMinSketch;

let mut sketch = CountMinSketch::new(100, 5); // width=100, depth=5

sketch.update(&"alice".as_bytes());
sketch.update(&"bob".as_bytes());
sketch.update(&"alice".as_bytes());

// Estimate frequency
let alice_count = sketch.estimate(&"alice".as_bytes());
println!("Alice seen: {} times", alice_count); // ~2
```

### DDSketch for Quantiles

```rust
use sketch_oxide::quantiles::DDSketch;

let mut sketch = DDSketch::new(0.01)?; // 1% relative error

for i in 1..=100 {
    sketch.add(i as f64);
}

// Query quantiles
let p50 = sketch.quantile(0.50); // Median
let p99 = sketch.quantile(0.99); // 99th percentile

println!("p50: {}, p99: {}", p50, p99);
```

## Next Steps

1. **Explore algorithms** - See [Choosing an Algorithm](choosing-algorithm.md)
2. **Read detailed guides** - Visit [Algorithm Documentation](../algorithms/)
3. **Check language guides** - See [Languages](../languages/)
4. **View benchmarks** - Check [Performance](../benchmarks/)

## Getting Help

- 📖 [Full Documentation](../index.md)
- 🔍 [API Reference](../api/)
- 💬 [Discussions](https://github.com/sketchoxide/sketch_oxide/discussions)
- 🐛 [Issue Tracker](https://github.com/sketchoxide/sketch_oxide/issues)
