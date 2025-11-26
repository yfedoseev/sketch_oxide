using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Frequency;

/// <summary>
/// Frequent Items sketch for finding heavy hitters with error bounds.
///
/// Provides frequency estimation with both upper and lower bounds on counts.
/// This is useful when you need guaranteed bounds on frequency estimates,
/// not just point estimates.
/// </summary>
/// <remarks>
/// Note: FrequentItems does not support merging as the algorithm state cannot be
/// meaningfully combined across multiple instances.
/// </remarks>
public sealed class FrequentItems : NativeSketch
{
    private readonly double _error;

    /// <summary>
    /// Creates a new Frequent Items sketch with the specified error bound.
    /// </summary>
    /// <param name="error">Error parameter controlling accuracy. Must be in range (0, 1).
    /// Smaller values give better accuracy but use more space.</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if error is outside (0, 1).</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    public FrequentItems(double error)
    {
        if (error <= 0 || error >= 1)
            throw new ArgumentOutOfRangeException(nameof(error), error, "Error must be in range (0, 1)");

        _error = error;
        NativePtr = SketchOxideNative.frequentitems_new(error);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native FrequentItems");
    }

    /// <summary>
    /// Private constructor for deserialization.
    /// </summary>
    private FrequentItems(double error, nuint ptr)
    {
        _error = error;
        NativePtr = ptr;
    }

    /// <summary>
    /// Gets the error parameter of this sketch.
    /// </summary>
    public double Error
    {
        get
        {
            CheckAlive();
            return _error;
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
                SketchOxideNative.frequentitems_update(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
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
    /// Queries the frequency bounds for an element.
    /// </summary>
    /// <param name="data">The bytes representing the item to query.</param>
    /// <returns>The lower and upper bounds on the item's frequency.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public FrequencyBounds Query(ReadOnlySpan<byte> data)
    {
        CheckAlive();
        if (data == null) throw new ArgumentNullException(nameof(data));

        unsafe
        {
            fixed (byte* ptr = data)
            {
                SketchOxideNative.frequentitems_query(
                    NativePtr,
                    new Span<byte>(ptr, data.Length).ToArray(),
                    (ulong)data.Length,
                    out ulong lower,
                    out ulong upper);
                return new FrequencyBounds(lower, upper);
            }
        }
    }

    /// <summary>
    /// Queries the frequency bounds for a string element.
    /// </summary>
    /// <param name="value">The string item to query.</param>
    /// <returns>The lower and upper bounds on the item's frequency.</returns>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public FrequencyBounds Query(string value)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        return Query(Encoding.UTF8.GetBytes(value));
    }

    /// <summary>
    /// Serializes the sketch to a byte array.
    /// </summary>
    /// <returns>Serialized sketch bytes.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public byte[] Serialize()
    {
        CheckAlive();
        return SketchOxideNative.frequentitems_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a Frequent Items sketch from a byte array.
    /// </summary>
    /// <param name="data">Serialized sketch bytes.</param>
    /// <param name="error">The error parameter used when creating the original sketch.</param>
    /// <returns>A new FrequentItems instance.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentException">Thrown if data is invalid.</exception>
    public static FrequentItems Deserialize(byte[] data, double error)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.frequentitems_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize FrequentItems: invalid data");

        return new FrequentItems(error, ptr);
    }

    /// <summary>
    /// Returns a string representation of the sketch.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "FrequentItems(disposed)";
        return $"FrequentItems(error={_error})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.frequentitems_free(NativePtr);
            NativePtr = 0;
        }
    }
}
