using System;
using SketchOxide.Frequency;
using Xunit;

namespace SketchOxide.Tests;

/// <summary>
/// Unit tests for CountMinSketch frequency estimator.
/// Tests cover constructor validation, update operations, estimation accuracy,
/// serialization/deserialization, merge functionality, and disposal behavior.
/// </summary>
public class CountMinSketchTests : IDisposable
{
    private CountMinSketch? _sketch;

    public CountMinSketchTests()
    {
        _sketch = new CountMinSketch(0.001, 0.01);
    }

    public void Dispose()
    {
        _sketch?.Dispose();
    }

    #region Constructor Tests

    [Fact]
    public void Constructor_ValidParameters_CreatesSketch()
    {
        using var sketch = new CountMinSketch(0.01, 0.001);
        Assert.Equal(0.01, sketch.Epsilon);
        Assert.Equal(0.001, sketch.Delta);
    }

    [Theory]
    [InlineData(0.0, 0.01)]    // Epsilon = 0
    [InlineData(-0.1, 0.01)]   // Epsilon < 0
    [InlineData(1.0, 0.01)]    // Epsilon = 1
    [InlineData(1.5, 0.01)]    // Epsilon > 1
    public void Constructor_InvalidEpsilon_ThrowsArgumentOutOfRangeException(double epsilon, double delta)
    {
        var ex = Assert.Throws<ArgumentOutOfRangeException>(() => new CountMinSketch(epsilon, delta));
        Assert.Equal("epsilon", ex.ParamName);
        Assert.Contains("Epsilon must be in range (0, 1)", ex.Message);
    }

    [Theory]
    [InlineData(0.01, 0.0)]    // Delta = 0
    [InlineData(0.01, -0.1)]   // Delta < 0
    [InlineData(0.01, 1.0)]    // Delta = 1
    [InlineData(0.01, 1.5)]    // Delta > 1
    public void Constructor_InvalidDelta_ThrowsArgumentOutOfRangeException(double epsilon, double delta)
    {
        var ex = Assert.Throws<ArgumentOutOfRangeException>(() => new CountMinSketch(epsilon, delta));
        Assert.Equal("delta", ex.ParamName);
        Assert.Contains("Delta must be in range (0, 1)", ex.Message);
    }

    [Theory]
    [InlineData(0.001, 0.001)]  // High accuracy
    [InlineData(0.01, 0.01)]    // Standard accuracy
    [InlineData(0.1, 0.1)]      // Lower accuracy
    [InlineData(0.5, 0.5)]      // Minimum accuracy
    [InlineData(0.999, 0.999)]  // Near boundary
    public void Constructor_ValidBoundaryParameters_Succeeds(double epsilon, double delta)
    {
        using var sketch = new CountMinSketch(epsilon, delta);
        Assert.Equal(epsilon, sketch.Epsilon);
        Assert.Equal(delta, sketch.Delta);
    }

    #endregion

    #region Update Tests

    [Fact]
    public void Update_WithBytes_DoesNotThrow()
    {
        _sketch!.Update(new byte[] { 1, 2, 3, 4 });
        var estimate = _sketch.Estimate(new byte[] { 1, 2, 3, 4 });
        Assert.True(estimate >= 1, "Estimate should be at least the actual count");
    }

    [Fact]
    public void Update_WithString_DoesNotThrow()
    {
        _sketch!.Update("test-item");
        var estimate = _sketch.Estimate("test-item");
        Assert.True(estimate >= 1, "Estimate should be at least the actual count");
    }

    [Fact]
    public void Update_WithNullBytes_ThrowsArgumentNullException()
    {
        Assert.Throws<ArgumentNullException>(() => _sketch!.Update((byte[])null!));
    }

    [Fact]
    public void Update_WithNullString_ThrowsArgumentNullException()
    {
        Assert.Throws<ArgumentNullException>(() => _sketch!.Update((string)null!));
    }

