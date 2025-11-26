using BenchmarkDotNet.Attributes;
using SketchOxide.Frequency;
using SketchOxide.Reconciliation;
using SketchOxide.RangeFilters;
using SketchOxide.Streaming;
using System;
using System.Linq;

namespace SketchOxide.Benchmarks;

/// <summary>
/// Benchmarks for 5 new Tier 1 sketch algorithms.
///
/// Run with: dotnet run -c Release --project SketchOxide.Benchmarks --filter *NewSketches*
/// Or:       dotnet run --project SketchOxide.Benchmarks -- --job short --filter *NewSketches*
/// </summary>
[SimpleJob(warmupCount: 5, targetCount: 10)]
[MemoryDiagnoser]
[PlainExporter]
public class NewSketchesBenchmarks
{
    // ============================================================================
    // HeavyKeeper Benchmarks
    // ============================================================================

    private HeavyKeeper _heavyKeeper = null!;
    private byte[] _testData = new byte[64];
    private Random _random = null!;

    [GlobalSetup]
    public void Setup()
    {
        _random = new Random(12345);
        _random.NextBytes(_testData);
        _heavyKeeper = new HeavyKeeper(100, 0.001, 0.01);

        // Pre-populate with some data
        for (int i = 0; i < 1000; i++)
        {
            _heavyKeeper.Update($"item_{i % 50}");
        }
    }

    [Benchmark(Description = "HeavyKeeper - Update")]
    public void HeavyKeeper_Update()
    {
        _heavyKeeper.Update(_testData);
    }

    [Benchmark(Description = "HeavyKeeper - Estimate")]
    public uint HeavyKeeper_Estimate()
    {
        return _heavyKeeper.Estimate(_testData);
    }

    [Benchmark(Description = "HeavyKeeper - TopK")]
    public (ulong, uint)[] HeavyKeeper_TopK()
    {
        return _heavyKeeper.TopK();
    }

    [Benchmark(Description = "HeavyKeeper - Decay")]
    public void HeavyKeeper_Decay()
    {
        _heavyKeeper.Decay();
    }

    [Benchmark(Description = "HeavyKeeper - Bulk Update (100)")]
    public void HeavyKeeper_BulkUpdate()
    {
        for (int i = 0; i < 100; i++)
        {
            _random.NextBytes(_testData);
            _heavyKeeper.Update(_testData);
        }
    }

    // ============================================================================
    // RatelessIBLT Benchmarks
    // ============================================================================

    private RatelessIBLT _iblt1 = null!;
    private RatelessIBLT _iblt2 = null!;

    [IterationSetup(Target = nameof(RatelessIBLT_Insert))]
    public void SetupIBLT()
    {
        _iblt1 = new RatelessIBLT(100, 32);
        _iblt2 = new RatelessIBLT(100, 32);
    }

    [IterationCleanup(Target = nameof(RatelessIBLT_Insert))]
    public void CleanupIBLT()
    {
        _iblt1?.Dispose();
        _iblt2?.Dispose();
    }

    [Benchmark(Description = "RatelessIBLT - Insert")]
    public void RatelessIBLT_Insert()
    {
        _iblt1.Insert("test_key", "test_value");
    }

    [Benchmark(Description = "RatelessIBLT - Delete")]
    public void RatelessIBLT_Delete()
    {
        _iblt1.Insert("test_key", "test_value");
        _iblt1.Delete("test_key", "test_value");
    }

    [Benchmark(Description = "RatelessIBLT - Subtract")]
    public void RatelessIBLT_Subtract()
    {
        using var iblt1 = new RatelessIBLT(100, 32);
        using var iblt2 = new RatelessIBLT(100, 32);

        iblt1.Insert("key1", "value1");
        iblt2.Insert("key2", "value2");

        iblt1.Subtract(iblt2);
    }

    [Benchmark(Description = "RatelessIBLT - Bulk Operations (100)")]
    public void RatelessIBLT_BulkOperations()
    {
        using var iblt = new RatelessIBLT(200, 32);

        for (int i = 0; i < 100; i++)
        {
            iblt.Insert($"key_{i}", $"value_{i}");
        }
    }

