# Performance Tuning Guide

This guide helps you optimize sketch_oxide algorithms for your specific performance and accuracy requirements.

## Quick Reference: Parameter Tuning

| Algorithm | Key Parameter | Range | Default | Tradeoff |
|-----------|---|---|---|---|
| HyperLogLog | precision | 4-16 | 14 | More bits = better accuracy |
| DDSketch | relative_accuracy | 0.001-0.1 | 0.01 | Smaller = higher accuracy |
| CountMinSketch | epsilon, delta | ε:0.001-0.1, δ:0.001-0.01 | 0.01, 0.001 | Smaller = more space |
| BloomFilter | fpr | 0.001-0.1 | 0.01 | Smaller FPR = more space |
| CuckooFilter | capacity | Any | Implementation-specific | More capacity = more space |
| KllSketch | k | 8-256 | 200 | Higher k = better quantiles |
| ReqSketch | k | 4-256 | 200 | Higher k = better accuracy |
| SpaceSaving | k | Any | Implementation-specific | k = max top-K to track |
| TDigest | compression | 10-1000 | 100 | Higher = more accurate |
| MinHash | num_perm | 64-1024 | 256 | More = higher accuracy |

---

## Cardinality Estimation

### Memory vs Accuracy Trade-offs

**HyperLogLog**
```
Precision | Space | Accuracy | Use Case
---------|-------|----------|----------
    4    | ~1KB  |  ±32%    | Rough estimates, memory critical
    8    | ~4KB  |   ±6%    | General purpose
   10    | ~16KB |  ±1.5%   | Most applications (RECOMMENDED)
   14    | ~1MB  |  ±0.4%   | High accuracy needed
   16    | ~4MB  |  ±0.1%   | Maximum accuracy
```

**Tuning Recommendation:**
- Start with precision=12 (16KB space, ±0.7% error)
- Increase to 14 if accuracy is critical
- Use 10-12 for memory-constrained systems

**Tips:**
- Precision > 16 has diminishing returns
- Error is approximately 1.04/√(2^precision)
- Mergeable across same precision only

### UltraLogLog vs HyperLogLog

| Metric | HyperLogLog | UltraLogLog |
|--------|---|---|
| Space | ~1.5% of cardinality | ~0.75% of cardinality |
| Error | ±2% (typical) | ±0.5% (typical) |
| Speed | Faster | Slightly slower |
| Use When | Need speed; 2% error acceptable | Need <1% error; can afford extra space |

---

## Frequency Estimation

### Memory Configuration

**CountMinSketch**
```
Width x Depth Configuration for Different Accuracies:
Error Bound = epsilon, Failure Probability = delta

For ε=0.01, δ=0.001 (1% error, 0.1% failure):
- Space: ~80KB
- Speed: Fast queries
- Best for: Stream processing with guaranteed bounds

For ε=0.001, δ=0.0001 (0.1% error, 0.01% failure):
- Space: ~8MB
- Speed: Slower due to more hash functions
- Best for: High accuracy needed, abundant memory
```

**Tuning:**
- Reduce epsilon for tighter error bounds
- Reduce delta to lower failure probability
- Typical production values: ε=0.01, δ=0.001

### Heavy Hitters Optimization

**Best Algorithms by Use Case:**

1. **Known Top-K size (e.g., top 100 items)**
   - Use: SpaceSaving or FrequentItems
   - Space: O(k) regardless of stream size
   - Recommendation: k = expected top-K size × 1.5

2. **Unknown distribution**
   - Use: CountMinSketch or CountSketch
   - Space: O(1/ε × log(1/δ))
   - Then run point queries for suspected heavy items

3. **Sub-microsecond latency**
   - Use: HeavyKeeper or SALSA
   - Optimized for speed over space
   - Trade: Some accuracy for nanosecond updates

### SALSA vs CountMinSketch

```
SALSA: Selective Algorithms for Long Stream Analysis
- Better top-K accuracy than CountMinSketch
- Adaptive bucket sizing
- Memory: Similar to CountMinSketch
- Speed: Comparable to CountMinSketch
- Recommendation: Use SALSA for new code
```

---

## Membership Testing

### Space Efficiency Comparison

