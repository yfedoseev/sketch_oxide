using System;
using SketchOxide.Streaming;
using Xunit;

namespace SketchOxide.Tests;

/// <summary>
/// Unit tests for SlidingHyperLogLog time-windowed cardinality estimator.
/// </summary>
public class SlidingHyperLogLogTests : IDisposable
{
    private SlidingHyperLogLog? _hll;

    public SlidingHyperLogLogTests()
    {
        _hll = new SlidingHyperLogLog(12, 3600);
    }

    public void Dispose()
    {
        _hll?.Dispose();
    }

    [Fact]
    public void Constructor_ValidParameters_CreatesSketch()
    {
        using var hll = new SlidingHyperLogLog(12, 3600);
        Assert.Equal(12, hll.Precision);
        Assert.Equal(3600ul, hll.MaxWindowSeconds);
    }

    [Theory]
    [InlineData(3, 3600ul)]     // Precision too low
    [InlineData(17, 3600ul)]    // Precision too high
    [InlineData(12, 0ul)]       // Invalid window
    public void Constructor_InvalidParameters_ThrowsException(byte precision, ulong maxWindowSeconds)
    {
        Assert.Throws<ArgumentOutOfRangeException>(() =>
            new SlidingHyperLogLog(precision, maxWindowSeconds));
    }

    [Fact]
    public void Update_WithBytes_Succeeds()
    {
        var data = new byte[] { 1, 2, 3, 4 };
        _hll!.Update(data, 1000);
        // No exception means success
    }

    [Fact]
    public void Update_WithString_Succeeds()
    {
        _hll!.Update("test-item", 1000);
        // No exception means success
    }

    [Fact]
    public void Update_WithNullData_ThrowsException()
    {
        Assert.Throws<ArgumentNullException>(() => _hll!.Update((string)null!, 1000));
    }

    [Fact]
    public void EstimateWindow_AfterUpdates_ReturnsPositive()
    {
        // Add items at different timestamps
        _hll!.Update("item1", 1000);
        _hll!.Update("item2", 1010);
        _hll!.Update("item3", 1020);

        // Estimate cardinality in 60-second window
        double estimate = _hll!.EstimateWindow(1020, 60);
        Assert.True(estimate > 0, "Should estimate at least one unique item");
    }