    // ============================================================================
    // Grafite Benchmarks
    // ============================================================================

    private Grafite _grafite = null!;
    private ulong[] _grafiteKeys = null!;

    [GlobalSetup(Target = nameof(Grafite_MayContainRange))]
    public void SetupGrafite()
    {
        _grafiteKeys = Enumerable.Range(0, 1000).Select(i => (ulong)(i * 10)).ToArray();
        _grafite = new Grafite(_grafiteKeys, 6);
    }

    [Benchmark(Description = "Grafite - Build Filter (1000 keys)")]
    public Grafite Grafite_Build()
    {
        var keys = Enumerable.Range(0, 1000).Select(i => (ulong)(i * 10)).ToArray();
        return new Grafite(keys, 6);
    }

    [Benchmark(Description = "Grafite - MayContainRange")]
    public bool Grafite_MayContainRange()
    {
        return _grafite.MayContainRange(100, 200);
    }

    [Benchmark(Description = "Grafite - MayContain (Point Query)")]
    public bool Grafite_MayContain()
    {
        return _grafite.MayContain(500);
    }

    [Benchmark(Description = "Grafite - ExpectedFpr")]
    public double Grafite_ExpectedFpr()
    {
        return _grafite.ExpectedFpr(100);
    }

    [Benchmark(Description = "Grafite - Multiple Range Queries (100)")]
    public int Grafite_MultipleQueries()
    {
        int count = 0;
        for (int i = 0; i < 100; i++)
        {
            if (_grafite.MayContainRange((ulong)(i * 10), (ulong)(i * 10 + 50)))
            {
                count++;
            }
        }
        return count;
    }

    // ============================================================================
    // MementoFilter Benchmarks
    // ============================================================================

    private MementoFilter _memento = null!;

    [GlobalSetup(Target = nameof(MementoFilter_Insert))]
    public void SetupMemento()
    {
        _memento = new MementoFilter(1000, 0.01);
    }

    [Benchmark(Description = "MementoFilter - Insert")]
    public void MementoFilter_Insert()
    {
        _memento.Insert((ulong)_random.Next(0, 10000), "test_value");
    }

    [Benchmark(Description = "MementoFilter - MayContainRange")]
    public bool MementoFilter_MayContainRange()
    {
        return _memento.MayContainRange(100, 200);
    }

    [Benchmark(Description = "MementoFilter - Bulk Insert (100)")]
    public void MementoFilter_BulkInsert()
    {
        using var filter = new MementoFilter(1000, 0.01);

        for (int i = 0; i < 100; i++)
        {
            filter.Insert((ulong)i, $"value_{i}");
        }
    }

    [Benchmark(Description = "MementoFilter - Dynamic Expansion")]
    public void MementoFilter_DynamicExpansion()
    {
        using var filter = new MementoFilter(1000, 0.01);

        // Insert keys with increasing gaps to trigger expansion
        filter.Insert(10, "value1");
        filter.Insert(1000, "value2");
        filter.Insert(10000, "value3");
        filter.Insert(100000, "value4");
    }

    // ============================================================================
    // SlidingHyperLogLog Benchmarks
    // ============================================================================

    private SlidingHyperLogLog _slidingHll = null!;
    private ulong _currentTimestamp = 10000;

    [GlobalSetup(Target = nameof(SlidingHLL_Update))]
    public void SetupSlidingHLL()
    {
        _slidingHll = new SlidingHyperLogLog(12, 3600);

        // Pre-populate with data
        for (ulong i = 0; i < 1000; i++)
        {
            _slidingHll.Update($"item_{i}", _currentTimestamp + i);
        }
    }

    [Benchmark(Description = "SlidingHLL - Update")]
    public void SlidingHLL_Update()
    {
        _slidingHll.Update(_testData, _currentTimestamp);
    }

    [Benchmark(Description = "SlidingHLL - EstimateWindow")]
    public double SlidingHLL_EstimateWindow()
    {
        return _slidingHll.EstimateWindow(_currentTimestamp, 600);
    }

    [Benchmark(Description = "SlidingHLL - EstimateTotal")]
    public double SlidingHLL_EstimateTotal()
    {
        return _slidingHll.EstimateTotal();
    }

