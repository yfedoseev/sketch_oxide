using System;
using SketchOxide.Native;

namespace SketchOxide.Quantiles;

/// <summary>
/// KLL (Karnin-Lang-Liberty) quantile sketch for efficient quantile estimation.
///
/// Provides space-efficient quantile estimation with near-optimal accuracy guarantees.
/// KLL is used in production systems including Apache Druid, Apache Spark, and Amazon.
/// </summary>
/// <remarks>
/// The KLL sketch provides a provably optimal tradeoff between space and accuracy for
/// quantile estimation. It is particularly efficient for streaming scenarios where
/// memory is limited.
/// </remarks>
public sealed class KllSketch : NativeSketch
{
    private readonly uint _k;

    /// <summary>
    /// Creates a new KLL sketch with the specified k parameter.
    /// </summary>
    /// <param name="k">Size parameter controlling accuracy. Must be greater than 0.
    /// Higher values give better accuracy but use more space. Typical values: 100-2000.</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if k is 0.</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    public KllSketch(uint k)
    {
        if (k == 0)
            throw new ArgumentOutOfRangeException(nameof(k), k, "K must be greater than 0");

        _k = k;
        NativePtr = SketchOxideNative.kll_new(k);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native KllSketch");
    }

    /// <summary>
    /// Private constructor for deserialization.
    /// </summary>
    private KllSketch(uint k, nuint ptr)
    {
        _k = k;
        NativePtr = ptr;
    }

    /// <summary>
    /// Gets the k parameter of this sketch.
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
    /// Updates the sketch with a new value.
    /// </summary>
    /// <param name="value">The value to add to the sketch.</param>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public void Update(double value)
    {
        CheckAlive();
        SketchOxideNative.kll_update(NativePtr, value);
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

        return SketchOxideNative.kll_query(NativePtr, rank);
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
        return SketchOxideNative.kll_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a KLL sketch from a byte array.
    /// </summary>
    /// <param name="data">Serialized sketch bytes.</param>
    /// <param name="k">The k parameter used when creating the original sketch.</param>
    /// <returns>A new KllSketch instance.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentException">Thrown if data is invalid.</exception>
    public static KllSketch Deserialize(byte[] data, uint k)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.kll_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize KllSketch: invalid data");

        return new KllSketch(k, ptr);
    }

    /// <summary>
    /// Returns a string representation of the sketch.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "KllSketch(disposed)";
        return $"KllSketch(k={_k})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.kll_free(NativePtr);
            NativePtr = 0;
        }
    }
}
