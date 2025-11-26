package com.sketches_oxide.quantiles;

import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.MergeableSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * KllSketch - Apache Ecosystem Standard Quantile Sketch.
 *
 * A highly efficient quantile sketch from the Apache DataSketches library family.
 * KLL (Karnin-Lang-Liberty) provides near-optimal space usage with proven
 * theoretical guarantees and excellent practical performance.
 *
 * <p><strong>Algorithm:</strong>
 * <ul>
 *   <li>Maintains a hierarchy of compactors at different levels</li>
 *   <li>Each level samples with probability 2^(-level)</li>
 *   <li>Items flow from level 0 up as capacity fills</li>
 *   <li>Provides epsilon-approximate quantiles with bounded error</li>
 *   <li>Space: O(k * log(n/k)) where k is the accuracy parameter</li>
 * </ul>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Apache Spark, Flink, Druid integration</li>
 *   <li>Database query optimization</li>
 *   <li>Real-time analytics</li>
 *   <li>Any application using Apache DataSketches ecosystem</li>
 * </ul>
 *
 * <p><strong>Features:</strong>
 * <ul>
 *   <li>Near-optimal space efficiency</li>
 *   <li>Fast updates and queries</li>
 *   <li>Proven error bounds</li>
 *   <li>Rank queries (inverse of quantile)</li>
 *   <li>Compatible with Apache ecosystem</li>
 * </ul>
 *
 * <p><strong>Error Bounds:</strong>
 * The normalized rank error is approximately 1.65 / k, meaning for k=200,
 * the rank error is about 0.8% (so p99 could be anywhere from p98.2 to p99.8).
 *
 * <p><strong>Example:</strong>
 * <pre>
 * try (KllSketch sketch = new KllSketch(200)) {
 *     // Add values
 *     for (double value : values) {
 *         sketch.update(value);
 *     }
 *
 *     // Query quantiles
 *     System.out.println("Median: " + sketch.quantile(0.5));
 *     System.out.println("p95: " + sketch.quantile(0.95));
 *
 *     // Query rank: what fraction of values are below 100?
 *     System.out.println("Rank of 100: " + sketch.rank(100.0));
 *
 *     // Get error bound
 *     System.out.println("Normalized rank error: " + sketch.normalizedRankError());
 * }
 * </pre>
 *
 * @see DDSketch
 * @see TDigest
 * @see <a href="https://datasketches.apache.org/docs/KLL/KLLSketch.html">Apache KLL Sketch</a>
 * @see <a href="https://arxiv.org/abs/1603.05346">KLL Paper</a>
 */
public final class KllSketch extends NativeSketch implements MergeableSketch<KllSketch> {

    private final int k;

    /**
     * Create a new KllSketch.
     *
     * @param k the accuracy parameter, controls space/accuracy tradeoff
     *          Higher values = more accurate but more space
     *          Typical values: 100-800, recommended default is 200
     *          Error is approximately 1.65/k
     * @throws IllegalArgumentException if k &lt;= 0
     * @throws RuntimeException if native memory allocation fails
     */
    public KllSketch(int k) {
        if (k <= 0) {
            throw new IllegalArgumentException("k must be positive, got: " + k);
        }

        this.k = k;
        this.nativePtr = SketchOxideNative.kllsketch_new(k);

        if (this.nativePtr == 0) {
            throw new RuntimeException("Failed to allocate KllSketch");
        }
    }

    /**
     * Private constructor for deserialization.
     */
    private KllSketch(long nativePtr, int k) {
        this.nativePtr = nativePtr;
        this.k = k;
    }

    /**
     * Add a value to the sketch.
     *
     * @param value the value to add
     * @throws IllegalStateException if the sketch has been closed
     */
    public void update(double value) {
        checkAlive();
        SketchOxideNative.kllsketch_update(nativePtr, value);
    }

    /**
     * Get the estimated value at a given quantile.
     *
     * @param q the quantile to query (0 &lt;= q &lt;= 1)
     * @return the estimated value at quantile q
     * @throws IllegalStateException if the sketch has been closed
     * @throws IllegalArgumentException if q not in [0, 1]
     */
    public double quantile(double q) {
        checkAlive();
        if (q < 0 || q > 1) {
            throw new IllegalArgumentException("quantile must be in [0, 1], got: " + q);
        }
        return SketchOxideNative.kllsketch_quantile(nativePtr, q);
    }

    /**
     * Get the estimated rank of a value.
     *
     * Returns the estimated fraction of values less than or equal to the given value.
     * This is the CDF (cumulative distribution function) evaluated at the value.
     *
     * @param value the value to query
     * @return the estimated rank (fraction of values &lt;= value), in range [0, 1]
     * @throws IllegalStateException if the sketch has been closed
     */
    public double rank(double value) {
        checkAlive();
        // Rank is the inverse of quantile - use binary search
        double lo = 0.0, hi = 1.0;
        for (int i = 0; i < 50; i++) {
            double mid = (lo + hi) / 2;
            if (quantile(mid) < value) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        return (lo + hi) / 2;
    }

    /**
     * Get the normalized rank error bound.
     *
     * This is the maximum error in the rank estimate. For example, if this
     * returns 0.01, then a query for p50 could return a value whose true
     * rank is anywhere between 0.49 and 0.51.
     *
     * @return the normalized rank error (approximately 1.65/k)
     * @throws IllegalStateException if the sketch has been closed
     */
    public double normalizedRankError() {
        checkAlive();
        // Standard KLL normalized rank error formula
        return 1.65 / k;
    }

    /**
     * Merge another KllSketch into this one.
     *
     * After merging, this sketch represents the combined distribution.
     * Both sketches should have the same k parameter for optimal results.
     *
     * @param other the sketch to merge into this one
     * @throws IllegalStateException if either sketch is closed
     * @throws IllegalArgumentException if k parameters don't match
     * @throws NullPointerException if other is null
     */
    @Override
    public void merge(KllSketch other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (this.k != other.k) {
            throw new IllegalArgumentException(
                    "Cannot merge KllSketches with different k: " + this.k + " vs " + other.k);
        }

        SketchOxideNative.kllsketch_merge(this.nativePtr, other.nativePtr);
    }

    /**
     * Serialize the sketch to binary format.
     *
     * @return binary representation of the sketch
     * @throws IllegalStateException if the sketch has been closed
     */
    public byte[] serialize() {
        checkAlive();
        return SketchOxideNative.kllsketch_serialize(nativePtr);
    }

    /**
     * Deserialize a KllSketch from binary format.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new KllSketch instance
     * @throws IllegalArgumentException if data is invalid or corrupt
     * @throws NullPointerException if data is null
     */
    public static KllSketch deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.kllsketch_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized KllSketch data");
        }

        return new KllSketch(ptr, 200);
    }

    /**
     * Get the k parameter.
     *
     * @return the k value used at creation
     */
    public int getK() {
        return k;
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
            SketchOxideNative.kllsketch_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        if (nativePtr != 0) {
            return String.format("KllSketch(k=%d, rankError=%.4f)", k, normalizedRankError());
        }
        return "KllSketch(closed)";
    }
}