    [Fact]
    public void Update_WithEmptyBytes_DoesNotThrow()
    {
        _sketch!.Update(Array.Empty<byte>());
        var estimate = _sketch.Estimate(Array.Empty<byte>());
        Assert.True(estimate >= 1, "Estimate should be at least 1 for empty bytes");
    }

    [Fact]
    public void Update_WithEmptyString_DoesNotThrow()
    {
        _sketch!.Update(string.Empty);
        var estimate = _sketch.Estimate(string.Empty);
        Assert.True(estimate >= 1, "Estimate should be at least 1 for empty string");
    }

    #endregion

    #region Estimate Tests

    [Fact]
    public void Estimate_NeverUnderestimates()
    {
        const int actualCount = 100;
        for (int i = 0; i < actualCount; i++)
        {
            _sketch!.Update("frequent-item");
        }

        var estimate = _sketch!.Estimate("frequent-item");
        Assert.True(estimate >= (ulong)actualCount,
            $"Count-Min Sketch should never underestimate. Actual: {actualCount}, Estimate: {estimate}");
    }

    [Fact]
    public void Estimate_UnseenItem_ReturnsZeroOrSmallValue()
    {
        // Add some items to the sketch
        for (int i = 0; i < 100; i++)
        {
            _sketch!.Update($"item-{i}");
        }

        // Query an item that was never added
        var estimate = _sketch!.Estimate("never-added-item");

        // For a well-configured sketch, unseen items should have low estimates
        // With epsilon=0.001, delta=0.01, the false estimate should be small relative to total count
        Assert.True(estimate <= 10, $"Unseen item estimate should be small, got {estimate}");
    }

    [Fact]
    public void Estimate_WithNullBytes_ThrowsArgumentNullException()
    {
        Assert.Throws<ArgumentNullException>(() => _sketch!.Estimate((byte[])null!));
    }

    [Fact]
    public void Estimate_WithNullString_ThrowsArgumentNullException()
    {
        Assert.Throws<ArgumentNullException>(() => _sketch!.Estimate((string)null!));
    }

    [Fact]
    public void Estimate_Monotonicity_IncreasesWithUpdates()
    {
        ulong previousEstimate = 0;

        for (int i = 1; i <= 50; i++)
        {
            _sketch!.Update("monotonic-item");
            var currentEstimate = _sketch.Estimate("monotonic-item");

            Assert.True(currentEstimate >= previousEstimate,
                $"Estimate should be monotonically non-decreasing. Previous: {previousEstimate}, Current: {currentEstimate}");
            Assert.True(currentEstimate >= (ulong)i,
                $"Estimate should never underestimate actual count. Actual: {i}, Estimate: {currentEstimate}");

            previousEstimate = currentEstimate;
        }
    }

    [Fact]
    public void Estimate_MultipleItems_TracksSeparately()
    {
        // Add different counts for different items
        for (int i = 0; i < 100; i++) _sketch!.Update("item-a");
        for (int i = 0; i < 50; i++) _sketch!.Update("item-b");
        for (int i = 0; i < 10; i++) _sketch!.Update("item-c");

        var estimateA = _sketch!.Estimate("item-a");
        var estimateB = _sketch.Estimate("item-b");
        var estimateC = _sketch.Estimate("item-c");

        Assert.True(estimateA >= 100, $"Item A estimate should be >= 100, got {estimateA}");
        Assert.True(estimateB >= 50, $"Item B estimate should be >= 50, got {estimateB}");
        Assert.True(estimateC >= 10, $"Item C estimate should be >= 10, got {estimateC}");
    }

    #endregion

    #region Merge Tests

    [Fact]
    public void Merge_CompatibleSketches_CombinesCounts()
    {
        using var other = new CountMinSketch(0.001, 0.01);

        for (int i = 0; i < 50; i++)
        {
            _sketch!.Update("shared-item");
            other.Update("shared-item");
        }

        var beforeMerge = _sketch!.Estimate("shared-item");
        _sketch.Merge(other);
        var afterMerge = _sketch.Estimate("shared-item");

        Assert.True(afterMerge >= 100, $"Merged count should be >= 100, got {afterMerge}");
        Assert.True(afterMerge >= beforeMerge, "Merged estimate should be >= original estimate");
    }

