using System;
using SketchOxide.Membership;
using Xunit;

namespace SketchOxide.Tests;

/// <summary>
/// Unit tests for BloomFilter probabilistic set membership.
/// Tests cover constructor validation, insert operations, membership queries,
/// false positive behavior, serialization/deserialization, and disposal.
/// </summary>
public class BloomFilterTests : IDisposable
{
    private BloomFilter? _filter;

    public BloomFilterTests()
    {
        _filter = new BloomFilter(10000, 0.01);
    }

    public void Dispose()
    {
        _filter?.Dispose();
    }

    #region Constructor Tests

    [Fact]
    public void Constructor_ValidParameters_CreatesFilter()
    {
        using var filter = new BloomFilter(1000, 0.01);
        Assert.Equal(1000ul, filter.ExpectedElements);
        Assert.Equal(0.01, filter.FalsePositiveRate);
    }

    [Fact]
    public void Constructor_ZeroExpectedElements_ThrowsArgumentOutOfRangeException()
    {
        var ex = Assert.Throws<ArgumentOutOfRangeException>(() => new BloomFilter(0, 0.01));
        Assert.Equal("expectedElements", ex.ParamName);
        Assert.Contains("Expected elements must be greater than 0", ex.Message);
    }

    [Theory]
    [InlineData(0.0)]    // FPR = 0
    [InlineData(-0.1)]   // FPR < 0
    [InlineData(1.0)]    // FPR = 1
    [InlineData(1.5)]    // FPR > 1
    public void Constructor_InvalidFalsePositiveRate_ThrowsArgumentOutOfRangeException(double fpr)
    {
        var ex = Assert.Throws<ArgumentOutOfRangeException>(() => new BloomFilter(1000, fpr));
        Assert.Equal("falsePositiveRate", ex.ParamName);
        Assert.Contains("False positive rate must be in range (0, 1)", ex.Message);
    }

    [Theory]
    [InlineData(1ul, 0.001)]       // Minimum elements
    [InlineData(100ul, 0.01)]      // Small filter
    [InlineData(10000ul, 0.01)]    // Medium filter
    [InlineData(1000000ul, 0.1)]   // Large filter, higher FPR
    [InlineData(1000ul, 0.0001)]   // Low FPR
    [InlineData(1000ul, 0.999)]    // Near boundary FPR
    public void Constructor_ValidBoundaryParameters_Succeeds(ulong expectedElements, double fpr)
    {
        using var filter = new BloomFilter(expectedElements, fpr);
        Assert.Equal(expectedElements, filter.ExpectedElements);
        Assert.Equal(fpr, filter.FalsePositiveRate);
    }

    #endregion

    #region Insert Tests

    [Fact]
    public void Insert_WithBytes_DoesNotThrow()
    {
        _filter!.Insert(new byte[] { 1, 2, 3, 4 });
        Assert.True(_filter.Contains(new byte[] { 1, 2, 3, 4 }), "Inserted item should be found");
    }

    [Fact]
    public void Insert_WithString_DoesNotThrow()
    {
        _filter!.Insert("test-item");
        Assert.True(_filter.Contains("test-item"), "Inserted string should be found");
    }

    [Fact]
    public void Insert_WithNullBytes_ThrowsArgumentNullException()
    {
        Assert.Throws<ArgumentNullException>(() => _filter!.Insert((byte[])null!));
    }

    [Fact]
    public void Insert_WithNullString_ThrowsArgumentNullException()
    {
        Assert.Throws<ArgumentNullException>(() => _filter!.Insert((string)null!));
    }

    [Fact]
    public void Insert_WithEmptyBytes_DoesNotThrow()
    {
        _filter!.Insert(Array.Empty<byte>());
        Assert.True(_filter.Contains(Array.Empty<byte>()), "Inserted empty bytes should be found");
    }

    [Fact]
    public void Insert_WithEmptyString_DoesNotThrow()
    {
        _filter!.Insert(string.Empty);
        Assert.True(_filter.Contains(string.Empty), "Inserted empty string should be found");
    }

    [Fact]
    public void Insert_DuplicateItems_DoesNotAffectContains()
    {
        for (int i = 0; i < 100; i++)
        {
            _filter!.Insert("duplicate-item");
        }

        Assert.True(_filter!.Contains("duplicate-item"), "Duplicate insertions should still be found");
    }

    #endregion

    #region Contains Tests

    [Fact]
    public void Contains_InsertedItem_ReturnsTrue()
    {
        _filter!.Insert("definitely-inserted");
        Assert.True(_filter.Contains("definitely-inserted"), "Inserted item must be found (no false negatives)");
    }

