using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Membership;

/// <summary>
/// Cache-optimized Blocked Bloom Filter for probabilistic set membership testing.
///
/// A variant of Bloom filter that groups bits into cache-line-sized blocks,
/// improving cache locality and memory access patterns. This results in
/// significantly faster lookups, especially for large filters.
/// </summary>
/// <remarks>
/// <para>
/// The Blocked Bloom filter provides:
/// - Better cache performance than standard Bloom filter
/// - Single cache-line access per lookup
/// - Slightly higher false positive rate than standard Bloom filter
/// - Same memory usage as standard Bloom filter
/// </para>
/// <para>
/// Ideal for applications where:
/// - Lookup performance is critical
/// - Filter size exceeds CPU cache
/// - High throughput is required
/// </para>
/// </remarks>
public sealed class BlockedBloomFilter : NativeSketch
{
    private readonly ulong _size;
    private readonly double _fpr;

    /// <summary>
    /// Creates a new Blocked Bloom filter with the specified expected number of elements and false positive rate.
    /// </summary>
    /// <param name="expectedElements">Expected number of elements to be inserted. Must be greater than 0.</param>
    /// <param name="falsePositiveRate">Desired false positive rate (e.g., 0.01 for 1%). Must be in range (0, 1).</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if expectedElements is 0 or falsePositiveRate is outside (0, 1).</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    public BlockedBloomFilter(ulong expectedElements, double falsePositiveRate)
    {
        if (expectedElements == 0)
            throw new ArgumentOutOfRangeException(nameof(expectedElements), expectedElements, "Expected elements must be greater than 0");
        if (falsePositiveRate <= 0 || falsePositiveRate >= 1)
            throw new ArgumentOutOfRangeException(nameof(falsePositiveRate), falsePositiveRate, "False positive rate must be in range (0, 1)");

        _size = expectedElements;
        _fpr = falsePositiveRate;
        NativePtr = SketchOxideNative.blockedbloomfilter_new(expectedElements, falsePositiveRate);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native BlockedBloomFilter");
    }

    /// <summary>
    /// Private constructor for deserialization.
    /// </summary>
    private BlockedBloomFilter(ulong size, double fpr, nuint ptr)
    {
        _size = size;
        _fpr = fpr;
        NativePtr = ptr;
    }

    /// <summary>
    /// Gets the expected number of elements this filter was configured for.
    /// </summary>
    public ulong ExpectedElements
    {
        get
        {
            CheckAlive();
            return _size;
        }
    }

    /// <summary>
    /// Gets the target false positive rate.
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
                SketchOxideNative.blockedbloomfilter_insert(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
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
    /// <c>false</c> if the element is definitely not in the set (no false negatives).
    /// </returns>
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
                return SketchOxideNative.blockedbloomfilter_contains(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
            }
        }
    }

    /// <summary>
    /// Tests whether a string element might be in the set.
    /// </summary>
    /// <param name="value">The string to query.</param>
    /// <returns>
    /// <c>true</c> if the element might be in the set (may be a false positive);
    /// <c>false</c> if the element is definitely not in the set (no false negatives).
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
        return SketchOxideNative.blockedbloomfilter_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a Blocked Bloom filter from a byte array.
    /// </summary>
    /// <param name="data">Serialized filter bytes.</param>
    /// <param name="expectedElements">The expected elements parameter used when creating the original filter.</param>
    /// <param name="falsePositiveRate">The false positive rate parameter used when creating the original filter.</param>
    /// <returns>A new BlockedBloomFilter instance.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentException">Thrown if data is invalid.</exception>
    public static BlockedBloomFilter Deserialize(byte[] data, ulong expectedElements, double falsePositiveRate)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.blockedbloomfilter_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize BlockedBloomFilter: invalid data");

        return new BlockedBloomFilter(expectedElements, falsePositiveRate, ptr);
    }

    /// <summary>
    /// Returns a string representation of the filter.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "BlockedBloomFilter(disposed)";
        return $"BlockedBloomFilter(expectedElements={_size}, fpr={_fpr:P2})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.blockedbloomfilter_free(NativePtr);
            NativePtr = 0;
        }
    }
}
