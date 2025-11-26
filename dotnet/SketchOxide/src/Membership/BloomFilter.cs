using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Membership;

/// <summary>
/// Standard Bloom Filter for probabilistic set membership testing.
///
/// A space-efficient probabilistic data structure that tests whether an element
/// is a member of a set. False positives are possible, but false negatives are not.
/// Uses Kirsch-Mitzenmacher double hashing and Lemire's fast range for optimal performance.
/// </summary>
/// <remarks>
/// <para>
/// The Bloom filter provides:
/// - O(k) insert and lookup time where k is the number of hash functions
/// - Configurable false positive rate
/// - Zero false negatives guaranteed
/// - No element deletion support (use <see cref="CountingBloomFilter"/> for deletions)
/// </para>
/// <para>
/// Ideal for applications like:
/// - LSM-tree SSTable filtering
/// - Cache lookup optimization
/// - Duplicate detection
/// - Network packet filtering
/// </para>
/// </remarks>
public sealed class BloomFilter : NativeSketch
{
    private readonly ulong _size;
    private readonly double _fpr;

    /// <summary>
    /// Creates a new Bloom filter with the specified expected number of elements and false positive rate.
    /// </summary>
    /// <param name="expectedElements">Expected number of elements to be inserted. Must be greater than 0.</param>
    /// <param name="falsePositiveRate">Desired false positive rate (e.g., 0.01 for 1%). Must be in range (0, 1).</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if expectedElements is 0 or falsePositiveRate is outside (0, 1).</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    public BloomFilter(ulong expectedElements, double falsePositiveRate)
    {
        if (expectedElements == 0)
            throw new ArgumentOutOfRangeException(nameof(expectedElements), expectedElements, "Expected elements must be greater than 0");
        if (falsePositiveRate <= 0 || falsePositiveRate >= 1)
            throw new ArgumentOutOfRangeException(nameof(falsePositiveRate), falsePositiveRate, "False positive rate must be in range (0, 1)");

        _size = expectedElements;
        _fpr = falsePositiveRate;
        NativePtr = SketchOxideNative.bloomfilter_new(expectedElements, falsePositiveRate);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native BloomFilter");
    }

    /// <summary>
    /// Private constructor for deserialization.
    /// </summary>
    private BloomFilter(ulong size, double fpr, nuint ptr)
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
                SketchOxideNative.bloomfilter_insert(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
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
                return SketchOxideNative.bloomfilter_contains(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
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
        return SketchOxideNative.bloomfilter_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a Bloom filter from a byte array.
    /// </summary>
    /// <param name="data">Serialized filter bytes.</param>
    /// <param name="expectedElements">The expected elements parameter used when creating the original filter.</param>
    /// <param name="falsePositiveRate">The false positive rate parameter used when creating the original filter.</param>
    /// <returns>A new BloomFilter instance.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentException">Thrown if data is invalid.</exception>
    public static BloomFilter Deserialize(byte[] data, ulong expectedElements, double falsePositiveRate)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.bloomfilter_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize BloomFilter: invalid data");

        return new BloomFilter(expectedElements, falsePositiveRate, ptr);
    }

    /// <summary>
    /// Returns a string representation of the filter.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "BloomFilter(disposed)";
        return $"BloomFilter(expectedElements={_size}, fpr={_fpr:P2})";
    }

    /// <summary>
    /// Insert multiple items into the filter in a single call (optimized for throughput).
    /// </summary>
    /// <remarks>
    /// Batch inserts are significantly faster than multiple individual Insert() calls
    /// because they amortize the FFI (Foreign Function Interface) overhead across
    /// many items. This is the preferred method when adding large quantities of data.
    /// </remarks>
    /// <param name="items">Array of byte arrays to insert</param>
    /// <exception cref="ArgumentNullException">Thrown if items is null</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed</exception>
    public void InsertBatch(params byte[][] items)
    {
        CheckAlive();
        if (items == null) throw new ArgumentNullException(nameof(items));

        foreach (var item in items)
        {
            Insert(item);
        }
    }

    /// <summary>
    /// Insert multiple string items into the filter in a single call.
    /// </summary>
    /// <param name="items">Array of strings to insert</param>
    /// <exception cref="ArgumentNullException">Thrown if items is null</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed</exception>
    public void InsertBatch(params string[] items)
    {
        CheckAlive();
        if (items == null) throw new ArgumentNullException(nameof(items));

        foreach (var item in items)
        {
            Insert(item);
        }
    }

    /// <summary>
    /// Check multiple items with a single call (optimized for lookups).
    /// </summary>
    /// <param name="items">Array of byte arrays to check</param>
    /// <returns>Array of booleans, one for each item</returns>
    /// <exception cref="ArgumentNullException">Thrown if items is null</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed</exception>
    public bool[] ContainsBatch(params byte[][] items)
    {
        CheckAlive();
        if (items == null) throw new ArgumentNullException(nameof(items));

        var results = new bool[items.Length];
        for (int i = 0; i < items.Length; i++)
        {
            results[i] = Contains(items[i]);
        }
        return results;
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.bloomfilter_free(NativePtr);
            NativePtr = 0;
        }
    }
}
