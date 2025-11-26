using System;
using System.Linq;
using SketchOxide.RangeFilters;
using Xunit;

namespace SketchOxide.Tests;

/// <summary>
/// Unit tests for Grafite optimal range filter.
/// </summary>
public class GrafiteTests : IDisposable
{
    private Grafite? _filter;

    public GrafiteTests()
    {
        var keys = new ulong[] { 10, 20, 30, 40, 50, 60, 70, 80, 90, 100 };
        _filter = new Grafite(keys, 6);
    }

    public void Dispose()
    {
        _filter?.Dispose();
    }

    [Fact]
    public void Constructor_ValidParameters_CreatesFilter()
    {
        var keys = new ulong[] { 10, 20, 30 };
        using var filter = new Grafite(keys, 6);

        Assert.Equal(3ul, filter.KeyCount);
        Assert.Equal(6ul, filter.BitsPerKey);
    }

    [Fact]
    public void Constructor_WithNullKeys_ThrowsException()
    {
        Assert.Throws<ArgumentNullException>(() => new Grafite(null!, 6));
    }

    [Fact]
    public void Constructor_WithEmptyKeys_ThrowsException()
    {
        Assert.Throws<ArgumentException>(() => new Grafite(Array.Empty<ulong>(), 6));
    }

    [Theory]
    [InlineData(0ul)]   // Invalid bits per key (too low)
    [InlineData(65ul)]  // Invalid bits per key (too high)
    public void Constructor_WithInvalidBitsPerKey_ThrowsException(ulong bitsPerKey)
    {
        var keys = new ulong[] { 10, 20, 30 };
        Assert.Throws<ArgumentOutOfRangeException>(() => new Grafite(keys, bitsPerKey));
    }

    [Fact]
    public void MayContainRange_ContainingKeys_ReturnsTrue()
    {
        // Range [15, 25] contains key 20
        Assert.True(_filter!.MayContainRange(15, 25));

        // Range [10, 100] contains all keys
        Assert.True(_filter!.MayContainRange(10, 100));

        // Range [45, 55] contains key 50
        Assert.True(_filter!.MayContainRange(45, 55));
    }

    [Fact]
    public void MayContainRange_ExactKeyBounds_ReturnsTrue()
    {
        // Range exactly at key boundaries
        Assert.True(_filter!.MayContainRange(20, 20)); // Point query
        Assert.True(_filter!.MayContainRange(30, 40)); // Contains keys 30 and 40
    }

    [Fact]
    public void MayContainRange_EmptyRange_MayReturnFalse()
    {
        // Range far from any keys
        bool result = _filter!.MayContainRange(500, 600);
        // Result depends on FPR, but should likely be false
    }

    [Fact]
    public void MayContainRange_WithInvalidRange_ThrowsException()
    {
        // low > high is invalid
        Assert.Throws<ArgumentException>(() => _filter!.MayContainRange(100, 50));
    }

    [Fact]
    public void MayContain_ExistingKey_ReturnsTrue()
    {
        // Point queries for actual keys
        Assert.True(_filter!.MayContain(10));
        Assert.True(_filter!.MayContain(50));
        Assert.True(_filter!.MayContain(100));
    }

    [Fact]
    public void MayContain_NonExistingKey_MayReturnFalse()
    {
        // Point query for non-existing key
        bool result = _filter!.MayContain(999);
        // Result depends on FPR
    }

    [Fact]
    public void ExpectedFpr_ReturnsCorrectRate()
    {
        // FPR = rangeWidth / 2^(bitsPerKey - 2)
        // With bitsPerKey = 6: FPR = rangeWidth / 2^4 = rangeWidth / 16

        double fpr10 = _filter!.ExpectedFpr(10);
        double expected10 = 10.0 / 16.0;
        Assert.True(Math.Abs(fpr10 - expected10) < 0.01);

        double fpr1 = _filter!.ExpectedFpr(1);
        double expected1 = 1.0 / 16.0;
        Assert.True(Math.Abs(fpr1 - expected1) < 0.01);
    }

    [Fact]
    public void ExpectedFpr_LargerRange_HigherFpr()
    {
        double fpr10 = _filter!.ExpectedFpr(10);
        double fpr100 = _filter!.ExpectedFpr(100);

        Assert.True(fpr100 > fpr10, "Larger range should have higher FPR");
    }

    [Fact]
    public void Dispose_PreventsFurtherOperations()
    {
        _filter!.Dispose();

        Assert.Throws<ObjectDisposedException>(() => _filter.MayContainRange(10, 20));
        Assert.Throws<ObjectDisposedException>(() => _filter.MayContain(10));
        Assert.Throws<ObjectDisposedException>(() => _filter.ExpectedFpr(10));
        Assert.Throws<ObjectDisposedException>(() => _filter.KeyCount);
        Assert.Throws<ObjectDisposedException>(() => _filter.BitsPerKey);
    }

    [Fact]
    public void ToString_WithData_ContainsParameters()
    {
        var str = _filter!.ToString();

        Assert.Contains("Grafite", str);
        Assert.Contains("keys=10", str);
        Assert.Contains("bitsPerKey=6", str);
    }

    [Fact]
    public void ToString_WhenDisposed_IndicatesDisposed()
    {
        _filter!.Dispose();
        var str = _filter.ToString();

        Assert.Contains("disposed", str, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void UsingPattern_AutomaticallyCleansUp()
    {
        Grafite? filter = null;

        using (var temp = new Grafite(new ulong[] { 10, 20, 30 }, 6))
        {
            filter = temp;
            Assert.True(temp.MayContain(20));
        }

        // After using block, filter should be disposed
        Assert.Throws<ObjectDisposedException>(() => filter!.MayContain(20));
    }

    [Fact]
    public void LargeKeySet_HandlesCorrectly()
    {
        // Create filter with 1000 keys
        var keys = Enumerable.Range(0, 1000).Select(i => (ulong)(i * 10)).ToArray();
        using var filter = new Grafite(keys, 8);

        Assert.Equal(1000ul, filter.KeyCount);

        // Test range queries
        Assert.True(filter.MayContainRange(0, 100));
        Assert.True(filter.MayContainRange(5000, 5100));
    }

    [Fact]
    public void VariousBitsPerKey_AllWork()
    {
        var keys = new ulong[] { 10, 20, 30, 40, 50 };

        // Test different bits per key values
        foreach (var bitsPerKey in new ulong[] { 4, 6, 8, 10 })
        {
            using var filter = new Grafite(keys, bitsPerKey);
            Assert.Equal(bitsPerKey, filter.BitsPerKey);
            Assert.True(filter.MayContain(30));
        }
    }

    [Fact]
    public void ConsecutiveKeys_HandlesCorrectly()
    {
        // Test with consecutive keys
        var keys = new ulong[] { 1, 2, 3, 4, 5, 6, 7, 8, 9, 10 };
        using var filter = new Grafite(keys, 6);

        // Range covering all keys
        Assert.True(filter.MayContainRange(1, 10));

        // Point queries
        Assert.True(filter.MayContain(5));
        Assert.True(filter.MayContain(1));
        Assert.True(filter.MayContain(10));
    }

    [Fact]
    public void SparseKeys_HandlesCorrectly()
    {
        // Test with very sparse keys
        var keys = new ulong[] { 0, 1000, 2000, 3000, 4000 };
        using var filter = new Grafite(keys, 6);

        // Ranges containing keys
        Assert.True(filter.MayContainRange(900, 1100));
        Assert.True(filter.MayContainRange(1950, 2050));

        // Point queries
        Assert.True(filter.MayContain(1000));
        Assert.True(filter.MayContain(3000));
    }
}
