# Choosing an Algorithm

This guide helps you select the right SketchOxide algorithm for your use case.

## Decision Tree

### Problem: Count Unique Items (Cardinality)
- **High-speed, approximate count** → **HyperLogLog**
  - Speed: 🟢 Excellent | Accuracy: 🟡 ~2% error | Memory: 🟢 O(log log N)
  - Best for: Unique visitors, unique IPs, unique items

- **Need better accuracy** → **UltraLogLog**
  - Speed: 🟢 Excellent | Accuracy: 🟢 <1% error | Memory: 🟢 O(log log N)
  - Best for: Situations where HyperLogLog's 2% error is too high

- **Need set operations** → **Theta Sketch**
  - Speed: 🟢 Excellent | Accuracy: 🟡 Configurable | Memory: 🟡 Moderate
  - Best for: Union/intersection of unique counts across datasets

- **Ultra-compressed** → **CPC Sketch**
  - Speed: 🟡 Good | Accuracy: 🟡 ~1% error | Memory: 🟢 Ultra-compact
  - Best for: When memory is extremely limited

### Problem: Check if Item Exists (Membership Testing)
- **Classic use case** → **Bloom Filter**
  - Speed: 🟢 O(k) lookups | Memory: 🟢 Compact | False Positives: 🟡 Configurable
  - Best for: URL blacklists, email verification, password checking
  - Can't: Delete items after insertion

- **Need to delete items** → **Counting Bloom Filter**
  - Speed: 🟢 O(k) | Memory: 🟡 Uses more space | Deletions: 🟢 Yes
  - Best for: Dynamic sets where items might be removed

- **Extreme speed for lookups** → **Binary Fuse Filter**
  - Speed: 🟢🟢 Fastest | Memory: 🟢 Compact | False Positives: 🟡 Low
  - Best for: Static sets with extreme performance needs

- **Dynamic insertions/deletions** → **Stable Bloom Filter**
  - Speed: 🟢 Good | Memory: 🟢 Bounded | Deletions: 🟢 Streaming-ready
  - Best for: Streams with continuous adds/removes

- **Better memory efficiency** → **Cuckoo Filter**
  - Speed: 🟢 Good | Memory: 🟢 More efficient | Deletions: 🟢 Yes
  - Best for: Large sets where memory overhead matters

- **Balanced performance** → **Ribbon Filter**
  - Speed: 🟢 Good | Memory: 🟢 Balanced | False Positives: 🟡 Low
  - Best for: General-purpose membership queries

### Problem: Track Item Frequencies (Heavy Hitters)
- **Simple frequency counting** → **Count-Min Sketch**
  - Speed: 🟢 O(log N) | Memory: 🟢 O(width×depth) | Accuracy: 🟡 Conservative
  - Best for: Finding approximate frequencies in streams
  - Weakness: Always overestimates

- **Better accuracy** → **Conservative Count-Min**
  - Speed: 🟢 O(log N) | Memory: 🟢 Moderate | Accuracy: 🟢 Improved
  - Best for: When overestimation matters

- **Find top-K items** → **Space-Saving**
  - Speed: 🟢 O(1) average | Memory: 🟢 Compact | Accuracy: 🟢 Good
  - Best for: Finding top 100 products, top keywords, hot IPs

- **More accurate frequencies** → **Count Sketch**
  - Speed: 🟢 O(log N) | Memory: 🟡 Moderate | Accuracy: 🟢 Better
  - Best for: When you need balanced overestimation

- **Advanced heavy hitter detection** → **Elastic Sketch**
  - Speed: 🟡 Adaptive | Memory: 🟡 Adaptive | Accuracy: 🟢 Very good
  - Best for: Heavy hitter detection with high accuracy

- **Find frequent items with guarantees** → **Frequent Items**
  - Speed: 🟢 Good | Memory: 🟢 Compact | Accuracy: 🟢 With guarantees
  - Best for: Streaming frequent item discovery

### Problem: Estimate Distribution Quantiles (Percentiles)
- **Fast percentile estimation** → **DDSketch**
  - Speed: 🟢 Excellent | Memory: 🟢 Compact | Accuracy: 🟡 Relative error
  - Best for: Latency percentiles (p50, p95, p99), response times

- **Better accuracy guarantees** → **REQ Sketch**
  - Speed: 🟢 Good | Memory: 🟡 Moderate | Accuracy: 🟢 Rank error bounds
  - Best for: When you need error bounds

- **High-quality approximation** → **T-Digest**
  - Speed: 🟡 Good | Memory: 🟡 Moderate | Accuracy: 🟢 Very good
  - Best for: Detailed distribution analysis, many queries

- **Space-optimal** → **KLL Sketch**
  - Speed: 🟡 Good | Memory: 🟢 Very efficient | Accuracy: 🟢 Excellent
  - Best for: When memory is critical

