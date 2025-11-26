using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Membership;

/// <summary>
/// Stable Bloom Filter for membership testing in unbounded data streams.
///
/// A variant of Bloom filter designed for infinite or unbounded data streams.
/// Unlike standard Bloom filters that eventually saturate, Stable Bloom filters
/// continuously evict stale information to make room for new elements, maintaining
/// a stable false positive rate over time.
/// </summary>
/// <remarks>
/// <para>
/// The Stable Bloom filter provides:
/// - Continuous operation on unbounded streams
/// - Stable false positive rate over time
/// - Automatic eviction of old elements
/// - Bounded memory usage regardless of stream length
/// - Biased toward recent elements (older elements may be forgotten)
/// </para>
/// <para>
/// How it works:
/// On each insert, the filter randomly decrements a subset of counters before
/// setting the bits for the new element. This creates a "forgetting" mechanism
/// that prevents saturation while maintaining approximate membership testing.
/// </para>
/// <para>
/// Ideal for applications where:
/// - Data stream is potentially infinite
/// - Recent elements are more important than old ones
/// - Memory must be bounded
/// - Examples: network monitoring, duplicate detection in streams, web crawling
/// </para>
/// </remarks>
public sealed class StableBloomFilter : NativeSketch
{
    private readonly ulong _size;
    private readonly double _fpr;

    /// <summary>
    /// Creates a new Stable Bloom filter with the specified parameters.
    /// </summary>
    /// <param name="cells">Number of cells in the filter. More cells means better accuracy but more memory.</param>
    /// <param name="falsePositiveRate">Target stable false positive rate (e.g., 0.01 for 1%). Must be in range (0, 1).</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if cells is 0 or falsePositiveRate is outside (0, 1).</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    public StableBloomFilter(ulong cells, double falsePositiveRate)
    {
        if (cells == 0)
            throw new ArgumentOutOfRangeException(nameof(cells), cells, "Number of cells must be greater than 0");
        if (falsePositiveRate <= 0 || falsePositiveRate >= 1)
            throw new ArgumentOutOfRangeException(nameof(falsePositiveRate), falsePositiveRate, "False positive rate must be in range (0, 1)");

        _size = cells;
        _fpr = falsePositiveRate;
        NativePtr = SketchOxideNative.stablebloomfilter_new(cells, falsePositiveRate);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native StableBloomFilter");
    }

    /// <summary>
    /// Private constructor for deserialization.
    /// </summary>
    private StableBloomFilter(ulong size, double fpr, nuint ptr)
    {
        _size = size;
        _fpr = fpr;
        NativePtr = ptr;
    }

    /// <summary>
    /// Gets the number of cells in this filter.
    /// </summary>
    public ulong Cells
    {
        get
        {
            CheckAlive();
            return _size;
        }
    }

    /// <summary>
    /// Gets the target stable false positive rate.
    /// </summary>
    public double FalsePositiveRate
    {
        get
        {
            CheckAlive();
            return _fpr;
        }
    }

    /// <summary>
    /// Inserts an element into the filter.
    /// </summary>
    /// <param name="data">The bytes representing the element to insert.</param>
    /// <remarks>
    /// This operation also randomly decrements some existing counters,
    /// gradually forgetting old elements to prevent saturation.
    /// </remarks>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public void Insert(ReadOnlySpan<byte> data)
    {
        CheckAlive();
        if (data == null) throw new ArgumentNullException(nameof(data));

        unsafe
        {
            fixed (byte* ptr = data)
            {
                SketchOxideNative.stablebloomfilter_insert(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
            }
        }
    }

    /// <summary>
    /// Inserts a string element into the filter.
    /// </summary>
    /// <param name="value">The string to insert.</param>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public void Insert(string value)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        Insert(Encoding.UTF8.GetBytes(value));
    }

    /// <summary>
    /// Tests whether an element might be in the set.
    /// </summary>
    /// <param name="data">The bytes representing the element to query.</param>
    /// <returns>
    /// <c>true</c> if the element might be in the set (may be a false positive);
    /// <c>false</c> if the element is definitely not in the set.
    /// </returns>
    /// <remarks>
    /// Note that elements inserted long ago may return false even if they were
    /// previously inserted, as the filter continuously forgets old information.
    /// This is expected behavior for stream processing.
    /// </remarks>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public bool Contains(ReadOnlySpan<byte> data)
    {
        CheckAlive();
        if (data == null) throw new ArgumentNullException(nameof(data));

        unsafe
        {
            fixed (byte* ptr = data)
            {
                return SketchOxideNative.stablebloomfilter_contains(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
            }
        }
    }

    /// <summary>
    /// Tests whether a string element might be in the set.
    /// </summary>
    /// <param name="value">The string to query.</param>
    /// <returns>
    /// <c>true</c> if the element might be in the set (may be a false positive);
    /// <c>false</c> if the element is definitely not in the set.
    /// </returns>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public bool Contains(string value)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        return Contains(Encoding.UTF8.GetBytes(value));
    }

    /// <summary>
    /// Serializes the filter to a byte array.
    /// </summary>
    /// <returns>Serialized filter bytes.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public byte[] Serialize()
    {
        CheckAlive();
        return SketchOxideNative.stablebloomfilter_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a Stable Bloom filter from a byte array.
    /// </summary>
    /// <param name="data">Serialized filter bytes.</param>
    /// <param name="cells">The cells parameter used when creating the original filter.</param>
    /// <param name="falsePositiveRate">The false positive rate parameter used when creating the original filter.</param>
    /// <returns>A new StableBloomFilter instance.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentException">Thrown if data is invalid.</exception>
    public static StableBloomFilter Deserialize(byte[] data, ulong cells, double falsePositiveRate)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.stablebloomfilter_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize StableBloomFilter: invalid data");

        return new StableBloomFilter(cells, falsePositiveRate, ptr);
    }

    /// <summary>
    /// Returns a string representation of the filter.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "StableBloomFilter(disposed)";
        return $"StableBloomFilter(cells={_size}, fpr={_fpr:P2})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.stablebloomfilter_free(NativePtr);
            NativePtr = 0;
        }
    }
}
