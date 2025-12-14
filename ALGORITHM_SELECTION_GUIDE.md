# Algorithm Selection Guide

This guide helps you choose the right probabilistic data structure for your use case. All 41 algorithms in sketch_oxide are optimized for specific problem domains.

## Quick Decision Tree

**Do you need to count distinct elements?**
- ✅ Yes → Go to [Cardinality Estimation](#cardinality-estimation)
- ❌ No → Continue

**Do you need to find top-K frequent items?**
- ✅ Yes → Go to [Frequency Estimation](#frequency-estimation)
- ❌ No → Continue

**Do you need to check set membership?**
- ✅ Yes → Go to [Membership Testing](#membership-testing)
- ❌ No → Continue

**Do you need percentiles or quantiles?**
- ✅ Yes → Go to [Quantiles](#quantiles)
- ❌ No → Continue

**Do you need time-windowed statistics?**
- ✅ Yes → Go to [Streaming Algorithms](#streaming)
- ❌ No → Continue

**Do you need to compare dataset similarity?**
- ✅ Yes → Go to [Similarity](#similarity)
- ❌ No → Continue

---

## Cardinality Estimation

Count the number of distinct elements in a stream with sub-linear space.

| Algorithm | Space | Accuracy | Speed | Use When |
|-----------|-------|----------|-------|----------|
| **HyperLogLog** | ~1.5% of cardinality | ±2% | Fast | You need standard cardinality estimation; balanced for most workloads |
| **UltraLogLog** | ~0.75% of cardinality | ±0.5% | Fast | You need higher accuracy than HLL; slightly larger space |
| **CpcSketch** | Adaptive (typically smaller) | Excellent | Medium | Streaming where accuracy matters more than speed |
| **QSketch** | Proportional to error bound | Tunable | Varies | You need different precision levels in different queries |
| **ThetaSketch** | Proportional to k-parameter | Tunable | Fast | You need set operations (union, intersection) on cardinality sketches |

### Selection Criteria

- **Best overall**: HyperLogLog (proven, fast, small space)
- **Highest accuracy**: UltraLogLog (when 2% error is too much)
- **Need set operations**: ThetaSketch (union cardinalities of streams)
- **Extreme accuracy**: CpcSketch (at cost of compilation time)
- **Custom trade-offs**: QSketch (fine-tune precision/space)

---

## Frequency Estimation

Find top-K items and estimate item frequencies in a stream.

| Algorithm | Space | Top-K Support | Heavy Hitters | Use When |
|-----------|-------|---|---|----------|
| **CountMinSketch** | O(w×d) | Via merge | Yes | Default choice for frequency; guaranteed error bounds |
| **CountSketch** | O(w×d) | Via merge | Good | Similar to CM but slightly different guarantees |
| **SpaceSaving** | O(k) | Native | Excellent | You know approx top-K size; very space-efficient |
| **FrequentItems** | O(k) | Native | Excellent | You want exact top-K; space proportional to k only |
| **ElasticSketch** | Small | Via merge | Excellent | Modern algorithm; better top-K than CM/CS |
| **SALSA** | Tunable | Native | Excellent | You need adaptive heavy-hitter detection |
| **ConservativeCountMin** | O(w×d) | Via merge | Yes | You need guaranteed NO false negatives |
| **HeavyKeeper** | Small | Native | Excellent | Recent algorithm for network monitoring |
| **RemovableUniversalSketch** | Dynamic | Via merge | Good | You can remove items and re-estimate |
| **NitroSketch** | Tunable | Via merge | Good | You need sub-microsecond update latency |

### Selection Criteria

- **Best overall**: CountMinSketch (proven, works well in practice)
- **Best space**: SpaceSaving (if you know approximate top-K size)
- **Highest accuracy**: FrequentItems + CountMinSketch (combine for redundancy)
- **Fastest**: HeavyKeeper (modern optimized algorithm)
- **No false misses**: ConservativeCountMin (safety-critical applications)
- **High-speed networking**: NitroSketch (samples updates to reduce latency)

---

## Membership Testing

Check if an element is in a set with probabilistic guarantees.

| Algorithm | Insert | Query | Delete | Space | Use When |
|-----------|--------|-------|--------|-------|----------|
| **BloomFilter** | O(k) | O(k) | No | ~10 bits/item | Standard choice; good balance |
| **BlockedBloomFilter** | O(k) | O(k) | No | ~10 bits/item | CPU cache-friendly version of Bloom |
| **CountingBloomFilter** | O(k) | O(k) | Yes | ~20 bits/item | You need deletions but want Bloom-like structure |
| **CuckooFilter** | O(1)* | O(1) | Yes | ~12 bits/item | Better space efficiency than Bloom; dynamic |
| **BinaryFuseFilter** | No | O(1) | No | ~9 bits/item | Static set known upfront; best space efficiency |
| **RibbonFilter** | O(1)** | O(1) | No | ~9 bits/item | Modern static filter; better than BFF in practice |
| **StableBloomFilter** | O(k) | O(k) | No | ~10 bits/item | Sliding window membership (decays old items) |
| **LearnedBloomFilter** | Training | O(1) | No | ~3-5 bits/item | Patterns in data; ML-enhanced membership |
| **VacuumFilter** | O(1) | O(1) | Yes | ~12-14 bits/item | Dynamic filter; best among deletion-supporting |

### Selection Criteria

- **Static set**: BinaryFuseFilter or RibbonFilter (smallest space)
- **Dynamic insertions**: CuckooFilter or VacuumFilter (best space for deletions)
- **No deletions, dynamic**: BloomFilter or BlockedBloomFilter
- **ML-friendly data**: LearnedBloomFilter (70-80% space savings vs Bloom)
- **Sliding window**: StableBloomFilter (time-decaying membership)
- **Guaranteed no false negatives**: Any except LearnedBloomFilter

---

## Quantiles

Estimate percentiles, medians, and rank-based queries in a stream.

| Algorithm | Space | Error | Use When |
|-----------|-------|-------|----------|
| **DDSketch** | Small | Configurable | Default; good balance of space/accuracy |
| **KllSketch** | Tunable | Low | High-accuracy percentiles; larger k = better accuracy |
| **TDigest** | Small | Medium | Approximate, fast; good for histograms |
| **ReqSketch** | Tunable | Guaranteed | You need error guarantees |
| **SplineSketch** | Very small | Medium | Extreme space constraints |

### Selection Criteria

- **Best overall**: DDSketch (proven, configurable error)
- **Highest accuracy**: KllSketch (larger k parameter gives better bounds)
- **Space-constrained**: SplineSketch (fits extreme constraints)
- **Guaranteed bounds**: ReqSketch (provable error guarantees)
- **Histograms**: TDigest (visualizes distributions well)

---

## Streaming

Count items in time-windowed streams.

| Algorithm | Window Type | Accuracy | Use When |
|-----------|-------------|----------|----------|
| **SlidingWindowCounter** | Sliding | High | You need exact counts in recent window |
| **ExponentialHistogram** | Sliding | Approximate | You need counts at any point in time |
| **SlidingHyperLogLog** | Sliding | Approximate | Distinct count in recent time window |

### Selection Criteria

- **Exact counts**: SlidingWindowCounter
- **Any historical query**: ExponentialHistogram
- **Distinct elements**: SlidingHyperLogLog (combines HLL with windowing)

---

## Similarity

Compare two datasets without storing all elements.

| Algorithm | Use When |
|-----------|----------|
| **MinHash** | You need Jaccard similarity; works on sets |
| **SimHash** | You need document/text similarity; detects near-duplicates |

---

## Sampling

Maintain uniform random sample of a stream.

| Algorithm | Weights | Use When |
|-----------|---------|----------|
| **ReservoirSampling** | Uniform | Standard random sampling |
| **VarOptSampling** | Weighted | Items have different importance/weights |

---

## Range Filters

Query which keys fall within a range.

| Algorithm | Use When |
|-----------|----------|
| **GRF** | Small key sets, precise ranges |
| **Grafite** | Larger key sets, good practical performance |
| **MementoFilter** | Time-windowed range queries |

---

## Pattern Matching

### Need to...
- **Detect exact duplicates**: BloomFilter or BinaryFuseFilter
- **Find top-10 items**: CountMinSketch or FrequentItems
- **Estimate dataset size**: HyperLogLog
- **Find 99th percentile**: KllSketch or TDigest
- **High-speed counting**: HeavyKeeper or SALSA
- **Compress boolean array**: StableBloomFilter
- **Machine learning preprocessing**: LearnedBloomFilter

---

## Performance Comparison Matrix

*Approximate relative performance for 1M items:*

```
Space Efficiency (smaller is better):
BinaryFuseFilter ████ Excellent
RibbonFilter     ████ Excellent
CuckooFilter     █████ Good
BloomFilter      ██████ Good
VacuumFilter     ██████ Good
CountMinSketch   ███████ Fair

Update Speed:
NitroSketch      ████ Sub-microsecond
HeavyKeeper      ████ Fast (ns)
CuckooFilter     ████ Fast (ns)
HyperLogLog      █████ Medium (us)
CountMinSketch   ██████ Slower (us)

Query Speed:
RibbonFilter     ████ Fast
BinaryFuseFilter ████ Fast
HyperLogLog      █████ Medium
CountMinSketch   ██████ Variable by depth
```

---

## Decision Flowchart

```
START
  ├─ Distinct count? → HyperLogLog
  ├─ Frequent items? → SpaceSaving or CountMinSketch
  ├─ Set membership?
  │   ├─ Static set → BinaryFuseFilter
  │   ├─ Dynamic set → CuckooFilter
  │   └─ Need deletes → VacuumFilter
  ├─ Percentiles? → DDSketch or KllSketch
  ├─ Time window? → SlidingWindowCounter or ExponentialHistogram
  ├─ Similarity? → MinHash or SimHash
  └─ Sampling? → ReservoirSampling or VarOptSampling
```

---

## See Also

- **PERFORMANCE_GUIDE.md** - Tuning parameters for each algorithm
- **INTEGRATION_PATTERNS.md** - Usage examples with real data formats
- **ALGORITHMS_COMPLETE.md** - Full technical details of all 41 algorithms
