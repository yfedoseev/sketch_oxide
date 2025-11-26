using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Frequency;

/// <summary>
/// Elastic Sketch for adaptive frequency estimation.
///
/// A two-layer sketch that adapts to the skewness of the data distribution.
/// Heavy hitters are stored in a hash table (heavy part) while the light part
/// uses Count-Min style counters. This provides better accuracy for heavy hitters
/// while maintaining efficiency for the long tail.
/// </summary>
/// <remarks>
/// Note: ElasticSketch does not support merging as the adaptive state cannot be
/// meaningfully combined across multiple instances.
/// </remarks>
public sealed class ElasticSketch : NativeSketch
{
    private readonly uint _bucketCount;
    private readonly uint _depth;

    /// <summary>
    /// Creates a new Elastic Sketch with the specified parameters.
    /// </summary>
    /// <param name="bucketCount">Number of buckets in the heavy part. Must be greater than 0.</param>
    /// <param name="depth">Depth of the light part (number of hash functions). Must be greater than 0.</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if bucketCount or depth is 0.</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    public ElasticSketch(uint bucketCount, uint depth)
    {
        if (bucketCount == 0)
            throw new ArgumentOutOfRangeException(nameof(bucketCount), bucketCount, "Bucket count must be greater than 0");
        if (depth == 0)
            throw new ArgumentOutOfRangeException(nameof(depth), depth, "Depth must be greater than 0");

        _bucketCount = bucketCount;
        _depth = depth;
        NativePtr = SketchOxideNative.elasticsketch_new(bucketCount, depth);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native ElasticSketch");
    }

    /// <summary>
    /// Private constructor for deserialization.
    /// </summary>
    private ElasticSketch(uint bucketCount, uint depth, nuint ptr)
    {
        _bucketCount = bucketCount;
        _depth = depth;
        NativePtr = ptr;
    }

    /// <summary>
    /// Gets the number of buckets in the heavy part.
    /// </summary>
    public uint BucketCount
    {
        get
        {
            CheckAlive();
            return _bucketCount;
        }
    }

    /// <summary>
    /// Gets the depth of the light part.
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
                SketchOxideNative.elasticsketch_update(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
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
                return SketchOxideNative.elasticsketch_estimate(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
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
        return SketchOxideNative.elasticsketch_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes an Elastic Sketch from a byte array.
    /// </summary>
    /// <param name="data">Serialized sketch bytes.</param>
    /// <param name="bucketCount">The bucket_count parameter used when creating the original sketch.</param>
    /// <param name="depth">The depth parameter used when creating the original sketch.</param>
    /// <returns>A new ElasticSketch instance.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentException">Thrown if data is invalid.</exception>
    public static ElasticSketch Deserialize(byte[] data, uint bucketCount, uint depth)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.elasticsketch_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize ElasticSketch: invalid data");

        return new ElasticSketch(bucketCount, depth, ptr);
    }

    /// <summary>
    /// Returns a string representation of the sketch.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "ElasticSketch(disposed)";
        return $"ElasticSketch(bucketCount={_bucketCount}, depth={_depth})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.elasticsketch_free(NativePtr);
            NativePtr = 0;
        }
    }
}
