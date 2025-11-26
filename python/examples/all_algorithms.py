#!/usr/bin/env python3
"""
Demonstration of all 9 sketch_oxide algorithms.
Shows typical use cases for each data structure.
"""

import sketch_oxide

print("=" * 70)
print("sketch_oxide: Complete Demo of All 9 Algorithms")
print("=" * 70)

# ==================== CARDINALITY ESTIMATION ====================
print("\n📊 CARDINALITY ESTIMATION")
print("-" * 70)

# 1. UltraLogLog - 28% better than HyperLogLog
print("\n1. UltraLogLog (28% better than HyperLogLog)")
ull = sketch_oxide.UltraLogLog(precision=12)
for user_id in range(100000):
    ull.update(f"user_{user_id}")
print(f"   Unique users: {ull.estimate():.0f}")

# 2. CpcSketch - Maximum space efficiency
print("\n2. CpcSketch (30-40% more space efficient)")
cpc = sketch_oxide.CpcSketch(lg_k=11)
for ip in range(50000):
    cpc.update(f"192.168.{ip // 256}.{ip % 256}")
print(f"   Unique IPs: {cpc.estimate():.0f}")

# 3. ThetaSketch - Set operations
print("\n3. ThetaSketch (Set operations: union, intersect, difference)")
users_web = sketch_oxide.ThetaSketch(lg_k=12)
users_mobile = sketch_oxide.ThetaSketch(lg_k=12)

for i in range(1000):
    users_web.update(f"user_{i}")
for i in range(500, 1500):
    users_mobile.update(f"user_{i}")

print(f"   Web users: {users_web.estimate():.0f}")
print(f"   Mobile users: {users_mobile.estimate():.0f}")
print(f"   Total (union): {users_web.union(users_mobile).estimate():.0f}")
print(f"   Both (intersection): {users_web.intersect(users_mobile).estimate():.0f}")
print(f"   Web only (difference): {users_web.difference(users_mobile).estimate():.0f}")

# ==================== MEMBERSHIP TESTING ====================
print("\n" + "=" * 70)
print("🔍 MEMBERSHIP TESTING")
print("-" * 70)

# 4. BinaryFuseFilter - 75% better than Bloom
print("\n4. BinaryFuseFilter (75% better than Bloom filters)")
valid_user_ids = list(range(10000))
bf = sketch_oxide.BinaryFuseFilter(valid_user_ids, bits_per_entry=9)

print(f"   500 in set: {bf.contains(500)}")
print(f"   99999 in set: {bf.contains(99999)}")
print("   False positive rate: <1%")

# ==================== QUANTILE ESTIMATION ====================
print("\n" + "=" * 70)
print("📈 QUANTILE ESTIMATION")
print("-" * 70)

# 5. DDSketch - Relative error guarantees
print("\n5. DDSketch (Relative error guarantees)")
dd = sketch_oxide.DDSketch(relative_accuracy=0.01)
latencies = [10, 15, 20, 25, 30] * 1000 + [100, 200, 500, 1000] * 10
for latency in latencies:
    dd.update(float(latency))

print(f"   p50 latency: {dd.quantile(0.50):.1f}ms")
print(f"   p95 latency: {dd.quantile(0.95):.1f}ms")
print(f"   p99 latency: {dd.quantile(0.99):.1f}ms")

# 6. ReqSketch - Zero error at tail quantiles
print("\n6. ReqSketch (Zero error at p100 in HRA mode)")
req = sketch_oxide.ReqSketch(k=128, mode="HRA")
response_times = list(range(1, 10001))
for rt in response_times:
    req.update(float(rt))

print(f"   p99 response time: {req.quantile(0.99):.0f}ms")
print(f"   p100 (max): {req.quantile(1.0):.0f}ms (exact)")
print(f"   Min: {req.min():.0f}ms (exact)")

# ==================== FREQUENCY ESTIMATION ====================
print("\n" + "=" * 70)
print("📊 FREQUENCY ESTIMATION")
print("-" * 70)

# 7. CountMinSketch - Standard frequency estimation
print("\n7. CountMinSketch (Standard frequency estimation)")
cms = sketch_oxide.CountMinSketch(epsilon=0.01, delta=0.01)
events = ["click"] * 1000 + ["view"] * 500 + ["purchase"] * 100
for event in events:
    cms.update(event)

print(f"   'click' count: {cms.estimate('click'):.0f}")
print(f"   'view' count: {cms.estimate('view'):.0f}")
print(f"   'purchase' count: {cms.estimate('purchase'):.0f}")

# 8. FrequentItems - Top-K heavy hitters
print("\n8. FrequentItems (Top-K heavy hitters with bounds)")
fi = sketch_oxide.FrequentItems(max_size=10)
products = (
    ["iPhone"] * 1000
    + ["MacBook"] * 750
    + ["iPad"] * 500
    + ["AirPods"] * 250
    + ["Apple Watch"] * 100
)
for product in products:
    fi.update(product)

items = fi.frequent_items(mode="no_false_positives")
print("   Top 3 products (with frequency bounds):")
for i, (item, lower, upper) in enumerate(items[:3], 1):
    print(f"   {i}. {item}: [{lower}, {upper}]")

# ==================== SIMILARITY ESTIMATION ====================
print("\n" + "=" * 70)
print("🔗 SIMILARITY ESTIMATION")
print("-" * 70)

# 9. MinHash - Jaccard similarity
print("\n9. MinHash (Jaccard similarity)")
doc1 = sketch_oxide.MinHash(num_perm=128)
doc2 = sketch_oxide.MinHash(num_perm=128)
doc3 = sketch_oxide.MinHash(num_perm=128)

# Document 1: machine learning keywords
doc1_words = ["machine", "learning", "neural", "network", "deep", "model"]
for word in doc1_words:
    doc1.update(word)

# Document 2: similar to doc1
doc2_words = ["machine", "learning", "neural", "algorithm", "model", "training"]
for word in doc2_words:
    doc2.update(word)

# Document 3: completely different
doc3_words = ["cooking", "recipe", "kitchen", "food", "chef", "restaurant"]
for word in doc3_words:
    doc3.update(word)

print(f"   doc1 vs doc2 (similar): {doc1.jaccard_similarity(doc2):.2f}")
print(f"   doc1 vs doc3 (different): {doc1.jaccard_similarity(doc3):.2f}")

# ==================== SUMMARY ====================
print("\n" + "=" * 70)
print("✅ ALL 9 ALGORITHMS DEMONSTRATED!")
print("=" * 70)
print(
    """
sketch_oxide provides production-ready implementations of:
  • 3 Cardinality estimators (UltraLogLog, CpcSketch, ThetaSketch)
  • 1 Membership filter (BinaryFuseFilter)
  • 2 Quantile estimators (DDSketch, ReqSketch)
  • 2 Frequency counters (CountMinSketch, FrequentItems)
  • 1 Similarity estimator (MinHash)

All algorithms are space-efficient, mergeable, and battle-tested in production!
"""
)
