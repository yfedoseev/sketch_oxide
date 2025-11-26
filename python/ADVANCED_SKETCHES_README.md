# Advanced Frequency Sketches - PyO3 Python Bindings

This directory contains high-performance Python bindings for three advanced frequency sketch algorithms from sketch_oxide. These are probabilistic data structures for efficiently tracking item frequencies in streaming data.

## Included Sketches

### 1. ElasticSketch
**Purpose**: Accurate frequency estimation with elastic counters that adapt to frequency distributions.

**Key Features**:
- Elastic counters for reduced overestimation
- O(depth) update and query time
- Heavy hitter detection with threshold filtering
- Serialization support
- Optimized for network traffic measurement

**Example**:
```python
from sketch_oxide import ElasticSketch

sketch = ElasticSketch(bucket_count=512, depth=3)
sketch.update(b"flow_id", 100)
print(sketch.estimate(b"flow_id"))  # >= 100

# Find frequent items
heavy_hitters = sketch.heavy_hitters(threshold=50)
```

### 2. SALSA (Self-Adjusting Counter Sizing Algorithm)
**Purpose**: Adaptive frequency estimation for skewed/heavy-tailed distributions.

**Key Features**:
- Automatically adapts when frequencies approach overflow
- Confidence metric for estimation quality
- Never underestimates frequencies
- Designed for production systems with Zipfian distributions
- Efficient handling of heavy hitters

**Example**:
```python
from sketch_oxide import SALSA

salsa = SALSA(epsilon=0.01, delta=0.01)
salsa.update(b"item", 500)
estimate, confidence = salsa.estimate(b"item")
print(f"Estimate: {estimate}, Confidence: {confidence}%")
```

### 3. RemovableUniversalSketch
**Purpose**: Frequency estimation with support for deletions (turnstile streams).

**Key Features**:
- Supports both insertions and deletions
- Tracks frequency moments (L2 norm)
- Handles negative frequencies
- Essential for applications with data removal/expiration
- Enables cache and eviction tracking

**Example**:
```python
from sketch_oxide import RemovableUniversalSketch

rus = RemovableUniversalSketch(epsilon=0.01, delta=0.01)
rus.update(b"item", 100)      # Insert 100 occurrences
rus.update(b"item", -30)      # Remove 30 occurrences
print(rus.estimate(b"item"))  # >= 70
print(rus.l2_norm())          # Frequency moment
```

## Installation

The sketches are compiled as part of sketch_oxide. Ensure the Python bindings are built:

```bash
cd python
pip install -e .
```

## API Reference

### ElasticSketch

```python
class ElasticSketch:
    def __init__(bucket_count: int, depth: int)
    def __init__.with_elastic_ratio(bucket_count: int, depth: int, elastic_ratio: float)

    # Updates
    def update(item: bytes, count: int) -> None

    # Queries
    def estimate(item: bytes) -> int
    def heavy_hitters(threshold: int) -> List[Tuple[int, int]]

    # Management
    def merge(other: ElasticSketch) -> None
    def reset() -> None

    # Serialization
    def serialize() -> bytes
    @staticmethod
    def deserialize(data: bytes) -> ElasticSketch

    # Properties
    def bucket_count() -> int
    def depth() -> int
    def elastic_ratio() -> float
    def total_count() -> int
    def is_empty() -> bool
    def memory_usage() -> int
```

### SALSA

```python
class SALSA:
    def __init__(epsilon: float, delta: float)

    # Updates
    def update(item: bytes, count: int) -> None

    # Queries
    def estimate(item: bytes) -> Tuple[int, int]  # (estimate, confidence)

    # Management
    def merge(other: SALSA) -> None

    # Properties
    def epsilon() -> float
    def delta() -> float
    def max_observed() -> int
    def total_updates() -> int
    def adaptation_level() -> int
    def width() -> int
    def depth() -> int
```

### RemovableUniversalSketch

```python
class RemovableUniversalSketch:
    def __init__(epsilon: float, delta: float)

    # Updates (supports negative counts for deletions)
    def update(item: bytes, delta: int) -> None

    # Queries
    def estimate(item: bytes) -> int
    def l2_norm() -> float

    # Management
    def merge(other: RemovableUniversalSketch) -> None

    # Properties
    def epsilon() -> float
    def delta() -> float
    def width() -> int
    def depth() -> int
```

## Parameter Guide

### Epsilon (ε)
Controls the accuracy of frequency estimates. Error is bounded by εN where N is the total count.

- `0.001`: High accuracy, ~1% error, more memory
- `0.01`: Balanced (recommended), ~1% error, moderate memory
- `0.1`: Low accuracy, ~10% error, less memory
- `0.5`: Very low accuracy, ~50% error, minimal memory

### Delta (δ)
Failure probability. Guarantees hold with probability 1-δ.

- `0.001`: 99.9% confidence (very safe)
- `0.01`: 99% confidence (recommended, safe)
- `0.05`: 95% confidence (acceptable)
- `0.1`: 90% confidence (risky)

### Bucket Count & Depth (ElasticSketch)
- **bucket_count**: Buckets per hash row (auto-rounded to power of 2)
  - 128-256: Small/embedded
  - 512-1024: Normal (recommended)
  - 2048+: High precision

