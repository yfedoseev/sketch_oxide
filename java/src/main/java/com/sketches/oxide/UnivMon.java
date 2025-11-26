package com.sketches.oxide;

/**
 * UnivMon: Universal Monitoring for Multiple Metrics (SIGCOMM 2016)
 *
 * <p>UnivMon is a universal sketch that supports <b>multiple simultaneous metrics</b> from
 * a single data structure. This eliminates the need for separate specialized sketches for
 * different metrics, significantly reducing memory overhead in production monitoring systems.</p>
 *
 * <h2>Key Innovation: Hierarchical Streaming with Adaptive Sampling</h2>
 * <p>UnivMon uses L layers (L = log n) with exponentially decreasing sampling rates:</p>
 * <ul>
 *   <li>Layer 0: Sample rate = 1.0 (all items)</li>
 *   <li>Layer i: Sample rate = 2^(-i) (exponentially fewer items)</li>
 * </ul>
 *
 * <h2>Supported Metrics (from ONE sketch!)</h2>
 * <ol>
 *   <li><b>L1 Norm</b> (sum of frequencies): Traffic volume, total events</li>
 *   <li><b>L2 Norm</b> (sum of squared frequencies): Variability, load balance</li>
 *   <li><b>Entropy</b> (Shannon entropy): Diversity, uniformity</li>
 *   <li><b>Heavy Hitters</b>: Most frequent items, top contributors</li>
 *   <li><b>Change Detection</b>: Temporal anomalies, distribution shifts</li>
 *   <li><b>Flow Size Distribution</b>: Per-flow statistics</li>
 * </ol>
 *
 * <h2>Production Use Cases</h2>
 * <ul>
 *   <li><b>Network Monitoring</b>: Track bandwidth, flows, protocols simultaneously</li>
 *   <li><b>Cloud Analytics</b>: Unified telemetry across multiple dimensions</li>
 *   <li><b>Real-time Anomaly Detection</b>: Detect traffic spikes, DDoS, data skew</li>
 *   <li><b>Multi-tenant Systems</b>: Per-tenant metrics without multiplicative overhead</li>
 *   <li><b>System Performance</b>: CPU, memory, disk I/O from single data structure</li>
 * </ul>
 *
 * <h2>Mathematical Guarantees</h2>
 * <p>For stream size n and error parameters (epsilon, delta):</p>
 * <ul>
 *   <li><b>L1/L2 error</b>: O(epsilon * ||f||_2) with probability 1-delta</li>
 *   <li><b>Entropy error</b>: O(epsilon * H) where H is true entropy</li>
 *   <li><b>Heavy hitters</b>: All items with frequency greater than or equal to epsilon * L1 are found</li>
 *   <li><b>Space</b>: O((log n / epsilon²) * log(1/delta)) - logarithmic in stream size!</li>
 * </ul>
 *
 * <h2>Example Usage</h2>
 * <pre>
 * // Create UnivMon for stream of up to 1 million items
 * try (UnivMon univmon = new UnivMon(1_000_000, 0.01, 0.01)) {
 *     // Update with network packets
 *     univmon.update("192.168.1.1".getBytes(), 1500.0); // IP -&gt; packet size
 *     univmon.update("192.168.1.2".getBytes(), 800.0);
 *     univmon.update("192.168.1.1".getBytes(), 1200.0);
 *
 *     // Query multiple metrics from SAME sketch
 *     double totalTraffic = univmon.estimateL1();    // Total bytes
 *     double loadBalance = univmon.estimateL2();     // Traffic variability
 *     double diversity = univmon.estimateEntropy();  // IP diversity
 *
 *     System.out.println("Total traffic: " + totalTraffic + " bytes");
 *     System.out.println("Load balance: " + loadBalance);
 *     System.out.println("Diversity: " + diversity + " bits");
 *
 *     // Detect changes over time
 *     UnivMon baseline = new UnivMon(1_000_000, 0.01, 0.01);
 *     // ... update baseline with different time window ...
 *     double changeMagnitude = univmon.detectChange(baseline);
 *     System.out.println("Change magnitude: " + changeMagnitude);
 * }
 * </pre>
 *
 * <p><b>Thread Safety:</b> This class is NOT thread-safe. External synchronization
 * is required for concurrent access.</p>
 *
 * @see <a href="https://dl.acm.org/doi/10.1145/2934872.2934906">
 *      One Sketch to Rule Them All: Rethinking Network Flow Monitoring with UnivMon (SIGCOMM 2016)</a>
 */
