using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Universal;

/// <summary>
/// UnivMon: Universal Monitoring for Multiple Metrics (SIGCOMM 2016).
///
/// UnivMon supports **multiple simultaneous metrics** from a single data structure,
/// eliminating the need for separate specialized sketches for different metrics.
/// </summary>
/// <remarks>
/// Key Innovation: Hierarchical Streaming with Adaptive Sampling
///
/// UnivMon uses L layers (L = log n) with exponentially decreasing sampling rates:
/// - Layer 0: Sample rate = 1.0 (all items)
/// - Layer i: Sample rate = 2^(-i) (exponentially fewer items)
///
/// Supported Metrics (from ONE sketch!):
/// 1. L1 Norm (sum of frequencies): Traffic volume, total events
/// 2. L2 Norm (sum of squared frequencies): Variability, load balance
/// 3. Entropy (Shannon entropy): Diversity, uniformity
/// 4. Heavy Hitters: Most frequent items, top contributors
/// 5. Change Detection: Temporal anomalies, distribution shifts
/// 6. Flow Size Distribution: Per-flow statistics
///
/// Production Use Cases (2025):
/// - Network Monitoring: Track bandwidth, flows, protocols simultaneously
/// - Cloud Analytics: Unified telemetry across multiple dimensions
/// - Real-time Anomaly Detection: Detect traffic spikes, DDoS, data skew
/// - Multi-tenant Systems: Per-tenant metrics without multiplicative overhead
/// - System Performance: CPU, memory, disk I/O from single data structure
///
/// Mathematical Guarantees:
/// For stream size n and error parameters (ε, δ):
/// - L1/L2 error: O(ε * ||f||_2) with probability 1-δ
/// - Entropy error: O(ε * H) where H is true entropy
/// - Heavy hitters: All items with frequency ≥ ε * L1 are found
/// - Space: O((log n / ε²) * log(1/δ)) - logarithmic in stream size!
///
/// References:
/// - Liu, Z., et al. (2016). "One Sketch to Rule Them All: Rethinking Network Flow
///   Monitoring with UnivMon." SIGCOMM.
/// </remarks>
public sealed class UnivMon : NativeSketch
{
    private readonly ulong _maxStreamSize;
    private readonly double _epsilon;
    private readonly double _delta;

    /// <summary>
    /// Creates a new UnivMon sketch for multi-metric monitoring.
    /// </summary>
    /// <param name="maxStreamSize">Expected maximum number of items in stream (determines layer count).</param>
    /// <param name="epsilon">Error parameter: estimates are within ε * metric with probability 1-δ.</param>
    /// <param name="delta">Failure probability: guarantees hold with probability 1-δ.</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if parameters are invalid.</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native allocation fails.</exception>
    public UnivMon(ulong maxStreamSize, double epsilon, double delta)
    {
        if (maxStreamSize == 0)
            throw new ArgumentOutOfRangeException(nameof(maxStreamSize), maxStreamSize, "maxStreamSize must be > 0");
        if (epsilon <= 0 || epsilon >= 1)
            throw new ArgumentOutOfRangeException(nameof(epsilon), epsilon, "epsilon must be in (0, 1)");
        if (delta <= 0 || delta >= 1)
            throw new ArgumentOutOfRangeException(nameof(delta), delta, "delta must be in (0, 1)");

        _maxStreamSize = maxStreamSize;
        _epsilon = epsilon;
        _delta = delta;
        NativePtr = SketchOxideNative.univmon_new(maxStreamSize, epsilon, delta);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native UnivMon");
    }

    /// <summary>
    /// Gets the maximum stream size this sketch was configured for.
    /// </summary>
    public ulong MaxStreamSize
    {
        get
        {
            CheckAlive();
            return _maxStreamSize;
        }
    }

    /// <summary>
    /// Gets the epsilon error parameter.
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
    /// Gets the delta failure probability.
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
    /// Updates the sketch with an item and associated value.
    /// </summary>
    /// <param name="data">The item bytes (e.g., flow key, user ID).</param>
    /// <param name="value">The value associated with this item (e.g., packet size, transaction amount).</param>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    /// <exception cref="InvalidOperationException">Thrown if update fails.</exception>
    public unsafe void Update(ReadOnlySpan<byte> data, double value)
    {
        CheckAlive();
        if (data == null) throw new ArgumentNullException(nameof(data));

        fixed (byte* ptr = data)
        {
            int result = SketchOxideNative.univmon_update(NativePtr, ptr, (ulong)data.Length, value);
            if (result != 0)
                throw new InvalidOperationException("Failed to update UnivMon");
        }
    }

    /// <summary>
    /// Updates the sketch with a string item and value.
    /// </summary>
    /// <param name="key">The string key.</param>
    /// <param name="value">The value associated with this key.</param>
    /// <exception cref="ArgumentNullException">Thrown if key is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public void Update(string key, double value)
    {
        if (key == null) throw new ArgumentNullException(nameof(key));
        Update(Encoding.UTF8.GetBytes(key), value);
    }

    /// <summary>
    /// Estimates the L1 norm (sum of all frequencies).
    /// </summary>
    /// <returns>L1 norm estimate (total traffic volume, total events, etc.).</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public double EstimateL1()
    {
        CheckAlive();
        return SketchOxideNative.univmon_estimate_l1(NativePtr);
    }

    /// <summary>
    /// Estimates the L2 norm (sum of squared frequencies).
    /// </summary>
    /// <returns>L2 norm estimate (variability, load balance metric).</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public double EstimateL2()
    {
        CheckAlive();
        return SketchOxideNative.univmon_estimate_l2(NativePtr);
    }

    /// <summary>
    /// Estimates the Shannon entropy of the distribution.
    /// </summary>
    /// <returns>Entropy estimate (diversity, uniformity metric).</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public double EstimateEntropy()
    {
        CheckAlive();
        return SketchOxideNative.univmon_estimate_entropy(NativePtr);
    }

    /// <summary>
    /// Detects the magnitude of change between two UnivMon sketches.
    /// </summary>
    /// <param name="other">The other UnivMon sketch to compare against.</param>
    /// <returns>Change magnitude (L2 distance between distributions).</returns>
    /// <exception cref="ArgumentNullException">Thrown if other is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if either sketch is disposed.</exception>
    public double DetectChange(UnivMon other)
    {
        CheckAlive();
        if (other == null) throw new ArgumentNullException(nameof(other));
        other.CheckAlive();

        return SketchOxideNative.univmon_detect_change(NativePtr, other.NativePtr);
    }

    /// <summary>
    /// Gets statistics about the sketch.
    /// </summary>
    /// <returns>A tuple containing (numLayers, totalMemory, samplesProcessed).</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch is disposed.</exception>
    public unsafe (ulong NumLayers, ulong TotalMemory, ulong SamplesProcessed) GetStats()
    {
        CheckAlive();

        ulong numLayers = 0, totalMemory = 0, samplesProcessed = 0;

        SketchOxideNative.univmon_stats(
            NativePtr, &numLayers, &totalMemory, &samplesProcessed);

        return (numLayers, totalMemory, samplesProcessed);
    }

    /// <summary>
    /// Returns a string representation of the sketch.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "UnivMon(disposed)";
        return $"UnivMon(maxStreamSize={_maxStreamSize}, epsilon={_epsilon}, delta={_delta})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.univmon_free(NativePtr);
            NativePtr = 0;
        }
    }
}
