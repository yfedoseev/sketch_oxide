package com.sketches.oxide;

/**
 * NitroSketch: High-Performance Network Telemetry Sketch (SIGCOMM 2019)
 *
 * <p>NitroSketch is a wrapper sketch optimized for high-speed network monitoring
 * in software switches and DPDK environments. It achieves 100Gbps line rate through:</p>
 * <ul>
 *   <li>Selective update sampling (probabilistic load shedding)</li>
 *   <li>Background synchronization for accuracy recovery</li>
 *   <li>Sub-microsecond update latency</li>
 * </ul>
 *
 * <h2>Algorithm Overview</h2>
 * <p>NitroSketch wraps any existing sketch (CountMinSketch in this implementation) and:</p>
 * <ol>
 *   <li>Samples updates probabilistically (e.g., only update 10% of items)</li>
 *   <li>Maintains counts of sampled vs unsampled items</li>
 *   <li>Uses background sync to adjust estimates for unsampled items</li>
 * </ol>
 *
 * <h2>Key Innovation</h2>
 * <p>Traditional sketches update on every packet, creating CPU bottlenecks at high speeds.
 * NitroSketch selectively samples updates while maintaining accuracy through synchronization.</p>
 *
 * <h2>Production Use Cases</h2>
 * <ul>
 *   <li><b>Software-Defined Networking (SDN)</b>: High-speed packet processing</li>
 *   <li><b>Network Traffic Monitoring</b>: Per-flow tracking at 100Gbps+</li>
 *   <li><b>DDoS Detection</b>: Real-time flow analysis with bounded memory</li>
 *   <li><b>Cloud Telemetry</b>: Network analytics in virtualized environments</li>
 *   <li><b>Real-time Analytics</b>: Stream processing with CPU constraints</li>
 * </ul>
 *
 * <h2>Performance Characteristics</h2>
 * <ul>
 *   <li>Update Latency: less than 100ns (sub-microsecond)</li>
 *   <li>Throughput: greater than 100K updates/sec per core</li>
 *   <li>Accuracy: Comparable to base sketch after synchronization</li>
 *   <li>Memory: Same as wrapped sketch</li>
 * </ul>
 *
 * <h2>Example Usage</h2>
 * <pre>
 * // Wrap a CountMinSketch with 10% sampling
 * try (NitroSketch nitro = new NitroSketch(0.01, 0.01, 0.1)) {
 *     // Update with automatic sampling
 *     for (int i = 0; i &lt; 10000; i++) {
 *         nitro.updateSampled(String.format("flow_%d", i % 100).getBytes());
 *     }
 *
 *     // Synchronize for accurate estimates
 *     nitro.sync(1.0);
 *
 *     // Query flow frequency
 *     long freq = nitro.query("flow_0".getBytes());
 *     System.out.println("Flow frequency: " + freq);
 *
 *     // Get sampling statistics
 *     System.out.println("Sample rate: " + nitro.sampleRate());
 *     System.out.println("Sampled: " + nitro.sampledCount());
 *     System.out.println("Unsampled: " + nitro.unsampledCount());
 * }
 * </pre>
 *
 * <p><b>Thread Safety:</b> This class is NOT thread-safe. External synchronization
 * is required for concurrent access.</p>
 *
 * @see <a href="https://dl.acm.org/doi/10.1145/3341302.3342076">
 *      NitroSketch: Robust and General Sketch-based Monitoring in Software Switches (SIGCOMM 2019)</a>
 */
public class NitroSketch implements AutoCloseable {

    static {
        System.loadLibrary("sketch_oxide_jni");
    }

    private long nativeHandle;
    private boolean closed = false;

