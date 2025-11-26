using System;
using System.Linq;
using SketchOxide.Quantiles;
using Xunit;

namespace SketchOxide.Tests;

/// <summary>
/// Unit tests for DDSketch quantile estimator.
/// Tests cover constructor validation, update operations, quantile queries,
/// statistical accuracy, serialization/deserialization, and disposal.
/// </summary>
public class DDSketchTests : IDisposable
{
    private DDSketch? _sketch;

    public DDSketchTests()
    {
        _sketch = new DDSketch(0.01);
    }

    public void Dispose()
    {
        _sketch?.Dispose();
    }

    #region Constructor Tests

    [Fact]
    public void Constructor_ValidRelativeAccuracy_CreatesSketch()
    {
        using var sketch = new DDSketch(0.01);
        Assert.Equal(0.01, sketch.RelativeAccuracy);
    }

    [Theory]
    [InlineData(0.0)]    // Accuracy = 0
    [InlineData(-0.1)]   // Accuracy < 0
    [InlineData(1.0)]    // Accuracy = 1
    [InlineData(1.5)]    // Accuracy > 1
    public void Constructor_InvalidRelativeAccuracy_ThrowsArgumentOutOfRangeException(double accuracy)
    {
        var ex = Assert.Throws<ArgumentOutOfRangeException>(() => new DDSketch(accuracy));
        Assert.Equal("relativeAccuracy", ex.ParamName);
        Assert.Contains("Relative accuracy must be in range (0, 1)", ex.Message);
    }

    [Theory]
    [InlineData(0.001)]   // High accuracy (0.1% error)
    [InlineData(0.01)]    // Standard accuracy (1% error)
    [InlineData(0.05)]    // Medium accuracy (5% error)
    [InlineData(0.1)]     // Lower accuracy (10% error)
    [InlineData(0.5)]     // Low accuracy (50% error)
    [InlineData(0.999)]   // Near boundary
    public void Constructor_ValidBoundaryAccuracy_Succeeds(double accuracy)
    {
        using var sketch = new DDSketch(accuracy);
        Assert.Equal(accuracy, sketch.RelativeAccuracy);
    }

    #endregion

    #region Update Tests

    [Fact]
    public void Update_WithPositiveValue_DoesNotThrow()
    {
        _sketch!.Update(100.0);
        var median = _sketch.Median();
        Assert.True(median > 0, "Median should be positive after adding positive value");
    }

    [Fact]
    public void Update_WithZero_DoesNotThrow()
    {
        _sketch!.Update(0.0);
        // Zero value should be handled
        var min = _sketch.Min();
        Assert.True(min <= 0.001, "Min should be near zero after adding zero");
    }

    [Fact]
    public void Update_WithNegativeValue_DoesNotThrow()
    {
        _sketch!.Update(-100.0);
        var min = _sketch.Min();
        Assert.True(min < 0, "Min should be negative after adding negative value");
    }

    [Fact]
    public void Update_MultipleValues_TracksDistribution()
    {
        for (int i = 1; i <= 100; i++)
        {
            _sketch!.Update(i);
        }

        var min = _sketch!.Min();
        var max = _sketch.Max();
        var median = _sketch.Median();

        Assert.True(min <= 2, $"Min should be near 1, got {min}");
        Assert.True(max >= 99, $"Max should be near 100, got {max}");
        Assert.True(median >= 40 && median <= 60, $"Median should be near 50, got {median}");
    }

    [Fact]
    public void Update_VerySmallValues_HandlesCorrectly()
    {
        _sketch!.Update(0.0001);
        _sketch.Update(0.0002);
        _sketch.Update(0.0003);

        var median = _sketch.Median();
        Assert.True(median > 0 && median < 0.001, $"Should handle very small values, got median {median}");
    }

    [Fact]
    public void Update_VeryLargeValues_HandlesCorrectly()
    {
        _sketch!.Update(1e9);
        _sketch.Update(2e9);
        _sketch.Update(3e9);

        var median = _sketch.Median();
        Assert.True(median > 1e9 && median < 3e9, $"Should handle very large values, got median {median}");
    }

    #endregion

    #region Quantile Tests

