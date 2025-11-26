using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Cardinality;

/// <summary>
/// Theta Sketch for cardinality estimation with set operations support.
///
/// Enables union and intersection operations on sketches.
/// </summary>
public sealed class ThetaSketch : NativeSketch, IMergeableSketch<ThetaSketch>
{
    private readonly uint _lgK;

    /// <summary>
    /// Creates a new Theta sketch.
    /// </summary>
    /// <param name="lgK">Log2 of size parameter.</param>
    public ThetaSketch(uint lgK)
    {
        if (lgK < 4 || lgK > 20)
            throw new ArgumentOutOfRangeException(nameof(lgK), lgK, "lgK must be in range [4, 20]");

        _lgK = lgK;
        NativePtr = SketchOxideNative.theta_new(lgK);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native Theta sketch");
    }

    /// <summary>
    /// Gets the lgK parameter.
    /// </summary>
    public uint LgK
    {
        get
        {
            CheckAlive();
            return _lgK;
        }
    }

    /// <summary>
    /// Updates the sketch with a new element.
    /// </summary>
    public void Update(ReadOnlySpan<byte> data)
    {
        CheckAlive();
        if (data == null) throw new ArgumentNullException(nameof(data));

        unsafe
        {
            fixed (byte* ptr = data)
            {
                SketchOxideNative.theta_update(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
            }
        }
    }

    /// <summary>
    /// Updates the sketch with a string element.
    /// </summary>
    public void Update(string value)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        Update(Encoding.UTF8.GetBytes(value));
    }

    /// <summary>
    /// Estimates the cardinality.
    /// </summary>
    public double Estimate()
    {
        CheckAlive();
        return SketchOxideNative.theta_estimate(NativePtr);
    }

    /// <summary>
    /// Merges (unions) another Theta sketch into this one.
    /// </summary>
    public void Merge(ThetaSketch other)
    {
        CheckAlive();
        if (other == null) throw new ArgumentNullException(nameof(other));
        other.CheckAlive();

        if (_lgK != other._lgK)
            throw new ArgumentException($"Cannot merge sketches with different lgK: {_lgK} != {other._lgK}");

        SketchOxideNative.theta_merge(NativePtr, other.NativePtr);
    }

    /// <summary>
    /// Serializes the sketch.
    /// </summary>
    public byte[] Serialize()
    {
        CheckAlive();
        return SketchOxideNative.theta_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a Theta sketch.
    /// </summary>
    public static ThetaSketch Deserialize(byte[] data)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.theta_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize Theta sketch");

        var sketch = new ThetaSketch(12) { NativePtr = ptr };
        return sketch;
    }

    /// <summary>
    /// Returns a string representation.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "ThetaSketch(disposed)";
        return $"ThetaSketch(lgK={_lgK}, estimate={Estimate():F0})";
    }

    /// <summary>
    /// Frees the native instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.theta_free(NativePtr);
            NativePtr = 0;
        }
    }
}