public class UnivMon implements AutoCloseable {

    static {
        System.loadLibrary("sketch_oxide_jni");
    }

    private long nativeHandle;
    private boolean closed = false;

    /**
     * Creates a new UnivMon sketch.
     *
     * @param maxStreamSize Expected maximum number of items in stream (determines layer count)
     * @param epsilon Error parameter: estimates are within epsilon * metric with probability 1-delta
     * @param delta Failure probability: guarantees hold with probability 1-delta
     * @throws IllegalArgumentException if parameters are invalid
     */
    public UnivMon(long maxStreamSize, double epsilon, double delta) {
        if (maxStreamSize <= 0) {
            throw new IllegalArgumentException("maxStreamSize must be > 0");
        }
        if (epsilon <= 0.0 || epsilon >= 1.0) {
            throw new IllegalArgumentException("Epsilon must be in range (0.0, 1.0)");
        }
        if (delta <= 0.0 || delta >= 1.0) {
            throw new IllegalArgumentException("Delta must be in range (0.0, 1.0)");
        }

        this.nativeHandle = nativeNew(maxStreamSize, epsilon, delta);
        if (this.nativeHandle == 0) {
            throw new RuntimeException("Failed to create UnivMon");
        }
    }

    /**
     * Updates the sketch with an item and its value.
     *
     * <p>Uses hierarchical sampling: each layer samples with rate 2^(-i).
     * This enables efficient multi-metric estimation.</p>
     *
     * @param item The item (as byte array) to update
     * @param value The value/weight for this item (e.g., packet size, count)
     * @throws IllegalStateException if the sketch is closed
     * @throws NullPointerException if item is null
     * @throws IllegalArgumentException if value is negative
     */
    public void update(byte[] item, double value) {
        checkNotClosed();
        if (item == null) {
            throw new NullPointerException("Item cannot be null");
        }
        if (value < 0.0) {
            throw new IllegalArgumentException("Value must be >= 0");
        }
        nativeUpdate(nativeHandle, item, value);
    }

    /**
     * Estimates the L1 norm (sum of all frequencies).
     *
     * <p>The L1 norm represents the total "mass" in the stream:</p>
     * <ul>
     *   <li>Network: Total bytes transferred</li>
     *   <li>E-commerce: Total revenue</li>
     *   <li>Logs: Total event count</li>
     * </ul>
     *
     * <p><b>Accuracy:</b> Error bounded by O(epsilon * L2) with probability 1-delta</p>
     *
     * @return Estimated L1 norm (sum of all values)
     * @throws IllegalStateException if the sketch is closed
     */
    public double estimateL1() {
        checkNotClosed();
        return nativeEstimateL1(nativeHandle);
    }

    /**
     * Estimates the L2 norm (sum of squared frequencies).
     *
     * <p>The L2 norm measures the "spread" or variability in the stream:</p>
     * <ul>
     *   <li>Network: Load balance across flows</li>
     *   <li>Databases: Query distribution uniformity</li>
     *   <li>Systems: Resource usage variance</li>
     * </ul>
     *
     * <p><b>Accuracy:</b> Error bounded by O(epsilon * L2) with probability 1-delta</p>
     *
     * @return Estimated L2 norm (square root of sum of squared values)
     * @throws IllegalStateException if the sketch is closed
     */
    public double estimateL2() {
        checkNotClosed();
        return nativeEstimateL2(nativeHandle);
    }

