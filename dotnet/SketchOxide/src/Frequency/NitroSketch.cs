using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Frequency;

/// <summary>
/// NitroSketch: High-Performance Network Telemetry Sketch (SIGCOMM 2019).
///
/// NitroSketch achieves 100Gbps line rate through selective update sampling
/// (probabilistic load shedding) and background synchronization for accuracy recovery.
/// Sub-microsecond update latency.
/// </summary>
/// <remarks>
/// Algorithm Overview:
/// NitroSketch wraps any existing sketch (CountMinSketch, HyperLogLog, etc.) and:
/// 1. Samples updates probabilistically (e.g., only update 10% of items)
/// 2. Maintains counts of sampled vs unsampled items
/// 3. Uses background sync to adjust estimates for unsampled items
///
/// Key Innovation:
/// Traditional sketches update on every packet, creating CPU bottlenecks at high speeds.
/// NitroSketch selectively samples updates while maintaining accuracy through synchronization.
///
/// Production Use Cases (2025):
/// - Software-Defined Networking (SDN): High-speed packet processing
/// - Network Traffic Monitoring: Per-flow tracking at 100Gbps+
/// - DDoS Detection: Real-time flow analysis with bounded memory
/// - Cloud Telemetry: Network analytics in virtualized environments
/// - Real-time Analytics: Stream processing with CPU constraints
///
/// Performance Characteristics:
/// - Update Latency: Less than 100ns (sub-microsecond)
/// - Throughput: Greater than 100K updates/sec per core
/// - Accuracy: Comparable to base sketch after synchronization
/// - Memory: Same as wrapped sketch
///
/// References:
/// - Liu, Z., et al. "NitroSketch: Robust and General Sketch-based Monitoring in
///   Software Switches" (SIGCOMM 2019)
/// </remarks>
public sealed class NitroSketch : NativeSketch
{
    private readonly double _epsilon;
    private readonly double _delta;
    private readonly double _sampleRate;

    /// <summary>
    /// Creates a new NitroSketch wrapping a CountMinSketch with sampling.
    /// </summary>
    /// <param name="epsilon">Error bound for the base sketch (typically 0.001 - 0.01).</param>
    /// <param name="delta">Failure probability for the base sketch (typically 0.01 - 0.1).</param>
    /// <param name="sampleRate">Sampling rate (0.0 to 1.0): 1.0 = no sampling, 0.1 = sample 10%.</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if parameters are invalid.</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native allocation fails.</exception>
    public NitroSketch(double epsilon, double delta, double sampleRate)
    {
        if (epsilon <= 0 || epsilon >= 1)
            throw new ArgumentOutOfRangeException(nameof(epsilon), epsilon, "epsilon must be in (0, 1)");
        if (delta <= 0 || delta >= 1)
            throw new ArgumentOutOfRangeException(nameof(delta), delta, "delta must be in (0, 1)");
        if (sampleRate <= 0 || sampleRate > 1)
            throw new ArgumentOutOfRangeException(nameof(sampleRate), sampleRate, "sampleRate must be in (0, 1]");

        _epsilon = epsilon;
        _delta = delta;
        _sampleRate = sampleRate;
        NativePtr = SketchOxideNative.nitro_sketch_new(epsilon, delta, sampleRate);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native NitroSketch");
    }

    /// <summary>
    /// Gets the sample rate (probability of updating base sketch).
    /// </summary>
    public double SampleRate
    {
        get
        {
            CheckAlive();
            return _sampleRate;
        }
    }

    /// <summary>
    /// Updates the sketch with a sampled item (automatic probabilistic sampling).
    /// </summary>
    /// <param name="data">The bytes to update.</param>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public unsafe void UpdateSampled(ReadOnlySpan<byte> data)
    {
        CheckAlive();
        if (data == null) throw new ArgumentNullException(nameof(data));

        fixed (byte* ptr = data)
        {
            SketchOxideNative.nitro_sketch_update_sampled(NativePtr, ptr, (ulong)data.Length);
        }
    }

    /// <summary>
    /// Updates the sketch with a string item (automatic sampling).
    /// </summary>
    /// <param name="value">The string to update.</param>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public void UpdateSampled(string value)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        UpdateSampled(Encoding.UTF8.GetBytes(value));
    }

    /// <summary>
    /// Queries the frequency estimate for an item.
    /// </summary>
    /// <param name="data">The item bytes to query.</param>
    /// <returns>Estimated frequency count.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public unsafe uint Query(ReadOnlySpan<byte> data)
    {
        CheckAlive();
        if (data == null) throw new ArgumentNullException(nameof(data));

        fixed (byte* ptr = data)
        {
            return SketchOxideNative.nitro_sketch_query(NativePtr, ptr, (ulong)data.Length);
        }
    }

    /// <summary>
    /// Queries the frequency estimate for a string item.
    /// </summary>
    /// <param name="value">The string to query.</param>
    /// <returns>Estimated frequency count.</returns>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public uint Query(string value)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        return Query(Encoding.UTF8.GetBytes(value));
    }

    /// <summary>
    /// Synchronizes the sketch for accuracy recovery.
    /// Should be called periodically to maintain accurate estimates.
    /// </summary>
    /// <param name="syncRatio">Synchronization ratio (typically 1.0 for full sync).</param>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    /// <exception cref="InvalidOperationException">Thrown if sync fails.</exception>
    public void Sync(double syncRatio = 1.0)
    {
        CheckAlive();
        int result = SketchOxideNative.nitro_sketch_sync(NativePtr, syncRatio);
        if (result != 0)
            throw new InvalidOperationException("Failed to synchronize NitroSketch");
    }

    /// <summary>
    /// Gets statistics about the sketch.
    /// </summary>
    /// <returns>A tuple containing (sampleRate, sampledCount, unsampledCount, totalItemsEstimated).</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public unsafe (double SampleRate, ulong SampledCount, ulong UnsampledCount, ulong TotalItemsEstimated) GetStats()
    {
        CheckAlive();

        double sampleRate = 0.0;
        ulong sampledCount = 0, unsampledCount = 0, totalItemsEstimated = 0;

        SketchOxideNative.nitro_sketch_stats(
            NativePtr, &sampleRate, &sampledCount, &unsampledCount, &totalItemsEstimated);

        return (sampleRate, sampledCount, unsampledCount, totalItemsEstimated);
    }

    /// <summary>
    /// Returns a string representation of the sketch.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "NitroSketch(disposed)";
        return $"NitroSketch(epsilon={_epsilon}, delta={_delta}, sampleRate={_sampleRate:F2})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.nitro_sketch_free(NativePtr);
            NativePtr = 0;
        }
    }
}
