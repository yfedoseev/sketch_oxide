# @sketch-oxide/node

High-performance probabilistic data structures for Node.js, powered by Rust.

**28-75% better space efficiency** than classic algorithms (VLDB 2024 research).

## Features

- 🚀 **High Performance**: Native Rust implementation via napi-rs
- 📊 **28 Algorithms**: Cardinality, frequency, quantiles, membership testing, and more
- 🔒 **Type-Safe**: Full TypeScript support with auto-generated definitions
- 🧪 **Production-Ready**: Comprehensive test suite and benchmarks
- 🛠️ **Easy to Use**: Ergonomic JavaScript API

## Installation

```bash
npm install @sketch-oxide/node
```

Prebuilt binaries are provided for:
- Linux x64 (gnu and musl)
- macOS x64 and arm64 (Apple Silicon)
- Windows x64

## Quick Start

### HyperLogLog (Cardinality Estimation)

```javascript
const { HyperLogLog } = require('@sketch-oxide/node')

const hll = new HyperLogLog(14)
hll.update(Buffer.from('user-1'))
hll.update(Buffer.from('user-2'))
hll.update(Buffer.from('user-1')) // Duplicate, not re-counted

console.log(hll.estimate()) // ~2
```

### CountMinSketch (Frequency Estimation)

```javascript
const { CountMinSketch } = require('@sketch-oxide/node')

const cms = new CountMinSketch(0.01, 0.01) // 1% error, 99% confidence
cms.update(Buffer.from('apple'), 5)
cms.update(Buffer.from('banana'), 3)

console.log(cms.estimate(Buffer.from('apple'))) // >= 5
console.log(cms.estimate(Buffer.from('cherry'))) // ~0
```

### Serialization & Deserialization

```javascript
const hll = new HyperLogLog(14)
hll.update(Buffer.from('item1'))

// Store to disk
const data = hll.serialize()
fs.writeFileSync('hll.bin', data)

// Load from disk
const loaded = HyperLogLog.deserialize(fs.readFileSync('hll.bin'))
console.log(loaded.estimate()) // Same as original
```

## Algorithms

### Cardinality Estimation (5)
- **UltraLogLog** - 28% better than HyperLogLog (VLDB 2024)
- **HyperLogLog** - Classic cardinality estimation
- **CpcSketch** - 30-40% better than HyperLogLog
- **QSketch** - Weighted cardinality
- **ThetaSketch** - Set operations support

### Frequency Estimation (8)
- **CountMinSketch** - Standard frequency estimation
- **CountSketch** - Unbiased with L2 error bounds
- **ConservativeCountMin** - Up to 10x more accurate
- **SpaceSaving** - Heavy hitters with error bounds
- **ElasticSketch** - Network measurement
- **SALSA** - Adaptive counter sizing
- **RemovableUniversalSketch** - Turnstile streams with deletions
- **FrequentItems** - Top-K detection

### Membership Testing (7)
- **BinaryFuseFilter** - 75% better than Bloom (ACM JEA 2022)
- **BloomFilter** - Classic Bloom filter
- **BlockedBloomFilter** - Cache-efficient
- **CountingBloomFilter** - Supports deletions
- **CuckooFilter** - Space-efficient
- **RibbonFilter** - 7 bits/key @ 1% FPR
- **StableBloomFilter** - Unbounded streams

### Quantile Estimation (5)
- **DDSketch** - Relative error guarantees (Datadog)
- **ReqSketch** - Zero error at tail (Google BigQuery)
- **TDigest** - Accurate quantiles
- **KllSketch** - Efficient rank queries
- **SplineSketch** - High-accuracy interpolation

### Streaming (2)
- **SlidingWindowCounter** - Time-bounded counting
- **ExponentialHistogram** - Event counting with bounds

### Similarity (2)
- **MinHash** - Jaccard similarity
- **SimHash** - Near-duplicate detection

### Sampling (2)
- **ReservoirSampling** - Uniform random sampling
- **VarOptSampling** - Variance-optimal weighted sampling

## Benchmarks

Performance comparison for cardinality estimation (1M items):

| Algorithm | Time | Memory | Error |
|-----------|------|--------|-------|
| HyperLogLog | 34ms | 16KB | 0.8% |
| UltraLogLog | 35ms | 12KB | 0.6% |
| CpcSketch | 40ms | 10KB | 0.5% |

