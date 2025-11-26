using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Frequency;

/// <summary>
/// Removable Universal Sketch (RUS) for frequency estimation with support for deletions.
///
/// A universal sketch that supports both insertions and deletions of elements.
/// Unlike Count-Min Sketch which only supports insertions, RUS can accurately
/// track frequency changes when items are removed from the stream.
/// Also provides L2 norm estimation of the frequency vector.
/// </summary>
/// <remarks>
/// Note: RemovableUniversalSketch does not support merging as the deletion-aware
/// state cannot be meaningfully combined across multiple instances.
/// </remarks>
public sealed class RemovableUniversalSketch : NativeSketch
{
    private readonly double _epsilon;
    private readonly double _delta;

    /// <summary>
    /// Creates a new Removable Universal Sketch with the specified accuracy parameters.
    /// </summary>
    /// <param name="epsilon">Error factor, must be in range (0, 1). Smaller values give better accuracy but use more space.</param>
    /// <param name="delta">Failure probability, must be in range (0, 1). Smaller values give higher confidence but use more space.</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if epsilon or delta is outside (0, 1).</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    public RemovableUniversalSketch(double epsilon, double delta)
    {
        if (epsilon <= 0 || epsilon >= 1)
            throw new ArgumentOutOfRangeException(nameof(epsilon), epsilon, "Epsilon must be in range (0, 1)");
        if (delta <= 0 || delta >= 1)
            throw new ArgumentOutOfRangeException(nameof(delta), delta, "Delta must be in range (0, 1)");

        _epsilon = epsilon;
        _delta = delta;
        NativePtr = SketchOxideNative.russketch_new(epsilon, delta);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native RemovableUniversalSketch");
    }

    /// <summary>
    /// Private constructor for deserialization.
    /// </summary>
    private RemovableUniversalSketch(double epsilon, double delta, nuint ptr)
    {
        _epsilon = epsilon;
        _delta = delta;
        NativePtr = ptr;
    }

    /// <summary>
    /// Gets the epsilon (error factor) of this sketch.
    /// </summary>
    public double Epsilon
    {
        get
        {
            CheckAlive();
            return _epsilon;
        }
    }

    /// <summary>
    /// Gets the delta (failure probability) of this sketch.
    /// </summary>
    public double Delta
    {
        get
        {
            CheckAlive();
            return _delta;
        }
    }

    /// <summary>
    /// Updates the sketch with a new element and a delta value (can be positive or negative).
    /// </summary>
    /// <param name="data">The bytes representing the item to update.</param>
    /// <param name="delta">The change in count. Use positive values for insertions, negative for deletions.</param>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public void Update(ReadOnlySpan<byte> data, int delta)
    {
        CheckAlive();
        if (data == null) throw new ArgumentNullException(nameof(data));

        unsafe
        {
            fixed (byte* ptr = data)
            {
                SketchOxideNative.russketch_update(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length, delta);
            }
        }
    }

    /// <summary>
    /// Updates the sketch with a string element and a delta value (can be positive or negative).
    /// </summary>
    /// <param name="value">The string item to update.</param>
    /// <param name="delta">The change in count. Use positive values for insertions, negative for deletions.</param>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public void Update(string value, int delta)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        Update(Encoding.UTF8.GetBytes(value), delta);
    }

    /// <summary>
    /// Inserts an element (increments count by 1).
    /// </summary>
    /// <param name="data">The bytes representing the item to insert.</param>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public void Insert(ReadOnlySpan<byte> data)
    {
        Update(data, 1);
    }

    /// <summary>
    /// Inserts a string element (increments count by 1).
    /// </summary>
    /// <param name="value">The string item to insert.</param>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public void Insert(string value)
    {
        Update(value, 1);
    }

    /// <summary>
    /// Removes an element (decrements count by 1).
    /// </summary>
    /// <param name="data">The bytes representing the item to remove.</param>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public void Remove(ReadOnlySpan<byte> data)
    {
        Update(data, -1);
    }

    /// <summary>
    /// Removes a string element (decrements count by 1).
    /// </summary>
    /// <param name="value">The string item to remove.</param>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public void Remove(string value)
    {
        Update(value, -1);
    }

    /// <summary>
    /// Estimates the frequency count of an element.
    /// </summary>
    /// <param name="data">The bytes representing the item to query.</param>
    /// <returns>The estimated count for the item. May be negative if more deletions than insertions occurred.</returns>
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
                return SketchOxideNative.russketch_estimate(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
            }
        }
    }

    /// <summary>
    /// Estimates the frequency count of a string element.
    /// </summary>
    /// <param name="value">The string item to query.</param>
    /// <returns>The estimated count for the item. May be negative if more deletions than insertions occurred.</returns>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public double Estimate(string value)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        return Estimate(Encoding.UTF8.GetBytes(value));
    }

    /// <summary>
    /// Computes the L2 norm (Euclidean norm) of the frequency vector.
    /// </summary>
    /// <returns>The estimated L2 norm, which is sqrt(sum of squared frequencies).</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public double L2Norm()
    {
        CheckAlive();
        return SketchOxideNative.russketch_l2_norm(NativePtr);
    }

    /// <summary>
    /// Serializes the sketch to a byte array.
    /// </summary>
    /// <returns>Serialized sketch bytes.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public byte[] Serialize()
    {
        CheckAlive();
        return SketchOxideNative.russketch_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a Removable Universal Sketch from a byte array.
    /// </summary>
    /// <param name="data">Serialized sketch bytes.</param>
    /// <param name="epsilon">The epsilon parameter used when creating the original sketch.</param>
    /// <param name="delta">The delta parameter used when creating the original sketch.</param>
    /// <returns>A new RemovableUniversalSketch instance.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentException">Thrown if data is invalid.</exception>
    public static RemovableUniversalSketch Deserialize(byte[] data, double epsilon, double delta)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.russketch_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize RemovableUniversalSketch: invalid data");

        return new RemovableUniversalSketch(epsilon, delta, ptr);
    }

    /// <summary>
    /// Returns a string representation of the sketch.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "RemovableUniversalSketch(disposed)";
        return $"RemovableUniversalSketch(epsilon={_epsilon}, delta={_delta})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.russketch_free(NativePtr);
            NativePtr = 0;
        }
    }
}
