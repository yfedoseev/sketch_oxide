using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Cardinality;

/// <summary>
/// HyperLogLog cardinality estimator for counting unique elements.
///
/// Provides space-efficient cardinality estimation with configurable precision.
/// Uses approximately <c>2^precision</c> bytes of memory.
/// </summary>
public sealed class HyperLogLog : NativeSketch, IMergeableSketch<HyperLogLog>
{
    private readonly uint _precision;

    /// <summary>
    /// Creates a new HyperLogLog sketch with the specified precision.
    /// </summary>
    /// <param name="precision">Precision in range [4, 16]. Determines accuracy and space usage.</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if precision is outside [4, 16].</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    public HyperLogLog(uint precision)
    {
        if (precision < 4 || precision > 16)
            throw new ArgumentOutOfRangeException(nameof(precision), precision, "Precision must be in range [4, 16]");

        _precision = precision;
        NativePtr = SketchOxideNative.hyperloglog_new(precision);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native HyperLogLog");
    }

    /// <summary>
    /// Gets the precision of this sketch.
    /// </summary>
    public uint Precision
    {
        get
        {
            CheckAlive();
            return _precision;
        }
    }

    /// <summary>
    /// Updates the sketch with a new element.
    /// </summary>
    /// <param name="data">The bytes to add to the sketch.</param>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public unsafe void Update(ReadOnlySpan<byte> data)
    {
        CheckAlive();
        if (data == null) throw new ArgumentNullException(nameof(data));

        fixed (byte* ptr = data)
        {
            SketchOxideNative.hyperloglog_update(NativePtr, ptr, (ulong)data.Length);
        }
    }

    /// <summary>
    /// Updates the sketch with a string element.
    /// </summary>
    /// <param name="value">The string to add to the sketch.</param>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public void Update(string value)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        Update(Encoding.UTF8.GetBytes(value));
    }

    /// <summary>
    /// Estimates the cardinality (number of unique elements).
    /// </summary>
    /// <returns>The estimated number of unique elements.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public double Estimate()
    {
        CheckAlive();
        return SketchOxideNative.hyperloglog_estimate(NativePtr);
    }

    /// <summary>
    /// Estimates the cardinality as a long value (rounded).
    /// </summary>
    /// <returns>The estimated number of unique elements, rounded to nearest long.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public long EstimateLong()
    {
        return (long)Math.Round(Estimate());
    }

    /// <summary>
    /// Merges another HyperLogLog into this one.
    /// </summary>
    /// <param name="other">The sketch to merge. Must have the same precision.</param>
    /// <exception cref="ArgumentNullException">Thrown if other is null.</exception>
    /// <exception cref="ArgumentException">Thrown if precisions don't match.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if either sketch is disposed.</exception>
    public void Merge(HyperLogLog other)
    {
        CheckAlive();
        if (other == null) throw new ArgumentNullException(nameof(other));
        other.CheckAlive();

        if (_precision != other._precision)
            throw new ArgumentException($"Cannot merge sketches with different precisions: {_precision} != {other._precision}");

        SketchOxideNative.hyperloglog_merge(NativePtr, other.NativePtr);
    }

    /// <summary>
    /// Serializes the sketch to a byte array.
    /// </summary>
    /// <returns>Serialized sketch bytes.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public byte[] Serialize()
    {
        CheckAlive();
        return SketchOxideNative.hyperloglog_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a HyperLogLog from a byte array.
    /// </summary>
    /// <param name="data">Serialized sketch bytes.</param>
    /// <returns>A new HyperLogLog instance.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentException">Thrown if data is invalid.</exception>
    public static HyperLogLog Deserialize(byte[] data)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.hyperloglog_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize HyperLogLog: invalid data");

        var hll = new HyperLogLog(SketchOxideNative.hyperloglog_precision(ptr))
        {
            NativePtr = ptr
        };
        return hll;
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
    /// Returns a string representation of the sketch.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return $"HyperLogLog(disposed)";
        return $"HyperLogLog(precision={_precision}, estimate={Estimate():F0})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.hyperloglog_free(NativePtr);
            NativePtr = 0;
        }
    }
}
