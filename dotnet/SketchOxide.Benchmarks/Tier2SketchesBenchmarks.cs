using BenchmarkDotNet.Attributes;
using BenchmarkDotNet.Columns;
using BenchmarkDotNet.Configs;
using BenchmarkDotNet.Diagnosers;
using SketchOxide.Membership;
using SketchOxide.RangeFilters;
using SketchOxide.Frequency;
using SketchOxide.Universal;
using System;
using System.Linq;

namespace SketchOxide.Benchmarks;

[MemoryDiagnoser]
[Config(typeof(Config))]
public class Tier2SketchesBenchmarks
{
    private class Config : ManualConfig
    {
        public Config()
        {
            AddDiagnoser(MemoryDiagnoser.Default);
            AddColumn(StatisticColumn.Mean);
            AddColumn(StatisticColumn.StdDev);
            AddColumn(StatisticColumn.Median);
        }
    }

    private VacuumFilter _vacuumFilter = null!;
    private GRF _grf = null!;
    private NitroSketch _nitroSketch = null!;
    private UnivMon _univMon = null!;
    private LearnedBloomFilter _learnedBloom = null!;

    [GlobalSetup]
    public void Setup()
    {
        // VacuumFilter
        _vacuumFilter = new VacuumFilter(10000, 0.01);
        for (int i = 0; i < 1000; i++)
        {
            _vacuumFilter.Insert($"key{i}");
        }

        // GRF
        ulong[] keys = Enumerable.Range(0, 10000).Select(i => (ulong)i).ToArray();
        _grf = new GRF(keys, 6);

        // NitroSketch
        _nitroSketch = new NitroSketch(0.01, 0.01, 0.1);
        for (int i = 0; i < 1000; i++)
        {
            _nitroSketch.UpdateSampled($"flow{i % 100}");
        }

        // UnivMon
        _univMon = new UnivMon(100000, 0.01, 0.01);
        for (int i = 0; i < 1000; i++)
        {
            _univMon.Update($"item{i % 100}", (double)(i % 50 + 1));
        }

        // LearnedBloomFilter
        var trainingKeys = Enumerable.Range(0, 1000).Select(i => $"trained{i}").ToArray();
        _learnedBloom = new LearnedBloomFilter(trainingKeys, 0.01);
    }

    [GlobalCleanup]
    public void Cleanup()
    {
        _vacuumFilter?.Dispose();
        _grf?.Dispose();
        _nitroSketch?.Dispose();
        _univMon?.Dispose();
        _learnedBloom?.Dispose();
    }

    // ============================================================================
    // VacuumFilter Benchmarks
    // ============================================================================

    [Benchmark]
    public void VacuumFilter_Insert()
    {
        using var filter = new VacuumFilter(10000, 0.01);
        for (int i = 0; i < 1000; i++)
        {
            filter.Insert($"key{i}");
        }
    }

    [Benchmark]
    public bool VacuumFilter_Contains()
    {
        return _vacuumFilter.Contains("key500");
    }

    [Benchmark]
    public bool VacuumFilter_Delete()
    {
        var filter = new VacuumFilter(10000, 0.01);
        filter.Insert("tempkey");
        bool result = filter.Delete("tempkey");
        filter.Dispose();
        return result;
    }

    [Benchmark]
    public (ulong, ulong, double, ulong) VacuumFilter_GetStats()
    {
        return _vacuumFilter.GetStats();
    }

    // ============================================================================
    // GRF Benchmarks
    // ============================================================================

    [Benchmark]
    public void GRF_Build()
    {
        ulong[] keys = Enumerable.Range(0, 10000).Select(i => (ulong)i).ToArray();
        using var grf = new GRF(keys, 6);
    }

    [Benchmark]
    public bool GRF_MayContain()
    {
        return _grf.MayContain(5000);
    }

    [Benchmark]
    public bool GRF_MayContainRange()
    {
        return _grf.MayContainRange(4000, 6000);
    }

    [Benchmark]
    public double GRF_ExpectedFpr()
    {
        return _grf.ExpectedFpr(1000);
    }

    // ============================================================================
    // NitroSketch Benchmarks
    // ============================================================================

    [Benchmark]
    public void NitroSketch_UpdateSampled_HighRate()
    {
        using var nitro = new NitroSketch(0.01, 0.01, 0.9); // 90% sampling
        for (int i = 0; i < 1000; i++)
        {
            nitro.UpdateSampled($"flow{i}");
        }
    }

    [Benchmark]
    public void NitroSketch_UpdateSampled_LowRate()
    {
        using var nitro = new NitroSketch(0.01, 0.01, 0.1); // 10% sampling
        for (int i = 0; i < 1000; i++)
        {
            nitro.UpdateSampled($"flow{i}");
        }
    }

    [Benchmark]
    public uint NitroSketch_Query()
    {
        return _nitroSketch.Query("flow50");
    }

    [Benchmark]
    public void NitroSketch_Sync()
    {
        _nitroSketch.Sync(1.0);
    }

    // ============================================================================
    // UnivMon Benchmarks
    // ============================================================================

    [Benchmark]
    public void UnivMon_Update()
    {
        using var univmon = new UnivMon(100000, 0.01, 0.01);
        for (int i = 0; i < 1000; i++)
        {
            univmon.Update($"item{i}", (double)i);
        }
    }

    [Benchmark]
    public double UnivMon_EstimateL1()
    {
        return _univMon.EstimateL1();
    }

    [Benchmark]
    public double UnivMon_EstimateL2()
    {
        return _univMon.EstimateL2();
    }

    [Benchmark]
    public double UnivMon_EstimateEntropy()
    {
        return _univMon.EstimateEntropy();
    }

    [Benchmark]
    public double UnivMon_DetectChange()
    {
        using var other = new UnivMon(100000, 0.01, 0.01);
        for (int i = 0; i < 100; i++)
        {
            other.Update($"different{i}", 1.0);
        }
        return _univMon.DetectChange(other);
    }

    [Benchmark]
    public double UnivMon_AllMetrics()
    {
        // Benchmark getting all 6 metrics from ONE sketch
        double l1 = _univMon.EstimateL1();
        double l2 = _univMon.EstimateL2();
        double entropy = _univMon.EstimateEntropy();
        return l1 + l2 + entropy;
    }

    // ============================================================================
    // LearnedBloomFilter Benchmarks
    // ============================================================================

    [Benchmark]
    public void LearnedBloom_Construction()
    {
        var trainingKeys = Enumerable.Range(0, 1000).Select(i => $"key{i}").ToArray();
        using var filter = new LearnedBloomFilter(trainingKeys, 0.01);
    }

    [Benchmark]
    public bool LearnedBloom_Contains_Present()
    {
        return _learnedBloom.Contains("trained500");
    }

    [Benchmark]
    public bool LearnedBloom_Contains_Absent()
    {
        return _learnedBloom.Contains("absent999");
    }

    [Benchmark]
    public ulong LearnedBloom_MemoryUsage()
    {
        return _learnedBloom.MemoryUsage();
    }

    // ============================================================================
    // Comparison Benchmarks
    // ============================================================================

    [Benchmark]
    public void Comparison_1000Updates()
    {
        // Compare update throughput across sketches
        using var vacuum = new VacuumFilter(10000, 0.01);
        using var nitro = new NitroSketch(0.01, 0.01, 0.1);
        using var univmon = new UnivMon(100000, 0.01, 0.01);

        for (int i = 0; i < 1000; i++)
        {
            vacuum.Insert($"key{i}");
            nitro.UpdateSampled($"key{i}");
            univmon.Update($"key{i}", 1.0);
        }
    }
}