    [Fact]
    public void Merge_IncompatibleEpsilon_ThrowsArgumentException()
    {
        using var other = new CountMinSketch(0.01, 0.01); // Different epsilon

        var ex = Assert.Throws<ArgumentException>(() => _sketch!.Merge(other));
        Assert.Contains("Cannot merge sketches with different parameters", ex.Message);
    }

    [Fact]
    public void Merge_IncompatibleDelta_ThrowsArgumentException()
    {
        using var other = new CountMinSketch(0.001, 0.001); // Different delta

        var ex = Assert.Throws<ArgumentException>(() => _sketch!.Merge(other));
        Assert.Contains("Cannot merge sketches with different parameters", ex.Message);
    }

    [Fact]
    public void Merge_WithNull_ThrowsArgumentNullException()
    {
        var ex = Assert.Throws<ArgumentNullException>(() => _sketch!.Merge(null!));
        Assert.Equal("other", ex.ParamName);
    }

    [Fact]
    public void Merge_WithDisposedOther_ThrowsObjectDisposedException()
    {
        var other = new CountMinSketch(0.001, 0.01);
        other.Dispose();

        Assert.Throws<ObjectDisposedException>(() => _sketch!.Merge(other));
    }

    [Fact]
    public void Merge_DisjointSets_PreservesBothCounts()
    {
        using var other = new CountMinSketch(0.001, 0.01);

        for (int i = 0; i < 30; i++) _sketch!.Update("only-in-first");
        for (int i = 0; i < 40; i++) other.Update("only-in-second");

        _sketch!.Merge(other);

        var estimateFirst = _sketch.Estimate("only-in-first");
        var estimateSecond = _sketch.Estimate("only-in-second");

        Assert.True(estimateFirst >= 30, $"First item estimate should be >= 30, got {estimateFirst}");
        Assert.True(estimateSecond >= 40, $"Second item estimate should be >= 40, got {estimateSecond}");
    }

    #endregion

    #region Serialization Tests

    [Fact]
    public void Serialize_WithData_ReturnsNonEmptyBytes()
    {
        for (int i = 0; i < 100; i++)
        {
            _sketch!.Update($"item-{i}");
        }

        var serialized = _sketch!.Serialize();
        Assert.NotNull(serialized);
        Assert.True(serialized.Length > 0, "Serialized data should not be empty");
    }

    [Fact]
    public void Serialize_EmptySketch_ReturnsBytes()
    {
        var serialized = _sketch!.Serialize();
        Assert.NotNull(serialized);
        Assert.True(serialized.Length > 0, "Even empty sketch should serialize");
    }

    [Fact]
    public void SerializationRoundTrip_PreservesEstimates()
    {
        for (int i = 0; i < 100; i++)
        {
            _sketch!.Update("test-item");
        }
        for (int i = 0; i < 50; i++)
        {
            _sketch!.Update("another-item");
        }

        var originalEstimate1 = _sketch!.Estimate("test-item");
        var originalEstimate2 = _sketch.Estimate("another-item");

        var serialized = _sketch.Serialize();
        using var restored = CountMinSketch.Deserialize(serialized, 0.001, 0.01);

        var restoredEstimate1 = restored.Estimate("test-item");
        var restoredEstimate2 = restored.Estimate("another-item");

        Assert.Equal(originalEstimate1, restoredEstimate1);
        Assert.Equal(originalEstimate2, restoredEstimate2);
    }

    [Fact]
    public void Deserialize_InvalidData_ThrowsArgumentException()
    {
        var invalidData = new byte[] { 1, 2, 3 };

        var ex = Assert.Throws<ArgumentException>(() => CountMinSketch.Deserialize(invalidData, 0.01, 0.01));
        Assert.Contains("Failed to deserialize", ex.Message);
    }

