using System;
using SketchOxide.Native;

namespace SketchOxide.RangeFilters;

/// <summary>
/// Grafite: Optimal Range Filter with Robust FPR Bounds.
///
/// The first optimal range filter with adversarial-robust guarantees, providing
/// a false positive rate of L / 2^(B-2) where L is query range size and B is bits per key.
/// </summary>
/// <remarks>
/// Grafite works by:
/// 1. Collecting and sorting keys to be filtered
/// 2. Assigning B-bit fingerprints based on position
/// 3. For range queries: binary search + fingerprint checking
///
/// Key Properties:
/// - Optimal FPR: L / 2^(B-2) for range width L
/// - Adversarial Robust: Worst-case bounds hold even with adversarial queries
/// - No False Negatives: Always returns true for ranges containing keys
/// - Space Efficient: B bits per key (typically 4-8 bits)
///
/// Production Use Cases (2025):
/// - LSM-tree range queries (RocksDB, LevelDB)
/// - Database index optimization
/// - Time-series databases
/// - Financial market data (range lookups on timestamps)
/// - Log aggregation systems
/// </remarks>
public sealed class Grafite : NativeSketch
{
    private readonly ulong _bitsPerKey;
    private readonly ulong _keyCount;

    /// <summary>
    /// Builds a new Grafite filter from sorted keys.
    /// </summary>
    /// <param name="keys">Array of sorted keys (must be sorted ascending).</param>
    /// <param name="bitsPerKey">Number of bits per key (typically 4-8).</param>
    /// <exception cref="ArgumentNullException">Thrown if keys is null.</exception>
    /// <exception cref="ArgumentException">Thrown if keys array is empty or bitsPerKey is invalid.</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native allocation fails.</exception>
    public unsafe Grafite(ulong[] keys, ulong bitsPerKey)
    {
        if (keys == null) throw new ArgumentNullException(nameof(keys));
        if (keys.Length == 0)
            throw new ArgumentException("keys array must not be empty", nameof(keys));
        if (bitsPerKey == 0 || bitsPerKey > 64)
            throw new ArgumentOutOfRangeException(nameof(bitsPerKey), bitsPerKey,
                "bitsPerKey must be in range [1, 64]");

        _keyCount = (ulong)keys.Length;
        _bitsPerKey = bitsPerKey;

        fixed (ulong* keysPtr = keys)
        {
            NativePtr = SketchOxideNative.grafite_build(keysPtr, (ulong)keys.Length, bitsPerKey);
        }

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native Grafite filter");
    }

    /// <summary>
    /// Gets the number of bits per key.
    /// </summary>
    public ulong BitsPerKey
    {
        get
        {
            CheckAlive();
            return _bitsPerKey;
        }
    }

    /// <summary>
    /// Gets the number of keys in the filter.
    /// </summary>
    public ulong KeyCount
    {
        get
        {
            CheckAlive();
            return _keyCount;
        }
    }

    /// <summary>
    /// Checks if a range [low, high] may contain any keys.
    /// </summary>
    /// <param name="low">Lower bound of the range (inclusive).</param>
    /// <param name="high">Upper bound of the range (inclusive).</param>
    /// <returns>True if the range may contain keys, false if definitely empty.</returns>
    /// <exception cref="ArgumentException">Thrown if low > high.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public bool MayContainRange(ulong low, ulong high)
    {
        CheckAlive();
        if (low > high)
            throw new ArgumentException($"Invalid range: low ({low}) > high ({high})");

        return SketchOxideNative.grafite_may_contain_range(NativePtr, low, high);
    }

    /// <summary>
    /// Checks if a specific key may be present (point query).
    /// </summary>
    /// <param name="key">The key to check.</param>
    /// <returns>True if the key may be present, false if definitely absent.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public bool MayContain(ulong key)
    {
        CheckAlive();
        return SketchOxideNative.grafite_may_contain(NativePtr, key);
    }

    /// <summary>
    /// Gets the expected false positive rate for a given range width.
    /// </summary>
    /// <param name="rangeWidth">The width of the query range.</param>
    /// <returns>Expected FPR = rangeWidth / 2^(bitsPerKey - 2).</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public double ExpectedFpr(ulong rangeWidth)
    {
        CheckAlive();
        return SketchOxideNative.grafite_expected_fpr(NativePtr, rangeWidth);
    }

    /// <summary>
    /// Returns a string representation of the filter.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "Grafite(disposed)";
        return $"Grafite(keys={_keyCount}, bitsPerKey={_bitsPerKey})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.grafite_free(NativePtr);
            NativePtr = 0;
        }
    }
}
