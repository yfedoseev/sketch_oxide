using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Sampling;

/// <summary>
/// Reservoir sampling for maintaining a uniform random sample of k items from a stream.
///
/// Uses Algorithm R (Vitter) to maintain a reservoir of k items such that at any point,
/// each item in the stream has an equal probability of being in the sample.
/// </summary>
/// <remarks>
/// Reservoir sampling is useful when you need a representative sample from a stream
/// of unknown (or very large) size without storing all items. The algorithm guarantees
/// that each item has exactly k/n probability of being in the sample (where n is
/// the total number of items seen).
/// </remarks>
public sealed class ReservoirSampling : NativeSketch
{
    private readonly uint _k;

    /// <summary>
    /// Creates a new reservoir sampler that maintains k items.
    /// </summary>
    /// <param name="k">The size of the reservoir (number of items to sample). Must be greater than 0.</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if k is 0.</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    public ReservoirSampling(uint k)
    {
        if (k == 0)
            throw new ArgumentOutOfRangeException(nameof(k), k, "K must be greater than 0");

        _k = k;
        NativePtr = SketchOxideNative.reservoirsampling_new(k);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native ReservoirSampling");
    }

    /// <summary>
    /// Private constructor for deserialization.
    /// </summary>
    private ReservoirSampling(uint k, nuint ptr)
    {
        _k = k;
        NativePtr = ptr;
    }

    /// <summary>
    /// Gets the reservoir size (k) of this sampler.
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
    /// Updates the reservoir with a new item from the stream.
    /// The item may or may not be added to the reservoir based on the sampling algorithm.
    /// </summary>
    /// <param name="data">The bytes representing the item.</param>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sampler is disposed.</exception>
    public void Update(ReadOnlySpan<byte> data)
    {
        CheckAlive();
        if (data == null) throw new ArgumentNullException(nameof(data));

        unsafe
        {
            fixed (byte* ptr = data)
            {
                SketchOxideNative.reservoirsampling_update(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
            }
        }
    }

    /// <summary>
    /// Updates the reservoir with a string item from the stream.
    /// </summary>
    /// <param name="value">The string item.</param>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sampler is disposed.</exception>
    public void Update(string value)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        Update(Encoding.UTF8.GetBytes(value));
    }

    /// <summary>
    /// Returns the current sample as an array of byte arrays.
    /// </summary>
    /// <returns>The sampled items. May contain fewer than k items if fewer have been seen.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sampler is disposed.</exception>
    public byte[][] Sample()
    {
        CheckAlive();

        byte[] rawData = SketchOxideNative.reservoirsampling_sample(NativePtr, out uint count);

        if (rawData == null || rawData.Length == 0 || count == 0)
            return Array.Empty<byte[]>();

        var result = new byte[count][];

        // Parse the serialized data format: [len(4 bytes), item(len bytes)]...
        int offset = 0;
        for (uint i = 0; i < count && offset < rawData.Length; i++)
        {
            if (offset + 4 > rawData.Length) break;

            // Read item length (4 bytes, little-endian)
            int itemLen = BitConverter.ToInt32(rawData, offset);
            offset += 4;

            if (offset + itemLen > rawData.Length) break;

            // Read item bytes
            result[i] = new byte[itemLen];
            Array.Copy(rawData, offset, result[i], 0, itemLen);
            offset += itemLen;
        }

        return result;
    }

    /// <summary>
    /// Returns the current sample as an array of strings.
    /// </summary>
    /// <returns>The sampled items decoded as UTF-8 strings.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sampler is disposed.</exception>
    public string[] SampleAsStrings()
    {
        byte[][] bytes = Sample();
        var result = new string[bytes.Length];
        for (int i = 0; i < bytes.Length; i++)
        {
            result[i] = Encoding.UTF8.GetString(bytes[i]);
        }
        return result;
    }

    /// <summary>
    /// Gets the number of items currently in the reservoir.
    /// This may be less than k if fewer items have been seen.
    /// </summary>
    public int Count
    {
        get
        {
            CheckAlive();
            return Sample().Length;
        }
    }

    /// <summary>
    /// Serializes the sampler to a byte array.
    /// </summary>
    /// <returns>Serialized sampler bytes.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sampler is disposed.</exception>
    public byte[] Serialize()
    {
        CheckAlive();
        return SketchOxideNative.reservoirsampling_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a reservoir sampler from a byte array.
    /// </summary>
    /// <param name="data">Serialized sampler bytes.</param>
    /// <param name="k">The k parameter used when creating the original sampler.</param>
    /// <returns>A new ReservoirSampling instance.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentException">Thrown if data is invalid.</exception>
    public static ReservoirSampling Deserialize(byte[] data, uint k)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.reservoirsampling_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize ReservoirSampling: invalid data");

        return new ReservoirSampling(k, ptr);
    }

    /// <summary>
    /// Returns a string representation of the sampler.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "ReservoirSampling(disposed)";
        return $"ReservoirSampling(k={_k}, count={Count})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.reservoirsampling_free(NativePtr);
            NativePtr = 0;
        }
    }
}