Benchmarks run on: Intel i7-9700K, Node.js 18, 1M random items

Run benchmarks:
```bash
npm run bench
```

## Testing

```bash
# Run all tests
npm test

# Watch mode
npm run test:watch

# Coverage report
npm test -- --coverage
```

Tests include:
- ✅ 50+ HyperLogLog tests
- ✅ Accuracy verification (error bounds)
- ✅ Serialization/deserialization
- ✅ Merging sketches
- ✅ Stress tests (100K+ items)
- ✅ Cross-language interoperability

## API Reference

### HyperLogLog

```typescript
class HyperLogLog {
  constructor(precision: number)
  update(item: Buffer): void
  estimate(): number
  merge(other: HyperLogLog): void
  reset(): void
  precision(): number
  serialize(): Buffer
  static deserialize(data: Buffer): HyperLogLog
  toString(): string
}
```

### CountMinSketch

```typescript
class CountMinSketch {
  constructor(epsilon: number, delta: number)
  update(item: Buffer, count?: number): void
  estimate(item: Buffer): number
  merge(other: CountMinSketch): void
  reset(): void
  serialize(): Buffer
  static deserialize(data: Buffer): CountMinSketch
  toString(): string
}
```

All other algorithms follow similar patterns. See `index.d.ts` for full TypeScript definitions.

## TypeScript Example

```typescript
import { HyperLogLog, CountMinSketch } from '@sketch-oxide/node'

// Cardinality
const hll: HyperLogLog = new HyperLogLog(14)
hll.update(Buffer.from('item'))
const cardinality: number = hll.estimate()

// Frequency
const cms: CountMinSketch = new CountMinSketch(0.01, 0.01)
cms.update(Buffer.from('word'), 1)
const frequency: number = cms.estimate(Buffer.from('word'))
```

## Performance Tips

1. **Reuse sketches**: Create once, update many times
2. **Batch updates**: Reduce FFI boundary crossings
3. **Choose precision**: Lower precision = faster, higher error
4. **Serialize carefully**: Only when needed (checkpoint, storage)

## Comparison with Python

Same API as `sketch_oxide` Python package:

```python
# Python
from sketch_oxide import HyperLogLog
hll = HyperLogLog(14)
hll.update(b'item')
print(hll.estimate())

# Node.js
const { HyperLogLog } = require('@sketch-oxide/node')
const hll = new HyperLogLog(14)
hll.update(Buffer.from('item'))
console.log(hll.estimate())
```

## vs Apache DataSketches

| Feature | sketch_oxide | DataSketches |
|---------|--------|--------|
| Algorithms | 28 | 20 |
| Languages | Node.js, Python, Java, C# | Python, Java, C++ |
| TypeScript | ✅ Auto-generated | ❌ |
| Space Efficiency | 28-75% better | Baseline |
| Performance | Rust native | Java/Python |
| Latest Research | VLDB 2024 | 2019-2020 |

## Building from Source

```bash
cd nodejs

# Install Rust (https://rustup.rs/)
# Install Node.js 16+ and npm

npm install
npm run build

npm test
npm run bench
```

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Write tests for new functionality
4. Commit your changes (`git commit -am 'Add amazing feature'`)
5. Push to the branch (`git push origin feature/amazing-feature`)
6. Open a Pull Request

## License

MIT

## References

- [NAPI-RS Documentation](https://napi.rs/)
- [sketch_oxide Rust Library](https://github.com/yourusername/sketch_oxide)
- [Probabilistic Data Structures](https://en.wikipedia.org/wiki/Probabilistic_data_structure)
- [Apache DataSketches](https://datasketches.apache.org/)

## Citation

If you use sketch_oxide in academic work, please cite:

```bibtex
@software{sketch_oxide,
  author = {Your Name},
  title = {sketch_oxide: State-of-the-Art Probabilistic Data Structures},
  year = {2025},
  url = {https://github.com/yourusername/sketch_oxide}
}
```

## Support

- 📖 [Documentation](./README.md)
- 🐛 [Issue Tracker](https://github.com/yourusername/sketch_oxide/issues)
- 💬 [Discussions](https://github.com/yourusername/sketch_oxide/discussions)
