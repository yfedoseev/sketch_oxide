package com.sketches_oxide.quantiles;

import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.MergeableSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * DDSketch - Quantile Estimation with Relative Error Guarantees.
 *
 * A quantile sketch that provides relative error guarantees on the quantile values
 * themselves, rather than on ranks. This makes it particularly suitable for metrics
 * where the magnitude of values varies greatly (e.g., latency distributions).
 *
 * <p><strong>Algorithm:</strong>
 * <ul>
 *   <li>Maps values to buckets using a logarithmic mapping</li>
 *   <li>Bucket boundaries grow geometrically</li>
 *   <li>Relative accuracy: for any quantile q, the returned value v satisfies
 *       |v - true_value| &lt;= alpha * true_value</li>
 *   <li>Space: O(log(max/min) / log(1 + alpha)) buckets</li>
 * </ul>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Latency monitoring (p50, p95, p99)</li>
 *   <li>SLO/SLA compliance checking</li>
 *   <li>Performance benchmarking</li>
 *   <li>Any metric where relative error is more meaningful than absolute</li>
 * </ul>
 *
 * <p><strong>Relative vs Absolute Error:</strong>
 * <ul>
 *   <li>Relative: "within 1% of the true value" - good for varying magnitudes</li>
 *   <li>Absolute: "within rank epsilon" - good for uniform distributions</li>
 *   <li>DDSketch is ideal when you care about percentage accuracy</li>
 * </ul>
 *
 * <p><strong>Example:</strong>
 * <pre>
 * // Monitor API latencies with 1% relative accuracy
 * try (DDSketch sketch = new DDSketch(0.01)) {
 *     // Record latencies in milliseconds
 *     for (double latency : latencies) {
 *         sketch.update(latency);
 *     }
 *
 *     // Get percentiles
 *     System.out.println("p50: " + sketch.quantile(0.5) + "ms");
 *     System.out.println("p95: " + sketch.quantile(0.95) + "ms");
 *     System.out.println("p99: " + sketch.quantile(0.99) + "ms");
 *
 *     // Get multiple quantiles efficiently
 *     double[] percentiles = sketch.quantiles(new double[]{0.5, 0.9, 0.95, 0.99});
 * }
 * </pre>
 *
 * @see <a href="https://arxiv.org/abs/1908.10693">DDSketch Paper</a>
 */
public final class DDSketch extends NativeSketch implements MergeableSketch<DDSketch> {

    private final double relativeAccuracy;

    /**
     * Create a new DDSketch.
     *
     * @param relativeAccuracy the relative accuracy parameter alpha (0 &lt; alpha &lt; 1)
     *                         Smaller values give more accurate results but use more space.
     *                         Typical values: 0.01 (1%), 0.001 (0.1%)
     * @throws IllegalArgumentException if relativeAccuracy not in (0, 1)
     * @throws RuntimeException if native memory allocation fails
     */
    public DDSketch(double relativeAccuracy) {
        if (relativeAccuracy <= 0 || relativeAccuracy >= 1) {
            throw new IllegalArgumentException(
                    "relativeAccuracy must be in range (0, 1), got: " + relativeAccuracy);
        }

        this.relativeAccuracy = relativeAccuracy;
        this.nativePtr = SketchOxideNative.ddsketch_new(relativeAccuracy);

        if (this.nativePtr == 0) {
            throw new RuntimeException("Failed to allocate DDSketch");
        }
    }

    /**
     * Private constructor for deserialization.
     */
    private DDSketch(long nativePtr, double relativeAccuracy) {
        this.nativePtr = nativePtr;
        this.relativeAccuracy = relativeAccuracy;
    }

    /**
     * Add a value to the sketch.
     *
     * @param value the value to add (must be positive for logarithmic mapping)
     * @throws IllegalStateException if the sketch has been closed
     */
    public void update(double value) {
        checkAlive();
        SketchOxideNative.ddsketch_update(nativePtr, value);
    }

