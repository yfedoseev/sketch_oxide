# Integration Patterns & Real-World Examples

Practical examples showing how to use sketch_oxide with popular data tools and libraries.

## Table of Contents
- [DuckDB Integration](#duckdb-integration)
- [Polars/Pandas Integration](#polarspandas-integration)
- [Streaming Aggregation](#streaming-aggregation)
- [Web Analytics](#web-analytics)
- [Database Integration](#database-integration)

---

## DuckDB Integration

### Scenario: Distinct User Count Per Day

Track distinct user IDs across millions of events, using HyperLogLog for efficient storage.

```python
import duckdb
from sketch_oxide import HyperLogLog
import json

# Create table with daily event logs
con = duckdb.connect()
con.execute("""
    CREATE TABLE events AS
    SELECT
        DATE(timestamp) as event_date,
        user_id,
        event_type
    FROM read_csv('events.csv')
""")

# Compute distinct counts per day using sketches
daily_distinct = {}
for row in con.execute("SELECT DISTINCT event_date FROM events"):
    event_date = row[0]
    hll = HyperLogLog(14)  # 14 bits precision = ±0.4% accuracy

    for user_id in con.execute(
        "SELECT user_id FROM events WHERE event_date = ?",
        [event_date]
    ):
        hll.update(str(user_id[0]).encode())

    daily_distinct[str(event_date)] = hll.estimate()

print(json.dumps(daily_distinct, indent=2))
```

### Scenario: Top-K Most Frequent Queries

Identify the 100 most frequent SQL queries using SpaceSaving.

```python
from sketch_oxide import SpaceSaving

# Stream queries from log file
ss = SpaceSaving(k=100)  # Track top 100

with open('query_log.txt') as f:
    for query in f:
        ss.update(query.strip().encode())

# Get top-K with frequencies
top_queries = ss.top_k()
for query, count in top_queries:
    print(f"{count:10d} - {query}")
```

---

## Polars/Pandas Integration

### Scenario: Approximate Quantiles for Time-Series

Calculate percentiles efficiently without materializing the entire distribution.

```python
import polars as pl
from sketch_oxide import DDSketch
import pandas as pd

# Read time-series data
df = pl.read_csv('metrics.csv')

# Group by hour and compute quantiles
quantiles = {}
for hour, group_df in df.group_by('hour'):
    sketch = DDSketch(relative_accuracy=0.01)  # 1% accuracy

    for value in group_df['metric_value']:
        sketch.add(float(value))

    quantiles[int(hour[0])] = {
        'p50': sketch.quantile(0.50),
        'p95': sketch.quantile(0.95),
        'p99': sketch.quantile(0.99),
        'count': sketch.count()
    }

# Convert to DataFrame for analysis
result_df = pd.DataFrame.from_dict(quantiles, orient='index')
print(result_df)
```

### Scenario: Detect Duplicate Records in Large Dataset

Find potential duplicates without comparing all pairs (O(n²) → O(1) per item).

```python
import polars as pl
from sketch_oxide import SimHash
from collections import defaultdict

df = pl.read_csv('large_dataset.csv')

# Build SimHash fingerprints
simhash_map = defaultdict(list)
for row in df.iter_rows(named=True):
    text = f"{row['name']}{row['email']}{row['phone']}"
    sh = SimHash()
    sh.update(text.encode())

    fingerprint = sh.fingerprint()
    simhash_map[fingerprint].append(row)

# Find potential duplicates (same fingerprint = highly similar)
duplicates = {k: v for k, v in simhash_map.items() if len(v) > 1}
print(f"Found {len(duplicates)} potential duplicate groups")
for fingerprint, records in list(duplicates.items())[:5]:
    print(f"Fingerprint {fingerprint}: {len(records)} similar records")
```

---

## Streaming Aggregation

### Scenario: Real-Time Traffic Analysis

Monitor network traffic with sub-microsecond latency using HeavyKeeper.

```python
from sketch_oxide import HeavyKeeper
import socket

# Initialize heavy keeper for top flows
hk = HeavyKeeper(epsilon=0.01, gamma=0.01)

# Process packets in real-time
sock = socket.socket(socket.AF_PACKET, socket.SOCK_RAW)

def process_packets(count=10000):
    for _ in range(count):
        packet = sock.recvfrom(65535)[0]

        # Extract source IP as key
        src_ip = packet[26:30]

        # Update with bytes transferred (weight)
        hk.insert(src_ip, len(packet))

process_packets()

# Get top talkers (heavy hitters)
print("Top IP addresses by traffic:")
for ip in [b'\xc0\xa8\x00\x01', b'\xc0\xa8\x00\x02']:  # Example IPs
    count = hk.query(ip)
    print(f"  {ip}: {count} bytes")
```

### Scenario: Adaptive Sampling with NitroSketch

Reduce computation burden in high-speed streams.

```python
from sketch_oxide import NitroSketch

# Create sketch with 10% sampling
nitro = NitroSketch(epsilon=0.01, delta=0.001, sample_rate=0.1)

# Update with sampled items
for event in event_stream:
    nitro.update_sampled(event.key.encode())

# Synchronize periodically for accurate estimates
if event_count % 10000 == 0:
    nitro.sync(unsample_weight=10.0)  # Adjust for unsampled items

# Query with automatic adjustment
estimate = nitro.query(b'some_key')
print(f"Estimated frequency: {estimate}")
```

---

## Web Analytics

### Scenario: Cookie-Less User Tracking

Estimate unique visitors without storing cookies or complete user lists.

```python
from sketch_oxide import BloomFilter, HyperLogLog
from datetime import date
import redis

redis_client = redis.Redis(decode_responses=True)

def track_visitor(ip_address, user_agent):
    """Track anonymous visitor using HyperLogLog for cardinality"""
    today = str(date.today())

    # Create unique identifier from IP + User-Agent
    visitor_id = f"{ip_address}|{user_agent}"

    # Add to today's HyperLogLog
    hll_key = f"unique_visitors:{today}"
    hll = redis_client.get(hll_key) or HyperLogLog(12)
    hll.update(visitor_id.encode())
    redis_client.set(hll_key, hll, ex=86400*7)  # Keep 7 days

    return hll.estimate()

def block_malicious_patterns(pattern_list):
    """Use BloomFilter to detect known malicious patterns"""
    bf = BloomFilter(n=1000000, fpr=0.001)

    for pattern in pattern_list:
        bf.insert(pattern.encode())

    return bf

# Usage
unique_visitors = track_visitor('192.168.1.100', 'Mozilla/5.0...')
print(f"Unique visitors today: {unique_visitors}")
```

### Scenario: A/B Testing with Reservoir Sampling

Uniformly sample user requests for A/B testing without bias.

```python
from sketch_oxide import ReservoirSampling
import random

# Sample 1000 users uniformly from traffic stream
reservoir = ReservoirSampling(size=1000)

def should_test_variant(user_id):
    """Decide if user should be in experiment"""
    # Hash user ID for deterministic variant assignment
    hash_val = hash(user_id) % 100

    # Add to reservoir for sampling statistics
    reservoir.update(int(user_id))

    return hash_val < 50  # 50% get variant A, 50% get variant B

# Track variant assignment
variant_assignments = defaultdict(list)
for user_id in range(100000):
    variant = 'A' if should_test_variant(user_id) else 'B'
    variant_assignments[variant].append(user_id)

print(f"Sample count: {reservoir.count()}")
print(f"Sample size: {reservoir.len()}")
```

---

## Database Integration

### Scenario: Index Bloom Filters for Negative Lookups

Speed up "does NOT exist" queries using Bloom filters.

```python
from sketch_oxide import BloomFilter
import sqlite3

# Create index of known IDs to avoid disk lookups
db = sqlite3.connect('products.db')
cursor = db.cursor()

# Build Bloom filter from product IDs
bf = BloomFilter(n=100000, fpr=0.001)

for (product_id,) in cursor.execute("SELECT id FROM products"):
    bf.insert(str(product_id).encode())

# Fast negative check
def exists_in_db(product_id):
    """Returns False if definitely not in DB, True if might be"""
    if not bf.contains(str(product_id).encode()):
        return False  # Definitely not in DB

    # Only query disk for possible candidates
    cursor.execute("SELECT 1 FROM products WHERE id = ?", (product_id,))
    return cursor.fetchone() is not None

# This avoids disk lookups for ~99.9% of non-existent products
```

### Scenario: Range Query Acceleration

Speed up range queries using range filters.

```python
from sketch_oxide import Grafite

# Build range filter for user IDs 1-1000000
user_ids = list(range(1, 1000001))
grf = Grafite.build(user_ids, bits_per_key=8)

def fast_range_query(min_id, max_id):
    """Check if range is likely in dataset"""
    if not grf.may_contain_range(min_id, max_id):
        return []  # No results possible

    # Only query database if range might exist
    cursor.execute(
        "SELECT * FROM users WHERE id BETWEEN ? AND ?",
        (min_id, max_id)
    )
    return cursor.fetchall()

results = fast_range_query(500000, 600000)
```

---

## Performance Comparison Examples

### Choosing Between Frequency Algorithms

```python
from sketch_oxide import CountMinSketch, SpaceSaving, HeavyKeeper
import time

# Measure space vs accuracy tradeoff
stream = ['product_' + str(i % 1000) for i in range(1000000)]

# CountMinSketch: Good error bounds
cms = CountMinSketch(epsilon=0.01, delta=0.001)
start = time.time()
for item in stream:
    cms.update(item.encode())
print(f"CountMinSketch: {time.time() - start:.3f}s")

# SpaceSaving: Exact top-K (if k is right)
ss = SpaceSaving(k=1000)
start = time.time()
for item in stream:
    ss.update(item.encode())
print(f"SpaceSaving: {time.time() - start:.3f}s")

# HeavyKeeper: Fastest (modern algorithm)
hk = HeavyKeeper(epsilon=0.01, gamma=0.01)
start = time.time()
for item in stream:
    hk.insert(item.encode(), 1)
print(f"HeavyKeeper: {time.time() - start:.3f}s")
```

---

## Multi-Language Examples

### Rust + Python Pipeline

```rust
// Rust producer: Fast sketch creation
use sketch_oxide::HyperLogLog;

#[derive(Serialize)]
pub struct SketchData {
    hll_bytes: Vec<u8>,
}

pub fn create_daily_sketch() -> SketchData {
    let mut hll = HyperLogLog::new(14).unwrap();
    // ... populate with data ...
    SketchData {
        hll_bytes: hll.to_bytes().unwrap()
    }
}
```

```python
# Python consumer: Deserialize and analyze
from sketch_oxide import HyperLogLog
import json

sketch_data = json.load(open('sketch.json'))
hll = HyperLogLog.from_bytes(bytes(sketch_data['hll_bytes']))
print(f"Distinct items: {hll.estimate()}")
```

### Node.js + C# Comparison

```javascript
// Node.js: Web frontend
const SketchOxide = require('sketch-oxide');

const bf = new SketchOxide.BloomFilter(1000000, 0.01);
app.post('/check', (req, res) => {
    const exists = bf.contains(req.body.item);
    res.json({ exists });
});
```

```csharp
// C#: .NET backend
using SketchOxide;

var bf = new BloomFilter(1000000, 0.01);
app.MapPost("/check", (ItemRequest req) => {
    var exists = bf.Contains(req.Item);
    return new { exists };
});
```

---

## Tips & Best Practices

### ✓ Do

- Use HyperLogLog for cardinality (proven, fast, small)
- Use CountMinSketch for frequency with error bounds
- Use BloomFilter for membership (classic, reliable)
- Use DDSketch for quantiles (modern, accurate)
- Store sketches in Redis for distributed systems
- Merge sketches across time windows
- Benchmark with your actual data distribution

### ✗ Don't

- Don't use Bloom filters when you need exact results
- Don't compare sketches of different precisions
- Don't assume LearnedBloomFilter is always better (it's not!)
- Don't skip parameter tuning (default often suboptimal)
- Don't use static sketches when you need updates
- Don't forget to account for memory in large-scale systems
- Don't ignore the accuracy/space trade-off

---

## Troubleshooting Integration

### "Sketch results don't match expectations"
- Verify you're using same algorithm (e.g., HLL precision)
- Check that data is being serialized/deserialized correctly
- Ensure you're comparing compatible sketch types

### "Out of memory in production"
- Reduce sketch precision/capacity parameters
- Use space-efficient algorithms (SpaceSaving vs CountMinSketch)
- Consider distributed sketches (merge smaller ones)

### "Performance below expectations"
- Profile to find actual bottleneck (sketch or I/O?)
- Ensure sketches are initialized once, reused many times
- Batch updates when possible (reduces function call overhead)

---

## See Also
- **ALGORITHM_SELECTION_GUIDE.md** - Choosing the right algorithm
- **PERFORMANCE_GUIDE.md** - Tuning parameters for efficiency
- **ALGORITHMS_COMPLETE.md** - Technical algorithm specifications
- **Language-specific READMEs** - Rust, Python, Node.js, Java, C# specific examples
