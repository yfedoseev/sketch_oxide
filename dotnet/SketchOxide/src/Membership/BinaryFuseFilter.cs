using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Membership;

/// <summary>
/// Binary Fuse Filter for static set membership testing.
///
/// An extremely space-efficient static filter that is constructed from a fixed set of items.
/// Once built, no new items can be added, but lookups are very fast and memory-efficient.
/// Provides approximately 9 bits per element space usage with extremely low false positive rates.
/// </summary>
/// <remarks>
/// <para>
/// The Binary Fuse filter provides:
/// - Extremely low space usage (~9 bits per element)
/// - Very fast lookups with excellent cache performance
/// - Static construction - items cannot be added after creation
/// - No deletion support
/// - Lower false positive rate than Bloom filters at similar space usage
/// </para>
/// <para>
/// Ideal for applications where:
/// - The set is known in advance and doesn't change
/// - Minimum memory usage is critical
/// - Very fast lookups are required
/// - Examples: spell checkers, URL blocklists, genome analysis
/// </para>
/// </remarks>
public sealed class BinaryFuseFilter : NativeSketch
{
    /// <summary>
    /// Private constructor - use <see cref="FromItems(byte[][])"/> to create instances.
    /// </summary>
    private BinaryFuseFilter(nuint ptr)
    {
        NativePtr = ptr;
    }

    /// <summary>
    /// Creates a Binary Fuse filter from a collection of items.
    /// </summary>
    /// <param name="items">Array of items to include in the filter. Each item is a byte array.</param>
    /// <returns>A new BinaryFuseFilter containing all specified items.</returns>
    /// <exception cref="ArgumentNullException">Thrown if items is null or contains null elements.</exception>
    /// <exception cref="ArgumentException">Thrown if items is empty or construction fails.</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    /// <remarks>
    /// The filter is built once from all provided items. After construction,
    /// no additional items can be added. All items should be unique for optimal
    /// filter construction.
    /// </remarks>
    public static BinaryFuseFilter FromItems(byte[][] items)
    {
        if (items == null) throw new ArgumentNullException(nameof(items));
        if (items.Length == 0) throw new ArgumentException("Items array cannot be empty", nameof(items));

        foreach (var item in items)
        {
            if (item == null) throw new ArgumentNullException(nameof(items), "Items array contains null element");
        }

        // Get the maximum item length for the native call
        ulong maxLen = 0;
        foreach (var item in items)
        {
            if ((ulong)item.Length > maxLen)
                maxLen = (ulong)item.Length;
        }

        nuint ptr = SketchOxideNative.binaryfusefilter_new(items, (ulong)items.Length, maxLen);

        if (ptr == 0)
            throw new ArgumentException("Failed to construct BinaryFuseFilter: construction failed (possibly duplicate items or allocation failure)");

        return new BinaryFuseFilter(ptr);
    }

    /// <summary>
    /// Creates a Binary Fuse filter from a collection of string items.
    /// </summary>
    /// <param name="items">Array of strings to include in the filter.</param>
    /// <returns>A new BinaryFuseFilter containing all specified items.</returns>
    /// <exception cref="ArgumentNullException">Thrown if items is null or contains null elements.</exception>
    /// <exception cref="ArgumentException">Thrown if items is empty or construction fails.</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    public static BinaryFuseFilter FromItems(string[] items)
    {
        if (items == null) throw new ArgumentNullException(nameof(items));

        byte[][] byteItems = new byte[items.Length][];
        for (int i = 0; i < items.Length; i++)
        {
            if (items[i] == null) throw new ArgumentNullException(nameof(items), "Items array contains null element");
            byteItems[i] = Encoding.UTF8.GetBytes(items[i]);
        }

        return FromItems(byteItems);
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
                return SketchOxideNative.binaryfusefilter_contains(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
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
        return SketchOxideNative.binaryfusefilter_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a Binary Fuse filter from a byte array.
    /// </summary>
    /// <param name="data">Serialized filter bytes.</param>
    /// <returns>A new BinaryFuseFilter instance.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentException">Thrown if data is invalid.</exception>
    public static BinaryFuseFilter Deserialize(byte[] data)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.binaryfusefilter_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize BinaryFuseFilter: invalid data");

        return new BinaryFuseFilter(ptr);
    }

    /// <summary>
    /// Returns a string representation of the filter.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "BinaryFuseFilter(disposed)";
        return "BinaryFuseFilter(static)";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.binaryfusefilter_free(NativePtr);
            NativePtr = 0;
        }
    }
}