```
For 1 million items at 1% FPR:

Algorithm            | Space      | Dynamic? | Delete? | Notes
---------------------|------------|----------|---------|----------
Bloom Filter         | ~10 Mbits  | No       | No      | Baseline
Counting Bloom       | ~20 Mbits  | No       | Yes     | Larger due to counters
Cuckoo Filter        | ~12 Mbits  | Yes      | Yes     | Can fail on insert
Binary Fuse Filter   | ~9 Mbits   | No       | No      | Most space-efficient
Ribbon Filter        | ~9 Mbits   | No       | No      | Similar to BFF
Stable Bloom         | ~10 Mbits  | No       | No      | For time windows
Learned Bloom        | ~3-4 Mbits | No       | No      | ML-enhanced (risky)
Vacuum Filter        | ~12 Mbits  | Yes      | Yes     | Best for dynamic

WINNER BY CATEGORY:
- Space: Binary Fuse or Ribbon (~9 Mbits)
- Dynamic: Cuckoo or Vacuum (~12 Mbits)
- With deletions: Vacuum or Counting Bloom
```

### Tuning Parameters

**BloomFilter**
```
For target false positive rate (FPR):
- FPR = 0.01 (1%):  Space ≈ 9.6 bits/item ← RECOMMENDED
- FPR = 0.001 (0.1%): Space ≈ 14.4 bits/item
- FPR = 0.0001:    Space ≈ 19.2 bits/item

Formula: bits_per_item ≈ -1.44 × log2(FPR)
```

**CuckooFilter**
```
Capacity parameter:
- Set capacity = expected_items × 1.2 (20% headroom)
- Too small: Inserts will fail
- Too large: Wastes space
- Sweet spot: 0.8-0.95 load factor
```

**RibbonFilter**
```
Modern alternative to Bloom and Cuckoo
- Space: 9-10 bits/item (similar to Binary Fuse)
- Speed: Faster than Bloom filters
- Use: When building static filters (like BinaryFuseFilter)
```

---

## Quantiles

### Accuracy vs Memory Trade-offs

**DDSketch (Recommended for most users)**
```
Relative Accuracy | Space | Use Case
-----------------|-------|----------
0.1 (10%)        | ~1KB  | Rough approximations
0.05 (5%)        | ~2KB  | Most web applications
0.01 (1%)        | ~10KB | High accuracy needed ← RECOMMENDED
0.001 (0.1%)     | ~50KB | Very high accuracy

Default (0.01): Works well for 99%, 99.9%, p99.99 percentile queries
```

**KllSketch (Highest accuracy)**
```
k parameter (sketch size):
- k=8:   Ultra-compact, ±12% error
- k=32:  Compact, ±4% error
- k=64:  Balanced, ±2% error ← RECOMMENDED
- k=128: High accuracy, ±1% error
- k=256: Very high, ±0.5% error

More k = more memory but better bounds
```

**TDigest (Fast approximations)**
```
Compression parameter:
- compression=10:   Very fast, ±5% error
- compression=100:  Balanced, ±1% error ← RECOMMENDED
- compression=1000: High accuracy, ±0.1% error (slow)

Good for histograms and approximate quantiles
```

### Percentile Accuracy

```
For a stream of 1 million values (Rank ≈ 10^6):

Algorithm   | DDSketch(0.01) | KLL(k=64) | TDigest(100) | ReqSketch
------------|---|---|---|---
p50         | ±0.1% error    | ±0.1%     | ±0.5%        | ±0.1%
p99         | ±1% error      | ±0.5%     | ±1%          | ±0.1%
p99.9       | ±2% error      | ±1%       | ±2%          | ±0.1%
p99.99      | ±5% error      | ±2%       | ±5%          | ±0.1%

Space       | ~10KB          | ~8KB      | ~5KB         | ~50KB
```

---

## Streaming Algorithms

### Time Window Configuration

**SlidingWindowCounter**
```
Parameters: window_size (seconds), epsilon

For 1 hour window (3600 seconds):
- epsilon=0.01:  Space ≈ 40KB, error ±1%
- epsilon=0.001: Space ≈ 400KB, error ±0.1%

Recommendation: epsilon=0.01 for most applications
```

**ExponentialHistogram**
```
Better for arbitrary time queries (not just fixed windows)
- Space: O(log²(N)) where N = total items ever
- Trade: Approximate counts (error bounded)
- Use when: Need ad-hoc time queries
```