    /**
     * Creates a new NitroSketch wrapping a CountMinSketch.
     *
     * @param epsilon Error parameter for the base CountMinSketch
     * @param delta Failure probability for the base CountMinSketch
     * @param sampleRate Probability of updating base sketch (0.0 to 1.0)
     *                   - 1.0 = update every item (no sampling)
     *                   - 0.1 = update 10% of items
     *                   - 0.01 = update 1% of items
     * @throws IllegalArgumentException if parameters are invalid
     */
    public NitroSketch(double epsilon, double delta, double sampleRate) {
        if (epsilon <= 0.0 || epsilon >= 1.0) {
            throw new IllegalArgumentException("Epsilon must be in range (0.0, 1.0)");
        }
        if (delta <= 0.0 || delta >= 1.0) {
            throw new IllegalArgumentException("Delta must be in range (0.0, 1.0)");
        }
        if (sampleRate <= 0.0 || sampleRate > 1.0) {
            throw new IllegalArgumentException("Sample rate must be in range (0.0, 1.0]");
        }

        this.nativeHandle = nativeNew(epsilon, delta, sampleRate);
        if (this.nativeHandle == 0) {
            throw new RuntimeException("Failed to create NitroSketch");
        }
    }

    /**
     * Updates with selective sampling.
     *
     * <p>Uses hash-based sampling to decide whether to update the base sketch.
     * This enables high-speed processing by reducing CPU load.</p>
     *
     * @param key The item to possibly add
     * @throws IllegalStateException if the sketch is closed
     * @throws NullPointerException if key is null
     */
    public void updateSampled(byte[] key) {
        checkNotClosed();
        if (key == null) {
            throw new NullPointerException("Key cannot be null");
        }
        nativeUpdateSampled(nativeHandle, key);
    }

    /**
     * Queries the frequency of a key.
     *
     * <p>Returns the estimated frequency from the base sketch.
     * For accurate results, call sync() periodically to adjust for unsampled items.</p>
     *
     * @param key The item/flow key to query
     * @return Estimated frequency (may be underestimated if sync() not called)
     * @throws IllegalStateException if the sketch is closed
     * @throws NullPointerException if key is null
     */
    public long query(byte[] key) {
        checkNotClosed();
        if (key == null) {
            throw new NullPointerException("Key cannot be null");
        }
        return nativeQuery(nativeHandle, key);
    }

    /**
     * Synchronizes to adjust for unsampled items.
     *
     * <p>Background synchronization adjusts the base sketch to account for items
     * that were not sampled. This recovers accuracy while maintaining high throughput.</p>
     *
     * @param unsampledWeight Weight to apply to unsampled items (typically 1.0)
     *                       - Higher values increase compensation for unsampled items
     *                       - Lower values are more conservative
     * @throws IllegalStateException if the sketch is closed
     */
    public void sync(double unsampledWeight) {
        checkNotClosed();
        nativeSync(nativeHandle, unsampledWeight);
    }

    /**
     * Returns the configured sample rate.
     *
     * @return Sample rate (0.0 to 1.0)
     * @throws IllegalStateException if the sketch is closed
     */
    public double sampleRate() {
        checkNotClosed();
        return nativeSampleRate(nativeHandle);
    }

    /**
     * Returns the count of sampled items.
     *
     * @return Number of items that were sampled (updated in base sketch)
     * @throws IllegalStateException if the sketch is closed
     */
    public long sampledCount() {
        checkNotClosed();
        return nativeSampledCount(nativeHandle);
    }

    /**
     * Returns the count of unsampled items.
     *
     * @return Number of items that were NOT sampled (skipped)
     * @throws IllegalStateException if the sketch is closed
     */
    public long unsampledCount() {
        checkNotClosed();
        return nativeUnsampledCount(nativeHandle);
    }

    /**
     * Resets sampling statistics (keeps base sketch).
     *
     * <p>Useful for starting a new measurement period while retaining the sketch state.</p>
     *
     * @throws IllegalStateException if the sketch is closed
     */
    public void resetStats() {
        checkNotClosed();
        nativeResetStats(nativeHandle);
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
            throw new IllegalStateException("NitroSketch has been closed");
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
    private static native long nativeNew(double epsilon, double delta, double sampleRate);
    private static native void nativeUpdateSampled(long handle, byte[] key);
    private static native long nativeQuery(long handle, byte[] key);
    private static native void nativeSync(long handle, double unsampledWeight);
    private static native double nativeSampleRate(long handle);
    private static native long nativeSampledCount(long handle);
    private static native long nativeUnsampledCount(long handle);
    private static native void nativeResetStats(long handle);
    private static native void nativeFree(long handle);
}
