using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Membership;

/// <summary>
/// Cuckoo Filter for probabilistic set membership testing with deletion support.
///
/// A space-efficient probabilistic data structure that supports adding, removing,
/// and testing elements for set membership. Uses cuckoo hashing with fingerprints
/// for better space efficiency than counting Bloom filters.
/// </summary>
/// <remarks>
/// <para>
/// The Cuckoo filter provides:
/// - Support for element deletion (unlike standard Bloom filter)
/// - Better space efficiency than Counting Bloom filter
/// - Insert may fail if filter is too full (returns false)
/// - Lookup and delete operations are always successful
/// - Approximately 95% load factor before insertions fail
/// </para>
/// <para>
/// Ideal for applications where:
/// - Element deletion is required
/// - Space efficiency is important
/// - Insert failures can be handled gracefully
/// </para>
/// </remarks>
public sealed class CuckooFilter : NativeSketch
{
    private readonly ulong _size;

    /// <summary>
    /// Creates a new Cuckoo filter with the specified capacity.
    /// </summary>
    /// <param name="size">Maximum number of elements the filter can hold. Must be greater than 0.</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if size is 0.</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    public CuckooFilter(ulong size)
    {
        if (size == 0)
            throw new ArgumentOutOfRangeException(nameof(size), size, "Size must be greater than 0");

        _size = size;
        NativePtr = SketchOxideNative.cuckoofilter_new(size);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native CuckooFilter");
    }

    /// <summary>
    /// Private constructor for deserialization.
    /// </summary>
    private CuckooFilter(ulong size, nuint ptr)
    {
        _size = size;
        NativePtr = ptr;
    }

    /// <summary>
    /// Gets the capacity of this filter.
    /// </summary>
    public ulong Capacity
    {
        get
        {
            CheckAlive();
            return _size;
        }
    }

    /// <summary>
    /// Inserts an element into the filter.
    /// </summary>
    /// <param name="data">The bytes representing the element to insert.</param>
    /// <returns>
    /// <c>true</c> if the element was successfully inserted;
    /// <c>false</c> if the filter is too full and insertion failed.
    /// </returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public bool Insert(ReadOnlySpan<byte> data)
    {
        CheckAlive();
        if (data == null) throw new ArgumentNullException(nameof(data));

        unsafe
        {
            fixed (byte* ptr = data)
            {
                return SketchOxideNative.cuckoofilter_insert(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
            }
        }
    }

    /// <summary>
    /// Inserts a string element into the filter.
    /// </summary>
    /// <param name="value">The string to insert.</param>
    /// <returns>
    /// <c>true</c> if the element was successfully inserted;
    /// <c>false</c> if the filter is too full and insertion failed.
    /// </returns>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public bool Insert(string value)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        return Insert(Encoding.UTF8.GetBytes(value));
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
                return SketchOxideNative.cuckoofilter_contains(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
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
    /// Removes an element from the filter.
    /// </summary>
    /// <param name="data">The bytes representing the element to remove.</param>
    /// <returns>
    /// <c>true</c> if the element was found and removed;
    /// <c>false</c> if the element was not found in the filter.
    /// </returns>
    /// <remarks>
    /// Unlike Counting Bloom filters, Cuckoo filters can safely handle remove
    /// operations for elements that may not have been inserted, though this
    /// could still cause issues if the fingerprint matches another element.
    /// </remarks>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public bool Remove(ReadOnlySpan<byte> data)
    {
        CheckAlive();
        if (data == null) throw new ArgumentNullException(nameof(data));

        unsafe
        {
            fixed (byte* ptr = data)
            {
                return SketchOxideNative.cuckoofilter_remove(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
            }
        }
    }

    /// <summary>
    /// Removes a string element from the filter.
    /// </summary>
    /// <param name="value">The string to remove.</param>
    /// <returns>
    /// <c>true</c> if the element was found and removed;
    /// <c>false</c> if the element was not found in the filter.
    /// </returns>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public bool Remove(string value)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        return Remove(Encoding.UTF8.GetBytes(value));
    }

    /// <summary>
    /// Serializes the filter to a byte array.
    /// </summary>
    /// <returns>Serialized filter bytes.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public byte[] Serialize()
    {
        CheckAlive();
        return SketchOxideNative.cuckoofilter_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a Cuckoo filter from a byte array.
    /// </summary>
    /// <param name="data">Serialized filter bytes.</param>
    /// <param name="size">The size parameter used when creating the original filter.</param>
    /// <returns>A new CuckooFilter instance.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentException">Thrown if data is invalid.</exception>
    public static CuckooFilter Deserialize(byte[] data, ulong size)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.cuckoofilter_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize CuckooFilter: invalid data");

        return new CuckooFilter(size, ptr);
    }

    /// <summary>
    /// Returns a string representation of the filter.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "CuckooFilter(disposed)";
        return $"CuckooFilter(capacity={_size})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.cuckoofilter_free(NativePtr);
            NativePtr = 0;
        }
    }
}
