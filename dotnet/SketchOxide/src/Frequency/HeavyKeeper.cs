using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Frequency;

/// <summary>
/// HeavyKeeper: High-precision top-k heavy hitter detection with exponential decay.
///
/// Identifies the most frequent items in a data stream using probabilistic counting
/// with exponential decay to protect heavy hitters while removing infrequent items.
/// </summary>
/// <remarks>
/// HeavyKeeper maintains a count array and min-heap to track the top-k most frequent items.
/// Uses exponential decay (typically 1.08) to age counts, naturally separating frequent
/// from infrequent items.
///
/// Time Complexity:
/// - Update: O(d) where d is depth (typically 4-6)
/// - Estimate: O(d) for count estimation
/// - TopK: O(1) to return cached results
/// - Decay: O(d × w) where w is width
///
/// Space: O(d × w × 32 bits + k × 96 bits)
/// </remarks>
public sealed class HeavyKeeper : NativeSketch
{
    private readonly uint _k;
    private readonly double _epsilon;
    private readonly double _delta;

    /// <summary>
    /// Creates a new HeavyKeeper sketch for tracking top-k frequent items.
    /// </summary>
    /// <param name="k">Number of top items to track.</param>
    /// <param name="epsilon">Error bound (typically 0.001 - 0.01).</param>
    /// <param name="delta">Failure probability (typically 0.01 - 0.1).</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if parameters are invalid.</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native allocation fails.</exception>
    public HeavyKeeper(uint k, double epsilon, double delta)
    {
        if (k == 0)
            throw new ArgumentOutOfRangeException(nameof(k), k, "k must be greater than 0");
        if (epsilon <= 0 || epsilon >= 1)
            throw new ArgumentOutOfRangeException(nameof(epsilon), epsilon, "epsilon must be in (0, 1)");
        if (delta <= 0 || delta >= 1)
            throw new ArgumentOutOfRangeException(nameof(delta), delta, "delta must be in (0, 1)");

        _k = k;
        _epsilon = epsilon;
        _delta = delta;
        NativePtr = SketchOxideNative.heavy_keeper_new(k, epsilon, delta);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native HeavyKeeper");
    }

    /// <summary>
    /// Gets the number of top items being tracked.
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
            SketchOxideNative.heavy_keeper_update(NativePtr, ptr, (ulong)data.Length);
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
    /// Estimates the frequency count for a specific item.
    /// </summary>
    /// <param name="data">The item bytes to estimate.</param>
    /// <returns>Estimated frequency count.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public unsafe uint Estimate(ReadOnlySpan<byte> data)
    {
        CheckAlive();
        if (data == null) throw new ArgumentNullException(nameof(data));

        fixed (byte* ptr = data)
        {
            return SketchOxideNative.heavy_keeper_estimate(NativePtr, ptr, (ulong)data.Length);
        }
    }

    /// <summary>
    /// Estimates the frequency count for a string item.
    /// </summary>
    /// <param name="value">The string to estimate.</param>
    /// <returns>Estimated frequency count.</returns>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public uint Estimate(string value)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        return Estimate(Encoding.UTF8.GetBytes(value));
    }

    /// <summary>
    /// Gets the current top-k heavy hitters.
    /// </summary>
    /// <returns>Array of (hash, count) tuples representing the top-k items.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public unsafe (ulong Hash, uint Count)[] TopK()
    {
        CheckAlive();

        // Allocate buffer for top-k results
        int itemSize = 16; // 8 bytes hash + 8 bytes count (padded)
        int bufferSize = (int)_k * itemSize;
        byte[] buffer = new byte[bufferSize];

        fixed (byte* ptr = buffer)
        {
            ulong count = SketchOxideNative.heavy_keeper_top_k(NativePtr, ptr, (ulong)bufferSize);

            var results = new (ulong, uint)[count];
            for (ulong i = 0; i < count; i++)
            {
                int offset = (int)(i * (ulong)itemSize);
                ulong hash = BitConverter.ToUInt64(buffer, offset);
                uint frequency = BitConverter.ToUInt32(buffer, offset + 8);
                results[i] = (hash, frequency);
            }
            return results;
        }
    }

    /// <summary>
    /// Applies exponential decay to age old items.
    /// </summary>
    /// <remarks>
    /// Decay should be called periodically to remove stale items and maintain
    /// accurate top-k tracking over time.
    /// </remarks>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public void Decay()
    {
        CheckAlive();
        SketchOxideNative.heavy_keeper_decay(NativePtr);
    }

    /// <summary>
    /// Update the sketch with multiple items in a single call.
    /// </summary>
    /// <param name="items">Array of byte arrays to add.</param>
    /// <exception cref="ArgumentNullException">Thrown if items is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
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
    /// <param name="items">Array of strings to add.</param>
    /// <exception cref="ArgumentNullException">Thrown if items is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
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
            return "HeavyKeeper(disposed)";
        return $"HeavyKeeper(k={_k}, epsilon={_epsilon}, delta={_delta})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.heavy_keeper_free(NativePtr);
            NativePtr = 0;
        }
    }
}