    [Fact]
    public void Quantile_ValidRange_ReturnsValue()
    {
        for (int i = 1; i <= 100; i++)
        {
            _sketch!.Update(i);
        }

        var q50 = _sketch!.Quantile(0.5);
        Assert.True(q50 > 0, "Quantile should return positive value for positive data");
    }

    [Theory]
    [InlineData(-0.1)]   // Below range
    [InlineData(1.1)]    // Above range
    [InlineData(-1.0)]   // Far below range
    [InlineData(2.0)]    // Far above range
    public void Quantile_OutOfRange_ThrowsArgumentOutOfRangeException(double q)
    {
        _sketch!.Update(100);
        var ex = Assert.Throws<ArgumentOutOfRangeException>(() => _sketch.Quantile(q));
        Assert.Equal("q", ex.ParamName);
        Assert.Contains("Quantile must be in range [0, 1]", ex.Message);
    }

    [Fact]
    public void Quantile_BoundaryValues_Succeed()
    {
        for (int i = 1; i <= 100; i++)
        {
            _sketch!.Update(i);
        }

        // These should not throw
        var q0 = _sketch!.Quantile(0.0);    // Minimum
        var q100 = _sketch.Quantile(1.0);   // Maximum

        Assert.True(q0 <= q100, $"q0 ({q0}) should be <= q100 ({q100})");
    }

    [Fact]
    public void Quantile_Ordering_Preserved()
    {
        // Add values from 1 to 1000
        for (int i = 1; i <= 1000; i++)
        {
            _sketch!.Update(i);
        }

        var p10 = _sketch!.Quantile(0.10);
        var p25 = _sketch.Quantile(0.25);
        var p50 = _sketch.Quantile(0.50);
        var p75 = _sketch.Quantile(0.75);
        var p90 = _sketch.Quantile(0.90);
        var p99 = _sketch.Quantile(0.99);

        Assert.True(p10 <= p25, $"p10 ({p10}) should be <= p25 ({p25})");
        Assert.True(p25 <= p50, $"p25 ({p25}) should be <= p50 ({p50})");
        Assert.True(p50 <= p75, $"p50 ({p50}) should be <= p75 ({p75})");
        Assert.True(p75 <= p90, $"p75 ({p75}) should be <= p90 ({p90})");
        Assert.True(p90 <= p99, $"p90 ({p90}) should be <= p99 ({p99})");
    }

    [Fact]
    public void Quantile_RelativeAccuracyGuarantee()
    {
        const double relativeAccuracy = 0.01;
        using var sketch = new DDSketch(relativeAccuracy);

        // Add values 1 to 10000
        for (int i = 1; i <= 10000; i++)
        {
            sketch.Update(i);
        }

        // Test specific quantiles
        var testQuantiles = new[] { 0.25, 0.50, 0.75, 0.90, 0.95, 0.99 };

        foreach (var q in testQuantiles)
        {
            var estimated = sketch.Quantile(q);
            var expected = 10000.0 * q;

            // Check relative error is within bounds
            var relativeError = Math.Abs(estimated - expected) / expected;

            // Allow some tolerance (2x the relative accuracy) for edge cases
            Assert.True(relativeError <= relativeAccuracy * 2,
                $"Quantile {q}: estimated={estimated}, expected~{expected}, error={relativeError:P2}");
        }
    }

    #endregion

    #region Min/Max/Median Tests

    [Fact]
    public void Min_ReturnsMinimumValue()
    {
        _sketch!.Update(50);
        _sketch.Update(10);
        _sketch.Update(90);
        _sketch.Update(5);
        _sketch.Update(75);

        var min = _sketch.Min();

        // With relative accuracy of 0.01 (1%), min should be within 1% of 5
        Assert.True(min >= 4.95 && min <= 5.05, $"Min should be near 5, got {min}");
    }

    [Fact]
    public void Max_ReturnsMaximumValue()
    {
        _sketch!.Update(50);
        _sketch.Update(10);
        _sketch.Update(90);
        _sketch.Update(5);
        _sketch.Update(75);

        var max = _sketch.Max();

        // With relative accuracy of 0.01 (1%), max should be within 1% of 90
        Assert.True(max >= 89.1 && max <= 90.9, $"Max should be near 90, got {max}");
    }

