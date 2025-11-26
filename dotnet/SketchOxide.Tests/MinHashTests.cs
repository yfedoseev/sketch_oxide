using System;
using SketchOxide.Similarity;
using Xunit;

namespace SketchOxide.Tests;

/// <summary>
/// Unit tests for MinHash similarity estimator.
/// Tests cover constructor validation, update operations, Jaccard similarity estimation,
/// serialization/deserialization, and disposal behavior.
/// </summary>
public class MinHashTests : IDisposable
{
    private MinHash? _sketch;

    public MinHashTests()
    {
        _sketch = new MinHash(128);
    }

    public void Dispose()
    {
        _sketch?.Dispose();
    }

    #region Constructor Tests

    [Fact]
    public void Constructor_ValidNumPermutations_CreatesSketch()
    {
        using var sketch = new MinHash(64);
        Assert.Equal(64u, sketch.NumPermutations);
    }

    [Fact]
    public void Constructor_ZeroPermutations_ThrowsArgumentOutOfRangeException()
    {
        var ex = Assert.Throws<ArgumentOutOfRangeException>(() => new MinHash(0));
        Assert.Equal("numPermutations", ex.ParamName);
        Assert.Contains("Number of permutations must be greater than 0", ex.Message);
    }

    [Theory]
    [InlineData(1u)]     // Minimum valid
    [InlineData(64u)]    // Common small
    [InlineData(128u)]   // Common medium
    [InlineData(256u)]   // Common large
    [InlineData(512u)]   // High accuracy
    [InlineData(1024u)]  // Very high accuracy
    public void Constructor_ValidBoundaryPermutations_Succeeds(uint numPermutations)
    {
        using var sketch = new MinHash(numPermutations);
        Assert.Equal(numPermutations, sketch.NumPermutations);
    }

    #endregion

    #region Update Tests

    [Fact]
    public void Update_WithBytes_DoesNotThrow()
    {
        _sketch!.Update(new byte[] { 1, 2, 3, 4 });
        // Should not throw, sketch is now non-empty
        Assert.Equal(128u, _sketch.NumPermutations);
    }

    [Fact]
    public void Update_WithString_DoesNotThrow()
    {
        _sketch!.Update("test-element");
        Assert.Equal(128u, _sketch.NumPermutations);
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
        Assert.Equal(128u, _sketch.NumPermutations);
    }

    [Fact]
    public void Update_WithEmptyString_DoesNotThrow()
    {
        _sketch!.Update(string.Empty);
        Assert.Equal(128u, _sketch.NumPermutations);
    }

    [Fact]
    public void Update_DuplicateItems_IdempotentForSets()
    {
        using var sketch1 = new MinHash(128);
        using var sketch2 = new MinHash(128);

        // Add same element multiple times to sketch1
        for (int i = 0; i < 10; i++)
        {
            sketch1.Update("duplicate-element");
        }

        // Add element once to sketch2
        sketch2.Update("duplicate-element");

        // Both sketches should represent the same set (single element)
        var similarity = sketch1.JaccardSimilarity(sketch2);
        Assert.Equal(1.0, similarity, precision: 2);
    }

    #endregion

    #region JaccardSimilarity Tests

    [Fact]
    public void JaccardSimilarity_IdenticalSets_ReturnsOne()
    {
        using var sketch1 = new MinHash(128);
        using var sketch2 = new MinHash(128);

        var elements = new[] { "a", "b", "c", "d", "e" };

        foreach (var elem in elements)
        {
            sketch1.Update(elem);
            sketch2.Update(elem);
        }

        var similarity = sketch1.JaccardSimilarity(sketch2);

        // Should be very close to 1.0
        Assert.True(similarity >= 0.95, $"Identical sets should have similarity ~1.0, got {similarity}");
    }

    [Fact]
    public void JaccardSimilarity_DisjointSets_ReturnsLowValue()
    {
        using var sketch1 = new MinHash(256);
        using var sketch2 = new MinHash(256);

        // Add completely different elements
        for (int i = 0; i < 100; i++)
        {
            sketch1.Update($"set1-element-{i}");
            sketch2.Update($"set2-element-{i}");
        }

        var similarity = sketch1.JaccardSimilarity(sketch2);

        // Disjoint sets should have similarity close to 0
        Assert.True(similarity <= 0.1, $"Disjoint sets should have low similarity, got {similarity}");
    }

