using System;
using SketchOxide.RangeFilters;
using Xunit;

namespace SketchOxide.Tests;

/// <summary>
/// Unit tests for MementoFilter dynamic range filter.
/// </summary>
public class MementoFilterTests : IDisposable
{
    private MementoFilter? _filter;

    public MementoFilterTests()
    {
        _filter = new MementoFilter(1000, 0.01);
    }

    public void Dispose()
    {
        _filter?.Dispose();
    }

    [Fact]
    public void Constructor_ValidParameters_CreatesFilter()
    {
        using var filter = new MementoFilter(1000, 0.01);
        Assert.Equal(1000ul, filter.ExpectedElements);
        Assert.Equal(0.01, filter.Fpr);
    }

    [Theory]
    [InlineData(0ul, 0.01)]     // Invalid expectedElements
    [InlineData(1000ul, 0.0)]   // Invalid fpr (too low)
    [InlineData(1000ul, 1.0)]   // Invalid fpr (too high)
    public void Constructor_InvalidParameters_ThrowsException(ulong expectedElements, double fpr)
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new MementoFilter(expectedElements, fpr));
    }

    [Fact]
    public void Insert_WithBytes_Succeeds()
    {
        var value = new byte[] { 1, 2, 3, 4 };
        _filter!.Insert(42, value);
        // No exception means success
    }

    [Fact]
    public void Insert_WithString_Succeeds()
    {
        _filter!.Insert(100, "test-value");
        // No exception means success
    }

    [Fact]
    public void Insert_WithNullValue_ThrowsException()
    {
        Assert.Throws<ArgumentNullException>(() => _filter!.Insert(42, (byte[])null!));
        Assert.Throws<ArgumentNullException>(() => _filter!.Insert(42, (string)null!));
    }

    [Fact]
    public void MayContainRange_AfterInsert_ReturnsTrue()
    {
        // Insert keys in range [100, 200]
        for (ulong i = 100; i <= 200; i += 10)
        {
            _filter!.Insert(i, $"value_{i}");
        }

        // Range containing inserted keys
        Assert.True(_filter!.MayContainRange(100, 200));
        Assert.True(_filter!.MayContainRange(90, 210));
    }

    [Fact]
    public void MayContainRange_EmptyFilter_MayReturnFalse()
    {
        // Empty filter
        bool result = _filter!.MayContainRange(100, 200);
        // Result depends on implementation, but empty filter may return false
    }

    [Fact]
    public void MayContainRange_WithInvalidRange_ThrowsException()
    {
        Assert.Throws<ArgumentException>(() => _filter!.MayContainRange(200, 100));
    }

    [Fact]
    public void Insert_DynamicExpansion_HandlesCorrectly()
    {
        // Insert keys in expanding ranges
        _filter!.Insert(10, "value1");
        _filter!.Insert(100, "value2");
        _filter!.Insert(1000, "value3");
        _filter!.Insert(10000, "value4");

        // All ranges should be queryable
        Assert.True(_filter!.MayContainRange(5, 15));
        Assert.True(_filter!.MayContainRange(95, 105));
    }

    [Fact]
    public void InsertBatch_ProcessesAllPairs()
    {
        var pairs = new[]
        {
            (10ul, new byte[] { 1, 2, 3 }),
            (20ul, new byte[] { 4, 5, 6 }),
            (30ul, new byte[] { 7, 8, 9 })
        };

        _filter!.InsertBatch(pairs);

        // Verify ranges containing these keys
        Assert.True(_filter!.MayContainRange(10, 30));
    }

    [Fact]
    public void InsertBatch_WithNull_ThrowsException()
    {
        Assert.Throws<ArgumentNullException>(() =>
            _filter!.InsertBatch((ValueTuple<ulong, byte[]>[])null!));
    }

    [Fact]
    public void Dispose_PreventsFurtherOperations()
    {
        _filter!.Dispose();

        Assert.Throws<ObjectDisposedException>(() => _filter.Insert(10, "value"));
        Assert.Throws<ObjectDisposedException>(() => _filter.MayContainRange(10, 20));
        Assert.Throws<ObjectDisposedException>(() => _filter.ExpectedElements);
        Assert.Throws<ObjectDisposedException>(() => _filter.Fpr);
    }

    [Fact]
    public void ToString_WithData_ContainsParameters()
    {
        var str = _filter!.ToString();

        Assert.Contains("MementoFilter", str);
        Assert.Contains("expectedElements=1000", str);
        Assert.Contains("fpr=0.01", str);
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
        MementoFilter? filter = null;

        using (var temp = new MementoFilter(1000, 0.01))
        {
            temp.Insert(42, "value");
            filter = temp;
        }

        // After using block, filter should be disposed
        Assert.Throws<ObjectDisposedException>(() => filter!.Insert(42, "value"));
    }

    [Fact]
    public void SequentialInserts_MaintainCorrectness()
    {
        // Insert keys sequentially
        for (ulong i = 0; i < 100; i++)
        {
            _filter!.Insert(i, $"value_{i}");
        }

        // Query ranges
        Assert.True(_filter!.MayContainRange(0, 50));
        Assert.True(_filter!.MayContainRange(50, 99));
        Assert.True(_filter!.MayContainRange(0, 99));
    }

    [Fact]
    public void RandomInserts_MaintainCorrectness()
    {
        var random = new Random(42);

        // Insert random keys
        var keys = new ulong[100];
        for (int i = 0; i < 100; i++)
        {
            keys[i] = (ulong)random.Next(0, 10000);
            _filter!.Insert(keys[i], $"value_{keys[i]}");
        }

        // Find min and max
        ulong min = keys[0];
        ulong max = keys[0];
        foreach (var key in keys)
        {
            if (key < min) min = key;
            if (key > max) max = key;
        }

        // Range covering all keys should return true
        Assert.True(_filter!.MayContainRange(min, max));
    }

    [Fact]
    public void LargeDataset_HandlesCorrectly()
    {
        const int itemCount = 1000;

        // Insert many items
        for (ulong i = 0; i < itemCount; i++)
        {
            _filter!.Insert(i * 10, $"value_{i}");
        }

        // Test various ranges
        Assert.True(_filter!.MayContainRange(0, 100));
        Assert.True(_filter!.MayContainRange(5000, 6000));
        Assert.True(_filter!.MayContainRange(0, (itemCount - 1) * 10));
    }

    [Fact]
    public void DifferentFprValues_AllWork()
    {
        foreach (var fpr in new[] { 0.001, 0.01, 0.05, 0.1 })
        {
            using var filter = new MementoFilter(1000, fpr);
            Assert.Equal(fpr, filter.Fpr);

            filter.Insert(100, "test");
            Assert.True(filter.MayContainRange(90, 110));
        }
    }

    [Fact]
    public void SparseKeys_HandlesCorrectly()
    {
        // Insert very sparse keys
        _filter!.Insert(0, "value0");
        _filter!.Insert(1000, "value1000");
        _filter!.Insert(2000, "value2000");
        _filter!.Insert(10000, "value10000");

        // Test ranges
        Assert.True(_filter!.MayContainRange(0, 100));
        Assert.True(_filter!.MayContainRange(900, 1100));
        Assert.True(_filter!.MayContainRange(9900, 10100));
    }
}
