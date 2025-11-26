using System;
using SketchOxide.Native;

namespace SketchOxide.Streaming;

/// <summary>
/// Sliding window counter for approximate counting over time-based windows.
///
/// Provides approximate counting of events within a sliding time window using
/// logarithmic space. The epsilon parameter controls the tradeoff between
/// accuracy and space usage.
/// </summary>
/// <remarks>
/// This sketch is useful for computing approximate counts of recent events,
/// such as "requests in the last hour" or "errors in the last 5 minutes".
/// </remarks>
public sealed class SlidingWindowCounter : NativeSketch
{
    private readonly ulong _windowSize;
    private readonly double _epsilon;

    /// <summary>
    /// Creates a new sliding window counter.
    /// </summary>
    /// <param name="windowSize">The size of the sliding window (in time units).</param>
    /// <param name="epsilon">Error parameter in range (0, 1). Smaller values give better accuracy.
    /// The count estimate will be within (1 +/- epsilon) of the true count.</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if windowSize is 0 or epsilon is outside (0, 1).</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    public SlidingWindowCounter(ulong windowSize, double epsilon)
    {
        if (windowSize == 0)
            throw new ArgumentOutOfRangeException(nameof(windowSize), windowSize, "Window size must be greater than 0");
        if (epsilon <= 0 || epsilon >= 1)
            throw new ArgumentOutOfRangeException(nameof(epsilon), epsilon, "Epsilon must be in range (0, 1)");

        _windowSize = windowSize;
        _epsilon = epsilon;
        NativePtr = SketchOxideNative.slidingwindowcounter_new(windowSize, epsilon);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native SlidingWindowCounter");
    }

    /// <summary>
    /// Private constructor for deserialization.
    /// </summary>
    private SlidingWindowCounter(ulong windowSize, double epsilon, nuint ptr)
    {
        _windowSize = windowSize;
        _epsilon = epsilon;
        NativePtr = ptr;
    }

    /// <summary>
    /// Gets the window size of this counter.
    /// </summary>
    public ulong WindowSize
    {
        get
        {
            CheckAlive();
            return _windowSize;
        }
    }

    /// <summary>
    /// Gets the epsilon (error) parameter of this counter.
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
    /// Increments the counter by 1 at the given timestamp.
    /// </summary>
    /// <param name="timestamp">The timestamp of the event.</param>
    /// <exception cref="ObjectDisposedException">Thrown if the counter is disposed.</exception>
    public void Increment(ulong timestamp)
    {
        CheckAlive();
        SketchOxideNative.slidingwindowcounter_increment(NativePtr, timestamp);
    }

    /// <summary>
    /// Increments the counter by the specified count at the given timestamp.
    /// </summary>
    /// <param name="timestamp">The timestamp of the events.</param>
    /// <param name="count">The number of events to add.</param>
    /// <exception cref="ObjectDisposedException">Thrown if the counter is disposed.</exception>
    public void IncrementBy(ulong timestamp, ulong count)
    {
        CheckAlive();
        SketchOxideNative.slidingwindowcounter_increment_by(NativePtr, timestamp, count);
    }

    /// <summary>
    /// Returns the approximate count of events in the specified time range.
    /// </summary>
    /// <param name="startTime">The start of the time range (inclusive).</param>
    /// <param name="endTime">The end of the time range (inclusive).</param>
    /// <returns>The approximate count of events in the range.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the counter is disposed.</exception>
    public ulong Count(ulong startTime, ulong endTime)
    {
        CheckAlive();
        return SketchOxideNative.slidingwindowcounter_count(NativePtr, startTime, endTime);
    }

    /// <summary>
    /// Returns the count as a StreamingResult with error bounds.
    /// </summary>
    /// <param name="startTime">The start of the time range (inclusive).</param>
    /// <param name="endTime">The end of the time range (inclusive).</param>
    /// <returns>A StreamingResult containing the estimate and bounds.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the counter is disposed.</exception>
    public StreamingResult CountWithBounds(ulong startTime, ulong endTime)
    {
        ulong estimate = Count(startTime, endTime);
        return new StreamingResult(estimate, _epsilon);
    }

    /// <summary>
    /// Serializes the counter to a byte array.
    /// </summary>
    /// <returns>Serialized counter bytes.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the counter is disposed.</exception>
    public byte[] Serialize()
    {
        CheckAlive();
        return SketchOxideNative.slidingwindowcounter_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a sliding window counter from a byte array.
    /// </summary>
    /// <param name="data">Serialized counter bytes.</param>
    /// <param name="windowSize">The window size used when creating the original counter.</param>
    /// <param name="epsilon">The epsilon used when creating the original counter.</param>
    /// <returns>A new SlidingWindowCounter instance.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentException">Thrown if data is invalid.</exception>
    public static SlidingWindowCounter Deserialize(byte[] data, ulong windowSize, double epsilon)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.slidingwindowcounter_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize SlidingWindowCounter: invalid data");

        return new SlidingWindowCounter(windowSize, epsilon, ptr);
    }

    /// <summary>
    /// Returns a string representation of the counter.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "SlidingWindowCounter(disposed)";
        return $"SlidingWindowCounter(windowSize={_windowSize}, epsilon={_epsilon})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.slidingwindowcounter_free(NativePtr);
            NativePtr = 0;
        }
    }
}
