using System;
using System.Collections.Generic;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Frequency;

/// <summary>
/// Space Saving algorithm for finding the top-k frequent items in a stream.
///
/// Provides deterministic guarantees for finding heavy hitters (frequent items)
/// using only O(k) space. Maintains a fixed number of counters and tracks
/// the most frequent items seen in the stream.
/// </summary>
/// <remarks>
/// Note: Space Saving does not support merging as the algorithm state cannot be
/// meaningfully combined across multiple instances.
/// </remarks>
public sealed class SpaceSaving : NativeSketch
{
    private readonly uint _k;

    /// <summary>
    /// Creates a new Space Saving sketch that tracks the top-k frequent items.
    /// </summary>
    /// <param name="k">The number of frequent items to track. Must be greater than 0.</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if k is 0.</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    public SpaceSaving(uint k)
    {
        if (k == 0)
            throw new ArgumentOutOfRangeException(nameof(k), k, "K must be greater than 0");

        _k = k;
        NativePtr = SketchOxideNative.spacesaving_new(k);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native SpaceSaving");
    }

    /// <summary>
    /// Private constructor for deserialization.
    /// </summary>
    private SpaceSaving(uint k, nuint ptr)
    {
        _k = k;
        NativePtr = ptr;
    }

    /// <summary>
    /// Gets the k value (number of items tracked) of this sketch.
    /// </summary>
    public uint K
    {
        get
        {
            CheckAlive();
            return _k;
        }
    }

    /// <summary>
    /// Updates the sketch with a new element, incrementing its count.
    /// </summary>
    /// <param name="data">The bytes representing the item to count.</param>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public void Update(ReadOnlySpan<byte> data)
    {
        CheckAlive();
        if (data == null) throw new ArgumentNullException(nameof(data));

        unsafe
        {
            fixed (byte* ptr = data)
            {
                SketchOxideNative.spacesaving_update(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
            }
        }
    }

    /// <summary>
    /// Updates the sketch with a string element, incrementing its count.
    /// </summary>
    /// <param name="value">The string item to count.</param>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public void Update(string value)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        Update(Encoding.UTF8.GetBytes(value));
    }

    /// <summary>
    /// Gets the top-k frequent items and their estimated counts.
    /// </summary>
    /// <returns>A list of frequent items ordered by estimated frequency (descending).</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public IReadOnlyList<FrequentItem> TopK()
    {
        CheckAlive();

        byte[] rawData = SketchOxideNative.spacesaving_top_k(NativePtr, out uint count);

        var result = new List<FrequentItem>((int)count);

        if (rawData == null || rawData.Length == 0 || count == 0)
            return result;

        // Parse the serialized data format: [len(4 bytes), item(len bytes), count(8 bytes)]...
        int offset = 0;
        for (uint i = 0; i < count && offset < rawData.Length; i++)
        {
            if (offset + 4 > rawData.Length) break;

            // Read item length (4 bytes, little-endian)
            int itemLen = BitConverter.ToInt32(rawData, offset);
            offset += 4;

            if (offset + itemLen > rawData.Length) break;

            // Read item bytes
            byte[] item = new byte[itemLen];
            Array.Copy(rawData, offset, item, 0, itemLen);
            offset += itemLen;

            if (offset + 8 > rawData.Length) break;

            // Read count (8 bytes, little-endian)
            ulong itemCount = BitConverter.ToUInt64(rawData, offset);
            offset += 8;

            result.Add(new FrequentItem(item, itemCount));
        }

        return result;
    }

    /// <summary>
    /// Serializes the sketch to a byte array.
    /// </summary>
    /// <returns>Serialized sketch bytes.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public byte[] Serialize()
    {
        CheckAlive();
        return SketchOxideNative.spacesaving_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a Space Saving sketch from a byte array.
    /// </summary>
    /// <param name="data">Serialized sketch bytes.</param>
    /// <param name="k">The k parameter used when creating the original sketch.</param>
    /// <returns>A new SpaceSaving instance.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentException">Thrown if data is invalid.</exception>
    public static SpaceSaving Deserialize(byte[] data, uint k)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.spacesaving_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize SpaceSaving: invalid data");

        return new SpaceSaving(k, ptr);
    }

    /// <summary>
    /// Returns a string representation of the sketch.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "SpaceSaving(disposed)";
        return $"SpaceSaving(k={_k})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.spacesaving_free(NativePtr);
            NativePtr = 0;
        }
    }
}