    [Fact]
    public void JaccardSimilarity_PartialOverlap_ReturnsIntermediateValue()
    {
        using var sketch1 = new MinHash(256);
        using var sketch2 = new MinHash(256);

        // 50 shared elements
        for (int i = 0; i < 50; i++)
        {
            sketch1.Update($"shared-{i}");
            sketch2.Update($"shared-{i}");
        }

        // 50 unique to each
        for (int i = 0; i < 50; i++)
        {
            sketch1.Update($"unique1-{i}");
            sketch2.Update($"unique2-{i}");
        }

        // Jaccard = 50 / 150 = 0.333...
        var similarity = sketch1.JaccardSimilarity(sketch2);

        Assert.True(similarity >= 0.25 && similarity <= 0.45,
            $"50% overlap should give ~0.33 similarity, got {similarity}");
    }

    [Fact]
    public void JaccardSimilarity_ReturnsValueInZeroToOne()
    {
        using var sketch1 = new MinHash(128);
        using var sketch2 = new MinHash(128);

        for (int i = 0; i < 50; i++)
        {
            sketch1.Update($"element-{i}");
            sketch2.Update($"element-{i + 25}"); // 50% overlap
        }

        var similarity = sketch1.JaccardSimilarity(sketch2);

        Assert.True(similarity >= 0.0, $"Similarity must be >= 0, got {similarity}");
        Assert.True(similarity <= 1.0, $"Similarity must be <= 1, got {similarity}");
    }

    [Fact]
    public void JaccardSimilarity_IsSymmetric()
    {
        using var sketch1 = new MinHash(128);
        using var sketch2 = new MinHash(128);

        sketch1.Update("a");
        sketch1.Update("b");
        sketch1.Update("c");

        sketch2.Update("b");
        sketch2.Update("c");
        sketch2.Update("d");

        var sim12 = sketch1.JaccardSimilarity(sketch2);
        var sim21 = sketch2.JaccardSimilarity(sketch1);

        Assert.Equal(sim12, sim21, precision: 10);
    }

    [Fact]
    public void JaccardSimilarity_WithNull_ThrowsArgumentNullException()
    {
        var ex = Assert.Throws<ArgumentNullException>(() => _sketch!.JaccardSimilarity(null!));
        Assert.Equal("other", ex.ParamName);
    }

    [Fact]
    public void JaccardSimilarity_DifferentPermutations_ThrowsArgumentException()
    {
        using var other = new MinHash(64); // Different number of permutations

        var ex = Assert.Throws<ArgumentException>(() => _sketch!.JaccardSimilarity(other));
        Assert.Contains("Cannot compare MinHash sketches with different permutation counts", ex.Message);
    }

    [Fact]
    public void JaccardSimilarity_WithDisposedOther_ThrowsObjectDisposedException()
    {
        var other = new MinHash(128);
        other.Update("test");
        other.Dispose();

        Assert.Throws<ObjectDisposedException>(() => _sketch!.JaccardSimilarity(other));
    }

    [Fact]
    public void JaccardSimilarity_AccuracyImprovesWith MorePermutations()
    {
        // With more permutations, estimates should be more accurate
        var lowPermSimilarities = new double[10];
        var highPermSimilarities = new double[10];

        for (int trial = 0; trial < 10; trial++)
        {
            using var low1 = new MinHash(32);
            using var low2 = new MinHash(32);
            using var high1 = new MinHash(512);
            using var high2 = new MinHash(512);

            // Create sets with known 50% overlap
            for (int i = 0; i < 100; i++)
            {
                low1.Update($"trial{trial}-shared-{i}");
                low2.Update($"trial{trial}-shared-{i}");
                high1.Update($"trial{trial}-shared-{i}");
                high2.Update($"trial{trial}-shared-{i}");
            }
            for (int i = 0; i < 100; i++)
            {
                low1.Update($"trial{trial}-unique1-{i}");
                low2.Update($"trial{trial}-unique2-{i}");
                high1.Update($"trial{trial}-unique1-{i}");
                high2.Update($"trial{trial}-unique2-{i}");
            }

            lowPermSimilarities[trial] = low1.JaccardSimilarity(low2);
            highPermSimilarities[trial] = high1.JaccardSimilarity(high2);
        }

        // Calculate variance
        var expectedJaccard = 100.0 / 300.0; // ~0.333
        var lowVariance = CalculateVariance(lowPermSimilarities, expectedJaccard);
        var highVariance = CalculateVariance(highPermSimilarities, expectedJaccard);

        // Higher permutation count should have lower variance
        Assert.True(highVariance <= lowVariance,
            $"Higher permutations should have lower variance. Low: {lowVariance}, High: {highVariance}");
    }

