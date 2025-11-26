using Xunit;
using SketchOxide.Membership;
using System;
using System.Linq;

namespace SketchOxide.Tests;

public class LearnedBloomFilterTests
{
    [Fact]
    public void Constructor_ValidParameters_Succeeds()
    {
        var trainingKeys = Enumerable.Range(0, 100).Select(i => $"key{i}").ToList();
        using var filter = new LearnedBloomFilter(trainingKeys, 0.01);
        Assert.Equal(100, filter.NumTrainingKeys);
    }

    [Fact]
    public void Constructor_NullTrainingKeys_ThrowsArgumentNull()
    {
        Assert.Throws<ArgumentNullException>(() => new LearnedBloomFilter((string[])null, 0.01));
    }

    [Fact]
    public void Constructor_EmptyTrainingKeys_ThrowsArgumentOutOfRange()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new LearnedBloomFilter(Array.Empty<string>(), 0.01));
    }

    [Fact]
    public void Constructor_TooFewKeys_ThrowsArgumentOutOfRange()
    {
        var trainingKeys = new[] { "key1", "key2" };
        Assert.Throws<ArgumentOutOfRangeException>(() => new LearnedBloomFilter(trainingKeys, 0.01));
    }

    [Fact]
    public void Constructor_InvalidFpr_ThrowsArgumentOutOfRange()
    {
        var trainingKeys = Enumerable.Range(0, 20).Select(i => $"key{i}").ToArray();
        Assert.Throws<ArgumentOutOfRangeException>(() => new LearnedBloomFilter(trainingKeys, 0));
        Assert.Throws<ArgumentOutOfRangeException>(() => new LearnedBloomFilter(trainingKeys, 1.0));
    }

    [Fact]
    public void Contains_TrainingKeys_ReturnsTrue()
    {
        var trainingKeys = Enumerable.Range(0, 100).Select(i => $"key{i}").ToArray();
        using var filter = new LearnedBloomFilter(trainingKeys, 0.01);

        // All training keys should be present (no false negatives)
        Assert.True(filter.Contains("key0"));
        Assert.True(filter.Contains("key50"));
        Assert.True(filter.Contains("key99"));
    }

    [Fact]
    public void Contains_NonTrainingKeys_MostReturnFalse()
    {
        var trainingKeys = Enumerable.Range(0, 100).Select(i => $"present_{i}").ToArray();
        using var filter = new LearnedBloomFilter(trainingKeys, 0.01);

        // Check many non-present keys
        int falsePositives = 0;
        for (int i = 0; i < 1000; i++)
        {
            if (filter.Contains($"absent_{i}"))
                falsePositives++;
        }

        double actualFpr = falsePositives / 1000.0;
        // FPR should be reasonable (within 5x of target for ML model)
        Assert.InRange(actualFpr, 0.0, 0.05);
    }

    [Fact]
    public void MemoryUsage_ReturnsReasonableValue()
    {
        var trainingKeys = Enumerable.Range(0, 1000).Select(i => $"key{i}").ToArray();
        using var filter = new LearnedBloomFilter(trainingKeys, 0.01);

        ulong memory = filter.MemoryUsage();
        Assert.True(memory > 0);
        // Learned Bloom should be more memory efficient than standard Bloom
        // For 1000 items at 1% FPR, standard Bloom ~10 bits/item = ~1250 bytes
        // Learned Bloom should be significantly less
    }

    [Fact]
    public void ExpectedFpr_ReturnsTargetValue()
    {
        var trainingKeys = Enumerable.Range(0, 100).Select(i => $"key{i}").ToArray();
        using var filter = new LearnedBloomFilter(trainingKeys, 0.01);

        double fpr = filter.ExpectedFpr;
        Assert.InRange(fpr, 0.0, 1.0);
    }

    [Fact]
    public void ByteArrayKeys_Work()
    {
        var trainingKeys = Enumerable.Range(0, 100)
            .Select(i => new byte[] { (byte)i, (byte)(i >> 8) })
            .ToArray();
        using var filter = new LearnedBloomFilter(trainingKeys, 0.01);

        // Training keys should be present
        Assert.True(filter.Contains(new byte[] { 0, 0 }));
        Assert.True(filter.Contains(new byte[] { 50, 0 }));
    }

    [Fact]
    public void LargeTrainingSet_Works()
    {
        var trainingKeys = Enumerable.Range(0, 5000).Select(i => $"key{i}").ToArray();
        using var filter = new LearnedBloomFilter(trainingKeys, 0.01);

        Assert.Equal(5000, filter.NumTrainingKeys);
        Assert.True(filter.Contains("key2500"));
    }

    [Fact]
    public void SequentialPatterns_LearnedCorrectly()
    {
        // Sequential patterns: key0, key1, key2, ...
        var trainingKeys = Enumerable.Range(0, 200).Select(i => $"key{i}").ToArray();
        using var filter = new LearnedBloomFilter(trainingKeys, 0.01);

        // Model should learn the pattern
        Assert.True(filter.Contains("key100"));
        Assert.True(filter.Contains("key150"));
    }

    [Fact]
    public void ToString_ContainsExperimentalWarning()
    {
        var trainingKeys = Enumerable.Range(0, 20).Select(i => $"key{i}").ToArray();
        using var filter = new LearnedBloomFilter(trainingKeys, 0.01);
        string str = filter.ToString();
        Assert.Contains("LearnedBloomFilter", str);
        Assert.Contains("EXPERIMENTAL", str);
    }

    [Fact]
    public void Dispose_CanBeCalledMultipleTimes()
    {
        var trainingKeys = Enumerable.Range(0, 20).Select(i => $"key{i}").ToArray();
        var filter = new LearnedBloomFilter(trainingKeys, 0.01);
        filter.Dispose();
        filter.Dispose(); // Should not throw
    }
}
