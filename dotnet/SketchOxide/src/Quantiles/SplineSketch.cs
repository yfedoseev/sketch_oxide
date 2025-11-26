using System;
using SketchOxide.Native;

namespace SketchOxide.Quantiles;

/// <summary>
/// Spline sketch for smooth distribution estimation with spline interpolation.
///
/// Provides quantile estimation using spline-based interpolation between buckets,
/// resulting in smoother distribution estimates compared to histogram-based approaches.
/// </summary>
/// <remarks>
/// Spline sketch is particularly useful when a smooth approximation of the underlying
/// distribution is desired, or when the data exhibits smooth, continuous patterns.
/// The spline interpolation provides more accurate estimates between bucket boundaries.
/// </remarks>
public sealed class SplineSketch : NativeSketch
{
    private readonly uint _maxBuckets;

    /// <summary>
    /// Creates a new Spline sketch with the specified maximum number of buckets.
    /// </summary>
    /// <param name="maxBuckets">Maximum number of buckets. Must be greater than 0.
    /// Higher values give better accuracy but use more space.</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if maxBuckets is 0.</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    public SplineSketch(uint maxBuckets)
    {
        if (maxBuckets == 0)
            throw new ArgumentOutOfRangeException(nameof(maxBuckets), maxBuckets, "MaxBuckets must be greater than 0");

        _maxBuckets = maxBuckets;
        NativePtr = SketchOxideNative.splinesketch_new(maxBuckets);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native SplineSketch");
    }

    /// <summary>
    /// Private constructor for deserialization.
    /// </summary>
    private SplineSketch(uint maxBuckets, nuint ptr)
    {
        _maxBuckets = maxBuckets;
        NativePtr = ptr;
    }

    /// <summary>
    /// Gets the maximum buckets parameter of this sketch.
    /// </summary>
    public uint MaxBuckets
    {
        get
        {
            CheckAlive();
            return _maxBuckets;
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
        SketchOxideNative.splinesketch_update(NativePtr, value);
    }

    /// <summary>
    /// Returns the estimated value at the given rank (quantile).
    /// </summary>
    /// <param name="rank">The rank to query, in range [0, 1].
    /// 0 = minimum, 0.5 = median, 1 = maximum.</param>
    /// <returns>The estimated value at the given rank.</returns>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if rank is outside [0, 1].</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public double Query(double rank)
    {
        CheckAlive();
        if (rank < 0 || rank > 1)
            throw new ArgumentOutOfRangeException(nameof(rank), rank, "Rank must be in range [0, 1]");

        return SketchOxideNative.splinesketch_query(NativePtr, rank);
    }

    /// <summary>
    /// Gets the estimated minimum value (p0).
    /// </summary>
    /// <returns>The estimated minimum value.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public double Min() => Query(0);

    /// <summary>
    /// Gets the estimated maximum value (p100).
    /// </summary>
    /// <returns>The estimated maximum value.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public double Max() => Query(1);

    /// <summary>
    /// Gets the estimated median value (p50).
    /// </summary>
    /// <returns>The estimated median value.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public double Median() => Query(0.5);

    /// <summary>
    /// Serializes the sketch to a byte array.
    /// </summary>
    /// <returns>Serialized sketch bytes.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public byte[] Serialize()
    {
        CheckAlive();
        return SketchOxideNative.splinesketch_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a Spline sketch from a byte array.
    /// </summary>
    /// <param name="data">Serialized sketch bytes.</param>
    /// <param name="maxBuckets">The maxBuckets parameter used when creating the original sketch.</param>
    /// <returns>A new SplineSketch instance.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentException">Thrown if data is invalid.</exception>
    public static SplineSketch Deserialize(byte[] data, uint maxBuckets)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.splinesketch_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize SplineSketch: invalid data");

        return new SplineSketch(maxBuckets, ptr);
    }

    /// <summary>
    /// Returns a string representation of the sketch.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "SplineSketch(disposed)";
        return $"SplineSketch(maxBuckets={_maxBuckets})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.splinesketch_free(NativePtr);
            NativePtr = 0;
        }
    }
}
