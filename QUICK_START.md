# Quick Start - PyO3 Python Bindings for Advanced Frequency Sketches

## What's New

Three brand new frequency sketch implementations are now available in Python:

1. **ElasticSketch** - Adaptive frequency estimation with elastic counters
2. **SALSA** - Self-adjusting counter sizing for skewed distributions
3. **RemovableUniversalSketch** - Turnstile-enabled sketches supporting insertions and deletions

## Installation

The sketches are part of the sketch_oxide package:

```bash
cd python
pip install -e .
```

## Basic Usage

### ElasticSketch - Find Heavy Flows

```python
from sketch_oxide import ElasticSketch

# Create sketch (512 buckets, 3 hash functions)
sketch = ElasticSketch(512, 3)

# Track network flows
sketch.update(b"flow_id_1", 1000)
sketch.update(b"flow_id_2", 500)
sketch.update(b"flow_id_3", 100)

# Check frequency
print(sketch.estimate(b"flow_id_1"))  # >= 1000

# Find all heavy hitters above threshold
heavy = sketch.heavy_hitters(threshold=300)
for item_hash, freq in heavy:
    print(f"Heavy flow: {freq} packets")
```

### SALSA - Handle Skewed Data

```python
from sketch_oxide import SALSA

# Create sketch with accuracy parameters
salsa = SALSA(epsilon=0.01, delta=0.01)

# Add items - SALSA handles Zipfian distributions well
salsa.update(b"popular_page", 10000)
salsa.update(b"medium_page", 1000)
salsa.update(b"rare_page", 10)

# Get estimate with confidence metric
estimate, confidence = salsa.estimate(b"popular_page")
print(f"Estimate: {estimate}, Confidence: {confidence}%")

# Merge sketches from different servers
other_salsa = SALSA(0.01, 0.01)
salsa.merge(other_salsa)
```

### RemovableUniversalSketch - Track Deletions

```python
from sketch_oxide import RemovableUniversalSketch

# Create sketch
rus = RemovableUniversalSketch(epsilon=0.01, delta=0.01)

# Track cache pages
rus.update(b"page_a", 1000)  # 1000 views
rus.update(b"page_b", 500)   # 500 views

# Remove some (cache eviction)
rus.update(b"page_a", -200)  # Evict 200 entries

# Check current state
print(rus.estimate(b"page_a"))  # >= 800

# Compute L2 norm (frequency moment)
l2 = rus.l2_norm()
print(f"L2 norm: {l2}")
```

## Key Methods

### ElasticSketch
- `new(bucket_count, depth)` - Create sketch
- `update(item, count)` - Add frequency
- `estimate(item)` - Query frequency
- `heavy_hitters(threshold)` - Find top items
- `merge(other)` - Combine sketches
- `serialize()` - Save to bytes

### SALSA
- `new(epsilon, delta)` - Create with accuracy params
- `update(item, count)` - Add frequency
- `estimate(item)` - Returns (estimate, confidence)
- `merge(other)` - Combine sketches
- `adaptation_level()` - Check adaptations

### RemovableUniversalSketch
- `new(epsilon, delta)` - Create sketch
- `update(item, delta)` - Add/remove (supports negative)
- `estimate(item)` - Query (can be negative)
- `l2_norm()` - Frequency moment
- `merge(other)` - Combine sketches

## Real-World Examples

### Network Traffic Monitoring

```python
traffic = ElasticSketch(1024, 4)

# Simulate traffic
flows = {
    b"tcp_80": 5000,
    b"tcp_443": 3000,
    b"udp_53": 10000,
}

for flow, packets in flows.items():
    traffic.update(flow, packets)

# Find DDoS sources
ddos = traffic.heavy_hitters(threshold=8000)
print(f"Potential DDoS sources: {len(ddos)}")
```

### Web Request Tracking

```python
requests = SALSA(epsilon=0.01, delta=0.01)

# Track URLs with Zipfian distribution
urls = [
    (b"/index.html", 5000),
    (b"/api/users", 500),
    (b"/admin/panel", 50),
]

for url, count in urls:
    requests.update(url, count)

# Check with confidence
est, conf = requests.estimate(b"/index.html")
print(f"Homepage: {est} requests ({conf}% confidence)")
```

### Cache Management

```python
cache = RemovableUniversalSketch(0.01, 0.01)

# Track page views
cache.update(b"home", 1000)
cache.update(b"product", 500)

# Evictions
cache.update(b"home", -100)
cache.update(b"product", -50)

# Check cache "energy"
print(f"Cache total: {cache.l2_norm():.0f}")
```

## Testing

Run comprehensive tests:

```bash
cd python

# Test all sketches
pytest test_elastic_sketch.py test_salsa.py test_removable_sketch.py -v

# Run demo
python demo_advanced_sketches.py
```

## Parameters Guide

### Epsilon (ε) - Accuracy
- `0.001` - Very high accuracy, more memory
- `0.01` - Balanced (recommended)
- `0.1` - Low accuracy, less memory

### Delta (δ) - Confidence
- `0.001` - 99.9% confidence (very safe)
- `0.01` - 99% confidence (recommended)
- `0.1` - 90% confidence (risky)

## Common Patterns

### Merging from Multiple Sources

```python
# Collect from different servers
sketches = [ElasticSketch(512, 3) for _ in range(3)]

# Process data in parallel...
sketches[0].update(b"flow_a", 100)
sketches[1].update(b"flow_a", 50)
sketches[2].update(b"flow_a", 25)

# Merge results
for i in range(1, len(sketches)):
    sketches[0].merge(sketches[i])

print(sketches[0].estimate(b"flow_a"))  # >= 175
```

### Serialization

```python
sketch = ElasticSketch(512, 3)
sketch.update(b"item", 100)

# Save
data = sketch.serialize()
with open("sketch.bin", "wb") as f:
    f.write(data)

# Restore
with open("sketch.bin", "rb") as f:
    data = f.read()
restored = ElasticSketch.deserialize(data)
```

## Error Handling

```python
try:
    sketch = ElasticSketch(0, 3)  # Invalid
except ValueError as e:
    print(f"Error: {e}")

try:
    sketch1 = ElasticSketch(512, 3)
    sketch2 = ElasticSketch(256, 3)
    sketch1.merge(sketch2)  # Incompatible
except ValueError as e:
    print(f"Merge error: {e}")
```

## Documentation

- **BINDINGS_SUMMARY.md** - Technical details
- **ADVANCED_SKETCHES_README.md** - Full API reference
- **demo_advanced_sketches.py** - Working examples
- **test_*.py** - Comprehensive test cases

## Performance

Typical performance on modern hardware:

- **Update**: ~1-2 microseconds
- **Query**: ~500-800 nanoseconds
- **Merge**: ~1-10 milliseconds
- **Serialize**: ~100-500 microseconds

## Next Steps

1. Try the demo: `python demo_advanced_sketches.py`
2. Run the tests: `pytest test_*.py -v`
3. Check ADVANCED_SKETCHES_README.md for full API
4. See BINDINGS_SUMMARY.md for technical details

## Support

For issues or questions:
1. Check test files for usage examples
2. Review documentation files
3. Run demo to see working implementations
4. Check error messages for invalid parameters

---

**Enjoy using the advanced frequency sketches!**
