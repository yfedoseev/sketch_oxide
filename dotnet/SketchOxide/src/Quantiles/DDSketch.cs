using System;
using SketchOxide.Native;

namespace SketchOxide.Quantiles;

/// <summary>
/// DDSketch quantile estimator with relative error guarantees.
///
/// Provides percentile estimation where the error is relative to the actual value,
/// making it particularly suitable for latency measurements and other metrics
/// where relative accuracy is more important than absolute accuracy.
/// </summary>
/// <remarks>
/// DDSketch guarantees that for any quantile q, the returned value v satisfies:
/// true_value * (1 - relative_accuracy) &lt;= v &lt;= true_value * (1 + relative_accuracy)
/// </remarks>
public sealed class DDSketch : NativeSketch
{
    private readonly double _relativeAccuracy;

    /// <summary>
    /// Creates a new DDSketch with the specified relative accuracy.
    /// </summary>
    /// <param name="relativeAccuracy">Relative accuracy in range (0, 1).
    /// Smaller values give better accuracy but use more space.
    /// A value of 0.01 means 1% relative error.</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if relativeAccuracy is outside (0, 1).</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    public DDSketch(double relativeAccuracy)
    {
        if (relativeAccuracy <= 0 || relativeAccuracy >= 1)
            throw new ArgumentOutOfRangeException(nameof(relativeAccuracy), relativeAccuracy, "Relative accuracy must be in range (0, 1)");

        _relativeAccuracy = relativeAccuracy;
        NativePtr = SketchOxideNative.ddsketch_new(relativeAccuracy);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native DDSketch");
    }

    /// <summary>
    /// Private constructor for deserialization.
    /// </summary>
    private DDSketch(double relativeAccuracy, nuint ptr)
    {
        _relativeAccuracy = relativeAccuracy;
        NativePtr = ptr;
    }

    /// <summary>
    /// Gets the relative accuracy parameter of this sketch.
    /// </summary>
    public double RelativeAccuracy
    {
        get
        {
            CheckAlive();
            return _relativeAccuracy;
        }
    }

    /// <summary>
    /// Updates the sketch with a new value.
    /// </summary>
    /// <param name="value">The value to add to the sketch.</param>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public void Update(double value)
    {
        CheckAlive();
        SketchOxideNative.ddsketch_update(NativePtr, value);
    }

    /// <summary>
    /// Returns the estimated value at the given quantile.
    /// </summary>
    /// <param name="q">The quantile to query, in range [0, 1].
    /// 0 = minimum, 0.5 = median, 1 = maximum.</param>
    /// <returns>The estimated value at quantile q.</returns>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if q is outside [0, 1].</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public double Quantile(double q)
    {
        CheckAlive();
        if (q < 0 || q > 1)
            throw new ArgumentOutOfRangeException(nameof(q), q, "Quantile must be in range [0, 1]");

        return SketchOxideNative.ddsketch_quantile(NativePtr, q);
    }

    /// <summary>
    /// Gets the estimated minimum value (p0).
    /// </summary>
    /// <returns>The estimated minimum value.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public double Min() => Quantile(0);

    /// <summary>
    /// Gets the estimated maximum value (p100).
    /// </summary>
    /// <returns>The estimated maximum value.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public double Max() => Quantile(1);

    /// <summary>
    /// Gets the estimated median value (p50).
    /// </summary>
    /// <returns>The estimated median value.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public double Median() => Quantile(0.5);

    /// <summary>
    /// Serializes the sketch to a byte array.
    /// </summary>
    /// <returns>Serialized sketch bytes.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public byte[] Serialize()
    {
        CheckAlive();
        return SketchOxideNative.ddsketch_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a DDSketch from a byte array.
    /// </summary>
    /// <param name="data">Serialized sketch bytes.</param>
    /// <param name="relativeAccuracy">The relative accuracy parameter used when creating the original sketch.</param>
    /// <returns>A new DDSketch instance.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentException">Thrown if data is invalid.</exception>
    public static DDSketch Deserialize(byte[] data, double relativeAccuracy)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.ddsketch_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize DDSketch: invalid data");

        return new DDSketch(relativeAccuracy, ptr);
    }

    /// <summary>
    /// Returns a string representation of the sketch.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "DDSketch(disposed)";
        return $"DDSketch(relativeAccuracy={_relativeAccuracy})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.ddsketch_free(NativePtr);
            NativePtr = 0;
        }
    }
}
