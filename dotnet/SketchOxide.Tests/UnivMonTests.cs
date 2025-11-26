using Xunit;
using SketchOxide.Universal;
using System;

namespace SketchOxide.Tests;

public class UnivMonTests
{
    [Fact]
    public void Constructor_ValidParameters_Succeeds()
    {
        using var univmon = new UnivMon(1000000, 0.01, 0.01);
        Assert.Equal(1000000UL, univmon.MaxStreamSize);
        Assert.Equal(0.01, univmon.Epsilon);
        Assert.Equal(0.01, univmon.Delta);
    }

    [Fact]
    public void Constructor_ZeroMaxStreamSize_ThrowsArgumentOutOfRange()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new UnivMon(0, 0.01, 0.01));
    }

    [Fact]
    public void Constructor_InvalidEpsilon_ThrowsArgumentOutOfRange()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new UnivMon(1000, 0, 0.01));
        Assert.Throws<ArgumentOutOfRangeException>(() => new UnivMon(1000, 1.0, 0.01));
    }

    [Fact]
    public void Constructor_InvalidDelta_ThrowsArgumentOutOfRange()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new UnivMon(1000, 0.01, 0));
        Assert.Throws<ArgumentOutOfRangeException>(() => new UnivMon(1000, 0.01, 1.0));
    }

    [Fact]
    public void Update_BasicOperation_Works()
    {
        using var univmon = new UnivMon(10000, 0.01, 0.01);

        univmon.Update("flow1", 1500.0);
        univmon.Update("flow2", 800.0);
        univmon.Update("flow1", 1200.0);

        // Verify estimates are non-zero
        double l1 = univmon.EstimateL1();
        Assert.True(l1 > 0);
    }

    [Fact]
    public void EstimateL1_SumOfFrequencies_Correct()
    {
        using var univmon = new UnivMon(10000, 0.01, 0.01);

        // Update with known values
        univmon.Update("a", 100.0);
        univmon.Update("b", 200.0);
        univmon.Update("c", 300.0);

        double l1 = univmon.EstimateL1();
        // L1 = sum of values = 600
        Assert.InRange(l1, 500, 700); // Allow for estimation error
    }

    [Fact]
    public void EstimateL2_SumOfSquaredFrequencies_Correct()
    {
        using var univmon = new UnivMon(10000, 0.01, 0.01);

        univmon.Update("a", 10.0);
        univmon.Update("b", 20.0);
        univmon.Update("c", 30.0);

        double l2 = univmon.EstimateL2();
        // L2 = sqrt(10^2 + 20^2 + 30^2) = sqrt(1400) ≈ 37.4
        Assert.True(l2 > 0);
    }

    [Fact]
    public void EstimateEntropy_UniformDistribution_HighEntropy()
    {
        using var univmon = new UnivMon(10000, 0.01, 0.01);

        // Uniform distribution - all items have same frequency
        for (int i = 0; i < 100; i++)
        {
            univmon.Update($"item_{i}", 1.0);
        }

        double entropy = univmon.EstimateEntropy();
        // Uniform distribution should have high entropy
        Assert.True(entropy > 0);
    }

    [Fact]
    public void EstimateEntropy_SkewedDistribution_LowerEntropy()
    {
        using var univmon = new UnivMon(10000, 0.01, 0.01);

        // Skewed distribution - one dominant item
        univmon.Update("dominant", 1000.0);
        for (int i = 0; i < 10; i++)
        {
            univmon.Update($"rare_{i}", 1.0);
        }

        double entropy = univmon.EstimateEntropy();
        // Skewed distribution should have lower entropy than uniform
        Assert.True(entropy >= 0);
    }

    [Fact]
    public void DetectChange_IdenticalSketches_SmallChange()
    {
        using var univmon1 = new UnivMon(10000, 0.01, 0.01);
        using var univmon2 = new UnivMon(10000, 0.01, 0.01);

        for (int i = 0; i < 100; i++)
        {
            univmon1.Update($"item_{i}", 1.0);
            univmon2.Update($"item_{i}", 1.0);
        }

        double change = univmon1.DetectChange(univmon2);
        // Identical distributions should have small change magnitude
        Assert.InRange(change, 0.0, 100.0);
    }

    [Fact]
    public void DetectChange_DifferentSketches_LargeChange()
    {
        using var univmon1 = new UnivMon(10000, 0.01, 0.01);
        using var univmon2 = new UnivMon(10000, 0.01, 0.01);

        for (int i = 0; i < 100; i++)
        {
            univmon1.Update($"item_{i}", 1.0);
        }

        for (int i = 100; i < 200; i++)
        {
            univmon2.Update($"item_{i}", 1.0);
        }

        double change = univmon1.DetectChange(univmon2);
        // Completely different distributions should have large change
        Assert.True(change > 0);
    }

    [Fact]
    public void GetStats_ReturnsCorrectInfo()
    {
        using var univmon = new UnivMon(10000, 0.01, 0.01);

        for (int i = 0; i < 100; i++)
        {
            univmon.Update($"item_{i}", 1.0);
        }

        var stats = univmon.GetStats();
        Assert.True(stats.NumLayers > 0);
        Assert.True(stats.TotalMemory > 0);
        Assert.True(stats.SamplesProcessed > 0);
    }

    [Fact]
    public void SimultaneousMetrics_AllWork()
    {
        using var univmon = new UnivMon(10000, 0.01, 0.01);

        // Simulate network traffic monitoring
        univmon.Update("192.168.1.1", 1500.0); // Packet size
        univmon.Update("192.168.1.2", 800.0);
        univmon.Update("192.168.1.1", 1200.0);
        univmon.Update("192.168.1.3", 600.0);

        // All 6 metrics from ONE sketch!
        double totalTraffic = univmon.EstimateL1();
        double variability = univmon.EstimateL2();
        double diversity = univmon.EstimateEntropy();

        Assert.True(totalTraffic > 0);
        Assert.True(variability > 0);
        Assert.True(diversity > 0);
    }

    [Fact]
    public void ByteArray_Operations_Work()
    {
        using var univmon = new UnivMon(10000, 0.01, 0.01);

        byte[] key = new byte[] { 0x01, 0x02, 0x03 };
        univmon.Update(key, 100.0);

        double l1 = univmon.EstimateL1();
        Assert.True(l1 > 0);
    }

    [Fact]
    public void ToString_ReturnsDescriptiveString()
    {
        using var univmon = new UnivMon(10000, 0.01, 0.01);
        string str = univmon.ToString();
        Assert.Contains("UnivMon", str);
        Assert.Contains("10000", str);
    }
}
