using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Reconciliation;

/// <summary>
/// Rateless IBLT (Invertible Bloom Lookup Table) for efficient set reconciliation.
///
/// Computes the symmetric difference between two sets in distributed systems without
/// knowing the difference size a priori. Essential for P2P networks, blockchain sync,
/// and distributed cache invalidation.
/// </summary>
/// <remarks>
/// Rateless IBLT uses k hash functions (typically k=3) to map each key-value pair
/// to k cells, maintaining XOR sums and counts for efficient difference computation.
///
/// Performance:
/// - Space: O(c × d) where c ≈ 1.5-2.0, d = expected difference size
/// - Insert/Delete: O(k) where k = number of hash functions
/// - Subtract: O(n) where n = number of cells
/// - Decode: O(d × k) where d = actual difference size
///
/// Use Cases:
/// - Ethereum block synchronization (5.6x faster than naive approaches)
/// - P2P network synchronization (BitTorrent, IPFS, blockchain nodes)
/// - Distributed cache invalidation
/// - Database replication
/// - File synchronization protocols
/// </remarks>
public sealed class RatelessIBLT : NativeSketch
{
    private readonly ulong _expectedDiff;
    private readonly ulong _cellSize;

    /// <summary>
    /// Creates a new RatelessIBLT for set reconciliation.
    /// </summary>
    /// <param name="expectedDiff">Expected size of symmetric difference.</param>
    /// <param name="cellSize">Maximum size for cell data (typically 32 bytes).</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if parameters are invalid.</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native allocation fails.</exception>
    public RatelessIBLT(ulong expectedDiff, ulong cellSize)
    {
        if (expectedDiff == 0)
            throw new ArgumentOutOfRangeException(nameof(expectedDiff), expectedDiff,
                "expectedDiff must be greater than 0");
        if (cellSize == 0)
            throw new ArgumentOutOfRangeException(nameof(cellSize), cellSize,
                "cellSize must be greater than 0");

        _expectedDiff = expectedDiff;
        _cellSize = cellSize;
        NativePtr = SketchOxideNative.rateless_iblt_new(expectedDiff, cellSize);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native RatelessIBLT");
    }

    /// <summary>
    /// Gets the expected difference size.
    /// </summary>
    public ulong ExpectedDiff
    {
        get
        {
            CheckAlive();
            return _expectedDiff;
        }
    }

    /// <summary>
    /// Gets the cell size.
    /// </summary>
    public ulong CellSize
    {
        get
        {
            CheckAlive();
            return _cellSize;
        }
    }

    /// <summary>
    /// Inserts a key-value pair into the IBLT.
    /// </summary>
    /// <param name="key">The key bytes.</param>
    /// <param name="value">The value bytes.</param>
    /// <exception cref="ArgumentNullException">Thrown if key or value is null.</exception>
    /// <exception cref="InvalidOperationException">Thrown if insertion fails.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the IBLT is disposed.</exception>
    public unsafe void Insert(ReadOnlySpan<byte> key, ReadOnlySpan<byte> value)
    {
        CheckAlive();
        if (key == null) throw new ArgumentNullException(nameof(key));
        if (value == null) throw new ArgumentNullException(nameof(value));

        fixed (byte* keyPtr = key)
        fixed (byte* valuePtr = value)
        {
            int result = SketchOxideNative.rateless_iblt_insert(
                NativePtr, keyPtr, (ulong)key.Length, valuePtr, (ulong)value.Length);
            if (result != 0)
                throw new InvalidOperationException("Failed to insert key-value pair");
        }
    }

    /// <summary>
    /// Inserts a string key-value pair into the IBLT.
    /// </summary>
    /// <param name="key">The key string.</param>
    /// <param name="value">The value string.</param>
    /// <exception cref="ArgumentNullException">Thrown if key or value is null.</exception>
    /// <exception cref="InvalidOperationException">Thrown if insertion fails.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the IBLT is disposed.</exception>
    public void Insert(string key, string value)
    {
        if (key == null) throw new ArgumentNullException(nameof(key));
        if (value == null) throw new ArgumentNullException(nameof(value));
        Insert(Encoding.UTF8.GetBytes(key), Encoding.UTF8.GetBytes(value));
    }

    /// <summary>
    /// Deletes a key-value pair from the IBLT.
    /// </summary>
    /// <param name="key">The key bytes.</param>
    /// <param name="value">The value bytes.</param>
    /// <exception cref="ArgumentNullException">Thrown if key or value is null.</exception>
    /// <exception cref="InvalidOperationException">Thrown if deletion fails.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the IBLT is disposed.</exception>
    public unsafe void Delete(ReadOnlySpan<byte> key, ReadOnlySpan<byte> value)
    {
        CheckAlive();
        if (key == null) throw new ArgumentNullException(nameof(key));
        if (value == null) throw new ArgumentNullException(nameof(value));

        fixed (byte* keyPtr = key)
        fixed (byte* valuePtr = value)
        {
            int result = SketchOxideNative.rateless_iblt_delete(
                NativePtr, keyPtr, (ulong)key.Length, valuePtr, (ulong)value.Length);
            if (result != 0)
                throw new InvalidOperationException("Failed to delete key-value pair");
        }
    }

    /// <summary>
    /// Deletes a string key-value pair from the IBLT.
    /// </summary>
    /// <param name="key">The key string.</param>
    /// <param name="value">The value string.</param>
    /// <exception cref="ArgumentNullException">Thrown if key or value is null.</exception>
    /// <exception cref="InvalidOperationException">Thrown if deletion fails.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the IBLT is disposed.</exception>
    public void Delete(string key, string value)
    {
        if (key == null) throw new ArgumentNullException(nameof(key));
        if (value == null) throw new ArgumentNullException(nameof(value));
        Delete(Encoding.UTF8.GetBytes(key), Encoding.UTF8.GetBytes(value));
    }

    /// <summary>
    /// Subtracts another IBLT from this one to compute the symmetric difference.
    /// </summary>
    /// <param name="other">The other IBLT to subtract.</param>
    /// <exception cref="ArgumentNullException">Thrown if other is null.</exception>
    /// <exception cref="InvalidOperationException">Thrown if subtraction fails.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if either IBLT is disposed.</exception>
    public void Subtract(RatelessIBLT other)
    {
        CheckAlive();
        if (other == null) throw new ArgumentNullException(nameof(other));
        other.CheckAlive();

        int result = SketchOxideNative.rateless_iblt_subtract(NativePtr, other.NativePtr);
        if (result != 0)
            throw new InvalidOperationException("Failed to subtract IBLTs");
    }

    /// <summary>
    /// Returns a string representation of the IBLT.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "RatelessIBLT(disposed)";
        return $"RatelessIBLT(expectedDiff={_expectedDiff}, cellSize={_cellSize})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.rateless_iblt_free(NativePtr);
            NativePtr = 0;
        }
    }
}