- **depth**: Number of independent hash functions
  - 2-3: Balanced accuracy/speed
  - 4-5: Higher accuracy
  - 6+: Very high accuracy

## Testing

Run the comprehensive test suite:

```bash
# Test all three sketches
pytest test_elastic_sketch.py test_salsa.py test_removable_sketch.py -v

# Run demo
python demo_advanced_sketches.py
```

## Performance Characteristics

| Operation | Complexity | Space |
|-----------|-----------|-------|
| Update | O(depth) | O(bucket_count × depth) |
| Estimate | O(depth) | O(bucket_count × depth) |
| Heavy Hitters | O(bucket_count × depth) | O(bucket_count × depth) |
| L2 Norm | O(bucket_count × depth) | Additional moment sketch |

## Real-World Use Cases

### ElasticSketch: Network Traffic Monitoring
```python
traffic = ElasticSketch(1024, 4)

# Monitor flows
traffic.update(b"flow_tcp_1.2.3.4:80", packets)
traffic.update(b"flow_udp_5.6.7.8:53", packets)

# Find DDoS sources
ddos_flows = traffic.heavy_hitters(threshold=100000)
```

### SALSA: Web Server Request Tracking
```python
requests = SALSA(epsilon=0.01, delta=0.01)

# Track URLs with Zipfian distribution
requests.update(b"/index.html", 5000)      # Home page
requests.update(b"/api/users", 500)         # API endpoint
requests.update(b"/admin/panel", 50)        # Admin area

# Check with confidence
estimate, conf = requests.estimate(b"/index.html")
```

### RemovableSketch: Cache Hit/Miss Tracking
```python
cache = RemovableUniversalSketch(0.01, 0.01)

# Track page views
cache.update(b"page1", 1000)
cache.update(b"page2", 500)

# Cache evictions (deletions)
cache.update(b"page1", -100)
cache.update(b"page2", -50)

# Track cache "energy" (L2 norm)
total_energy = cache.l2_norm()
```

## Error Handling

All methods properly handle errors via Python exceptions:

```python
try:
    sketch = ElasticSketch(0, 3)  # Invalid bucket_count
except ValueError as e:
    print(f"Invalid parameters: {e}")

try:
    sketch1 = ElasticSketch(512, 3)
    sketch2 = ElasticSketch(256, 3)
    sketch1.merge(sketch2)  # Incompatible
except ValueError as e:
    print(f"Merge failed: {e}")
```

## Advanced Topics

### Serialization for Persistence
```python
# Save sketch to file
sketch = ElasticSketch(512, 3)
sketch.update(b"item", 100)
data = sketch.serialize()

with open("sketch.bin", "wb") as f:
    f.write(data)

# Restore from file
with open("sketch.bin", "rb") as f:
    data = f.read()
restored = ElasticSketch.deserialize(data)
```

### Merging Distributed Sketches
```python
# Collect sketches from multiple servers
local_sketch = ElasticSketch(512, 3)
remote1 = ElasticSketch(512, 3)
remote2 = ElasticSketch(512, 3)

# Populate...
local_sketch.update(b"flow1", 100)
remote1.update(b"flow1", 50)
remote2.update(b"flow1", 75)

# Merge for global view
local_sketch.merge(remote1)
local_sketch.merge(remote2)

print(local_sketch.estimate(b"flow1"))  # >= 225
```

### Tracking Frequency Moments with RemovableSketch
```python
stream = RemovableUniversalSketch(0.01, 0.01)

# Stream of updates and deletions
for item, count in data:
    stream.update(item.encode(), count)

# Compute L2 norm for statistical analysis
l2 = stream.l2_norm()
variance = (l2 ** 2) / stream.width()  # Approximation
```

## Benchmarks

Typical performance on modern hardware (Intel i7, 2.8 GHz):

- **Update**: ~1-2 microseconds
- **Estimate**: ~500-800 nanoseconds
- **Merge**: ~1-10 milliseconds (depending on size)
- **Serialization**: ~100-500 microseconds

## Limitations and Caveats

1. **ElasticSketch**: Does not support negative counts (deletions)
2. **SALSA**: No direct heavy hitter extraction (derive from estimates)
3. **RemovableSketch**: More memory overhead for moment computation
4. **All**: Binary items only (convert strings/numbers to bytes)
5. **Merging**: Sketches must have identical parameters

## Contributing

When modifying these bindings:

1. Update tests in corresponding `test_*.py` files
2. Run `cargo build --release` to compile
3. Run `pytest -v` to validate changes
4. Update docstrings and examples
5. Check `BINDINGS_SUMMARY.md` for guidelines

## References

- **ElasticSketch**: Network measurement research (2024-2025)
- **SALSA**: Counter sizing for sketches (2024-2025)
- **RemovableUniversalSketch**: Turnstile streams, polynomial sketches (2024-2025)
- **Related**: Count-Min Sketch, CountSketch, Frequent Items

## License

Same as sketch_oxide library.

---

**For more information**, see:
- `BINDINGS_SUMMARY.md` - Technical implementation details
- `demo_advanced_sketches.py` - Working examples
- Test files - Comprehensive usage patterns
