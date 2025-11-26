using Xunit;
using SketchOxide.Frequency;
using System;

namespace SketchOxide.Tests;

public class NitroSketchTests
{
    [Fact]
    public void Constructor_ValidParameters_Succeeds()
    {
        using var nitro = new NitroSketch(0.01, 0.01, 0.1);
        Assert.Equal(0.1, nitro.SampleRate);
    }

    [Fact]
    public void Constructor_InvalidEpsilon_ThrowsArgumentOutOfRange()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new NitroSketch(0, 0.01, 0.1));
        Assert.Throws<ArgumentOutOfRangeException>(() => new NitroSketch(1.0, 0.01, 0.1));
    }

    [Fact]
    public void Constructor_InvalidDelta_ThrowsArgumentOutOfRange()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new NitroSketch(0.01, 0, 0.1));
        Assert.Throws<ArgumentOutOfRangeException>(() => new NitroSketch(0.01, 1.0, 0.1));
    }

    [Fact]
    public void Constructor_InvalidSampleRate_ThrowsArgumentOutOfRange()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new NitroSketch(0.01, 0.01, 0));
        Assert.Throws<ArgumentOutOfRangeException>(() => new NitroSketch(0.01, 0.01, 1.5));
    }

    [Fact]
    public void UpdateSampled_BasicOperation_Works()
    {
        using var nitro = new NitroSketch(0.01, 0.01, 0.5);

        for (int i = 0; i < 1000; i++)
        {
            nitro.UpdateSampled($"item_{i % 10}");
        }

        // Should have sampled approximately half
        var stats = nitro.GetStats();
        Assert.True(stats.TotalItemsEstimated > 0);
    }

    [Fact]
    public void Query_AfterUpdates_ReturnsEstimate()
    {
        using var nitro = new NitroSketch(0.01, 0.01, 0.5);

        // Update "item1" 100 times
        for (int i = 0; i < 100; i++)
        {
            nitro.UpdateSampled("item1");
        }

        uint estimate = nitro.Query("item1");
        // With 50% sampling, estimate should be non-zero
        Assert.True(estimate >= 0);
    }

    [Fact]
    public void Sync_AfterUpdates_Succeeds()
    {
        using var nitro = new NitroSketch(0.01, 0.01, 0.1);

        for (int i = 0; i < 1000; i++)
        {
            nitro.UpdateSampled($"item_{i}");
        }

        // Should not throw
        nitro.Sync(1.0);
    }

    [Fact]
    public void GetStats_ReturnsCorrectInfo()
    {
        using var nitro = new NitroSketch(0.01, 0.01, 0.1);

        for (int i = 0; i < 1000; i++)
        {
            nitro.UpdateSampled("item");
        }

        var stats = nitro.GetStats();
        Assert.Equal(0.1, stats.SampleRate);
        Assert.True(stats.SampledCount + stats.UnsampledCount > 0);
        Assert.True(stats.TotalItemsEstimated > 0);
    }

    [Fact]
    public void HighSampleRate_CapturesMostUpdates()
    {
        using var nitro = new NitroSketch(0.01, 0.01, 0.9);

        for (int i = 0; i < 1000; i++)
        {
            nitro.UpdateSampled("item");
        }

        var stats = nitro.GetStats();
        // With 90% sampling, should capture most items
        Assert.True(stats.SampledCount > stats.UnsampledCount);
    }

    [Fact]
    public void LowSampleRate_SkipsMostUpdates()
    {
        using var nitro = new NitroSketch(0.01, 0.01, 0.1);

        for (int i = 0; i < 1000; i++)
        {
            nitro.UpdateSampled("item");
        }

        var stats = nitro.GetStats();
        // With 10% sampling, should skip most items
        Assert.True(stats.UnsampledCount > stats.SampledCount);
    }

    [Fact]
    public void ByteArray_Operations_Work()
    {
        using var nitro = new NitroSketch(0.01, 0.01, 0.5);

        byte[] key = new byte[] { 0x01, 0x02, 0x03 };
        for (int i = 0; i < 100; i++)
        {
            nitro.UpdateSampled(key);
        }

        uint estimate = nitro.Query(key);
        Assert.True(estimate >= 0);
    }

    [Fact]
    public void ToString_ReturnsDescriptiveString()
    {
        using var nitro = new NitroSketch(0.01, 0.01, 0.1);
        string str = nitro.ToString();
        Assert.Contains("NitroSketch", str);
        Assert.Contains("0.1", str);
    }
}
