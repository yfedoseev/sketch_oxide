using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Frequency;

/// <summary>
/// SALSA (Self-Adjusting Lean Streaming Analytics) sketch for frequency estimation.
///
/// A novel sketch that dynamically adjusts counter sizes based on observed frequencies.
/// SALSA starts with small counters and expands them as needed, achieving better space
/// efficiency than fixed-width counter approaches while maintaining accuracy.
/// </summary>
/// <remarks>
/// Note: SALSA does not support merging as the self-adjusting state cannot be
/// meaningfully combined across multiple instances.
/// </remarks>
public sealed class SALSA : NativeSketch
{
    private readonly double _confidenceMetric;

    /// <summary>
    /// Creates a new SALSA sketch with the specified confidence metric.
    /// </summary>
    /// <param name="confidenceMetric">Confidence metric controlling the sketch behavior.
    /// Must be greater than 0. Higher values provide more confidence in estimates.</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if confidenceMetric is not positive.</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    public SALSA(double confidenceMetric)
    {
        if (confidenceMetric <= 0)
            throw new ArgumentOutOfRangeException(nameof(confidenceMetric), confidenceMetric, "Confidence metric must be greater than 0");

        _confidenceMetric = confidenceMetric;
        NativePtr = SketchOxideNative.salsa_new(confidenceMetric);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native SALSA");
    }

    /// <summary>
    /// Private constructor for deserialization.
    /// </summary>
    private SALSA(double confidenceMetric, nuint ptr)
    {
        _confidenceMetric = confidenceMetric;
        NativePtr = ptr;
    }

    /// <summary>
    /// Gets the confidence metric of this sketch.
    /// </summary>
    public double ConfidenceMetric
    {
        get
        {
            CheckAlive();
            return _confidenceMetric;
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
                SketchOxideNative.salsa_update(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
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
    /// Estimates the frequency count of an element.
    /// </summary>
    /// <param name="data">The bytes representing the item to query.</param>
    /// <returns>The estimated count for the item.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public ulong Estimate(ReadOnlySpan<byte> data)
    {
        CheckAlive();
        if (data == null) throw new ArgumentNullException(nameof(data));

        unsafe
        {
            fixed (byte* ptr = data)
            {
                return SketchOxideNative.salsa_estimate(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
            }
        }
    }

    /// <summary>
    /// Estimates the frequency count of a string element.
    /// </summary>
    /// <param name="value">The string item to query.</param>
    /// <returns>The estimated count for the item.</returns>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public ulong Estimate(string value)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        return Estimate(Encoding.UTF8.GetBytes(value));
    }

    /// <summary>
    /// Serializes the sketch to a byte array.
    /// </summary>
    /// <returns>Serialized sketch bytes.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public byte[] Serialize()
    {
        CheckAlive();
        return SketchOxideNative.salsa_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a SALSA sketch from a byte array.
    /// </summary>
    /// <param name="data">Serialized sketch bytes.</param>
    /// <param name="confidenceMetric">The confidence metric used when creating the original sketch.</param>
    /// <returns>A new SALSA instance.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentException">Thrown if data is invalid.</exception>
    public static SALSA Deserialize(byte[] data, double confidenceMetric)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.salsa_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize SALSA: invalid data");

        return new SALSA(confidenceMetric, ptr);
    }

    /// <summary>
    /// Returns a string representation of the sketch.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "SALSA(disposed)";
        return $"SALSA(confidenceMetric={_confidenceMetric})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.salsa_free(NativePtr);
            NativePtr = 0;
        }
    }
}
