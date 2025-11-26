using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Frequency;

/// <summary>
/// Count Sketch for frequency estimation with support for positive and negative updates.
///
/// Unlike Count-Min Sketch, Count Sketch can provide unbiased frequency estimates
/// and can handle both increments and decrements. The sketch uses width and depth
/// parameters to control accuracy and memory usage.
/// </summary>
public sealed class CountSketch : NativeSketch, IMergeableSketch<CountSketch>
{
    private readonly uint _width;
    private readonly uint _depth;

    /// <summary>
    /// Creates a new Count Sketch with the specified dimensions.
    /// </summary>
    /// <param name="width">Width of the sketch (number of counters per row). Must be greater than 0.</param>
    /// <param name="depth">Depth of the sketch (number of hash functions/rows). Must be greater than 0.</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if width or depth is 0.</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    public CountSketch(uint width, uint depth)
    {
        if (width == 0)
            throw new ArgumentOutOfRangeException(nameof(width), width, "Width must be greater than 0");
        if (depth == 0)
            throw new ArgumentOutOfRangeException(nameof(depth), depth, "Depth must be greater than 0");

        _width = width;
        _depth = depth;
        NativePtr = SketchOxideNative.countsketch_new(width, depth);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native CountSketch");
    }

    /// <summary>
    /// Private constructor for deserialization.
    /// </summary>
    private CountSketch(uint width, uint depth, nuint ptr)
    {
        _width = width;
        _depth = depth;
        NativePtr = ptr;
    }

    /// <summary>
    /// Gets the width (number of counters per row) of this sketch.
    /// </summary>
    public uint Width
    {
        get
        {
            CheckAlive();
            return _width;
        }
    }

    /// <summary>
    /// Gets the depth (number of hash functions) of this sketch.
    /// </summary>
    public uint Depth
    {
        get
        {
            CheckAlive();
            return _depth;
        }
    }

    /// <summary>
    /// Updates the sketch with a new element, incrementing its count by 1.
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
                SketchOxideNative.countsketch_update(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
            }
        }
    }

    /// <summary>
    /// Updates the sketch with a string element, incrementing its count by 1.
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
    /// Estimates the frequency count of an element.
    /// </summary>
    /// <param name="data">The bytes representing the item to query.</param>
    /// <returns>The estimated count for the item. Can be negative due to the nature of Count Sketch.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public double Estimate(ReadOnlySpan<byte> data)
    {
        CheckAlive();
        if (data == null) throw new ArgumentNullException(nameof(data));

        unsafe
        {
            fixed (byte* ptr = data)
            {
                return SketchOxideNative.countsketch_estimate(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
            }
        }
    }

    /// <summary>
    /// Estimates the frequency count of a string element.
    /// </summary>
    /// <param name="value">The string item to query.</param>
    /// <returns>The estimated count for the item. Can be negative due to the nature of Count Sketch.</returns>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public double Estimate(string value)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        return Estimate(Encoding.UTF8.GetBytes(value));
    }

    /// <summary>
    /// Merges another Count Sketch into this one.
    /// </summary>
    /// <param name="other">The sketch to merge. Must have the same width and depth.</param>
    /// <exception cref="ArgumentNullException">Thrown if other is null.</exception>
    /// <exception cref="ArgumentException">Thrown if sketches have different dimensions.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if either sketch is disposed.</exception>
    public void Merge(CountSketch other)
    {
        CheckAlive();
        if (other == null) throw new ArgumentNullException(nameof(other));
        other.CheckAlive();

        if (_width != other._width || _depth != other._depth)
            throw new ArgumentException($"Cannot merge sketches with different dimensions: ({_width}, {_depth}) != ({other._width}, {other._depth})");

        SketchOxideNative.countsketch_merge(NativePtr, other.NativePtr);
    }

    /// <summary>
    /// Serializes the sketch to a byte array.
    /// </summary>
    /// <returns>Serialized sketch bytes.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public byte[] Serialize()
    {
        CheckAlive();
        return SketchOxideNative.countsketch_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a Count Sketch from a byte array.
    /// </summary>
    /// <param name="data">Serialized sketch bytes.</param>
    /// <param name="width">The width parameter used when creating the original sketch.</param>
    /// <param name="depth">The depth parameter used when creating the original sketch.</param>
    /// <returns>A new CountSketch instance.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentException">Thrown if data is invalid.</exception>
    public static CountSketch Deserialize(byte[] data, uint width, uint depth)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.countsketch_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize CountSketch: invalid data");

        return new CountSketch(width, depth, ptr);
    }

    /// <summary>
    /// Returns a string representation of the sketch.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "CountSketch(disposed)";
        return $"CountSketch(width={_width}, depth={_depth})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.countsketch_free(NativePtr);
            NativePtr = 0;
        }
    }
}