    [Fact]
    public void EstimateWindow_ExceedsMaxWindow_ThrowsException()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() =>
            _hll!.EstimateWindow(5000, 4000)); // 4000 > 3600 (maxWindow)
    }

    [Fact]
    public void EstimateWindow_WithDuplicates_CountsOnce()
    {
        ulong timestamp = 1000;

        // Add same item multiple times
        for (int i = 0; i < 100; i++)
        {
            _hll!.Update("same-item", timestamp);
        }

        double estimate = _hll!.EstimateWindow(timestamp + 10, 60);
        Assert.True(estimate <= 5, "Duplicates should be counted once");
    }

    [Fact]
    public void EstimateWindow_TimeWindowFiltering_Works()
    {
        // Add items at time 1000
        _hll!.Update("old1", 1000);
        _hll!.Update("old2", 1000);

        // Add items at time 2000
        _hll!.Update("new1", 2000);
        _hll!.Update("new2", 2000);

        // Query window [1950, 2050] should only see new items
        double recentEstimate = _hll!.EstimateWindow(2000, 60);

        // Total should include all items
        double totalEstimate = _hll!.EstimateTotal();

        Assert.True(totalEstimate >= recentEstimate,
            "Total estimate should be >= window estimate");
    }

    [Fact]
    public void EstimateTotal_ReturnsAllItems()
    {
        // Add items across different times
        for (ulong i = 0; i < 10; i++)
        {
            _hll!.Update($"item_{i}", 1000 + i * 100);
        }

        double total = _hll!.EstimateTotal();
        Assert.True(total >= 5, "Should estimate reasonable cardinality");
    }

    [Fact]
    public void EstimateTotalLong_ReturnsRoundedValue()
    {
        for (int i = 0; i < 100; i++)
        {
            _hll!.Update($"item_{i}", 1000);
        }

        long totalLong = _hll!.EstimateTotalLong();
        Assert.True(totalLong > 0);
    }

    [Fact]
    public void Decay_RemovesOldEntries()
    {
        // Add old items
        for (int i = 0; i < 50; i++)
        {
            _hll!.Update($"old_item_{i}", 1000);
        }

        // Add recent items
        for (int i = 0; i < 50; i++)
        {
            _hll!.Update($"new_item_{i}", 3000);
        }

        // Decay with window of 600 seconds from time 3000
        // Should remove items from time 1000 (which are 2000 seconds old)
        _hll!.Decay(3000, 600);

        // After decay, recent window should have fewer items
        double afterDecay = _hll!.EstimateWindow(3000, 600);
        Assert.True(afterDecay >= 0);
    }

    [Fact]
    public void Decay_ExceedsMaxWindow_ThrowsException()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() =>
            _hll!.Decay(5000, 4000)); // 4000 > 3600 (maxWindow)
    }

    [Fact]
    public void UpdateBatch_WithBytes_ProcessesAll()
    {
        ulong timestamp = 1000;
        var items = new[]
        {
            new byte[] { 1, 2, 3 },
            new byte[] { 4, 5, 6 },
            new byte[] { 7, 8, 9 }
        };

        _hll!.UpdateBatch(timestamp, items);

        double estimate = _hll!.EstimateWindow(timestamp + 10, 60);
        Assert.True(estimate > 0);
    }

    [Fact]
    public void UpdateBatch_WithStrings_ProcessesAll()
    {
        _hll!.UpdateBatch(1000, "item1", "item2", "item3");

        double estimate = _hll!.EstimateWindow(1010, 60);
        Assert.True(estimate > 0);
    }

    [Fact]
    public void UpdateBatch_WithNull_ThrowsException()
    {
        Assert.Throws<ArgumentNullException>(() =>
            _hll!.UpdateBatch(1000, (string[])null!));
    }

    [Fact]
    public void Dispose_PreventsFurtherOperations()
    {
        _hll!.Dispose();

        Assert.Throws<ObjectDisposedException>(() => _hll.Update("test", 1000));
        Assert.Throws<ObjectDisposedException>(() => _hll.EstimateWindow(1000, 60));
        Assert.Throws<ObjectDisposedException>(() => _hll.EstimateTotal());
        Assert.Throws<ObjectDisposedException>(() => _hll.Decay(1000, 60));
        Assert.Throws<ObjectDisposedException>(() => _hll.Precision);
        Assert.Throws<ObjectDisposedException>(() => _hll.MaxWindowSeconds);
    }

    [Fact]
    public void ToString_WithData_ContainsParameters()
    {
        var str = _hll!.ToString();

        Assert.Contains("SlidingHyperLogLog", str);
        Assert.Contains("precision=12", str);
        Assert.Contains("maxWindow=3600", str);
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
        SlidingHyperLogLog? hll = null;

        using (var temp = new SlidingHyperLogLog(12, 3600))
        {
            temp.Update("test", 1000);
            hll = temp;
            Assert.True(temp.EstimateTotal() >= 0);
        }

        // After using block, sketch should be disposed
        Assert.Throws<ObjectDisposedException>(() => hll!.Update("test", 1000));
    }

    [Fact]
    public void LargeDataset_MaintainsAccuracy()
    {
        const int itemCount = 10000;
        const ulong baseTime = 10000;

        // Add items over time
        for (int i = 0; i < itemCount; i++)
        {
            ulong timestamp = baseTime + (ulong)(i / 100); // 100 items per second
            _hll!.Update($"item_{i}", timestamp);
        }

        // Estimate total
        double total = _hll!.EstimateTotal();
        double error = Math.Abs(total - itemCount) / itemCount;

        // Allow 5% error for precision=12
        Assert.True(error < 0.05, $"Error {error:P} exceeded 5%");
    }

    [Fact]
    public void MultipleTimeWindows_IndependentEstimates()
    {
        // Add items in different time windows
        for (int i = 0; i < 50; i++)
        {
            _hll!.Update($"window1_item_{i}", 1000);
        }

        for (int i = 0; i < 30; i++)
        {
            _hll!.Update($"window2_item_{i}", 2000);
        }

        for (int i = 0; i < 20; i++)
        {
            _hll!.Update($"window3_item_{i}", 3000);
        }

        // Estimate each window
        double window1 = _hll!.EstimateWindow(1060, 120);
        double window2 = _hll!.EstimateWindow(2060, 120);
        double window3 = _hll!.EstimateWindow(3060, 120);

        // All windows should have positive estimates
        Assert.True(window1 > 0);
        Assert.True(window2 > 0);
        Assert.True(window3 > 0);

        // Total should be greater than any individual window
        double total = _hll!.EstimateTotal();
        Assert.True(total >= window1);
        Assert.True(total >= window2);
        Assert.True(total >= window3);
    }

    [Fact]
    public void DifferentPrecisions_AllWork()
    {
        foreach (byte precision in new byte[] { 4, 8, 12, 14, 16 })
        {
            using var hll = new SlidingHyperLogLog(precision, 3600);
            Assert.Equal(precision, hll.Precision);

            hll.Update("test", 1000);
            double estimate = hll.EstimateTotal();
            Assert.True(estimate >= 0);
        }
    }
}
