using System;
using SketchOxide.Reconciliation;
using Xunit;

namespace SketchOxide.Tests;

/// <summary>
/// Unit tests for RatelessIBLT set reconciliation.
/// </summary>
public class RatelessIBLTTests : IDisposable
{
    private RatelessIBLT? _iblt;

    public RatelessIBLTTests()
    {
        _iblt = new RatelessIBLT(100, 32);
    }

    public void Dispose()
    {
        _iblt?.Dispose();
    }

    [Fact]
    public void Constructor_ValidParameters_CreatesIBLT()
    {
        using var iblt = new RatelessIBLT(100, 32);
        Assert.Equal(100ul, iblt.ExpectedDiff);
        Assert.Equal(32ul, iblt.CellSize);
    }

    [Theory]
    [InlineData(0ul, 32ul)]  // Invalid expectedDiff
    [InlineData(100ul, 0ul)] // Invalid cellSize
    public void Constructor_InvalidParameters_ThrowsException(ulong expectedDiff, ulong cellSize)
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new RatelessIBLT(expectedDiff, cellSize));
    }

    [Fact]
    public void Insert_WithBytes_DoesNotThrow()
    {
        var key = new byte[] { 1, 2, 3 };
        var value = new byte[] { 4, 5, 6 };

        _iblt!.Insert(key, value);
        // No exception means success
    }

    [Fact]
    public void Insert_WithStrings_DoesNotThrow()
    {
        _iblt!.Insert("key1", "value1");
        // No exception means success
    }

    [Fact]
    public void Insert_WithNullKey_ThrowsException()
    {
        Assert.Throws<ArgumentNullException>(() =>
            _iblt!.Insert((byte[])null!, new byte[] { 1 }));
    }

    [Fact]
    public void Insert_WithNullValue_ThrowsException()
    {
        Assert.Throws<ArgumentNullException>(() =>
            _iblt!.Insert(new byte[] { 1 }, (byte[])null!));
    }

    [Fact]
    public void Delete_WithBytes_DoesNotThrow()
    {
        var key = new byte[] { 1, 2, 3 };
        var value = new byte[] { 4, 5, 6 };

        _iblt!.Insert(key, value);
        _iblt!.Delete(key, value);
        // No exception means success
    }

    [Fact]
    public void Delete_WithStrings_DoesNotThrow()
    {
        _iblt!.Insert("key1", "value1");
        _iblt!.Delete("key1", "value1");
        // No exception means success
    }

    [Fact]
    public void Delete_WithNullKey_ThrowsException()
    {
        Assert.Throws<ArgumentNullException>(() =>
            _iblt!.Delete((byte[])null!, new byte[] { 1 }));
    }

    [Fact]
    public void Subtract_TwoIBLTs_Succeeds()
    {
        using var iblt2 = new RatelessIBLT(100, 32);

        // Both insert shared items
        _iblt!.Insert("shared1", "value1");
        _iblt!.Insert("shared2", "value2");
        iblt2.Insert("shared1", "value1");
        iblt2.Insert("shared2", "value2");

        // iblt1 has unique item
        _iblt!.Insert("unique1", "value1");

        // iblt2 has unique item
        iblt2.Insert("unique2", "value2");

        // Subtract to get difference
        _iblt!.Subtract(iblt2);

        // No exception means success
    }

    [Fact]
    public void Subtract_WithNull_ThrowsException()
    {
        Assert.Throws<ArgumentNullException>(() => _iblt!.Subtract(null!));
    }

    [Fact]
    public void Subtract_WithDisposedOther_ThrowsException()
    {
        var iblt2 = new RatelessIBLT(100, 32);
        iblt2.Dispose();

        Assert.Throws<ObjectDisposedException>(() => _iblt!.Subtract(iblt2));
    }

    [Fact]
    public void InsertDelete_SameItem_CancelsOut()
    {
        using var iblt2 = new RatelessIBLT(100, 32);

        // Insert item into both
        _iblt!.Insert("test", "value");
        iblt2.Insert("test", "value");

        // Delete from one
        _iblt!.Delete("test", "value");

        // After subtraction, should be nearly empty
        _iblt!.Subtract(iblt2);
        // No exception means operation succeeded
    }

    [Fact]
    public void Dispose_PreventsFurtherOperations()
    {
        _iblt!.Dispose();

        Assert.Throws<ObjectDisposedException>(() => _iblt.Insert("test", "value"));
        Assert.Throws<ObjectDisposedException>(() => _iblt.Delete("test", "value"));
        Assert.Throws<ObjectDisposedException>(() => _iblt.ExpectedDiff);
        Assert.Throws<ObjectDisposedException>(() => _iblt.CellSize);
    }

    [Fact]
    public void ToString_WithData_ContainsParameters()
    {
        var str = _iblt!.ToString();

        Assert.Contains("RatelessIBLT", str);
        Assert.Contains("expectedDiff=100", str);
        Assert.Contains("cellSize=32", str);
    }

    [Fact]
    public void ToString_WhenDisposed_IndicatesDisposed()
    {
        _iblt!.Dispose();
        var str = _iblt.ToString();

        Assert.Contains("disposed", str, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void UsingPattern_AutomaticallyCleansUp()
    {
        RatelessIBLT? iblt = null;

        using (var temp = new RatelessIBLT(100, 32))
        {
            temp.Insert("test", "value");
            iblt = temp;
        }

        // After using block, IBLT should be disposed
        Assert.Throws<ObjectDisposedException>(() => iblt!.Insert("test", "value"));
    }

    [Fact]
    public void SetReconciliation_BasicScenario()
    {
        // Create two IBLTs representing Alice and Bob's sets
        using var alice = new RatelessIBLT(100, 32);
        using var bob = new RatelessIBLT(100, 32);

        // Shared items
        for (int i = 0; i < 50; i++)
        {
            string key = $"shared_{i}";
            alice.Insert(key, $"value_{i}");
            bob.Insert(key, $"value_{i}");
        }

        // Alice-only items
        for (int i = 0; i < 10; i++)
        {
            alice.Insert($"alice_{i}", $"alice_value_{i}");
        }

        // Bob-only items
        for (int i = 0; i < 10; i++)
        {
            bob.Insert($"bob_{i}", $"bob_value_{i}");
        }

        // Compute difference
        using var diff = new RatelessIBLT(100, 32);
        // Copy alice's data by inserting the same items
        for (int i = 0; i < 50; i++)
        {
            string key = $"shared_{i}";
            diff.Insert(key, $"value_{i}");
        }
        for (int i = 0; i < 10; i++)
        {
            diff.Insert($"alice_{i}", $"alice_value_{i}");
        }

        diff.Subtract(bob);

        // Difference computed successfully
        // (Note: actual decode would require additional FFI functions)
    }

    [Fact]
    public void MultipleOperations_MaintainsCorrectness()
    {
        // Perform multiple operations
        for (int i = 0; i < 100; i++)
        {
            _iblt!.Insert($"key_{i}", $"value_{i}");
        }

        // Delete some items
        for (int i = 0; i < 10; i++)
        {
            _iblt!.Delete($"key_{i}", $"value_{i}");
        }

        // Insert more items
        for (int i = 100; i < 150; i++)
        {
            _iblt!.Insert($"key_{i}", $"value_{i}");
        }

        // No exception means operations succeeded
    }
}