    [Fact]
    public void Deserialize_NullData_ThrowsArgumentNullException()
    {
        var ex = Assert.Throws<ArgumentNullException>(() => CountMinSketch.Deserialize(null!, 0.01, 0.01));
        Assert.Equal("data", ex.ParamName);
    }

    #endregion

    #region Disposal Tests

    [Fact]
    public void Dispose_PreventsFurtherOperations()
    {
        _sketch!.Dispose();

        Assert.Throws<ObjectDisposedException>(() => _sketch.Update("test"));
        Assert.Throws<ObjectDisposedException>(() => _sketch.Estimate("test"));
        Assert.Throws<ObjectDisposedException>(() => _ = _sketch.Epsilon);
        Assert.Throws<ObjectDisposedException>(() => _ = _sketch.Delta);
    }

    [Fact]
    public void Dispose_CanBeCalledMultipleTimes()
    {
        _sketch!.Dispose();
        _sketch.Dispose(); // Should not throw
        _sketch.Dispose(); // Should not throw
    }

    [Fact]
    public void ToString_WithData_ContainsParameters()
    {
        _sketch!.Update("test");
        var str = _sketch.ToString();

        Assert.Contains("CountMinSketch", str);
        Assert.Contains("epsilon", str, StringComparison.OrdinalIgnoreCase);
        Assert.Contains("delta", str, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void ToString_WhenDisposed_IndicatesDisposed()
    {
        _sketch!.Dispose();
        var str = _sketch.ToString();

        Assert.Contains("disposed", str, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void UsingPattern_AutomaticallyCleansUp()
    {
        CountMinSketch? sketch = null;

        using (var cms = new CountMinSketch(0.01, 0.01))
        {
            cms.Update("test");
            sketch = cms;
            Assert.True(cms.Estimate("test") >= 1);
        }

        // After using block, sketch should be disposed
        Assert.Throws<ObjectDisposedException>(() => sketch!.Update("test"));
    }

    #endregion

    #region Large Dataset Tests

    [Fact]
    public void LargeDataset_MaintainsAccuracy()
    {
        using var largeSketch = new CountMinSketch(0.001, 0.01);
        const int uniqueItems = 10000;
        const int frequentItemCount = 1000;

        // Add many unique items
        for (int i = 0; i < uniqueItems; i++)
        {
            largeSketch.Update($"unique-{i}");
        }

        // Add a frequent item many times
        for (int i = 0; i < frequentItemCount; i++)
        {
            largeSketch.Update("frequent-item");
        }

        var frequentEstimate = largeSketch.Estimate("frequent-item");

        // Should never underestimate
        Assert.True(frequentEstimate >= (ulong)frequentItemCount,
            $"Estimate {frequentEstimate} should be >= actual {frequentItemCount}");

        // Error bound: epsilon * totalCount
        // Total count = uniqueItems + frequentItemCount = 11000
        // Expected max overestimate = 0.001 * 11000 = 11 (with high probability)
        var maxExpectedEstimate = (ulong)(frequentItemCount + 0.001 * (uniqueItems + frequentItemCount) * 10);
        Assert.True(frequentEstimate <= maxExpectedEstimate,
            $"Estimate {frequentEstimate} exceeds reasonable bound {maxExpectedEstimate}");
    }

    [Fact]
    public void HighFrequencyTracking_AccurateForHeavyHitters()
    {
        using var sketch = new CountMinSketch(0.0001, 0.001);

        // Zipfian-like distribution
        var counts = new[] { 10000, 5000, 2500, 1250, 625, 312, 156, 78 };

        for (int itemIdx = 0; itemIdx < counts.Length; itemIdx++)
        {
            for (int j = 0; j < counts[itemIdx]; j++)
            {
                sketch.Update($"zipf-item-{itemIdx}");
            }
        }

        // Verify ordering is preserved and no underestimation
        for (int i = 0; i < counts.Length; i++)
        {
            var estimate = sketch.Estimate($"zipf-item-{i}");
            Assert.True(estimate >= (ulong)counts[i],
                $"Item {i} with count {counts[i]} got estimate {estimate}");
        }
    }

    #endregion
}
