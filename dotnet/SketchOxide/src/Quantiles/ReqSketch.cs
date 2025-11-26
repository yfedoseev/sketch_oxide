using System;
using SketchOxide.Native;

namespace SketchOxide.Quantiles;

/// <summary>
/// Relative Error Quantile (REQ) sketch for accurate quantile estimation.
///
/// Provides exact min (p0) and max (p100) values with high accuracy for intermediate
/// quantiles. The k parameter controls the tradeoff between accuracy and space usage.
/// </summary>
/// <remarks>
/// REQ sketch is particularly useful when exact extremes are required along with
/// accurate percentile estimates. Higher k values provide better accuracy but use more memory.
/// </remarks>
public sealed class ReqSketch : NativeSketch
{
    private readonly uint _k;

    /// <summary>
    /// Creates a new REQ sketch with the specified k parameter.
    /// </summary>
    /// <param name="k">Size parameter controlling accuracy. Must be greater than 0.
    /// Higher values give better accuracy but use more space. Typical values: 12-256.</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if k is 0.</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    public ReqSketch(uint k)
    {
        if (k == 0)
            throw new ArgumentOutOfRangeException(nameof(k), k, "K must be greater than 0");

        _k = k;
        NativePtr = SketchOxideNative.reqsketch_new(k);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native ReqSketch");
    }

    /// <summary>
    /// Private constructor for deserialization.
    /// </summary>
    private ReqSketch(uint k, nuint ptr)
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
        SketchOxideNative.reqsketch_update(NativePtr, value);
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

        return SketchOxideNative.reqsketch_query(NativePtr, rank);
    }

    /// <summary>
    /// Gets the exact minimum value (p0).
    /// </summary>
    /// <returns>The exact minimum value.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public double Min() => Query(0);

    /// <summary>
    /// Gets the exact maximum value (p100).
    /// </summary>
    /// <returns>The exact maximum value.</returns>
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
        return SketchOxideNative.reqsketch_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a REQ sketch from a byte array.
    /// </summary>
    /// <param name="data">Serialized sketch bytes.</param>
    /// <param name="k">The k parameter used when creating the original sketch.</param>
    /// <returns>A new ReqSketch instance.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentException">Thrown if data is invalid.</exception>
    public static ReqSketch Deserialize(byte[] data, uint k)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.reqsketch_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize ReqSketch: invalid data");

        return new ReqSketch(k, ptr);
    }

    /// <summary>
    /// Returns a string representation of the sketch.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "ReqSketch(disposed)";
        return $"ReqSketch(k={_k})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.reqsketch_free(NativePtr);
            NativePtr = 0;
        }
    }
}
