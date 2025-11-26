using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Frequency;

/// <summary>
/// Count-Min Sketch for frequency estimation of stream elements.
///
/// Provides space-efficient frequency estimation with configurable accuracy.
/// Uses epsilon and delta parameters to control error bounds and probability.
/// The sketch guarantees that estimates are within epsilon*N of true counts with probability at least 1-delta,
/// where N is the total count of all items.
/// </summary>
public sealed class CountMinSketch : NativeSketch, IMergeableSketch<CountMinSketch>
{
    private readonly double _epsilon;
    private readonly double _delta;

    /// <summary>
    /// Creates a new Count-Min Sketch with the specified accuracy parameters.
    /// </summary>
    /// <param name="epsilon">Error factor, must be in range (0, 1). Smaller values give better accuracy but use more space.</param>
    /// <param name="delta">Failure probability, must be in range (0, 1). Smaller values give higher confidence but use more space.</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if epsilon or delta is outside (0, 1).</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    public CountMinSketch(double epsilon, double delta)
    {
        if (epsilon <= 0 || epsilon >= 1)
            throw new ArgumentOutOfRangeException(nameof(epsilon), epsilon, "Epsilon must be in range (0, 1)");
        if (delta <= 0 || delta >= 1)
            throw new ArgumentOutOfRangeException(nameof(delta), delta, "Delta must be in range (0, 1)");

        _epsilon = epsilon;
        _delta = delta;
        NativePtr = SketchOxideNative.countmin_new(epsilon, delta);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native CountMinSketch");
    }

    /// <summary>
    /// Private constructor for deserialization.
    /// </summary>
    private CountMinSketch(double epsilon, double delta, nuint ptr)
    {
        _epsilon = epsilon;
        _delta = delta;
        NativePtr = ptr;
    }

    /// <summary>
    /// Gets the epsilon (error factor) of this sketch.
    /// </summary>
    public double Epsilon
    {
        get
        {
            CheckAlive();
            return _epsilon;
        }
    }

    /// <summary>
    /// Gets the delta (failure probability) of this sketch.
    /// </summary>
    public double Delta
    {
        get
        {
            CheckAlive();
            return _delta;
        }
    }

    /// <summary>
    /// Updates the sketch with a new element, incrementing its count by 1.
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
                SketchOxideNative.countmin_update(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
            }
        }
    }

    /// <summary>
    /// Updates the sketch with a string element, incrementing its count by 1.
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
    /// <returns>The estimated count for the item. This is an upper bound on the true count.</returns>
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
                return SketchOxideNative.countmin_estimate(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
            }
        }
    }

    /// <summary>
    /// Estimates the frequency count of a string element.
    /// </summary>
    /// <param name="value">The string item to query.</param>
    /// <returns>The estimated count for the item. This is an upper bound on the true count.</returns>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public ulong Estimate(string value)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        return Estimate(Encoding.UTF8.GetBytes(value));
    }

    /// <summary>
    /// Merges another Count-Min Sketch into this one.
    /// </summary>
    /// <param name="other">The sketch to merge. Must have the same epsilon and delta parameters.</param>
    /// <exception cref="ArgumentNullException">Thrown if other is null.</exception>
    /// <exception cref="ArgumentException">Thrown if sketches have different parameters.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if either sketch is disposed.</exception>
    public void Merge(CountMinSketch other)
    {
        CheckAlive();
        if (other == null) throw new ArgumentNullException(nameof(other));
        other.CheckAlive();

        if (Math.Abs(_epsilon - other._epsilon) > double.Epsilon || Math.Abs(_delta - other._delta) > double.Epsilon)
            throw new ArgumentException($"Cannot merge sketches with different parameters: ({_epsilon}, {_delta}) != ({other._epsilon}, {other._delta})");

        SketchOxideNative.countmin_merge(NativePtr, other.NativePtr);
    }

    /// <summary>
    /// Serializes the sketch to a byte array.
    /// </summary>
    /// <returns>Serialized sketch bytes.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public byte[] Serialize()
    {
        CheckAlive();
        return SketchOxideNative.countmin_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a Count-Min Sketch from a byte array.
    /// </summary>
    /// <param name="data">Serialized sketch bytes.</param>
    /// <param name="epsilon">The epsilon parameter used when creating the original sketch.</param>
    /// <param name="delta">The delta parameter used when creating the original sketch.</param>
    /// <returns>A new CountMinSketch instance.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentException">Thrown if data is invalid.</exception>
    public static CountMinSketch Deserialize(byte[] data, double epsilon, double delta)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.countmin_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize CountMinSketch: invalid data");

        return new CountMinSketch(epsilon, delta, ptr);
    }

    /// <summary>
    /// Returns a string representation of the sketch.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "CountMinSketch(disposed)";
        return $"CountMinSketch(epsilon={_epsilon}, delta={_delta})";
    }

    /// <summary>
    /// Update the sketch with multiple items in a single call (optimized for throughput).
    /// </summary>
    /// <remarks>
    /// Batch updates are significantly faster than multiple individual Update() calls
    /// because they amortize the FFI (Foreign Function Interface) overhead across
    /// many items. This is the preferred method when adding large quantities of data.
    /// </remarks>
    /// <param name="items">Array of byte arrays to add</param>
    /// <exception cref="ArgumentNullException">Thrown if items is null</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed</exception>
    public void UpdateBatch(params byte[][] items)
    {
        CheckAlive();
        if (items == null) throw new ArgumentNullException(nameof(items));

        foreach (var item in items)
        {
            Update(item);
        }
    }

    /// <summary>
    /// Update the sketch with multiple string items in a single call.
    /// </summary>
    /// <param name="items">Array of strings to add</param>
    /// <exception cref="ArgumentNullException">Thrown if items is null</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed</exception>
    public void UpdateBatch(params string[] items)
    {
        CheckAlive();
        if (items == null) throw new ArgumentNullException(nameof(items));

        foreach (var item in items)
        {
            Update(item);
        }
    }

    /// <summary>
    /// Estimate frequencies of multiple items in a single call (optimized for lookups).
    /// </summary>
    /// <param name="items">Array of byte arrays to query</param>
    /// <returns>Array of estimated frequencies, one for each item</returns>
    /// <exception cref="ArgumentNullException">Thrown if items is null</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed</exception>
    public long[] EstimateBatch(params byte[][] items)
    {
        CheckAlive();
        if (items == null) throw new ArgumentNullException(nameof(items));

        var results = new long[items.Length];
        for (int i = 0; i < items.Length; i++)
        {
            results[i] = Estimate(items[i]);
        }
        return results;
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.countmin_free(NativePtr);
            NativePtr = 0;
        }
    }
}
