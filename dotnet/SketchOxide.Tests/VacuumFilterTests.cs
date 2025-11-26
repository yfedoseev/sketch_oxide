using Xunit;
using SketchOxide.Membership;
using System;

namespace SketchOxide.Tests;

public class VacuumFilterTests
{
    [Fact]
    public void Constructor_ValidParameters_Succeeds()
    {
        using var filter = new VacuumFilter(1000, 0.01);
        Assert.Equal(1000UL, filter.Capacity);
        Assert.Equal(0.01, filter.FalsePositiveRate);
    }

    [Fact]
    public void Constructor_ZeroCapacity_ThrowsArgumentOutOfRange()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new VacuumFilter(0, 0.01));
    }

    [Fact]
    public void Constructor_InvalidFpr_ThrowsArgumentOutOfRange()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new VacuumFilter(1000, 0));
        Assert.Throws<ArgumentOutOfRangeException>(() => new VacuumFilter(1000, 1.0));
        Assert.Throws<ArgumentOutOfRangeException>(() => new VacuumFilter(1000, -0.01));
    }

    [Fact]
    public void InsertAndContains_BasicOperation_Works()
    {
        using var filter = new VacuumFilter(1000, 0.01);

        filter.Insert("key1");
        filter.Insert("key2");
        filter.Insert("key3");

        Assert.True(filter.Contains("key1"));
        Assert.True(filter.Contains("key2"));
        Assert.True(filter.Contains("key3"));
        Assert.False(filter.Contains("nonexistent"));
    }

    [Fact]
    public void Delete_ExistingItem_ReturnsTrue()
    {
        using var filter = new VacuumFilter(1000, 0.01);

        filter.Insert("key1");
        Assert.True(filter.Contains("key1"));

        bool deleted = filter.Delete("key1");
        Assert.True(deleted);
        Assert.False(filter.Contains("key1"));
    }

    [Fact]
    public void Delete_NonexistentItem_ReturnsFalse()
    {
        using var filter = new VacuumFilter(1000, 0.01);

        bool deleted = filter.Delete("nonexistent");
        Assert.False(deleted);
    }

    [Fact]
    public void GetStats_ReturnsCorrectInfo()
    {
        using var filter = new VacuumFilter(1000, 0.01);

        filter.Insert("key1");
        filter.Insert("key2");
        filter.Insert("key3");

        var stats = filter.GetStats();
        Assert.Equal(1000UL, stats.Capacity);
        Assert.InRange(stats.NumItems, 1UL, 10UL); // Should have ~3 items
        Assert.InRange(stats.LoadFactor, 0.0, 1.0);
        Assert.True(stats.MemoryBits > 0);
    }

    [Fact]
    public void Clear_RemovesAllItems()
    {
        using var filter = new VacuumFilter(1000, 0.01);

        filter.Insert("key1");
        filter.Insert("key2");
        Assert.True(filter.Contains("key1"));

        filter.Clear();

        var stats = filter.GetStats();
        Assert.Equal(0UL, stats.NumItems);
        Assert.False(filter.Contains("key1"));
    }

    [Fact]
    public void Dispose_CanBeCalledMultipleTimes()
    {
        var filter = new VacuumFilter(1000, 0.01);
        filter.Dispose();
        filter.Dispose(); // Should not throw
    }

    [Fact]
    public void FalsePositiveRate_WithinBounds()
    {
        using var filter = new VacuumFilter(1000, 0.01);

        // Insert 100 items
        for (int i = 0; i < 100; i++)
        {
            filter.Insert($"present_{i}");
        }

        // Check 1000 non-present items
        int falsePositives = 0;
        for (int i = 0; i < 1000; i++)
        {
            if (filter.Contains($"absent_{i}"))
                falsePositives++;
        }

        double actualFpr = falsePositives / 1000.0;
        // Should be within 3x of target FPR (generous bound)
        Assert.InRange(actualFpr, 0.0, 0.03);
    }

    [Fact]
    public void ByteArray_Operations_Work()
    {
        using var filter = new VacuumFilter(1000, 0.01);

        byte[] key = new byte[] { 0x01, 0x02, 0x03, 0x04 };
        filter.Insert(key);
        Assert.True(filter.Contains(key));

        filter.Delete(key);
        Assert.False(filter.Contains(key));
    }

    [Fact]
    public void ToString_ReturnsDescriptiveString()
    {
        using var filter = new VacuumFilter(1000, 0.01);
        string str = filter.ToString();
        Assert.Contains("VacuumFilter", str);
        Assert.Contains("1000", str);
    }
}