    [Fact]
    public void Median_ReturnsMiddleValue()
    {
        // Add odd number of sequential values
        for (int i = 1; i <= 101; i++)
        {
            _sketch!.Update(i);
        }

        var median = _sketch!.Median();

        // Median should be near 51 (the actual middle value)
        Assert.True(median >= 45 && median <= 57, $"Median should be near 51, got {median}");
    }

    [Fact]
    public void MinMaxMedian_SingleValue_AllEqual()
    {
        _sketch!.Update(42.0);

        var min = _sketch.Min();
        var max = _sketch.Max();
        var median = _sketch.Median();

        Assert.Equal(min, max);
        Assert.Equal(min, median);
    }

    [Fact]
    public void MinMaxMedian_ConsistentWithQuantile()
    {
        for (int i = 1; i <= 100; i++)
        {
            _sketch!.Update(i);
        }

        var min = _sketch!.Min();
        var max = _sketch.Max();
        var median = _sketch.Median();

        var q0 = _sketch.Quantile(0.0);
        var q100 = _sketch.Quantile(1.0);
        var q50 = _sketch.Quantile(0.5);

        Assert.Equal(min, q0);
        Assert.Equal(max, q100);
        Assert.Equal(median, q50);
    }

    #endregion

    #region Distribution Tests

    [Fact]
    public void UniformDistribution_QuantilesAreLinear()
    {
        var random = new Random(42);

        // Generate uniform distribution [0, 1000]
        for (int i = 0; i < 10000; i++)
        {
            _sketch!.Update(random.NextDouble() * 1000);
        }

        var p25 = _sketch!.Quantile(0.25);
        var p50 = _sketch.Quantile(0.50);
        var p75 = _sketch.Quantile(0.75);

        // For uniform [0, 1000], expected: p25~250, p50~500, p75~750
        Assert.True(p25 >= 200 && p25 <= 300, $"p25 should be ~250, got {p25}");
        Assert.True(p50 >= 450 && p50 <= 550, $"p50 should be ~500, got {p50}");
        Assert.True(p75 >= 700 && p75 <= 800, $"p75 should be ~750, got {p75}");
    }

    [Fact]
    public void LatencyDistribution_TracksPercentiles()
    {
        // Simulate latency data: most values low, some high (typical service latencies)
        var random = new Random(42);

        for (int i = 0; i < 9000; i++)
        {
            // 90% of requests: 1-100ms
            _sketch!.Update(random.NextDouble() * 100);
        }

        for (int i = 0; i < 900; i++)
        {
            // 9% of requests: 100-500ms
            _sketch!.Update(100 + random.NextDouble() * 400);
        }

        for (int i = 0; i < 100; i++)
        {
            // 1% of requests: 500-2000ms (tail latency)
            _sketch!.Update(500 + random.NextDouble() * 1500);
        }

        var p50 = _sketch!.Quantile(0.50);
        var p90 = _sketch.Quantile(0.90);
        var p99 = _sketch.Quantile(0.99);

        // Verify ordering and reasonable ranges
        Assert.True(p50 < 100, $"p50 should be < 100ms, got {p50}");
        Assert.True(p90 >= 50 && p90 <= 200, $"p90 should be 50-200ms range, got {p90}");
        Assert.True(p99 > p90, $"p99 ({p99}) should be > p90 ({p90})");
    }

    [Fact]
    public void BimodalDistribution_HandlesCorrectly()
    {
        // Two clusters: around 100 and around 1000
        for (int i = 0; i < 5000; i++)
        {
            _sketch!.Update(100 + (i % 20)); // Values 100-119
        }
        for (int i = 0; i < 5000; i++)
        {
            _sketch!.Update(1000 + (i % 20)); // Values 1000-1019
        }

        var p25 = _sketch!.Quantile(0.25);
        var p75 = _sketch.Quantile(0.75);

        // p25 should be in lower cluster, p75 in upper
        Assert.True(p25 <= 150, $"p25 should be in lower cluster (<=150), got {p25}");
        Assert.True(p75 >= 950, $"p75 should be in upper cluster (>=950), got {p75}");
    }

    #endregion

    #region Serialization Tests

