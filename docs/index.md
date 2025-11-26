# SketchOxide Documentation

Welcome to **SketchOxide** - a high-performance, production-ready library of 28 probabilistic data structures written in Rust with bindings for Python, Java, Node.js, and C#/.NET.

## Quick Navigation

### 🚀 Getting Started
- **[Installation](getting-started/installation.md)** - Install SketchOxide in your language
- **[Quick Start](getting-started/quick-start.md)** - Run your first sketch in 5 minutes
- **[Choosing an Algorithm](getting-started/choosing-algorithm.md)** - Find the right sketch for your use case

### 📚 Algorithm Documentation
Comprehensive guides for all 28 algorithms organized by category:

**Cardinality Estimation (5 algorithms)**
- [HyperLogLog](algorithms/cardinality/hyperloglog.md) - Fast cardinality with O(1) space
- [UltraLogLog](algorithms/cardinality/ultraloglog.md) - Better accuracy than HyperLogLog
- [CPC Sketch](algorithms/cardinality/cpc-sketch.md) - Compressed probabilistic counting
- [Theta Sketch](algorithms/cardinality/theta-sketch.md) - Fast set operations
- [Q Sketch](algorithms/cardinality/qsketch.md) - Probabilistic cardinality

**Frequency Estimation (8 algorithms)**
- [Count-Min Sketch](algorithms/frequency/count-min-sketch.md) - Frequency estimation with worst-case guarantees
- [Count Sketch](algorithms/frequency/count-sketch.md) - Sketching with cancelations
- [Conservative Count-Min](algorithms/frequency/conservative-count-min.md) - Reduced overestimation
- [Space-Saving](algorithms/frequency/space-saving.md) - Top-K frequent items
- [Frequent Items](algorithms/frequency/frequent-items.md) - Find frequent elements
- [Elastic Sketch](algorithms/frequency/elastic-sketch.md) - Adaptive heavy hitter detection
- [SALSA](algorithms/frequency/salsa.md) - Sketching with analysis
- [Removable Universal Sketch](algorithms/frequency/removable-universal-sketch.md) - Support for deletions

**Membership Testing (7 algorithms)**
- [Bloom Filter](algorithms/membership/bloom-filter.md) - Classic membership testing
- [Blocked Bloom Filter](algorithms/membership/blocked-bloom-filter.md) - Cache-efficient variant
- [Counting Bloom Filter](algorithms/membership/counting-bloom-filter.md) - Support for deletions
- [Cuckoo Filter](algorithms/membership/cuckoo-filter.md) - Better space efficiency
- [Binary Fuse Filter](algorithms/membership/binary-fuse-filter.md) - Ultra-fast lookups
- [Ribbon Filter](algorithms/membership/ribbon-filter.md) - Balanced performance
- [Stable Bloom Filter](algorithms/membership/stable-bloom-filter.md) - Streams with insertions/deletions

**Quantile Estimation (5 algorithms)**
- [DDSketch](algorithms/quantiles/ddsketch.md) - Relative error quantile sketches
- [REQ Sketch](algorithms/quantiles/req-sketch.md) - Ranking error quantiles
- [T-Digest](algorithms/quantiles/tdigest.md) - Approximation for distributions
- [KLL Sketch](algorithms/quantiles/kll-sketch.md) - Space-optimal quantiles
- [Spline Sketch](algorithms/quantiles/spline-sketch.md) - Spline-based approximation

**Similarity (2 algorithms)**
- [MinHash](algorithms/similarity/minhash.md) - Approximate set similarity
- [SimHash](algorithms/similarity/simhash.md) - Document similarity hashing

**Sampling (2 algorithms)**
- [Reservoir Sampling](algorithms/sampling/reservoir-sampling.md) - Uniform random sampling
- [VarOpt Sampling](algorithms/sampling/varopt-sampling.md) - Weighted sampling with optimal variance

**Streaming (2 algorithms)**
- [Sliding Window Counter](algorithms/streaming/sliding-window-counter.md) - Count within time windows
- [Exponential Histogram](algorithms/streaming/exponential-histogram.md) - Sketching with exponent

### 🌐 Language-Specific Guides
- **[Rust](languages/rust.md)** - Using SketchOxide directly in Rust
- **[Python](languages/python.md)** - PyO3 bindings with NumPy integration
- **[Java](languages/java.md)** - JNI bindings with AutoCloseable pattern
- **[Node.js](languages/nodejs.md)** - NAPI TypeScript bindings
- **[C#/.NET](languages/dotnet.md)** - P/Invoke bindings with IDisposable pattern

### 📊 Performance & Benchmarks
- **[Benchmark Methodology](benchmarks/methodology.md)** - How we measure performance
- **[Rust Benchmarks](benchmarks/rust-benchmarks.md)** - vs pdatastructs, probabilistic-collections, streaming_algorithms
- **[Java Benchmarks](benchmarks/java-benchmarks.md)** - vs Apache DataSketches, stream-lib
- **[Node.js Benchmarks](benchmarks/nodejs-benchmarks.md)** - vs bloom-filters, hyperloglog, count-min-sketch
- **[C# Benchmarks](benchmarks/dotnet-benchmarks.md)** - vs Probably, BloomFilter.NetCore
- **[Performance Summary](../PERFORMANCE_SUMMARY.md)** - Space efficiency comparisons

### 🔌 API Reference
- **[API Documentation](api/)** - Complete API reference for all languages

## Key Features

✅ **28 State-of-the-Art Algorithms** - All major probabilistic data structures
✅ **Multi-Language Support** - Rust, Python, Java, Node.js, C#/.NET
✅ **Production-Ready** - Comprehensive tests, benchmarks, and documentation
✅ **High Performance** - 2024-2025 research-backed implementations
✅ **Memory Efficient** - 28-75% better space efficiency than classic algorithms
✅ **Well-Tested** - 500+ unit tests across all languages

## Most Popular Use Cases

- **Real-time Analytics** - HyperLogLog for unique visitor counting
- **Spam Detection** - Bloom filters for URL/email blacklisting
- **DDoS Protection** - Count-Min sketches for traffic pattern analysis
- **Search Engines** - Theta sketches for set operations at scale
- **Recommendation Systems** - MinHash for document/user similarity
- **Fraud Detection** - Frequent items detection with Space-Saving
- **Log Analysis** - Quantile sketches for latency percentiles

## Installation

Choose your language:

```bash
# Rust - Add to Cargo.toml
[dependencies]
sketch_oxide = "0.1"

# Python - Install via pip
pip install sketch-oxide

# Java - Add to pom.xml
<dependency>
    <groupId>io.sketchoxide</groupId>
    <artifactId>sketch-oxide</artifactId>
    <version>0.1.0</version>
</dependency>

# Node.js - Install via npm
npm install @sketchoxide/core

# C#/.NET - Install via NuGet
dotnet add package SketchOxide
```

See **[Installation Guide](getting-started/installation.md)** for detailed instructions.

## Contributing

SketchOxide welcomes contributions! See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

## License

Licensed under [MIT License](../LICENSE)

## Citation

If you use SketchOxide in your research, please cite:

```bibtex
@software{sketchoxide2025,
  title={SketchOxide: High-Performance Probabilistic Data Structures},
  year={2025}
}
```

## Support

- 📖 [Documentation](.) - Full documentation
- 🐛 [Issue Tracker](https://github.com/sketchoxide/sketch_oxide/issues) - Report bugs
- 💬 [Discussions](https://github.com/sketchoxide/sketch_oxide/discussions) - Ask questions