    /**
     * Estimates Shannon entropy of the stream.
     *
     * <p>Entropy measures the diversity or uniformity of the distribution:</p>
     * <ul>
     *   <li><b>High entropy</b>: Uniform distribution (many items with similar frequencies)</li>
     *   <li><b>Low entropy</b>: Skewed distribution (few dominant items)</li>
     * </ul>
     *
     * <p><b>Formula:</b> H = -Σ(p_i * log2(p_i)) where p_i = f_i / L1</p>
     *
     * <p><b>Applications:</b></p>
     * <ul>
     *   <li>Network: Traffic diversity (detect DDoS, port scans)</li>
     *   <li>Security: Access pattern analysis</li>
     *   <li>Analytics: User behavior diversity</li>
     * </ul>
     *
     * @return Estimated Shannon entropy in bits
     * @throws IllegalStateException if the sketch is closed
     */
    public double estimateEntropy() {
        checkNotClosed();
        return nativeEstimateEntropy(nativeHandle);
    }

    /**
     * Detects changes between two UnivMon sketches.
     *
     * <p>Measures the magnitude of change between two time windows or data sources.
     * Useful for anomaly detection, trend analysis, and monitoring.</p>
     *
     * <p><b>Return value interpretation:</b></p>
     * <ul>
     *   <li>0.0: Identical distributions</li>
     *   <li>Small (less than 1.0): Minor changes</li>
     *   <li>Large (greater than 10.0): Significant distribution shift</li>
     * </ul>
     *
     * <p><b>Applications:</b></p>
     * <ul>
     *   <li>Network: Traffic anomaly detection</li>
     *   <li>Security: Attack detection (DDoS, port scans)</li>
     *   <li>Systems: Performance degradation alerts</li>
     * </ul>
     *
     * @param other Another UnivMon sketch to compare against
     * @return Change magnitude (non-negative). Higher values indicate larger changes.
     * @throws IllegalStateException if either sketch is closed
     * @throws NullPointerException if other is null
     */
    public double detectChange(UnivMon other) {
        checkNotClosed();
        if (other == null) {
            throw new NullPointerException("Other UnivMon cannot be null");
        }
        other.checkNotClosed();
        return nativeDetectChange(nativeHandle, other.nativeHandle);
    }

    /**
     * Returns the number of layers in the sketch.
     *
     * @return Number of hierarchical layers
     * @throws IllegalStateException if the sketch is closed
     */
    public int numLayers() {
        checkNotClosed();
        return nativeNumLayers(nativeHandle);
    }

    /**
     * Returns the total number of updates processed.
     *
     * @return Total update count
     * @throws IllegalStateException if the sketch is closed
     */
    public long totalUpdates() {
        checkNotClosed();
        return nativeTotalUpdates(nativeHandle);
    }

    /**
     * Merges another UnivMon into this one.
     *
     * <p>After merging, this sketch represents the union of both streams.
     * Each layer is merged independently.</p>
     *
     * @param other The UnivMon to merge (must have compatible parameters)
     * @throws IllegalStateException if either sketch is closed
     * @throws NullPointerException if other is null
     * @throws RuntimeException if sketches are incompatible
     */
    public void merge(UnivMon other) {
        checkNotClosed();
        if (other == null) {
            throw new NullPointerException("Other UnivMon cannot be null");
        }
        other.checkNotClosed();
        nativeMerge(nativeHandle, other.nativeHandle);
    }

    /**
     * Closes the sketch and releases native resources.
     *
     * <p>After calling this method, any further operations will throw IllegalStateException.</p>
     */
    @Override
    public void close() {
        if (!closed && nativeHandle != 0) {
            nativeFree(nativeHandle);
            nativeHandle = 0;
            closed = true;
        }
    }

    private void checkNotClosed() {
        if (closed) {
            throw new IllegalStateException("UnivMon has been closed");
        }
    }

    @Override
    protected void finalize() throws Throwable {
        try {
            close();
        } finally {
            super.finalize();
        }
    }

    // Native methods
    private static native long nativeNew(long maxStreamSize, double epsilon, double delta);
    private static native void nativeUpdate(long handle, byte[] item, double value);
    private static native double nativeEstimateL1(long handle);
    private static native double nativeEstimateL2(long handle);
    private static native double nativeEstimateEntropy(long handle);
    private static native double nativeDetectChange(long handle1, long handle2);
    private static native int nativeNumLayers(long handle);
    private static native long nativeTotalUpdates(long handle);
    private static native void nativeMerge(long handle1, long handle2);
    private static native void nativeFree(long handle);
}
