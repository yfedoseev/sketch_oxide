using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Sampling;

/// <summary>
/// Variance-Optimal (VarOpt) sampling for weighted random sampling from a stream.
///
/// VarOpt sampling maintains a sample of k items while accounting for item weights,
/// providing better variance properties than simple weighted reservoir sampling.
/// </summary>
/// <remarks>
/// VarOpt is useful when items have different importance (weights) and you need
/// a representative sample that respects these weights. The algorithm provides
/// optimal variance for estimating weighted sums from the sample.
/// </remarks>
public sealed class VarOptSampling : NativeSketch
{
    private readonly uint _k;

    /// <summary>
    /// Creates a new VarOpt sampler that maintains k items.
    /// </summary>
    /// <param name="k">The size of the sample (number of items to maintain). Must be greater than 0.</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if k is 0.</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    public VarOptSampling(uint k)
    {
        if (k == 0)
            throw new ArgumentOutOfRangeException(nameof(k), k, "K must be greater than 0");

        _k = k;
        NativePtr = SketchOxideNative.varoptssampling_new(k);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native VarOptSampling");
    }

    /// <summary>
    /// Private constructor for deserialization.
    /// </summary>
    private VarOptSampling(uint k, nuint ptr)
    {
        _k = k;
        NativePtr = ptr;
    }

    /// <summary>
    /// Gets the sample size (k) of this sampler.
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
    /// Updates the sample with a new weighted item from the stream.
    /// </summary>
    /// <param name="data">The bytes representing the item.</param>
    /// <param name="weight">The weight of the item. Must be positive.</param>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if weight is not positive.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sampler is disposed.</exception>
    public void UpdateWeighted(ReadOnlySpan<byte> data, double weight)
    {
        CheckAlive();
        if (data == null) throw new ArgumentNullException(nameof(data));
        if (weight <= 0)
            throw new ArgumentOutOfRangeException(nameof(weight), weight, "Weight must be positive");

        unsafe
        {
            fixed (byte* ptr = data)
            {
                SketchOxideNative.varoptssampling_update_weighted(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length, weight);
            }
        }
    }

    /// <summary>
    /// Updates the sample with a weighted string item from the stream.
    /// </summary>
    /// <param name="value">The string item.</param>
    /// <param name="weight">The weight of the item. Must be positive.</param>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if weight is not positive.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sampler is disposed.</exception>
    public void UpdateWeighted(string value, double weight)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        UpdateWeighted(Encoding.UTF8.GetBytes(value), weight);
    }

    /// <summary>
    /// Returns the total weight of all items that have been processed.
    /// </summary>
    /// <returns>The sum of all item weights.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sampler is disposed.</exception>
    public double TotalWeight()
    {
        CheckAlive();
        return SketchOxideNative.varoptssampling_total_weight(NativePtr);
    }

    /// <summary>
    /// Serializes the sampler to a byte array.
    /// </summary>
    /// <returns>Serialized sampler bytes.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sampler is disposed.</exception>
    public byte[] Serialize()
    {
        CheckAlive();
        return SketchOxideNative.varoptssampling_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a VarOpt sampler from a byte array.
    /// </summary>
    /// <param name="data">Serialized sampler bytes.</param>
    /// <param name="k">The k parameter used when creating the original sampler.</param>
    /// <returns>A new VarOptSampling instance.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentException">Thrown if data is invalid.</exception>
    public static VarOptSampling Deserialize(byte[] data, uint k)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.varoptssampling_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize VarOptSampling: invalid data");

        return new VarOptSampling(k, ptr);
    }

    /// <summary>
    /// Returns a string representation of the sampler.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "VarOptSampling(disposed)";
        return $"VarOptSampling(k={_k}, totalWeight={TotalWeight()})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.varoptssampling_free(NativePtr);
            NativePtr = 0;
        }
    }
}
