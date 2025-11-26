using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Cardinality;

/// <summary>
/// CPC (Compressed Probabilistic Counting) Sketch for cardinality estimation.
///
/// Provides compressed counting with configurable accuracy.
/// </summary>
public sealed class CpcSketch : NativeSketch, IMergeableSketch<CpcSketch>
{
    private readonly uint _lgK;

    /// <summary>
    /// Creates a new CPC sketch.
    /// </summary>
    /// <param name="lgK">Log2 of accuracy parameter in range [4, 26].</param>
    public CpcSketch(uint lgK)
    {
        if (lgK < 4 || lgK > 26)
            throw new ArgumentOutOfRangeException(nameof(lgK), lgK, "lgK must be in range [4, 26]");

        _lgK = lgK;
        NativePtr = SketchOxideNative.cpc_new(lgK);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native CPC sketch");
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
                SketchOxideNative.cpc_update(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
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
        return SketchOxideNative.cpc_estimate(NativePtr);
    }

    /// <summary>
    /// Merges another CPC sketch into this one.
    /// </summary>
    public void Merge(CpcSketch other)
    {
        CheckAlive();
        if (other == null) throw new ArgumentNullException(nameof(other));
        other.CheckAlive();

        if (_lgK != other._lgK)
            throw new ArgumentException($"Cannot merge sketches with different lgK: {_lgK} != {other._lgK}");

        SketchOxideNative.cpc_merge(NativePtr, other.NativePtr);
    }

    /// <summary>
    /// Serializes the sketch.
    /// </summary>
    public byte[] Serialize()
    {
        CheckAlive();
        return SketchOxideNative.cpc_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a CPC sketch.
    /// </summary>
    public static CpcSketch Deserialize(byte[] data)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.cpc_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize CPC sketch");

        var sketch = new CpcSketch(10) { NativePtr = ptr };
        return sketch;
    }

    /// <summary>
    /// Returns a string representation.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "CpcSketch(disposed)";
        return $"CpcSketch(lgK={_lgK}, estimate={Estimate():F0})";
    }

    /// <summary>
    /// Frees the native instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.cpc_free(NativePtr);
            NativePtr = 0;
        }
    }
}
