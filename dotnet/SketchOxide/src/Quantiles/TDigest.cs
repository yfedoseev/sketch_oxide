using System;
using SketchOxide.Native;

namespace SketchOxide.Quantiles;

/// <summary>
/// T-Digest quantile estimator for accurate percentile estimation.
///
/// Provides efficient and accurate estimation of quantiles and CDFs (cumulative distribution functions).
/// T-Digest is widely used in production systems including Netflix, Apache Druid, and Elasticsearch.
/// </summary>
/// <remarks>
/// T-Digest uses adaptive clustering to provide very accurate estimates at the tails of the distribution
/// (near p0 and p100) while using less precision in the middle. This makes it ideal for latency
/// percentiles where tail accuracy is critical.
/// </remarks>
public sealed class TDigest : NativeSketch
{
    private readonly double _compression;

    /// <summary>
    /// Creates a new T-Digest with the specified compression factor.
    /// </summary>
    /// <param name="compression">Compression factor controlling accuracy and space usage.
    /// Typical values range from 50 to 500. Higher values give better accuracy but use more space.</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if compression is not positive.</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    public TDigest(double compression)
    {
        if (compression <= 0)
            throw new ArgumentOutOfRangeException(nameof(compression), compression, "Compression must be positive");

        _compression = compression;
        NativePtr = SketchOxideNative.tdigest_new(compression);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native TDigest");
    }

    /// <summary>
    /// Private constructor for deserialization.
    /// </summary>
    private TDigest(double compression, nuint ptr)
    {
        _compression = compression;
        NativePtr = ptr;
    }

    /// <summary>
    /// Gets the compression parameter of this sketch.
    /// </summary>
    public double Compression
    {
        get
        {
            CheckAlive();
            return _compression;
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
        SketchOxideNative.tdigest_update(NativePtr, value);
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

        return SketchOxideNative.tdigest_quantile(NativePtr, q);
    }

    /// <summary>
    /// Returns the estimated CDF (cumulative distribution function) value at the given point.
    /// This gives the fraction of values less than or equal to the given value.
    /// </summary>
    /// <param name="value">The value to compute the CDF at.</param>
    /// <returns>The estimated fraction of values &lt;= value, in range [0, 1].</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public double CDF(double value)
    {
        CheckAlive();
        return SketchOxideNative.tdigest_cdf(NativePtr, value);
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
        return SketchOxideNative.tdigest_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a T-Digest from a byte array.
    /// </summary>
    /// <param name="data">Serialized sketch bytes.</param>
    /// <param name="compression">The compression parameter used when creating the original sketch.</param>
    /// <returns>A new TDigest instance.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentException">Thrown if data is invalid.</exception>
    public static TDigest Deserialize(byte[] data, double compression)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.tdigest_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize TDigest: invalid data");

        return new TDigest(compression, ptr);
    }

    /// <summary>
    /// Returns a string representation of the sketch.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "TDigest(disposed)";
        return $"TDigest(compression={_compression})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.tdigest_free(NativePtr);
            NativePtr = 0;
        }
    }
}
