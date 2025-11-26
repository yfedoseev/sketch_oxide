using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Streaming;

/// <summary>
/// Sliding HyperLogLog: Time-windowed cardinality estimation.
///
/// Extends classic HyperLogLog with temporal awareness, enabling cardinality estimation
/// over sliding time windows. Essential for real-time analytics, DDoS detection, and
/// streaming applications.
/// </summary>
/// <remarks>
/// Sliding HyperLogLog maintains HyperLogLog registers augmented with timestamp metadata:
/// 1. Update: Hash item, update register with both value and timestamp
/// 2. Window Query: Filter registers by timestamp, estimate cardinality
/// 3. Decay: Remove expired entries to maintain window bounds
///
/// Time Complexity:
/// - Update: O(1)
/// - Window Query: O(m) where m = 2^precision
/// - Decay: O(m)
/// - Merge: O(m)
///
/// Space Complexity:
/// O(m) where m = 2^precision. Each register stores:
/// - Leading zero count: 1 byte
/// - Timestamp: 8 bytes
/// Total: ~9m bytes (e.g., 36KB for precision 12)
///
/// Production Use Cases (2025):
/// - Real-time Dashboards: Unique users in last N minutes
/// - DDoS Detection: Unique source IPs in sliding window
/// - Network Telemetry: Unique flows over time
/// - CDN Analytics: Geographic distribution over time
/// - Streaming Aggregation: Time-windowed distinct counts
/// </remarks>
public sealed class SlidingHyperLogLog : NativeSketch
{
    private readonly byte _precision;
    private readonly ulong _maxWindowSeconds;

    /// <summary>
    /// Creates a new SlidingHyperLogLog for time-windowed cardinality estimation.
    /// </summary>
    /// <param name="precision">Precision parameter (4-16). Determines accuracy and space.</param>
    /// <param name="maxWindowSeconds">Maximum window size in seconds.</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if parameters are invalid.</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native allocation fails.</exception>
    public SlidingHyperLogLog(byte precision, ulong maxWindowSeconds)
    {
        if (precision < 4 || precision > 16)
            throw new ArgumentOutOfRangeException(nameof(precision), precision,
                "Precision must be in range [4, 16]");
        if (maxWindowSeconds == 0)
            throw new ArgumentOutOfRangeException(nameof(maxWindowSeconds), maxWindowSeconds,
                "maxWindowSeconds must be greater than 0");

        _precision = precision;
        _maxWindowSeconds = maxWindowSeconds;
        NativePtr = SketchOxideNative.sliding_hll_new(precision, maxWindowSeconds);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native SlidingHyperLogLog");
    }

    /// <summary>
    /// Gets the precision of this sketch.
    /// </summary>
    public byte Precision
    {
        get
        {
            CheckAlive();
            return _precision;
        }
    }

    /// <summary>
    /// Gets the maximum window size in seconds.
    /// </summary>
    public ulong MaxWindowSeconds
    {
        get
        {
            CheckAlive();
            return _maxWindowSeconds;
        }
    }

    /// <summary>
    /// Updates the sketch with a new element at a given timestamp.
    /// </summary>
    /// <param name="data">The bytes to add to the sketch.</param>
    /// <param name="timestamp">Unix timestamp in seconds.</param>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="InvalidOperationException">Thrown if update fails.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public unsafe void Update(ReadOnlySpan<byte> data, ulong timestamp)
    {
        CheckAlive();
        if (data == null) throw new ArgumentNullException(nameof(data));

        fixed (byte* ptr = data)
        {
            int result = SketchOxideNative.sliding_hll_update(
                NativePtr, ptr, (ulong)data.Length, timestamp);
            if (result != 0)
                throw new InvalidOperationException("Failed to update sketch");
        }
    }

    /// <summary>
    /// Updates the sketch with a string element at a given timestamp.
    /// </summary>
    /// <param name="value">The string to add to the sketch.</param>
    /// <param name="timestamp">Unix timestamp in seconds.</param>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="InvalidOperationException">Thrown if update fails.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public void Update(string value, ulong timestamp)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        Update(Encoding.UTF8.GetBytes(value), timestamp);
    }

    /// <summary>
    /// Estimates cardinality within a sliding time window.
    /// </summary>
    /// <param name="currentTime">Current timestamp in seconds.</param>
    /// <param name="windowSeconds">Window size in seconds (must be <= maxWindowSeconds).</param>
    /// <returns>Estimated number of unique elements in the window.</returns>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if windowSeconds exceeds max.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public double EstimateWindow(ulong currentTime, ulong windowSeconds)
    {
        CheckAlive();
        if (windowSeconds > _maxWindowSeconds)
            throw new ArgumentOutOfRangeException(nameof(windowSeconds), windowSeconds,
                $"windowSeconds cannot exceed maxWindowSeconds ({_maxWindowSeconds})");

        return SketchOxideNative.sliding_hll_estimate_window(NativePtr, currentTime, windowSeconds);
    }

    /// <summary>
    /// Estimates total cardinality across all time.
    /// </summary>
    /// <returns>Estimated number of unique elements ever seen.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public double EstimateTotal()
    {
        CheckAlive();
        return SketchOxideNative.sliding_hll_estimate_total(NativePtr);
    }

    /// <summary>
    /// Estimates total cardinality as a long value (rounded).
    /// </summary>
    /// <returns>Estimated number of unique elements, rounded to nearest long.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public long EstimateTotalLong()
    {
        return (long)Math.Round(EstimateTotal());
    }

    /// <summary>
    /// Applies decay to remove entries outside the window.
    /// </summary>
    /// <param name="currentTime">Current timestamp in seconds.</param>
    /// <param name="windowSeconds">Window size in seconds.</param>
    /// <exception cref="InvalidOperationException">Thrown if decay fails.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public void Decay(ulong currentTime, ulong windowSeconds)
    {
        CheckAlive();
        if (windowSeconds > _maxWindowSeconds)
            throw new ArgumentOutOfRangeException(nameof(windowSeconds), windowSeconds,
                $"windowSeconds cannot exceed maxWindowSeconds ({_maxWindowSeconds})");

        int result = SketchOxideNative.sliding_hll_decay(NativePtr, currentTime, windowSeconds);
        if (result != 0)
            throw new InvalidOperationException("Failed to decay sketch");
    }

    /// <summary>
    /// Update the sketch with multiple items at the same timestamp.
    /// </summary>
    /// <param name="timestamp">Unix timestamp in seconds.</param>
    /// <param name="items">Array of byte arrays to add.</param>
    /// <exception cref="ArgumentNullException">Thrown if items is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public void UpdateBatch(ulong timestamp, params byte[][] items)
    {
        CheckAlive();
        if (items == null) throw new ArgumentNullException(nameof(items));

        foreach (var item in items)
        {
            Update(item, timestamp);
        }
    }

    /// <summary>
    /// Update the sketch with multiple string items at the same timestamp.
    /// </summary>
    /// <param name="timestamp">Unix timestamp in seconds.</param>
    /// <param name="items">Array of strings to add.</param>
    /// <exception cref="ArgumentNullException">Thrown if items is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public void UpdateBatch(ulong timestamp, params string[] items)
    {
        CheckAlive();
        if (items == null) throw new ArgumentNullException(nameof(items));

        foreach (var item in items)
        {
            Update(item, timestamp);
        }
    }

    /// <summary>
    /// Returns a string representation of the sketch.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "SlidingHyperLogLog(disposed)";
        return $"SlidingHyperLogLog(precision={_precision}, maxWindow={_maxWindowSeconds}s)";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.sliding_hll_free(NativePtr);
            NativePtr = 0;
        }
    }
}