    [Fact]
    public void Contains_NotInsertedItem_ReturnsFalse_Usually()
    {
        // Insert some items
        for (int i = 0; i < 100; i++)
        {
            _filter!.Insert($"inserted-{i}");
        }

        // Check items that were NOT inserted - most should return false
        int falsePositives = 0;
        const int testCount = 1000;

        for (int i = 0; i < testCount; i++)
        {
            if (_filter!.Contains($"not-inserted-{i}"))
            {
                falsePositives++;
            }
        }

        // With FPR=0.01 and only 100 inserted items (filter capacity 10000),
        // actual FPR should be very low
        Assert.True(falsePositives < testCount * 0.05,
            $"False positive count {falsePositives} is unexpectedly high (expected < {testCount * 0.05})");
    }

    [Fact]
    public void Contains_WithNullBytes_ThrowsArgumentNullException()
    {
        Assert.Throws<ArgumentNullException>(() => _filter!.Contains((byte[])null!));
    }

    [Fact]
    public void Contains_WithNullString_ThrowsArgumentNullException()
    {
        Assert.Throws<ArgumentNullException>(() => _filter!.Contains((string)null!));
    }

    [Fact]
    public void Contains_NoFalseNegatives_Guaranteed()
    {
        var insertedItems = new string[500];

        // Insert items
        for (int i = 0; i < insertedItems.Length; i++)
        {
            insertedItems[i] = $"item-{i}-{Guid.NewGuid()}";
            _filter!.Insert(insertedItems[i]);
        }

        // Verify ALL inserted items are found (no false negatives)
        foreach (var item in insertedItems)
        {
            Assert.True(_filter!.Contains(item),
                $"Bloom filter must never have false negatives. Item '{item}' was inserted but not found.");
        }
    }

    [Fact]
    public void Contains_BytesAndStringEquivalence()
    {
        var testString = "test-equivalence";
        var testBytes = System.Text.Encoding.UTF8.GetBytes(testString);

        _filter!.Insert(testString);

        Assert.True(_filter.Contains(testString), "String lookup should find inserted string");
        Assert.True(_filter.Contains(testBytes), "Byte lookup should find item inserted as string");
    }

    #endregion

    #region False Positive Rate Tests

    [Fact]
    public void FalsePositiveRate_ApproximatelyCorrect()
    {
        const ulong expectedElements = 10000;
        const double targetFpr = 0.01;

        using var filter = new BloomFilter(expectedElements, targetFpr);

        // Insert exactly expectedElements items
        for (ulong i = 0; i < expectedElements; i++)
        {
            filter.Insert($"item-{i}");
        }

        // Test with items that were NOT inserted
        int falsePositives = 0;
        const int testCount = 100000;

        for (int i = 0; i < testCount; i++)
        {
            if (filter.Contains($"nonexistent-{i}-{Guid.NewGuid()}"))
            {
                falsePositives++;
            }
        }

        double actualFpr = (double)falsePositives / testCount;

        // Allow 5x tolerance for statistical variance
        Assert.True(actualFpr <= targetFpr * 5,
            $"Actual FPR {actualFpr:P4} exceeds 5x target FPR {targetFpr:P4}");
    }

    [Fact]
    public void FalsePositiveRate_IncreasesWhenOverfilled()
    {
        const ulong expectedElements = 100;
        const double targetFpr = 0.01;

        using var filter = new BloomFilter(expectedElements, targetFpr);

        // Overfill by 10x
        for (int i = 0; i < (int)expectedElements * 10; i++)
        {
            filter.Insert($"overfill-item-{i}");
        }

        // FPR should be higher than target when overfilled
        int falsePositives = 0;
        const int testCount = 10000;

        for (int i = 0; i < testCount; i++)
        {
            if (filter.Contains($"test-query-{i}-{Guid.NewGuid()}"))
            {
                falsePositives++;
            }
        }

        double actualFpr = (double)falsePositives / testCount;

        // When overfilled, FPR should be significantly higher than target
        // This is expected behavior demonstrating the importance of proper sizing
        Assert.True(actualFpr > targetFpr,
            $"Overfilled filter should have higher FPR. Actual: {actualFpr:P4}, Target: {targetFpr:P4}");
    }

    #endregion

    #region Serialization Tests

    [Fact]
    public void Serialize_WithData_ReturnsNonEmptyBytes()
    {
        for (int i = 0; i < 100; i++)
        {
            _filter!.Insert($"item-{i}");
        }

        var serialized = _filter!.Serialize();
        Assert.NotNull(serialized);
        Assert.True(serialized.Length > 0, "Serialized data should not be empty");
    }

