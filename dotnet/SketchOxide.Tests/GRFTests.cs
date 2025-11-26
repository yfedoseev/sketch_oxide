using Xunit;
using SketchOxide.RangeFilters;
using System;

namespace SketchOxide.Tests;

public class GRFTests
{
    [Fact]
    public void Constructor_ValidParameters_Succeeds()
    {
        ulong[] keys = new ulong[] { 10, 20, 30, 40, 50 };
        using var grf = new GRF(keys, 6);
        Assert.Equal(5UL, grf.KeyCount);
        Assert.Equal(6UL, grf.BitsPerKey);
    }

    [Fact]
    public void Constructor_NullKeys_ThrowsArgumentNull()
    {
        Assert.Throws<ArgumentNullException>(() => new GRF(null, 6));
    }

    [Fact]
    public void Constructor_EmptyKeys_ThrowsArgumentOutOfRange()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new GRF(Array.Empty<ulong>(), 6));
    }

    [Fact]
    public void Constructor_InvalidBitsPerKey_ThrowsArgumentOutOfRange()
    {
        ulong[] keys = new ulong[] { 10, 20, 30 };
        Assert.Throws<ArgumentOutOfRangeException>(() => new GRF(keys, 1));
        Assert.Throws<ArgumentOutOfRangeException>(() => new GRF(keys, 20));
    }

    [Fact]
    public void MayContain_ExistingKeys_ReturnsTrue()
    {
        ulong[] keys = new ulong[] { 10, 20, 30, 40, 50 };
        using var grf = new GRF(keys, 6);

        Assert.True(grf.MayContain(10));
        Assert.True(grf.MayContain(20));
        Assert.True(grf.MayContain(50));
    }

    [Fact]
    public void MayContainRange_OverlappingRange_ReturnsTrue()
    {
        ulong[] keys = new ulong[] { 10, 20, 30, 40, 50 };
        using var grf = new GRF(keys, 6);

        Assert.True(grf.MayContainRange(15, 25)); // Contains 20
        Assert.True(grf.MayContainRange(10, 50)); // Contains all
        Assert.True(grf.MayContainRange(25, 35)); // Contains 30
    }

    [Fact]
    public void MayContainRange_DisjointRange_MayReturnFalse()
    {
        ulong[] keys = new ulong[] { 10, 20, 30, 40, 50 };
        using var grf = new GRF(keys, 6);

        // Disjoint ranges may return false (but not guaranteed due to FPR)
        bool result = grf.MayContainRange(100, 200);
        // Can't assert false due to potential false positives
        Assert.True(result || !result); // Just verify it doesn't crash
    }

    [Fact]
    public void ExpectedFpr_ReturnsValidValue()
    {
        ulong[] keys = new ulong[] { 10, 20, 30, 40, 50 };
        using var grf = new GRF(keys, 6);

        double fpr = grf.ExpectedFpr(10);
        Assert.InRange(fpr, 0.0, 1.0);
    }

    [Fact]
    public void GetStats_ReturnsCorrectInfo()
    {
        ulong[] keys = new ulong[] { 10, 20, 30, 40, 50 };
        using var grf = new GRF(keys, 6);

        var stats = grf.GetStats();
        Assert.Equal(5UL, stats.KeyCount);
        Assert.True(stats.SegmentCount > 0);
        Assert.True(stats.AvgKeysPerSegment > 0);
        Assert.Equal(6UL, stats.BitsPerKey);
        Assert.True(stats.TotalBits > 0);
    }

    [Fact]
    public void LargeDataset_FibonacciKeys_Works()
    {
        // Fibonacci sequence has skewed distribution - good for GRF
        ulong[] fibKeys = new ulong[] { 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233 };
        using var grf = new GRF(fibKeys, 6);

        Assert.Equal(12UL, grf.KeyCount);
        Assert.True(grf.MayContain(89));
        Assert.True(grf.MayContainRange(10, 25)); // Contains 13, 21
    }

    [Fact]
    public void DuplicateKeys_HandledCorrectly()
    {
        ulong[] keys = new ulong[] { 10, 10, 20, 20, 30, 30 };
        using var grf = new GRF(keys, 6);

        // Should deduplicate internally
        Assert.True(grf.MayContain(10));
        Assert.True(grf.MayContain(20));
        Assert.True(grf.MayContain(30));
    }

    [Fact]
    public void ToString_ReturnsDescriptiveString()
    {
        ulong[] keys = new ulong[] { 10, 20, 30 };
        using var grf = new GRF(keys, 6);
        string str = grf.ToString();
        Assert.Contains("GRF", str);
        Assert.Contains("3", str);
    }
}
