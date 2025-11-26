#!/usr/bin/env python3
"""
Demonstration of PyO3 Python bindings for advanced frequency sketches.
Shows practical usage of ElasticSketch, SALSA, and RemovableUniversalSketch.
"""

from sketch_oxide import SALSA, ElasticSketch, RemovableUniversalSketch


def demo_elastic_sketch():
    """Demonstrate ElasticSketch for network traffic monitoring."""
    print("=" * 70)
    print("ELASTIC SKETCH DEMO: Network Traffic Monitoring")
    print("=" * 70)

    sketch = ElasticSketch(bucket_count=512, depth=3)

    # Simulate network flows with varying frequencies
    flows = {
        b"tcp:192.168.1.100:80": 5000,
        b"tcp:192.168.1.101:443": 3000,
        b"tcp:192.168.1.102:22": 500,
        b"udp:8.8.8.8:53": 10000,
        b"icmp:10.0.0.1": 100,
        b"tcp:192.168.1.103:80": 2000,
    }

    print("\nAdding flows to sketch...")
    for flow, packets in flows.items():
        sketch.update(flow, packets)
        print(f"  {flow.decode('utf-8', errors='replace')}: {packets} packets")

    print(f"\nTotal packets processed: {sketch.total_count()}")
    print(f"Sketch parameters: bucket_count={sketch.bucket_count()}, depth={sketch.depth()}")
    print(f"Memory usage: {sketch.memory_usage()} bytes")

    print("\nFrequency estimates:")
    for flow, expected in flows.items():
        estimated = sketch.estimate(flow)
        print(f"  {flow.decode('utf-8', errors='replace')}: {estimated} (expected ~{expected})")

    print("\nHeavy hitters (threshold >= 1000):")
    hitters = sketch.heavy_hitters(threshold=1000)
    for item_hash, freq in hitters:
        print(f"  Hash {item_hash}: {freq} packets")

    print("\nSerialization:")
    data = sketch.serialize()
    print(f"  Serialized size: {len(data)} bytes")
    restored = ElasticSketch.deserialize(data)
    print(f"  Restored estimate for first flow: {restored.estimate(b'tcp:192.168.1.100:80')}")


def demo_salsa():
    """Demonstrate SALSA for skewed frequency distributions."""
    print("\n" + "=" * 70)
    print("SALSA DEMO: Handling Skewed Distributions with Adaptive Sizing")
    print("=" * 70)

    salsa = SALSA(epsilon=0.01, delta=0.01)

    print("\nAdding items with Zipfian distribution...")
    items = {}
    for i in range(1, 21):
        # Zipfian: frequency = 1000/i
        freq = 1000 // i
        key = f"item{i}".encode()
        salsa.update(key, freq)
        items[key] = freq
        if i % 5 == 0:
            print(f"  item{i}: {freq} updates")

    print(f"\nTotal updates: {salsa.total_updates()}")
    print(f"Max observed frequency: {salsa.max_observed()}")
    print(f"Adaptation level: {salsa.adaptation_level()}")

    print("\nEstimates with confidence (first 5 items):")
    for i in range(1, 6):
        key = f"item{i}".encode()
        estimate, confidence = salsa.estimate(key)
        print(f"  item{i}: {estimate} (confidence: {confidence}%, true: {items[key]})")

    print("\nMerging with another SALSA sketch...")
    salsa2 = SALSA(0.01, 0.01)
    for i in range(21, 26):
        freq = 500 // i
        salsa2.update(f"item{i}".encode(), freq)

    print(f"  SALSA2 total updates: {salsa2.total_updates()}")
    salsa.merge(salsa2)
    print(f"  Merged total updates: {salsa.total_updates()}")

    # Verify never-underestimate property
    print("\nVerifying never-underestimate guarantee:")
    underestimated = False
    for key, true_freq in items.items():
        est, _ = salsa.estimate(key)
        if est < true_freq:
            print(f"  WARNING: {key} underestimated ({est} < {true_freq})")
            underestimated = True

    if not underestimated:
        print("  ✓ All estimates >= true frequencies (guarantee satisfied)")