    /**
     * Get the estimated value at a given quantile.
     *
     * The returned value v satisfies: |v - true_value| &lt;= alpha * true_value,
     * where alpha is the relativeAccuracy parameter.
     *
     * @param q the quantile to query (0 &lt;= q &lt;= 1)
     *          0.5 = median, 0.95 = 95th percentile, etc.
     * @return the estimated value at quantile q
     * @throws IllegalStateException if the sketch has been closed
     * @throws IllegalArgumentException if q not in [0, 1]
     */
    public double quantile(double q) {
        checkAlive();
        if (q < 0 || q > 1) {
            throw new IllegalArgumentException("quantile must be in [0, 1], got: " + q);
        }
        return SketchOxideNative.ddsketch_quantile(nativePtr, q);
    }

    /**
     * Get estimated values at multiple quantiles efficiently.
     *
     * More efficient than calling quantile() multiple times as it
     * processes all queries in a single pass.
     *
     * @param quantiles array of quantiles to query (each in [0, 1])
     * @return array of estimated values, one per input quantile
     * @throws IllegalStateException if the sketch has been closed
     * @throws IllegalArgumentException if any quantile not in [0, 1]
     * @throws NullPointerException if quantiles is null
     */
    public double[] quantiles(double[] quantiles) {
        checkAlive();
        if (quantiles == null) {
            throw new NullPointerException("quantiles cannot be null");
        }

        double[] results = new double[quantiles.length];
        for (int i = 0; i < quantiles.length; i++) {
            if (quantiles[i] < 0 || quantiles[i] > 1) {
                throw new IllegalArgumentException(
                        "quantile must be in [0, 1], got: " + quantiles[i] + " at index " + i);
            }
            results[i] = SketchOxideNative.ddsketch_quantile(nativePtr, quantiles[i]);
        }
        return results;
    }

    /**
     * Merge another DDSketch into this one.
     *
     * After merging, this sketch represents the combined distribution.
     * Both sketches must have the same relativeAccuracy parameter.
     *
     * @param other the sketch to merge into this one
     * @throws IllegalStateException if either sketch is closed
     * @throws IllegalArgumentException if relativeAccuracy doesn't match
     * @throws NullPointerException if other is null
     */
    @Override
    public void merge(DDSketch other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (Math.abs(this.relativeAccuracy - other.relativeAccuracy) > 1e-9) {
            throw new IllegalArgumentException(
                    "Cannot merge DDSketches with different relativeAccuracy: " +
                            this.relativeAccuracy + " vs " + other.relativeAccuracy);
        }

        SketchOxideNative.ddsketch_merge(this.nativePtr, other.nativePtr);
    }

    /**
     * Serialize the sketch to binary format.
     *
     * @return binary representation of the sketch
     * @throws IllegalStateException if the sketch has been closed
     */
    public byte[] serialize() {
        checkAlive();
        return SketchOxideNative.ddsketch_serialize(nativePtr);
    }

    /**
     * Deserialize a DDSketch from binary format.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new DDSketch instance
     * @throws IllegalArgumentException if data is invalid or corrupt
     * @throws NullPointerException if data is null
     */
    public static DDSketch deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.ddsketch_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized DDSketch data");
        }

        return new DDSketch(ptr, 0.01);
    }

    /**
     * Get the relative accuracy parameter.
     *
     * @return the relativeAccuracy used at creation
     */
    public double getRelativeAccuracy() {
        return relativeAccuracy;
    }

    @Override
    public void close() {
        if (nativePtr != 0) {
            freeNative();
            nativePtr = 0;
        }
    }

    @Override
    protected void freeNative() {
        if (nativePtr != 0) {
            SketchOxideNative.ddsketch_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        if (nativePtr != 0) {
            return String.format("DDSketch(relativeAccuracy=%.4f)", relativeAccuracy);
        }
        return "DDSketch(closed)";
    }
}
