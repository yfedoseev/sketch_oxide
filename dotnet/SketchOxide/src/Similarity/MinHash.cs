using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Similarity;

/// <summary>
/// MinHash for estimating Jaccard similarity between sets.
///
/// MinHash uses multiple hash functions (permutations) to create a compact
/// signature that can be used to estimate the Jaccard similarity between
/// two sets without storing the original data.
/// </summary>
/// <remarks>
/// Jaccard similarity J(A,B) = |A intersect B| / |A union B|.
/// MinHash provides an unbiased estimate with standard error approximately 1/sqrt(num_permutations).
/// </remarks>
public sealed class MinHash : NativeSketch
{
    private readonly uint _numPermutations;

    /// <summary>
    /// Creates a new MinHash sketch with the specified number of permutations.
    /// </summary>
    /// <param name="numPermutations">Number of hash permutations. Must be greater than 0.
    /// Higher values give better accuracy but use more space. Typical values: 64-256.</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if numPermutations is 0.</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    public MinHash(uint numPermutations)
    {
        if (numPermutations == 0)
            throw new ArgumentOutOfRangeException(nameof(numPermutations), numPermutations, "Number of permutations must be greater than 0");

        _numPermutations = numPermutations;
        NativePtr = SketchOxideNative.minhash_new(numPermutations);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native MinHash");
    }

    /// <summary>
    /// Private constructor for deserialization.
    /// </summary>
    private MinHash(uint numPermutations, nuint ptr)
    {
        _numPermutations = numPermutations;
        NativePtr = ptr;
    }

    /// <summary>
    /// Gets the number of permutations used by this sketch.
    /// </summary>
    public uint NumPermutations
    {
        get
        {
            CheckAlive();
            return _numPermutations;
        }
    }

    /// <summary>
    /// Adds an element to the set represented by this MinHash.
    /// </summary>
    /// <param name="data">The bytes representing the element to add.</param>
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
                SketchOxideNative.minhash_update(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
            }
        }
    }

    /// <summary>
    /// Adds a string element to the set represented by this MinHash.
    /// </summary>
    /// <param name="value">The string element to add.</param>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public void Update(string value)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        Update(Encoding.UTF8.GetBytes(value));
    }

    /// <summary>
    /// Estimates the Jaccard similarity between this MinHash and another.
    /// </summary>
    /// <param name="other">The other MinHash to compare. Must have the same number of permutations.</param>
    /// <returns>The estimated Jaccard similarity in range [0, 1].</returns>
    /// <exception cref="ArgumentNullException">Thrown if other is null.</exception>
    /// <exception cref="ArgumentException">Thrown if permutation counts don't match.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if either sketch is disposed.</exception>
    public double JaccardSimilarity(MinHash other)
    {
        CheckAlive();
        if (other == null) throw new ArgumentNullException(nameof(other));
        other.CheckAlive();

        if (_numPermutations != other._numPermutations)
            throw new ArgumentException($"Cannot compare MinHash sketches with different permutation counts: {_numPermutations} != {other._numPermutations}");

        return SketchOxideNative.minhash_jaccard_similarity(NativePtr, other.NativePtr);
    }

    /// <summary>
    /// Serializes the sketch to a byte array.
    /// </summary>
    /// <returns>Serialized sketch bytes.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public byte[] Serialize()
    {
        CheckAlive();
        return SketchOxideNative.minhash_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a MinHash from a byte array.
    /// </summary>
    /// <param name="data">Serialized sketch bytes.</param>
    /// <param name="numPermutations">The number of permutations used when creating the original sketch.</param>
    /// <returns>A new MinHash instance.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentException">Thrown if data is invalid.</exception>
    public static MinHash Deserialize(byte[] data, uint numPermutations)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.minhash_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize MinHash: invalid data");

        return new MinHash(numPermutations, ptr);
    }

    /// <summary>
    /// Returns a string representation of the sketch.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "MinHash(disposed)";
        return $"MinHash(numPermutations={_numPermutations})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.minhash_free(NativePtr);
            NativePtr = 0;
        }
    }
}
