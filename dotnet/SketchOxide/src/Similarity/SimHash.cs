using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Similarity;

/// <summary>
/// SimHash for computing locality-sensitive hash fingerprints.
///
/// SimHash produces a fixed-size fingerprint (64 bits) that can be used to
/// detect near-duplicate content. Similar documents will have fingerprints
/// with small Hamming distance.
/// </summary>
/// <remarks>
/// SimHash is widely used for duplicate detection, including by Google for
/// web page deduplication. The algorithm is particularly effective for text
/// documents where small changes should result in similar fingerprints.
/// </remarks>
public sealed class SimHash : NativeSketch
{
    /// <summary>
    /// Creates a new SimHash instance.
    /// </summary>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    public SimHash()
    {
        NativePtr = SketchOxideNative.simhash_new();

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native SimHash");
    }

    /// <summary>
    /// Private constructor for deserialization.
    /// </summary>
    private SimHash(nuint ptr)
    {
        NativePtr = ptr;
    }

    /// <summary>
    /// Updates the SimHash with a new token/feature.
    /// </summary>
    /// <param name="data">The bytes representing the token to add.</param>
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
                SketchOxideNative.simhash_update(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
            }
        }
    }

    /// <summary>
    /// Updates the SimHash with a string token/feature.
    /// </summary>
    /// <param name="value">The string token to add.</param>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public void Update(string value)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        Update(Encoding.UTF8.GetBytes(value));
    }

    /// <summary>
    /// Returns the 64-bit SimHash fingerprint of the accumulated features.
    /// </summary>
    /// <returns>The 64-bit fingerprint.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public ulong Fingerprint()
    {
        CheckAlive();
        return SketchOxideNative.simhash_fingerprint(NativePtr);
    }

    /// <summary>
    /// Computes the Hamming distance between this SimHash and another.
    /// </summary>
    /// <param name="other">The other SimHash to compare.</param>
    /// <returns>The Hamming distance (number of differing bits) in range [0, 64].</returns>
    /// <exception cref="ArgumentNullException">Thrown if other is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if either sketch is disposed.</exception>
    public uint HammingDistance(SimHash other)
    {
        CheckAlive();
        if (other == null) throw new ArgumentNullException(nameof(other));
        other.CheckAlive();

        return SketchOxideNative.simhash_hamming_distance(Fingerprint(), other.Fingerprint());
    }

    /// <summary>
    /// Computes the similarity between this SimHash and another.
    /// </summary>
    /// <param name="other">The other SimHash to compare.</param>
    /// <returns>The similarity in range [0, 1], where 1 is identical.</returns>
    /// <exception cref="ArgumentNullException">Thrown if other is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if either sketch is disposed.</exception>
    public double Similarity(SimHash other)
    {
        CheckAlive();
        if (other == null) throw new ArgumentNullException(nameof(other));
        other.CheckAlive();

        return SketchOxideNative.simhash_similarity(Fingerprint(), other.Fingerprint());
    }

    /// <summary>
    /// Computes the Hamming distance between two fingerprints.
    /// </summary>
    /// <param name="fp1">The first fingerprint.</param>
    /// <param name="fp2">The second fingerprint.</param>
    /// <returns>The Hamming distance (number of differing bits) in range [0, 64].</returns>
    public static uint HammingDistance(ulong fp1, ulong fp2)
    {
        return SketchOxideNative.simhash_hamming_distance(fp1, fp2);
    }

    /// <summary>
    /// Computes the similarity between two fingerprints.
    /// </summary>
    /// <param name="fp1">The first fingerprint.</param>
    /// <param name="fp2">The second fingerprint.</param>
    /// <returns>The similarity in range [0, 1], where 1 is identical.</returns>
    public static double Similarity(ulong fp1, ulong fp2)
    {
        return SketchOxideNative.simhash_similarity(fp1, fp2);
    }

    /// <summary>
    /// Serializes the sketch to a byte array.
    /// </summary>
    /// <returns>Serialized sketch bytes.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public byte[] Serialize()
    {
        CheckAlive();
        return SketchOxideNative.simhash_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a SimHash from a byte array.
    /// </summary>
    /// <param name="data">Serialized sketch bytes.</param>
    /// <returns>A new SimHash instance.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentException">Thrown if data is invalid.</exception>
    public static SimHash Deserialize(byte[] data)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.simhash_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize SimHash: invalid data");

        return new SimHash(ptr);
    }

    /// <summary>
    /// Returns a string representation of the sketch.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "SimHash(disposed)";
        return $"SimHash(fingerprint=0x{Fingerprint():X16})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.simhash_free(NativePtr);
            NativePtr = 0;
        }
    }
}