def demo_removable_sketch():
    """Demonstrate RemovableUniversalSketch for turnstile streams."""
    print("\n" + "=" * 70)
    print("REMOVABLE UNIVERSAL SKETCH DEMO: Turnstile Streams & L2 Norm")
    print("=" * 70)

    rus = RemovableUniversalSketch(epsilon=0.01, delta=0.01)

    print("\nSimulating cache traffic with insertions and deletions...")

    # Phase 1: Initial cache population
    print("\nPhase 1: Cache population")
    cache_items = {
        b"home": 1000,
        b"product": 500,
        b"checkout": 200,
        b"admin": 50,
    }

    for item, count in cache_items.items():
        rus.update(item, count)
        print(f"  {item.decode()}: +{count}")

    l2_phase1 = rus.l2_norm()
    print(f"L2 norm after phase 1: {l2_phase1:.1f}")

    # Phase 2: Evictions (spam cleanup)
    print("\nPhase 2: Evictions and cleanup")
    evictions = {
        b"home": -100,
        b"product": -50,
        b"checkout": -20,
    }

    for item, delta in evictions.items():
        rus.update(item, delta)
        print(f"  {item.decode()}: {delta}")

    l2_phase2 = rus.l2_norm()
    print(f"L2 norm after phase 2: {l2_phase2:.1f}")

    # Phase 3: Verification
    print("\nPhase 3: Current state")
    print("Item frequencies after turnstile operations:")
    for item, original in cache_items.items():
        current = rus.estimate(item)
        evicted = -evictions.get(item, 0)
        expected = original - evicted
        print(f"  {item.decode()}: {current} (expected ~{expected})")

    print(f"L2 norm decrease: {l2_phase1:.1f} → {l2_phase2:.1f}")
    assert l2_phase2 <= l2_phase1, "L2 norm should decrease with deletions"
    print("✓ L2 norm decreased with deletions (correct)")

    # Merging distributed caches
    print("\nMerging multiple cache replicas...")
    rus2 = RemovableUniversalSketch(0.01, 0.01)
    rus3 = RemovableUniversalSketch(0.01, 0.01)

    # Replica 2
    for item, count in {b"home": 500, b"product": 250}.items():
        rus2.update(item, count)

    # Replica 3
    for item, count in {b"product": 200, b"checkout": 100}.items():
        rus3.update(item, count)

    print(f"  Replica 2 total: {rus2.l2_norm():.1f}")
    print(f"  Replica 3 total: {rus3.l2_norm():.1f}")

    rus.merge(rus2)
    rus.merge(rus3)

    print(f"  Merged total: {rus.l2_norm():.1f}")
    print("✓ Replicas successfully merged")


def demo_comparison():
    """Show key differences between the three sketches."""
    print("\n" + "=" * 70)
    print("SKETCH COMPARISON")
    print("=" * 70)

    comparison = """
    Feature                 ElasticSketch   SALSA               RemovableSketch
    ─────────────────────────────────────────────────────────────────────────
    Frequency Estimation    Yes             Yes                 Yes
    Deletions               No              No                  Yes (Turnstile)
    Heavy Hitters           Yes (direct)    No (via estimates)  No (via estimates)
    Confidence Metric       No              Yes (confidence)    No
    L2/Frequency Moments    No              No                  Yes (L2 norm)
    Negative Frequencies    No              No                  Yes
    Adaptive Parameters     No              Yes (counters)      No
    Elastic Counters        Yes             No                  No
    Use Cases:
    ─────────────────────────────────────────────────────────────────────────
    - ElasticSketch:         Network traffic, flow monitoring, heavy hitter detection
    - SALSA:                 Skewed distributions, heavy-tailed data, production systems
    - RemovableSketch:       Cache tracking, turnstile streams, moment analysis
    """

    print(comparison)


if __name__ == "__main__":
    demo_elastic_sketch()
    demo_salsa()
    demo_removable_sketch()
    demo_comparison()

    print("\n" + "=" * 70)
    print("All demonstrations completed successfully!")
    print("=" * 70)