    [Benchmark(Description = "SlidingHLL - Decay")]
    public void SlidingHLL_Decay()
    {
        _slidingHll.Decay(_currentTimestamp, 600);
    }

    [Benchmark(Description = "SlidingHLL - Bulk Update (100)")]
    public void SlidingHLL_BulkUpdate()
    {
        for (int i = 0; i < 100; i++)
        {
            _random.NextBytes(_testData);
            _slidingHll.Update(_testData, _currentTimestamp + (ulong)i);
        }
    }

    [Benchmark(Description = "SlidingHLL - Window Query Pattern")]
    public double SlidingHLL_WindowQueryPattern()
    {
        // Simulate real-world pattern: update then query
        _slidingHll.Update("new_item", _currentTimestamp);
        return _slidingHll.EstimateWindow(_currentTimestamp, 300);
    }
}

/// <summary>
/// Comprehensive accuracy and stress test benchmarks.
/// These tests focus on algorithm correctness under various conditions.
/// </summary>
[SimpleJob(warmupCount: 3, targetCount: 5)]
[MemoryDiagnoser]
public class NewSketchesAccuracyBenchmarks
{
    private Random _random = null!;

    [GlobalSetup]
    public void Setup()
    {
        _random = new Random(12345);
    }

    [Benchmark(Description = "HeavyKeeper - Top-K Accuracy (10K items)")]
    public int HeavyKeeper_AccuracyTest()
    {
        using var hk = new HeavyKeeper(10, 0.001, 0.01);

        // Create Zipfian distribution
        for (int i = 0; i < 10000; i++)
        {
            int itemId = i % 100;
            hk.Update($"item_{itemId}");
        }

        var topK = hk.TopK();
        return topK.Length;
    }

    [Benchmark(Description = "SlidingHLL - Cardinality Accuracy (10K items)")]
    public double SlidingHLL_AccuracyTest()
    {
        using var hll = new SlidingHyperLogLog(14, 10000);

        // Add 10,000 unique items
        for (int i = 0; i < 10000; i++)
        {
            hll.Update($"item_{i}", 1000 + (ulong)i);
        }

        double estimate = hll.EstimateTotal();
        double error = Math.Abs(estimate - 10000.0) / 10000.0;
        return error;
    }

    [Benchmark(Description = "Grafite - Range Query FPR Test")]
    public double Grafite_FprTest()
    {
        var keys = Enumerable.Range(0, 1000).Select(i => (ulong)(i * 100)).ToArray();
        using var filter = new Grafite(keys, 6);

        // Test FPR for various range widths
        double sumFpr = 0;
        for (int i = 0; i < 10; i++)
        {
            sumFpr += filter.ExpectedFpr((ulong)(i * 10));
        }
        return sumFpr / 10;
    }

    [Benchmark(Description = "MementoFilter - Dynamic Insert Performance")]
    public int MementoFilter_StressTest()
    {
        using var filter = new MementoFilter(5000, 0.01);

        // Insert 5000 random keys
        int inserted = 0;
        for (int i = 0; i < 5000; i++)
        {
            filter.Insert((ulong)_random.Next(0, 100000), $"value_{i}");
            inserted++;
        }
        return inserted;
    }

    [Benchmark(Description = "RatelessIBLT - Set Reconciliation (100 items)")]
    public void RatelessIBLT_ReconciliationTest()
    {
        using var alice = new RatelessIBLT(150, 32);
        using var bob = new RatelessIBLT(150, 32);

        // Shared items (50)
        for (int i = 0; i < 50; i++)
        {
            alice.Insert($"shared_{i}", $"value_{i}");
            bob.Insert($"shared_{i}", $"value_{i}");
        }

        // Alice-only (25)
        for (int i = 0; i < 25; i++)
        {
            alice.Insert($"alice_{i}", $"value_{i}");
        }

        // Bob-only (25)
        for (int i = 0; i < 25; i++)
        {
            bob.Insert($"bob_{i}", $"value_{i}");
        }

        // Compute difference
        alice.Subtract(bob);
    }
}