    private static double CalculateVariance(double[] values, double expectedMean)
    {
        double sumSquaredDiff = 0;
        foreach (var v in values)
        {
            sumSquaredDiff += (v - expectedMean) * (v - expectedMean);
        }
        return sumSquaredDiff / values.Length;
    }

    #endregion

    #region Serialization Tests

    [Fact]
    public void Serialize_WithData_ReturnsNonEmptyBytes()
    {
        for (int i = 0; i < 50; i++)
        {
            _sketch!.Update($"element-{i}");
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
    public void SerializationRoundTrip_PreservesSimilarity()
    {
        using var other = new MinHash(128);

        var sharedElements = new[] { "shared-1", "shared-2", "shared-3" };
        foreach (var elem in sharedElements)
        {
            _sketch!.Update(elem);
            other.Update(elem);
        }

        _sketch!.Update("unique-to-original");
        other.Update("unique-to-other");

        var originalSimilarity = _sketch.JaccardSimilarity(other);
        var serialized = _sketch.Serialize();

        using var restored = MinHash.Deserialize(serialized, 128);
        var restoredSimilarity = restored.JaccardSimilarity(other);

        Assert.Equal(originalSimilarity, restoredSimilarity, precision: 10);
    }

    [Fact]
    public void Deserialize_InvalidData_ThrowsArgumentException()
    {
        var invalidData = new byte[] { 1, 2, 3 };

        var ex = Assert.Throws<ArgumentException>(() => MinHash.Deserialize(invalidData, 128));
        Assert.Contains("Failed to deserialize", ex.Message);
    }

    [Fact]
    public void Deserialize_NullData_ThrowsArgumentNullException()
    {
        var ex = Assert.Throws<ArgumentNullException>(() => MinHash.Deserialize(null!, 128));
        Assert.Equal("data", ex.ParamName);
    }

    #endregion

    #region Disposal Tests

    [Fact]
    public void Dispose_PreventsFurtherOperations()
    {
        using var other = new MinHash(128);

        _sketch!.Dispose();

        Assert.Throws<ObjectDisposedException>(() => _sketch.Update("test"));
        Assert.Throws<ObjectDisposedException>(() => _sketch.JaccardSimilarity(other));
        Assert.Throws<ObjectDisposedException>(() => _ = _sketch.NumPermutations);
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

        Assert.Contains("MinHash", str);
        Assert.Contains("numPermutations", str, StringComparison.OrdinalIgnoreCase);
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
        MinHash? sketch = null;

        using (var mh = new MinHash(128))
        {
            mh.Update("test");
            sketch = mh;
            Assert.Equal(128u, mh.NumPermutations);
        }

        // After using block, sketch should be disposed
        Assert.Throws<ObjectDisposedException>(() => sketch!.Update("test"));
    }

    #endregion

    #region Set Operations Tests

    [Fact]
    public void SubsetRelationship_HigherSimilarity()
    {
        using var superSet = new MinHash(256);
        using var subSet = new MinHash(256);

        // Superset has more elements
        for (int i = 0; i < 100; i++)
        {
            superSet.Update($"element-{i}");
        }

        // Subset has fewer elements (all in superset)
        for (int i = 0; i < 50; i++)
        {
            subSet.Update($"element-{i}");
        }

        var similarity = superSet.JaccardSimilarity(subSet);

        // Jaccard = 50 / 100 = 0.5
        Assert.True(similarity >= 0.4 && similarity <= 0.6,
            $"Subset relationship should give ~0.5 similarity, got {similarity}");
    }

    [Fact]
    public void LargeSets_MaintainsAccuracy()
    {
        using var sketch1 = new MinHash(256);
        using var sketch2 = new MinHash(256);

        // Large sets with 50% overlap
        const int setSize = 10000;
        const int overlap = 5000;

        for (int i = 0; i < overlap; i++)
        {
            sketch1.Update($"shared-{i}");
            sketch2.Update($"shared-{i}");
        }

        for (int i = 0; i < setSize - overlap; i++)
        {
            sketch1.Update($"unique1-{i}");
            sketch2.Update($"unique2-{i}");
        }

        // Expected Jaccard = 5000 / 15000 = 0.333
        var similarity = sketch1.JaccardSimilarity(sketch2);

        Assert.True(similarity >= 0.28 && similarity <= 0.38,
            $"Large sets: expected ~0.33, got {similarity}");
    }

    [Fact]
    public void TextShingling_DocumentSimilarity()
    {
        using var doc1 = new MinHash(128);
        using var doc2 = new MinHash(128);

        // Simulate document shingling with word n-grams
        var text1 = "the quick brown fox jumps over the lazy dog";
        var text2 = "the quick brown cat jumps over the lazy dog";

        var words1 = text1.Split(' ');
        var words2 = text2.Split(' ');

        // Create 3-grams (shingles)
        for (int i = 0; i < words1.Length - 2; i++)
        {
            doc1.Update($"{words1[i]} {words1[i + 1]} {words1[i + 2]}");
        }
        for (int i = 0; i < words2.Length - 2; i++)
        {
            doc2.Update($"{words2[i]} {words2[i + 1]} {words2[i + 2]}");
        }

        var similarity = doc1.JaccardSimilarity(doc2);

        // Documents differ by one word, should have high similarity
        Assert.True(similarity >= 0.5, $"Similar documents should have high similarity, got {similarity}");
    }

    [Fact]
    public void BinaryData_HandledCorrectly()
    {
        using var sketch1 = new MinHash(128);
        using var sketch2 = new MinHash(128);

        // Add identical binary data to both
        var binaryData = new byte[] { 0x00, 0xFF, 0x7F, 0x80, 0x01 };
        sketch1.Update(binaryData);
        sketch2.Update(binaryData);

        var similarity = sketch1.JaccardSimilarity(sketch2);
        Assert.Equal(1.0, similarity, precision: 2);
    }

    #endregion

    #region Edge Cases

    [Fact]
    public void SingleElementSets_BothSame_ReturnOne()
    {
        using var sketch1 = new MinHash(128);
        using var sketch2 = new MinHash(128);

        sketch1.Update("only-element");
        sketch2.Update("only-element");

        var similarity = sketch1.JaccardSimilarity(sketch2);
        Assert.Equal(1.0, similarity, precision: 2);
    }

    [Fact]
    public void SingleElementSets_Different_ReturnZero()
    {
        using var sketch1 = new MinHash(128);
        using var sketch2 = new MinHash(128);

        sketch1.Update("element-1");
        sketch2.Update("element-2");

        var similarity = sketch1.JaccardSimilarity(sketch2);
        Assert.True(similarity <= 0.1, $"Different single elements should have ~0 similarity, got {similarity}");
    }

    [Fact]
    public void UnicodeStrings_HandledCorrectly()
    {
        using var sketch1 = new MinHash(128);
        using var sketch2 = new MinHash(128);

        var unicodeElements = new[]
        {
            "Hello",
            "Bonjour",
            "Hola"
        };

        foreach (var elem in unicodeElements)
        {
            sketch1.Update(elem);
            sketch2.Update(elem);
        }

        var similarity = sketch1.JaccardSimilarity(sketch2);
        Assert.True(similarity >= 0.95, $"Same unicode sets should have ~1.0 similarity, got {similarity}");
    }

    #endregion
}
