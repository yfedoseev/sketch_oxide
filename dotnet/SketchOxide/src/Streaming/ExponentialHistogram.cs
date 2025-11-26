using System;
using SketchOxide.Native;

namespace SketchOxide.Streaming;

/// <summary>
/// Exponential histogram for approximate counting over sliding windows.
///
/// Uses the Datar-Gionis-Indyk-Motwani (DGIM) algorithm to maintain approximate
/// counts over a sliding window using only O(1/epsilon * log^2(N)) space.
/// </summary>
/// <remarks>
/// Exponential histograms are useful for streaming scenarios where you need to
/// track counts of events in a sliding window efficiently. The epsilon parameter
/// controls the accuracy-space tradeoff.
/// </remarks>
public sealed class ExponentialHistogram : NativeSketch
{
    private readonly ulong _windowSize;
    private readonly double _epsilon;

    /// <summary>
    /// Creates a new exponential histogram.
    /// </summary>
    /// <param name="windowSize">The size of the sliding window (in time units).</param>
    /// <param name="epsilon">Error parameter in range (0, 1). Smaller values give better accuracy.
    /// The count estimate will be within (1 +/- epsilon) of the true count.</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if windowSize is 0 or epsilon is outside (0, 1).</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    public ExponentialHistogram(ulong windowSize, double epsilon)
    {
        if (windowSize == 0)
            throw new ArgumentOutOfRangeException(nameof(windowSize), windowSize, "Window size must be greater than 0");
        if (epsilon <= 0 || epsilon >= 1)
            throw new ArgumentOutOfRangeException(nameof(epsilon), epsilon, "Epsilon must be in range (0, 1)");

        _windowSize = windowSize;
        _epsilon = epsilon;
        NativePtr = SketchOxideNative.exponentialhistogram_new(windowSize, epsilon);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native ExponentialHistogram");
    }

    /// <summary>
    /// Private constructor for deserialization.
    /// </summary>
    private ExponentialHistogram(ulong windowSize, double epsilon, nuint ptr)
    {
        _windowSize = windowSize;
        _epsilon = epsilon;
        NativePtr = ptr;
    }

    /// <summary>
    /// Gets the window size of this histogram.
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
    /// Gets the epsilon (error) parameter of this histogram.
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
    /// Inserts an event at the given timestamp.
    /// </summary>
    /// <param name="timestamp">The timestamp of the event.</param>
    /// <exception cref="ObjectDisposedException">Thrown if the histogram is disposed.</exception>
    public void Insert(ulong timestamp)
    {
        CheckAlive();
        SketchOxideNative.exponentialhistogram_insert(NativePtr, timestamp);
    }

    /// <summary>
    /// Returns the approximate count of events in the specified time range.
    /// </summary>
    /// <param name="startTime">The start of the time range (inclusive).</param>
    /// <param name="endTime">The end of the time range (inclusive).</param>
    /// <returns>The approximate count of events in the range.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the histogram is disposed.</exception>
    public ulong Count(ulong startTime, ulong endTime)
    {
        CheckAlive();
        return SketchOxideNative.exponentialhistogram_count(NativePtr, startTime, endTime);
    }

    /// <summary>
    /// Returns the count as a StreamingResult with error bounds.
    /// </summary>
    /// <param name="startTime">The start of the time range (inclusive).</param>
    /// <param name="endTime">The end of the time range (inclusive).</param>
    /// <returns>A StreamingResult containing the estimate and bounds.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the histogram is disposed.</exception>
    public StreamingResult CountWithBounds(ulong startTime, ulong endTime)
    {
        ulong estimate = Count(startTime, endTime);
        return new StreamingResult(estimate, _epsilon);
    }

    /// <summary>
    /// Expires (removes) all events older than the specified current time minus window size.
    /// </summary>
    /// <param name="currentTime">The current timestamp.</param>
    /// <exception cref="ObjectDisposedException">Thrown if the histogram is disposed.</exception>
    public void Expire(ulong currentTime)
    {
        CheckAlive();
        SketchOxideNative.exponentialhistogram_expire(NativePtr, currentTime);
    }

    /// <summary>
    /// Serializes the histogram to a byte array.
    /// </summary>
    /// <returns>Serialized histogram bytes.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the histogram is disposed.</exception>
    public byte[] Serialize()
    {
        CheckAlive();
        return SketchOxideNative.exponentialhistogram_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes an exponential histogram from a byte array.
    /// </summary>
    /// <param name="data">Serialized histogram bytes.</param>
    /// <param name="windowSize">The window size used when creating the original histogram.</param>
    /// <param name="epsilon">The epsilon used when creating the original histogram.</param>
    /// <returns>A new ExponentialHistogram instance.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentException">Thrown if data is invalid.</exception>
    public static ExponentialHistogram Deserialize(byte[] data, ulong windowSize, double epsilon)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.exponentialhistogram_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize ExponentialHistogram: invalid data");

        return new ExponentialHistogram(windowSize, epsilon, ptr);
    }

    /// <summary>
    /// Returns a string representation of the histogram.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "ExponentialHistogram(disposed)";
        return $"ExponentialHistogram(windowSize={_windowSize}, epsilon={_epsilon})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.exponentialhistogram_free(NativePtr);
            NativePtr = 0;
        }
    }
}