    [Fact]
    public void Serialize_WithData_ReturnsNonEmptyBytes()
    {
        for (int i = 1; i <= 100; i++)
        {
            _sketch!.Update(i);
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
    public void SerializationRoundTrip_PreservesQuantiles()
    {
        for (int i = 1; i <= 1000; i++)
        {
            _sketch!.Update(i);
        }

        var originalP50 = _sketch!.Quantile(0.50);
        var originalP99 = _sketch.Quantile(0.99);
        var originalMin = _sketch.Min();
        var originalMax = _sketch.Max();

        var serialized = _sketch.Serialize();
        using var restored = DDSketch.Deserialize(serialized, 0.01);

        var restoredP50 = restored.Quantile(0.50);
        var restoredP99 = restored.Quantile(0.99);
        var restoredMin = restored.Min();
        var restoredMax = restored.Max();

        Assert.Equal(originalP50, restoredP50, precision: 4);
        Assert.Equal(originalP99, restoredP99, precision: 4);
        Assert.Equal(originalMin, restoredMin, precision: 4);
        Assert.Equal(originalMax, restoredMax, precision: 4);
    }

    [Fact]
    public void Deserialize_InvalidData_ThrowsArgumentException()
    {
        var invalidData = new byte[] { 1, 2, 3 };

        var ex = Assert.Throws<ArgumentException>(() => DDSketch.Deserialize(invalidData, 0.01));
        Assert.Contains("Failed to deserialize", ex.Message);
    }

    [Fact]
    public void Deserialize_NullData_ThrowsArgumentNullException()
    {
        var ex = Assert.Throws<ArgumentNullException>(() => DDSketch.Deserialize(null!, 0.01));
        Assert.Equal("data", ex.ParamName);
    }

    #endregion

    #region Disposal Tests

    [Fact]
    public void Dispose_PreventsFurtherOperations()
    {
        _sketch!.Dispose();

        Assert.Throws<ObjectDisposedException>(() => _sketch.Update(100));
        Assert.Throws<ObjectDisposedException>(() => _sketch.Quantile(0.5));
        Assert.Throws<ObjectDisposedException>(() => _sketch.Min());
        Assert.Throws<ObjectDisposedException>(() => _sketch.Max());
        Assert.Throws<ObjectDisposedException>(() => _sketch.Median());
        Assert.Throws<ObjectDisposedException>(() => _ = _sketch.RelativeAccuracy);
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
        _sketch!.Update(100);
        var str = _sketch.ToString();

        Assert.Contains("DDSketch", str);
        Assert.Contains("relativeAccuracy", str, StringComparison.OrdinalIgnoreCase);
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
        DDSketch? sketch = null;

        using (var dd = new DDSketch(0.01))
        {
            dd.Update(100);
            sketch = dd;
            Assert.True(dd.Quantile(0.5) > 0);
        }

        // After using block, sketch should be disposed
        Assert.Throws<ObjectDisposedException>(() => sketch!.Update(100));
    }

    #endregion

    #region Large Dataset Tests

    [Fact]
    public void LargeDataset_MaintainsAccuracy()
    {
        using var largeSketch = new DDSketch(0.01);
        const int count = 1000000;

        for (int i = 1; i <= count; i++)
        {
            largeSketch.Update(i);
        }

        var min = largeSketch.Min();
        var max = largeSketch.Max();
        var median = largeSketch.Median();

        // With 1% relative accuracy
        Assert.True(min <= 1.02, $"Min should be near 1, got {min}");
        Assert.True(max >= count * 0.98, $"Max should be near {count}, got {max}");
        Assert.True(median >= count * 0.49 && median <= count * 0.51,
            $"Median should be near {count / 2}, got {median}");
    }

    [Fact]
    public void HighPrecision_SmallRelativeAccuracy()
    {
        using var preciseSketch = new DDSketch(0.001); // 0.1% relative accuracy

        for (int i = 1; i <= 10000; i++)
        {
            preciseSketch.Update(i);
        }

        var p50 = preciseSketch.Quantile(0.5);
        var expected = 5000.0;
        var relativeError = Math.Abs(p50 - expected) / expected;

        Assert.True(relativeError <= 0.01,
            $"High precision sketch should have <1% error. Got {relativeError:P3}");
    }

    #endregion
}
