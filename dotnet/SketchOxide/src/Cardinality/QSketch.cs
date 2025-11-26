using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Cardinality;

/// <summary>
/// QSketch for weighted cardinality estimation.
///
/// Supports both unweighted and weighted updates for cardinality estimation.
/// </summary>
public sealed class QSketch : NativeSketch, IMergeableSketch<QSketch>
{
    private readonly uint _maxSamples;

    /// <summary>
    /// Creates a new QSketch.
    /// </summary>
    /// <param name="maxSamples">Maximum number of samples to maintain.</param>
    public QSketch(uint maxSamples)
    {
        if (maxSamples == 0)
            throw new ArgumentException("maxSamples must be positive");

        _maxSamples = maxSamples;
        NativePtr = SketchOxideNative.qsketch_new(maxSamples);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native QSketch");
    }

    /// <summary>
    /// Gets the maximum number of samples.
    /// </summary>
    public uint MaxSamples
    {
        get
        {
            CheckAlive();
            return _maxSamples;
        }
    }

    /// <summary>
    /// Updates the sketch with an unweighted element.
    /// </summary>
    public void Update(ReadOnlySpan<byte> data)
    {
        CheckAlive();
        if (data == null) throw new ArgumentNullException(nameof(data));

        unsafe
        {
            fixed (byte* ptr = data)
            {
                SketchOxideNative.qsketch_update(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
            }
        }
    }

    /// <summary>
    /// Updates the sketch with a weighted element.
    /// </summary>
    public void UpdateWeighted(ReadOnlySpan<byte> data, double weight)
    {
        CheckAlive();
        if (data == null) throw new ArgumentNullException(nameof(data));
        if (weight <= 0) throw new ArgumentException("Weight must be positive");

        unsafe
        {
            fixed (byte* ptr = data)
            {
                SketchOxideNative.qsketch_update_weighted(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length, weight);
            }
        }
    }

    /// <summary>
    /// Updates the sketch with a string element.
    /// </summary>
    public void Update(string value)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        Update(Encoding.UTF8.GetBytes(value));
    }

    /// <summary>
    /// Estimates the cardinality.
    /// </summary>
    public double Estimate()
    {
        CheckAlive();
        return SketchOxideNative.qsketch_estimate(NativePtr);
    }

    /// <summary>
    /// Merges another QSketch into this one.
    /// </summary>
    public void Merge(QSketch other)
    {
        CheckAlive();
        if (other == null) throw new ArgumentNullException(nameof(other));
        other.CheckAlive();

        SketchOxideNative.qsketch_merge(NativePtr, other.NativePtr);
    }

    /// <summary>
    /// Serializes the sketch.
    /// </summary>
    public byte[] Serialize()
    {
        CheckAlive();
        return SketchOxideNative.qsketch_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a QSketch.
    /// </summary>
    public static QSketch Deserialize(byte[] data)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.qsketch_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize QSketch");

        var sketch = new QSketch(256) { NativePtr = ptr };
        return sketch;
    }

    /// <summary>
    /// Returns a string representation.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "QSketch(disposed)";
        return $"QSketch(maxSamples={_maxSamples}, estimate={Estimate():F0})";
    }

    /// <summary>
    /// Frees the native instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.qsketch_free(NativePtr);
            NativePtr = 0;
        }
    }
}