    [Fact]
    public void Serialize_EmptyFilter_ReturnsBytes()
    {
        var serialized = _filter!.Serialize();
        Assert.NotNull(serialized);
        Assert.True(serialized.Length > 0, "Even empty filter should serialize");
    }

    [Fact]
    public void SerializationRoundTrip_PreservesMembership()
    {
        var testItems = new[] { "item-a", "item-b", "item-c", "item-d" };

        foreach (var item in testItems)
        {
            _filter!.Insert(item);
        }

        var serialized = _filter!.Serialize();
        using var restored = BloomFilter.Deserialize(serialized, 10000, 0.01);

        // Verify all inserted items are still found
        foreach (var item in testItems)
        {
            Assert.True(restored.Contains(item),
                $"Restored filter should contain '{item}'");
        }
    }

    [Fact]
    public void Deserialize_InvalidData_ThrowsArgumentException()
    {
        var invalidData = new byte[] { 1, 2, 3 };

        var ex = Assert.Throws<ArgumentException>(() => BloomFilter.Deserialize(invalidData, 1000, 0.01));
        Assert.Contains("Failed to deserialize", ex.Message);
    }

    [Fact]
    public void Deserialize_NullData_ThrowsArgumentNullException()
    {
        var ex = Assert.Throws<ArgumentNullException>(() => BloomFilter.Deserialize(null!, 1000, 0.01));
        Assert.Equal("data", ex.ParamName);
    }

    #endregion

    #region Disposal Tests

    [Fact]
    public void Dispose_PreventsFurtherOperations()
    {
        _filter!.Dispose();

        Assert.Throws<ObjectDisposedException>(() => _filter.Insert("test"));
        Assert.Throws<ObjectDisposedException>(() => _filter.Contains("test"));
        Assert.Throws<ObjectDisposedException>(() => _ = _filter.ExpectedElements);
        Assert.Throws<ObjectDisposedException>(() => _ = _filter.FalsePositiveRate);
    }

    [Fact]
    public void Dispose_CanBeCalledMultipleTimes()
    {
        _filter!.Dispose();
        _filter.Dispose(); // Should not throw
        _filter.Dispose(); // Should not throw
    }

    [Fact]
    public void ToString_WithData_ContainsParameters()
    {
        _filter!.Insert("test");
        var str = _filter.ToString();

        Assert.Contains("BloomFilter", str);
        Assert.Contains("expectedElements", str, StringComparison.OrdinalIgnoreCase);
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
        BloomFilter? filter = null;

        using (var bf = new BloomFilter(1000, 0.01))
        {
            bf.Insert("test");
            filter = bf;
            Assert.True(bf.Contains("test"));
        }

        // After using block, filter should be disposed
        Assert.Throws<ObjectDisposedException>(() => filter!.Insert("test"));
    }

    #endregion

    #region Large Dataset Tests

    [Fact]
    public void LargeDataset_MaintainsNoFalseNegatives()
    {
        using var largeFilter = new BloomFilter(100000, 0.001);
        var insertedItems = new string[50000];

        // Insert many items
        for (int i = 0; i < insertedItems.Length; i++)
        {
            insertedItems[i] = $"large-item-{i}";
            largeFilter.Insert(insertedItems[i]);
        }

        // Verify ALL inserted items are found
        foreach (var item in insertedItems)
        {
            Assert.True(largeFilter.Contains(item),
                $"Large dataset: no false negatives allowed. Item '{item}' not found.");
        }
    }

    [Fact]
    public void BinaryData_HandlesAllByteValues()
    {
        using var filter = new BloomFilter(1000, 0.01);

        // Test with binary data containing all byte values
        var binaryData = new byte[256];
        for (int i = 0; i < 256; i++)
        {
            binaryData[i] = (byte)i;
        }

        filter.Insert(binaryData);
        Assert.True(filter.Contains(binaryData), "Binary data with all byte values should be found");
    }

    [Fact]
    public void UnicodStrings_HandledCorrectly()
    {
        using var filter = new BloomFilter(1000, 0.01);

        var unicodeStrings = new[]
        {
            "Hello World",
            "Bonjour le monde",
            "Hola Mundo",
            "Hallo Welt",
            "Ciao mondo"
        };

        foreach (var str in unicodeStrings)
        {
            filter.Insert(str);
        }

        foreach (var str in unicodeStrings)
        {
            Assert.True(filter.Contains(str), $"Unicode string '{str}' should be found");
        }
    }

    #endregion
}