- **Specialized use** → **Spline Sketch**
  - Speed: 🟢 Fast | Memory: 🟡 Moderate | Accuracy: 🟡 Spline-based
  - Best for: Distribution shape analysis

### Problem: Find Similar Items (Similarity Estimation)
- **Set similarity (Jaccard)** → **MinHash**
  - Speed: 🟢 Good | Memory: 🟡 Moderate | Use: Set similarity
  - Best for: Duplicate detection, similar documents, similar users

- **Document/string similarity** → **SimHash**
  - Speed: 🟢 Excellent | Memory: 🟢 Compact | Use: Near-duplicate detection
  - Best for: Finding near-duplicate web pages, similar texts

### Problem: Sample from Streams (Sampling)
- **Uniform random sampling** → **Reservoir Sampling**
  - Speed: 🟢 Good | Memory: 🟡 O(k) for k samples | Use: Random samples
  - Best for: Getting random items without loading all data

- **Weighted sampling** → **VarOpt Sampling**
  - Speed: 🟢 Good | Memory: 🟡 O(k) for k samples | Use: Weighted samples
  - Best for: Sampling with different item weights/probabilities

### Problem: Count Over Time Windows (Streaming)
- **Count events in windows** → **Sliding Window Counter**
  - Speed: 🟢 Excellent | Memory: 🟢 Compact | Use: Time-windowed counts
  - Best for: Events per minute, requests per hour

- **Approximate histogram over time** → **Exponential Histogram**
  - Speed: 🟢 Good | Memory: 🟢 Logarithmic | Use: Exponential bucketing
  - Best for: Compressed time-series histograms

## Quick Comparison Table

| Problem | Best Choice | Time | Memory | Notes |
|---------|-------------|------|--------|-------|
| Unique count | HyperLogLog | O(1) | 16KB | Industry standard |
| Membership | Bloom Filter | O(k) | Compact | Classic & proven |
| Top-K frequent | Space-Saving | O(1) | Compact | Most practical |
| Percentiles | DDSketch | O(1) | Compact | Best for monitoring |
| Similarity | MinHash | O(1) | Compact | Set similarity |

## Real-World Examples

### Example 1: Analytics Dashboard
```
Task: Track daily unique visitors and their top actions
├─ HyperLogLog: Count unique visitors (precision=14)
├─ Count-Min Sketch: Track top 1000 URLs visited
└─ DDSketch: Track p50, p95, p99 visit duration

Memory: ~50KB per day, handles billions of visitors
```

### Example 2: DDoS Detection
```
Task: Detect attack patterns in real-time
├─ Bloom Filter: Block known malicious IPs
├─ Count-Min Sketch: Track requests per IP per minute
└─ Space-Saving: Find top attacking IPs

Memory: <1MB, detects threats in real-time
```

### Example 3: Search Engine Deduplication
```
Task: Find duplicate crawled pages
├─ SimHash: Quick duplicate detection
├─ MinHash: Find similar documents
└─ Bloom Filter: Track previously seen URLs

Memory: <100KB, process millions of URLs
```

### Example 4: Financial Transaction Analysis
```
Task: Monitor high-frequency trading patterns
├─ HyperLogLog: Unique traders per hour
├─ T-Digest: Quantiles of transaction amounts
├─ Space-Saving: Top trading pairs
└─ Sliding Window Counter: Transactions per minute

Memory: <500KB, handle 100K transactions/second
```

## By Use Case

### Web & Analytics
- **Unique visitors**: HyperLogLog
- **Top pages**: Space-Saving
- **Response times**: DDSketch
- **URL deduplication**: Bloom Filter + SimHash

### Security
- **IP Blacklist**: Bloom Filter
- **DDoS Detection**: Count-Min + Space-Saving
- **Anomaly Detection**: DDSketch

### Databases & Cache
- **Bloom Filter**: Avoid disk lookups
- **Count-Min**: Query statistics
- **T-Digest**: Index selectivity

### Real-Time Streams
- **Event counters**: Sliding Window Counter
- **Data aggregation**: Count-Min Sketch
- **Quality metrics**: DDSketch

### Machine Learning
- **Feature hashing**: SimHash
- **Similarity learning**: MinHash
- **Streaming statistics**: T-Digest

## Performance Guidelines

| Scale | Algorithm | Notes |
|-------|-----------|-------|
| 1M items | Any algorithm | All work fine |
| 1B items | HyperLogLog, Bloom | Designed for billions |
| 1T items | CPC, Theta | Compressed sketches |
| Streaming | All | Designed for single-pass |
| Merging | Most | Supported in design |

## Next Steps

1. Pick your problem from above
2. Read the [detailed algorithm documentation](../algorithms/)
3. Check language-specific examples in [Languages](../languages/)
4. Review [benchmarks](../benchmarks/) for your use case
