using System;
using SketchOxide.Cardinality;
using Xunit;

namespace SketchOxide.Tests;

/// <summary>
/// Unit tests for HyperLogLog cardinality estimator.
/// </summary>
public class HyperLogLogTests : IDisposable
{
    private HyperLogLog? _hll;

    public HyperLogLogTests()
    {
        _hll = new HyperLogLog(14);
    }

    public void Dispose()
    {
        _hll?.Dispose();
    }

    [Fact]
    public void Constructor_ValidPrecision_CreatesSketch()
    {
        using var hll = new HyperLogLog(14);
        Assert.Equal(14u, hll.Precision);
    }

    [Theory]
    [InlineData(3u)]   // Too low
    [InlineData(17u)]  // Too high
    public void Constructor_InvalidPrecision_ThrowsException(uint precision)
    {
        var ex = Assert.Throws<ArgumentOutOfRangeException>(() => new HyperLogLog(precision));
        Assert.Contains("Precision must be in range [4, 16]", ex.Message);
    }

    [Theory]
    [InlineData(4u)]   // Minimum
    [InlineData(10u)]  // Common
    [InlineData(16u)]  // Maximum
    public void Constructor_ValidBoundaryPrecision_Succeeds(uint precision)
    {
        using var hll = new HyperLogLog(precision);
        Assert.Equal(precision, hll.Precision);
    }

    [Fact]
    public void Update_WithBytes_DoesNotThrow()
    {
        _hll!.Update(new byte[] { 1, 2, 3, 4 });
        Assert.True(_hll.Estimate() >= 0);
    }

    [Fact]
    public void Update_WithString_DoesNotThrow()
    {
        _hll!.Update("test-item");
        Assert.True(_hll.Estimate() >= 0);
    }

    [Fact]
    public void Update_WithNull_ThrowsArgumentNullException()
    {
        Assert.Throws<ArgumentNullException>(() => _hll!.Update((string)null!));
    }

    [Fact]
    public void Estimate_EmptySketch_ReturnsZero()
    {
        var estimate = _hll!.Estimate();
        Assert.True(estimate == 0 || estimate < 1.0);
    }

    [Fact]
    public void Estimate_WithDuplicates_CountsOnce()
    {
        for (int i = 0; i < 100; i++)
        {
            _hll!.Update("same-item");
        }

        var estimate = _hll!.Estimate();
        Assert.True(estimate <= 5, "Duplicates should not increase estimate");
    }

    [Fact]
    public void Estimate_Accuracy_WithinErrorBounds()
    {
        const int itemCount = 10000;
        for (int i = 0; i < itemCount; i++)
        {
            _hll!.Update($"item-{i}");
        }

        var estimate = _hll!.Estimate();
        var error = Math.Abs(estimate - itemCount) / itemCount;

        // Allow 3% error for precision=14
        Assert.True(error < 0.03, $"Estimation error {error} exceeded 3%");
    }

    [Fact]
    public void EstimateLong_WithData_ReturnsRoundedValue()
    {
        for (int i = 0; i < 100; i++)
        {
            _hll!.Update($"item-{i}");
        }

        var longEstimate = _hll!.EstimateLong();
        Assert.True(longEstimate > 0);
    }

    [Fact]
    public void Merge_CompatibleSketches_Combines()
    {
        using var other = new HyperLogLog(14);

        for (int i = 0; i < 5000; i++)
        {
            _hll!.Update($"first-{i}");
            other.Update($"second-{i}");
        }

        var beforeMerge = _hll!.Estimate();
        _hll.Merge(other);
        var afterMerge = _hll.Estimate();

        Assert.True(afterMerge > beforeMerge, "Merged estimate should be larger");
        Assert.True(afterMerge < 15000, "Merged estimate should be reasonable");
    }

    [Fact]
    public void Merge_IncompatiblePrecision_ThrowsException()
    {
        using var other = new HyperLogLog(16);

        var ex = Assert.Throws<ArgumentException>(() => _hll!.Merge(other));
        Assert.Contains("Cannot merge sketches with different precisions", ex.Message);
    }

    [Fact]
    public void Merge_WithNull_ThrowsException()
    {
        var ex = Assert.Throws<ArgumentNullException>(() => _hll!.Merge(null!));
        Assert.Equal("other", ex.ParamName);
    }

    [Fact]
    public void Serialize_WithData_ReturnsBytes()
    {
        for (int i = 0; i < 1000; i++)
        {
            _hll!.Update($"item-{i}");
        }

        var serialized = _hll!.Serialize();
        Assert.NotNull(serialized);
        Assert.True(serialized.Length > 0);
    }

    [Fact]
    public void SerializationRoundTrip_PreservesEstimate()
    {
        for (int i = 0; i < 1000; i++)
        {
            _hll!.Update($"item-{i}");
        }

        var originalEstimate = _hll!.Estimate();
        var serialized = _hll.Serialize();

        using var restored = HyperLogLog.Deserialize(serialized);
        var restoredEstimate = restored.Estimate();

        Assert.Equal(originalEstimate, restoredEstimate, precision: 4);
    }

    [Fact]
    public void Deserialize_InvalidData_ThrowsException()
    {
        var invalidData = new byte[] { 1, 2, 3 };

        var ex = Assert.Throws<ArgumentException>(() => HyperLogLog.Deserialize(invalidData));
        Assert.Contains("Failed to deserialize", ex.Message);
    }

    [Fact]
    public void Deserialize_WithNull_ThrowsException()
    {
        var ex = Assert.Throws<ArgumentNullException>(() => HyperLogLog.Deserialize(null!));
        Assert.Equal("data", ex.ParamName);
    }

    [Fact]
    public void Dispose_PreventsFurtherOperations()
    {
        _hll!.Dispose();

        Assert.Throws<ObjectDisposedException>(() => _hll.Update("test"));
        Assert.Throws<ObjectDisposedException>(() => _hll.Estimate());
        Assert.Throws<ObjectDisposedException>(() => _hll.Precision);
    }

    [Fact]
    public void ToString_WithData_ContainsPrecisionAndEstimate()
    {
        _hll!.Update("test");
        var str = _hll.ToString();

        Assert.Contains("HyperLogLog", str);
        Assert.Contains("precision", str, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void ToString_WhenDisposed_IndicatesDisposed()
    {
        _hll!.Dispose();
        var str = _hll.ToString();

        Assert.Contains("disposed", str, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void UsingPattern_AutomaticallyCleansUp()
    {
        HyperLogLog? sketch = null;

        using (var hll = new HyperLogLog(14))
        {
            hll.Update("test");
            sketch = hll;
            Assert.True(hll.Estimate() >= 0);
        }

        // After using block, sketch should be disposed
        Assert.Throws<ObjectDisposedException>(() => sketch!.Update("test"));
    }

    [Fact]
    public void LargeDatasetTest_Accuracy()
    {
        const int itemCount = 1000000;

        // Use new sketch for large dataset
        using var largeHll = new HyperLogLog(14);

        for (int i = 0; i < itemCount; i++)
        {
            largeHll.Update($"user-{i}");
        }

        var estimate = largeHll.Estimate();
        var error = Math.Abs(estimate - itemCount) / itemCount;

        Assert.True(error < 0.03, $"Error on 1M items: {error}");
    }
}