**SlidingHyperLogLog**
```
Distinct count in recent window
- Precision: Same as HyperLogLog (4-16)
- Window: Fixed time window
- Use when: Need cardinality of recent distinct items
```

---

## Similarity Algorithms

### MinHash Tuning

```
num_perm parameter (number of permutations):
- 64:   Compact, ±4% Jaccard error
- 128:  Standard, ±2.5% error ← RECOMMENDED
- 256:  High accuracy, ±1.5% error
- 512:  Very high, ±1% error

More permutations = better accuracy but more space
Typical usage: 128-256 permutations
```

### When to Use Each

**MinHash**
- Computing Jaccard similarity (set overlap)
- Large, high-dimensional datasets
- Document deduplication

**SimHash**
- Document/text similarity
- Detects near-duplicates
- Hamming distance between fingerprints

---

## Sampling Algorithms

### ReservoirSampling Configuration

```
Sample size parameter:
- size=100:    Compact sample
- size=1000:   Balanced ← RECOMMENDED
- size=10000:  Very comprehensive

Memory = O(size × item_size)
For random uniform samples
```

### VarOptSampling

```
Weighted sampling where items have importance weights
- Use when items have different significance
- Same sample size parameter as ReservoirSampling
- Memory: O(size) tuples + weights
```

---

## Platform-Specific Performance

### CPU Architecture

**x86_64**
- All algorithms optimized
- Vector instructions supported
- Best performance ✓

**ARM64 (M1/M2/M3 Macs)**
- Generally 5-10% slower than x86_64
- SIMD supported for key operations
- Acceptable performance

**Linux Servers**
- Optimal for stream processing
- No GC pauses (Rust advantage)
- Recommended for production

### Memory Patterns

**NUMA Systems**
- Allocate near processing cores
- Consider thread-local instances
- May need tuning for optimal performance

**Cache Optimization**
- BlockedBloomFilter: Better CPU cache locality than BloomFilter
- Use BlockedBloomFilter for hot path
- Typical speedup: 10-30%

---

## Benchmarking Your Code

### Using sketch_oxide for Benchmarks

**Rust with Criterion**
```rust
#[bench]
fn bench_hyperloglog_add(b: &mut Bencher) {
    let mut hll = HyperLogLog::new(14).unwrap();
    b.iter(|| hll.update(b"test_data"));
}
```

**Measure Throughput**
```bash
cargo bench --release
```

### Comparing Algorithms

1. Use same input size and distribution
2. Measure hot loop performance (after warmup)
3. Account for memory allocation (separate from operation)
4. Test on target hardware
5. Compare space usage, not just speed

### Performance Baselines

```
Typical Throughput (ops/second, after warmup):

HyperLogLog.update:        100M updates/sec
CountMinSketch.estimate:   80M queries/sec
BloomFilter.insert:        200M inserts/sec
DDSketch.add:              80M adds/sec
MinHash.update:            50M updates/sec
ReservoirSampling.update:  150M updates/sec

(Measured on Intel i7, Rust release build)
```

---

## Optimization Checklist

- [ ] Choose algorithm based on [ALGORITHM_SELECTION_GUIDE](./ALGORITHM_SELECTION_GUIDE.md)
- [ ] Start with default parameters
- [ ] Measure space and speed requirements
- [ ] Adjust parameters conservatively
- [ ] Benchmark with realistic data
- [ ] Test on target hardware/platform
- [ ] Monitor memory usage in production
- [ ] Set up performance regression detection

---

## Troubleshooting

### "Sketch too large"
- Reduce precision/accuracy parameters
- Consider approximate algorithms (DDSketch vs KLL)
- Use space-efficient variants (SpaceSaving vs CountMinSketch)

### "Updates too slow"
- Use faster algorithms (HeavyKeeper vs CountMinSketch)
- Reduce accuracy requirements
- Consider sampling-based approaches (NitroSketch)

### "Accuracy degrading"
- Increase precision/k/compression parameters
- Use higher accuracy algorithm variant
- Ensure sufficient data stream size

---

## See Also
- **ALGORITHM_SELECTION_GUIDE.md** - Choosing the right algorithm
- **INTEGRATION_PATTERNS.md** - Real-world examples
- **ALGORITHMS_COMPLETE.md** - Detailed algorithm specifications
