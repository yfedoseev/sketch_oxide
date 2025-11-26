using System;
using SketchOxide.Native;

namespace SketchOxide.RangeFilters;

/// <summary>
/// GRF (Gorilla Range Filter): Shape-Based Range Filter for LSM-Trees (SIGMOD 2024).
///
/// GRF uses shape encoding to capture key distribution patterns, providing better false
/// positive rates than traditional range filters for skewed data distributions.
/// </summary>
/// <remarks>
/// Key Innovation:
/// Traditional range filters treat all ranges equally. GRF's shape encoding adapts to
/// key distribution, providing:
/// - Better FPR for skewed distributions (Zipf, power-law)
/// - Adaptive segments that match data patterns
/// - LSM-tree optimization for compaction and merge operations
///
/// Performance Characteristics:
/// - Build: O(n log n) for sorting + O(n) for segmentation
/// - Query: O(log n) binary search + O(k) segment checks
/// - Space: B bits per key (comparable to Grafite)
/// - FPR: Better than Grafite for skewed distributions
///
/// Production Use Cases (2025):
/// - RocksDB/LevelDB SSTable filters
/// - Time-series databases (InfluxDB, TimescaleDB)
/// - Log aggregation systems (Elasticsearch, Loki)
/// - Columnar databases (Parquet, ORC)
/// - Financial time-series data
///
/// References:
/// - "Gorilla Range Filter: Shape-Based Range Filtering for LSM-Trees" (SIGMOD 2024)
/// </remarks>
public sealed class GRF : NativeSketch
{
    private readonly ulong _keyCount;
    private readonly ulong _bitsPerKey;

    /// <summary>
    /// Builds a new GRF filter from a set of sorted keys.
    /// </summary>
    /// <param name="keys">Array of keys to build the filter from (will be sorted internally).</param>
    /// <param name="bitsPerKey">Number of bits per key (typically 4-8).</param>
    /// <exception cref="ArgumentNullException">Thrown if keys is null.</exception>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if parameters are invalid.</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native allocation fails.</exception>
    public unsafe GRF(ulong[] keys, ulong bitsPerKey)
    {
        if (keys == null)
            throw new ArgumentNullException(nameof(keys));
        if (keys.Length == 0)
            throw new ArgumentOutOfRangeException(nameof(keys), "Keys array cannot be empty");
        if (bitsPerKey < 2 || bitsPerKey > 16)
            throw new ArgumentOutOfRangeException(nameof(bitsPerKey), bitsPerKey, "BitsPerKey must be between 2 and 16");

        _keyCount = (ulong)keys.Length;
        _bitsPerKey = bitsPerKey;

        fixed (ulong* keysPtr = keys)
        {
            NativePtr = SketchOxideNative.grf_build(keysPtr, (ulong)keys.Length, bitsPerKey);
        }

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native GRF");
    }

    /// <summary>
    /// Gets the number of keys in the filter.
    /// </summary>
    public ulong KeyCount
    {
        get
        {
            CheckAlive();
            return SketchOxideNative.grf_key_count(NativePtr);
        }
    }

    /// <summary>
    /// Gets the bits per key configuration.
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
    /// Checks if a range may contain keys.
    /// </summary>
    /// <param name="low">Lower bound of the range (inclusive).</param>
    /// <param name="high">Upper bound of the range (inclusive).</param>
    /// <returns>True if the range may contain keys, false if definitely no keys in range.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public bool MayContainRange(ulong low, ulong high)
    {
        CheckAlive();
        return SketchOxideNative.grf_may_contain_range(NativePtr, low, high);
    }

    /// <summary>
    /// Checks if a specific key may be present (point query).
    /// </summary>
    /// <param name="key">The key to check.</param>
    /// <returns>True if the key may be present, false if definitely not present.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public bool MayContain(ulong key)
    {
        CheckAlive();
        return SketchOxideNative.grf_may_contain(NativePtr, key);
    }

    /// <summary>
    /// Gets the expected false positive rate for a given range width.
    /// </summary>
    /// <param name="rangeWidth">Width of the query range.</param>
    /// <returns>Expected false positive rate.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public double ExpectedFpr(ulong rangeWidth)
    {
        CheckAlive();
        return SketchOxideNative.grf_expected_fpr(NativePtr, rangeWidth);
    }

    /// <summary>
    /// Gets statistics about the GRF filter.
    /// </summary>
    /// <returns>A tuple containing (keyCount, segmentCount, avgKeysPerSegment, bitsPerKey, totalBits).</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public unsafe (ulong KeyCount, ulong SegmentCount, double AvgKeysPerSegment, ulong BitsPerKey, ulong TotalBits) GetStats()
    {
        CheckAlive();

        ulong keyCount = 0, segmentCount = 0, bitsPerKey = 0, totalBits = 0;
        double avgKeysPerSegment = 0.0;

        SketchOxideNative.grf_stats(
            NativePtr, &keyCount, &segmentCount, &avgKeysPerSegment, &bitsPerKey, &totalBits);

        return (keyCount, segmentCount, avgKeysPerSegment, bitsPerKey, totalBits);
    }

    /// <summary>
    /// Returns a string representation of the filter.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "GRF(disposed)";
        return $"GRF(keys={_keyCount}, bitsPerKey={_bitsPerKey})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.grf_free(NativePtr);
            NativePtr = 0;
        }
    }
}
