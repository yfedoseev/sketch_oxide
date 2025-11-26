using System;
using SketchOxide.Frequency;
using Xunit;

namespace SketchOxide.Tests;

/// <summary>
/// Unit tests for HeavyKeeper top-k heavy hitter detection.
/// </summary>
public class HeavyKeeperTests : IDisposable
{
    private HeavyKeeper? _hk;

    public HeavyKeeperTests()
    {
        _hk = new HeavyKeeper(10, 0.001, 0.01);
    }

    public void Dispose()
    {
        _hk?.Dispose();
    }

    [Fact]
    public void Constructor_ValidParameters_CreatesSketch()
    {
        using var hk = new HeavyKeeper(10, 0.001, 0.01);
        Assert.Equal(10u, hk.K);
    }

    [Theory]
    [InlineData(0u, 0.001, 0.01)]     // Invalid k
    [InlineData(10u, 0.0, 0.01)]      // Invalid epsilon (too low)
    [InlineData(10u, 1.0, 0.01)]      // Invalid epsilon (too high)
    [InlineData(10u, 0.001, 0.0)]     // Invalid delta (too low)
    [InlineData(10u, 0.001, 1.0)]     // Invalid delta (too high)
    public void Constructor_InvalidParameters_ThrowsException(uint k, double epsilon, double delta)
    {
        Assert.ThrowsAny<ArgumentOutOfRangeException>(() => new HeavyKeeper(k, epsilon, delta));
    }

    [Fact]
    public void Update_WithBytes_DoesNotThrow()
    {
        _hk!.Update(new byte[] { 1, 2, 3, 4 });
        Assert.True(_hk.Estimate(new byte[] { 1, 2, 3, 4 }) >= 0);
    }

    [Fact]
    public void Update_WithString_DoesNotThrow()
    {
        _hk!.Update("test-item");
        Assert.True(_hk.Estimate("test-item") >= 0);
    }

    [Fact]
    public void Update_WithNull_ThrowsArgumentNullException()
    {
        Assert.Throws<ArgumentNullException>(() => _hk!.Update((string)null!));
    }

    [Fact]
    public void Estimate_ReturnsFrequencyCount()
    {
        for (int i = 0; i < 100; i++)
        {
            _hk!.Update("frequent-item");
        }
        for (int i = 0; i < 5; i++)
        {
            _hk!.Update("rare-item");
        }

        uint frequentCount = _hk!.Estimate("frequent-item");
        uint rareCount = _hk!.Estimate("rare-item");

        Assert.True(frequentCount > rareCount, "Frequent item should have higher count");
        Assert.True(frequentCount >= 90, "Frequent item count should be close to 100");
    }

    [Fact]
    public void TopK_ReturnsHeavyHitters()
    {
        // Add items with different frequencies
        for (int i = 0; i < 1000; i++)
        {
            _hk!.Update($"item_{i % 5}"); // 5 items, each appearing 200 times
        }

        var topK = _hk!.TopK();
        Assert.NotEmpty(topK);
        Assert.True(topK.Length <= 10, "Should return at most k items");

        // Verify top items have counts
        foreach (var (hash, count) in topK)
        {
            Assert.True(count > 0, "Top-k items should have positive counts");
        }
    }

    [Fact]
    public void TopK_EmptySketch_ReturnsEmptyArray()
    {
        var topK = _hk!.TopK();
        Assert.Empty(topK);
    }

    [Fact]
    public void Decay_ReducesCounts()
    {
        // Add items
        for (int i = 0; i < 100; i++)
        {
            _hk!.Update("test-item");
        }

        uint countBefore = _hk!.Estimate("test-item");

        // Apply decay
        _hk!.Decay();

        uint countAfter = _hk!.Estimate("test-item");

        // Count should be reduced or stay same
        Assert.True(countAfter <= countBefore, "Decay should reduce or maintain counts");
    }

    [Fact]
    public void UpdateBatch_WithBytes_ProcessesAllItems()
    {
        var items = new[]
        {
            new byte[] { 1, 2, 3 },
            new byte[] { 4, 5, 6 },
            new byte[] { 7, 8, 9 }
        };

        _hk!.UpdateBatch(items);

        // Verify all items have been counted
        foreach (var item in items)
        {
            Assert.True(_hk!.Estimate(item) >= 1);
        }
    }

    [Fact]
    public void UpdateBatch_WithStrings_ProcessesAllItems()
    {
        _hk!.UpdateBatch("item1", "item2", "item3");

        Assert.True(_hk!.Estimate("item1") >= 1);
        Assert.True(_hk!.Estimate("item2") >= 1);
        Assert.True(_hk!.Estimate("item3") >= 1);
    }

    [Fact]
    public void UpdateBatch_WithNull_ThrowsException()
    {
        Assert.Throws<ArgumentNullException>(() => _hk!.UpdateBatch((string[])null!));
    }

    [Fact]
    public void Dispose_PreventsFurtherOperations()
    {
        _hk!.Dispose();

        Assert.Throws<ObjectDisposedException>(() => _hk.Update("test"));
        Assert.Throws<ObjectDisposedException>(() => _hk.Estimate("test"));
        Assert.Throws<ObjectDisposedException>(() => _hk.TopK());
        Assert.Throws<ObjectDisposedException>(() => _hk.Decay());
        Assert.Throws<ObjectDisposedException>(() => _hk.K);
    }

    [Fact]
    public void ToString_WithData_ContainsParameters()
    {
        var str = _hk!.ToString();

        Assert.Contains("HeavyKeeper", str);
        Assert.Contains("k=10", str);
    }

    [Fact]
    public void ToString_WhenDisposed_IndicatesDisposed()
    {
        _hk!.Dispose();
        var str = _hk.ToString();

        Assert.Contains("disposed", str, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void UsingPattern_AutomaticallyCleansUp()
    {
        HeavyKeeper? sketch = null;

        using (var hk = new HeavyKeeper(5, 0.001, 0.01))
        {
            hk.Update("test");
            sketch = hk;
            Assert.True(hk.Estimate("test") > 0);
        }

        // After using block, sketch should be disposed
        Assert.Throws<ObjectDisposedException>(() => sketch!.Update("test"));
    }

    [Fact]
    public void LargeDataset_MaintainsAccuracy()
    {
        const int itemCount = 10000;
        const int uniqueItems = 100;

        // Create Zipfian distribution
        for (int i = 0; i < itemCount; i++)
        {
            int itemId = i % uniqueItems;
            _hk!.Update($"item_{itemId}");
        }

        // Verify most frequent items
        var topK = _hk!.TopK();
        Assert.NotEmpty(topK);

        // Top items should have high counts
        if (topK.Length > 0)
        {
            var (_, count) = topK[0];
            Assert.True(count > 50, "Top item should have substantial count");
        }
    }
}
